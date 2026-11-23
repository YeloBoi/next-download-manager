use crate::{filename, validate_url, Download, DownloadError as Error, Result};
use reqwest::{header, Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, Mutex, Notify, Semaphore},
    task::JoinSet,
    time::Instant,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum EngineEvent {
    Metadata {
        name: String,
        total: Option<u64>,
        connections: usize,
        resumable: bool,
    },
    Progress(u64),
}

#[derive(Clone)]
pub struct NetworkLimits {
    global: Arc<Semaphore>,
    hosts: Arc<Mutex<HashMap<String, Arc<Semaphore>>>>,
    per_host: usize,
    rate: Arc<AtomicU64>,
    next_byte: Arc<Mutex<Instant>>,
    rate_changed: Arc<Notify>,
}

impl NetworkLimits {
    pub fn new(global: usize, per_host: usize, rate: u64) -> Self {
        Self {
            global: Arc::new(Semaphore::new(global.max(1))),
            hosts: Arc::default(),
            per_host: per_host.max(1),
            rate: Arc::new(AtomicU64::new(rate)),
            next_byte: Arc::new(Mutex::new(Instant::now())),
            rate_changed: Arc::new(Notify::new()),
        }
    }
    pub async fn set_rate(&self, bytes_per_second: u64) {
        self.rate.store(bytes_per_second, Ordering::Relaxed);
        *self.next_byte.lock().await = Instant::now();
        self.rate_changed.notify_waiters();
    }
    async fn throttle(&self, bytes: usize, cancel: &CancellationToken) -> Result<()> {
        loop {
            let changed = self.rate_changed.notified();
            let rate = self.rate.load(Ordering::Relaxed);
            if rate == 0 {
                return Ok(());
            }
            let mut next = self.next_byte.lock().await;
            let now = Instant::now();
            let start = (*next).max(now);
            *next = start + Duration::from_secs_f64(bytes as f64 / rate as f64);
            drop(next);
            tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), _ = tokio::time::sleep_until(start) => return Ok(()), _ = changed => {} }
        }
    }
    async fn permits(
        &self,
        url: &str,
        cancel: &CancellationToken,
    ) -> Result<(
        tokio::sync::OwnedSemaphorePermit,
        tokio::sync::OwnedSemaphorePermit,
    )> {
        let host = validate_url(url)?.host_str().unwrap_or_default().to_owned();
        let host_limit = self
            .hosts
            .lock()
            .await
            .entry(host)
            .or_insert_with(|| Arc::new(Semaphore::new(self.per_host)))
            .clone();
        // Acquire the host first so a busy host cannot consume the global budget while waiting.
        tokio::select! {
            _ = cancel.cancelled() => Err(Error::Cancelled),
            permits = async {
                let h = host_limit.acquire_owned().await.map_err(|_| Error::Cancelled)?;
                let g = self.global.clone().acquire_owned().await.map_err(|_| Error::Cancelled)?;
                Ok((g,h))
            } => permits
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Metadata {
    total: Option<u64>,
    validator: Option<String>,
    etag: bool,
    ranged: bool,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    version: u8,
    source: String,
    metadata: Metadata,
    segments: Vec<(u64, Option<u64>)>,
}

#[derive(Clone)]
pub struct Engine {
    client: Client,
    work_root: PathBuf,
    pub limits: NetworkLimits,
}

impl Engine {
    pub async fn preview(&self, source: &str, root: &Path) -> Result<DownloadPreview> {
        let url = validate_url(source)?;
        if source.len() > 16384 {
            return Err(Error::Invalid("The URL exceeds the length limit.".into()));
        }
        let response = self
            .client
            .head(url.clone())
            .header(header::ACCEPT_ENCODING, "identity")
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        let mut preview = DownloadPreview {
            name: filename(&header::HeaderMap::new(), &url),
            total: None,
            resumable: None,
            directory: root.to_path_buf(),
        };
        if let Ok(response) = response {
            if response.status().is_success() {
                check_html(&response)?;
                preview.name = filename(response.headers(), response.url());
                let header_value = |key| response.headers().get(key).and_then(|v| v.to_str().ok());
                if header_value(header::CONTENT_ENCODING)
                    .is_none_or(|v| v.eq_ignore_ascii_case("identity"))
                {
                    preview.total =
                        header_value(header::CONTENT_LENGTH).and_then(|v| v.parse().ok());
                    preview.resumable = header_value(header::ACCEPT_RANGES).map(|v| {
                        v.eq_ignore_ascii_case("bytes")
                            && (header_value(header::ETAG).is_some_and(|tag| tag.starts_with('"'))
                                || header_value(header::LAST_MODIFIED).is_some())
                    });
                }
            }
        }
        preview.directory = crate::suggested_directory(root, &preview.name);
        Ok(preview)
    }
    pub fn work_root(&self) -> &Path {
        &self.work_root
    }
    pub fn new(work_root: PathBuf, limits: NetworkLimits) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .read_timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                let url = attempt.url();
                if attempt.previous().len() >= 10
                    || !matches!(url.scheme(), "http" | "https")
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || (url.scheme() == "http"
                        && attempt.previous().iter().any(|p| p.scheme() == "https"))
                {
                    attempt.error("Unsafe or excessive redirect")
                } else {
                    attempt.follow()
                }
            }))
            .user_agent("NDM/0.1.0-alpha.1")
            .build()
            .map_err(network)?;
        Ok(Self {
            client,
            work_root,
            limits,
        })
    }

    async fn probe(&self, source: &str, cancel: &CancellationToken) -> Result<Metadata> {
        let _permits = self.limits.permits(source, cancel).await?;
        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(Error::Cancelled),
            response = self.client.get(source).header(header::RANGE, "bytes=0-0").header(header::ACCEPT_ENCODING, "identity").send() => response.map_err(network)?
        };
        check_html(&response)?;
        let ranged = response.status() == StatusCode::PARTIAL_CONTENT;
        let empty = response.status() == StatusCode::RANGE_NOT_SATISFIABLE
            && response
                .headers()
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                == Some("bytes */0");
        if !response.status().is_success() && !empty {
            return Err(Error::Http(response.status().as_u16()));
        }
        check_encoding(&response)?;
        let total = if ranged {
            let (start, end, total) = content_range(&response)?;
            if start != 0 || end != 0 {
                return Err(Error::Range);
            }
            Some(total)
        } else if empty {
            Some(0)
        } else {
            response.content_length()
        };
        let strong_etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.starts_with("W/") && v.starts_with('"'));
        let modified = response
            .headers()
            .get(header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok());
        let validator = strong_etag.or(modified).map(str::to_string);
        let metadata = Metadata {
            total,
            validator,
            etag: strong_etag.is_some(),
            ranged,
            name: filename(response.headers(), response.url()),
        };
        if ranged {
            let mut response = response;
            let mut count = 0;
            loop {
                let chunk = tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), c = response.chunk() => c.map_err(network)? };
                match chunk {
                    Some(bytes) => {
                        count += bytes.len();
                        if count > 1 {
                            return Err(Error::Range);
                        }
                    }
                    None => break,
                }
            }
            if count != 1 {
                return Err(Error::Incomplete);
            }
        }
        Ok(metadata)
    }

    pub async fn download(
        &self,
        job: &Download,
        connections: usize,
        cancel: CancellationToken,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<(PathBuf, String)> {
        validate_url(&job.url)?;
        // The caller supplies UUIDs. A second check keeps work paths inside our own root.
        if job.id.is_empty() || !job.id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err(Error::Invalid("Invalid download ID".into()));
        }
        let directory = self.work_root.join(&job.id);
        fs::create_dir_all(&directory).await?;
        let _lock = WorkLock::acquire(directory.join("lock")).await?;
        let metadata = self.probe(&job.url, &cancel).await?;
        let manifest_path = directory.join("manifest.json");
        let previous = match fs::read(&manifest_path).await {
            Ok(bytes) => Some(serde_json::from_slice::<Manifest>(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let resumed = match &previous {
            Some(old) => saved_bytes(&directory, old.segments.len()).await? > 0,
            None => false,
        };
        let previous = if resumed { previous } else { None };
        let manifest = if let Some(old) = previous {
            if old.version != 1
                || old.source != job.url
                || old.metadata.total != metadata.total
                || old.metadata.validator.is_none()
                || old.metadata.validator != metadata.validator
                || old.metadata.etag != metadata.etag
                || !metadata.ranged
                || old.segments.is_empty()
                || old.segments.len() > 8
                || old.segments != split_ranges(old.metadata.total, old.segments.len())
            {
                return Err(Error::Resume(
                    "the source changed or has no reliable byte-range validator".into(),
                ));
            }
            old
        } else {
            let count = if metadata.ranged
                && metadata.validator.is_some()
                && metadata.total.unwrap_or(0) >= 1024 * 1024
            {
                connections.clamp(1, 8)
            } else {
                1
            };
            let segments = split_ranges(metadata.total, count);
            let manifest = Manifest {
                version: 1,
                source: job.url.clone(),
                metadata: metadata.clone(),
                segments,
            };
            atomic_write(&manifest_path, &serde_json::to_vec(&manifest)?).await?;
            manifest
        };
        fs::create_dir_all(&job.directory).await?;
        if let Some(size) = metadata.total {
            let saved = saved_bytes(&directory, manifest.segments.len()).await?;
            let path = job.directory.clone();
            let available = tokio::task::spawn_blocking(move || fs2::available_space(path))
                .await
                .map_err(|e| Error::Invalid(e.to_string()))??;
            // Parts and final assembly coexist until publication is durable.
            if available < size.saturating_sub(saved).saturating_add(size) {
                return Err(Error::Invalid(
                    "Not enough free disk space for the parts and final file.".into(),
                ));
            }
        }
        let _ = events
            .send(EngineEvent::Metadata {
                name: metadata.name.clone(),
                total: metadata.total,
                connections: manifest.segments.len(),
                resumable: metadata.ranged && metadata.validator.is_some(),
            })
            .await;
        let mut output_job = job.clone();
        if output_job.name.is_empty() {
            output_job.name = metadata.name.clone();
        }
        let initial = saved_bytes(&directory, manifest.segments.len()).await?;
        let _ = events.send(EngineEvent::Progress(initial)).await;
        let bytes = Arc::new(AtomicU64::new(initial));
        if metadata.total == Some(0) {
            File::create(directory.join("0.part"))
                .await?
                .sync_all()
                .await?;
            return self.publish(&output_job, &directory, 1, &cancel).await;
        }
        let outcome = self
            .transfer(
                &directory,
                &manifest,
                bytes.clone(),
                cancel.clone(),
                events.clone(),
            )
            .await;
        if matches!(outcome, Err(Error::Range)) && !resumed {
            // Only a fresh job may discard failed range work automatically.
            clear_segments(&directory, manifest.segments.len()).await?;
            let mut single = manifest.clone();
            single.metadata.ranged = false;
            single.segments = vec![(0, metadata.total.map(|n| n.saturating_sub(1)))];
            atomic_write(&manifest_path, &serde_json::to_vec(&single)?).await?;
            bytes.store(0, Ordering::Relaxed);
            let _ = events.send(EngineEvent::Progress(0)).await;
            let _ = events
                .send(EngineEvent::Metadata {
                    name: metadata.name.clone(),
                    total: metadata.total,
                    connections: 1,
                    resumable: false,
                })
                .await;
            self.transfer(&directory, &single, bytes, cancel.clone(), events)
                .await?;
            return self.publish(&output_job, &directory, 1, &cancel).await;
        }
        outcome?;
        self.publish(&output_job, &directory, manifest.segments.len(), &cancel)
            .await
    }

    async fn transfer(
        &self,
        directory: &Path,
        manifest: &Manifest,
        bytes: Arc<AtomicU64>,
        cancel: CancellationToken,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<()> {
        let workers_cancel = cancel.child_token();
        let mut tasks = JoinSet::new();
        for (index, &(start, end)) in manifest.segments.iter().enumerate() {
            let engine = self.clone();
            let dir = directory.to_path_buf();
            let manifest = manifest.clone();
            let bytes = bytes.clone();
            let events = events.clone();
            let token = workers_cancel.clone();
            tasks.spawn(async move {
                engine
                    .segment(&dir, index, start, end, &manifest, bytes, token, events)
                    .await
            });
        }
        let mut failure = None;
        while let Some(result) = tasks.join_next().await {
            let result = result
                .map_err(|e| Error::Invalid(format!("Download worker failed: {e}")))
                .and_then(|r| r);
            if let Err(error) = result {
                if failure.is_none() {
                    failure = Some(error);
                    workers_cancel.cancel();
                }
            }
        }
        if let Some(error) = failure {
            Err(error)
        } else {
            Ok(())
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn segment(
        &self,
        directory: &Path,
        index: usize,
        start: u64,
        end: Option<u64>,
        manifest: &Manifest,
        bytes: Arc<AtomicU64>,
        cancel: CancellationToken,
        events: mpsc::Sender<EngineEvent>,
    ) -> Result<()> {
        let part = directory.join(format!("{index}.part"));
        let checkpoint = directory.join(format!("{index}.checkpoint"));
        let mut committed = read_checkpoint(&checkpoint).await?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&part)
            .await?;
        if file.metadata().await?.len() < committed {
            return Err(Error::Resume(
                "a saved part is shorter than its checkpoint".into(),
            ));
        }
        file.set_len(committed).await?;
        use tokio::io::AsyncSeekExt;
        file.seek(std::io::SeekFrom::Start(committed)).await?;
        let expected = end.map(|end| end.saturating_sub(start) + 1);
        if expected == Some(committed) {
            return Ok(());
        }
        if committed > 0 && (!manifest.metadata.ranged || manifest.metadata.validator.is_none()) {
            return Err(Error::Resume(
                "this server does not support safe resume".into(),
            ));
        }
        let mut last_error = Error::Incomplete;
        for attempt in 0..4_u32 {
            if cancel.is_cancelled() {
                return Err(Error::Cancelled);
            }
            let _permits = self.limits.permits(&manifest.source, &cancel).await?;
            let mut request = self
                .client
                .get(&manifest.source)
                .header(header::ACCEPT_ENCODING, "identity");
            if manifest.metadata.ranged {
                let range = match end {
                    Some(end) => format!("bytes={}-{end}", start + committed),
                    None => format!("bytes={}-", start + committed),
                };
                request = request.header(header::RANGE, range);
                if let Some(validator) = &manifest.metadata.validator {
                    request = request.header(header::IF_RANGE, validator);
                }
            }
            let response = tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), r = request.send() => r };
            let mut retry_after = None;
            let result = match response {
                Ok(mut response) => {
                    let status = response.status();
                    if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                        retry_after = response
                            .headers()
                            .get(header::RETRY_AFTER)
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(|s| Duration::from_secs(s.min(120)));
                        Err(Error::Http(status.as_u16()))
                    } else {
                        if manifest.metadata.ranged {
                            if status != StatusCode::PARTIAL_CONTENT {
                                return Err(if committed > 0 {
                                    Error::Resume("the range request was rejected".into())
                                } else {
                                    Error::Range
                                });
                            }
                            let (actual_start, actual_end, total) = content_range(&response)?;
                            if actual_start != start + committed
                                || Some(actual_end) != end
                                || Some(total) != manifest.metadata.total
                            {
                                return Err(Error::Range);
                            }
                            let key = if manifest.metadata.etag {
                                header::ETAG
                            } else {
                                header::LAST_MODIFIED
                            };
                            if let Some(actual) =
                                response.headers().get(key).and_then(|h| h.to_str().ok())
                            {
                                if manifest.metadata.validator.as_deref() != Some(actual) {
                                    return Err(Error::Resume("the file validator changed".into()));
                                }
                            }
                        } else if status != StatusCode::OK {
                            return Err(Error::Http(status.as_u16()));
                        }
                        check_html(&response)?;
                        check_encoding(&response)?;
                        self.receive(
                            &mut response,
                            &mut file,
                            &checkpoint,
                            &mut committed,
                            expected,
                            &bytes,
                            &cancel,
                            &events,
                        )
                        .await
                    }
                }
                Err(e) => Err(network(e)),
            };
            drop(_permits);
            match result {
                Ok(()) => return Ok(()),
                Err(Error::Network(e)) => last_error = Error::Network(e),
                Err(Error::Incomplete) => last_error = Error::Incomplete,
                Err(Error::Http(code)) if code == 429 || code >= 500 => {
                    last_error = Error::Http(code)
                }
                Err(e) => return Err(e),
            }
            if committed > 0 && (!manifest.metadata.ranged || manifest.metadata.validator.is_none())
            {
                return Err(Error::Resume(
                    "the interrupted server cannot safely resume".into(),
                ));
            }
            if attempt < 3 {
                let jitter = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_millis()
                    % 250;
                let delay = retry_after.unwrap_or(Duration::from_millis(
                    500 * 2_u64.pow(attempt) + u64::from(jitter),
                ));
                tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), _ = tokio::time::sleep(delay) => {} }
            }
        }
        Err(last_error)
    }

    #[allow(clippy::too_many_arguments)]
    async fn receive(
        &self,
        response: &mut Response,
        file: &mut File,
        checkpoint: &Path,
        committed: &mut u64,
        expected: Option<u64>,
        total: &AtomicU64,
        cancel: &CancellationToken,
        events: &mpsc::Sender<EngineEvent>,
    ) -> Result<()> {
        loop {
            let chunk = tokio::select! { _ = cancel.cancelled() => return Err(Error::Cancelled), c = response.chunk() => c.map_err(network)? };
            let Some(chunk) = chunk else { break };
            for chunk in chunk.chunks(64 * 1024) {
                if expected.is_some_and(|size| *committed + chunk.len() as u64 > size) {
                    return Err(Error::Range);
                }
                self.limits.throttle(chunk.len(), cancel).await?;
                file.write_all(chunk).await?;
                file.flush().await?;
                file.sync_data().await?;
                *committed += chunk.len() as u64;
                atomic_write(checkpoint, committed.to_string().as_bytes()).await?;
                let count =
                    total.fetch_add(chunk.len() as u64, Ordering::Relaxed) + chunk.len() as u64;
                // UI backpressure must never block the transfer or cancellation.
                let _ = events.try_send(EngineEvent::Progress(count));
            }
        }
        if expected.is_some_and(|size| size != *committed) {
            return Err(Error::Incomplete);
        }
        Ok(())
    }

    async fn publish(
        &self,
        job: &Download,
        work: &Path,
        count: usize,
        cancel: &CancellationToken,
    ) -> Result<(PathBuf, String)> {
        let name = if job.name.is_empty() {
            "download"
        } else {
            &job.name
        };
        let name = crate::sanitize_filename(name);
        let mut index = 0_u32;
        let (target, temporary, mut output) = loop {
            let candidate = if index == 0 {
                name.clone()
            } else {
                let path = Path::new(&name);
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("download");
                let extension = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| format!(".{s}"))
                    .unwrap_or_default();
                format!("{stem} ({index}){extension}")
            };
            let target = job.directory.join(candidate);
            let temporary = target.with_file_name(format!(
                "{}.part",
                target.file_name().unwrap_or_default().to_string_lossy()
            ));
            if fs::try_exists(&target).await? {
                index += 1;
                continue;
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .await
            {
                Ok(output) => break (target, temporary, output),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    index += 1;
                }
                Err(e) => return Err(e.into()),
            }
        };
        let result = async {
            let mut hash = Sha256::new();
            let mut buffer = vec![0; 64 * 1024];
            for index in 0..count {
                let mut file = File::open(work.join(format!("{index}.part"))).await?;
                loop {
                    if cancel.is_cancelled() {
                        return Err(Error::Cancelled);
                    }
                    let read = file.read(&mut buffer).await?;
                    if read == 0 {
                        break;
                    }
                    hash.update(&buffer[..read]);
                    output.write_all(&buffer[..read]).await?;
                }
            }
            output.flush().await?;
            output.sync_all().await?;
            let digest = format!("{:x}", hash.finalize());
            if job
                .expected_sha256
                .as_ref()
                .is_some_and(|expected| !digest.eq_ignore_ascii_case(expected))
            {
                return Err(Error::HashMismatch);
            }
            // A hard link publishes without replacing a racing writer's destination.
            fs::hard_link(&temporary, &target).await?;
            Ok((target.clone(), digest))
        }
        .await;
        drop(output);
        let _ = fs::remove_file(&temporary).await;
        result
    }

    pub async fn discard(&self, id: &str) -> Result<()> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return Err(Error::Invalid("Invalid download ID".into()));
        }
        let directory = self.work_root.join(id);
        match fs::remove_dir_all(directory).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DownloadPreview {
    pub name: String,
    pub total: Option<u64>,
    pub resumable: Option<bool>,
    pub directory: PathBuf,
}

fn network(error: reqwest::Error) -> Error {
    Error::Network(error.without_url().to_string())
}
fn check_html(response: &Response) -> Result<()> {
    let mime = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    if mime.starts_with("text/html") || mime.starts_with("application/xhtml+xml") {
        Err(Error::Html)
    } else {
        Ok(())
    }
}
fn check_encoding(response: &Response) -> Result<()> {
    match response
        .headers()
        .get(header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
    {
        None | Some("identity") => Ok(()),
        _ => Err(Error::Invalid(
            "The server ignored identity encoding; byte offsets cannot be trusted.".into(),
        )),
    }
}
fn content_range(response: &Response) -> Result<(u64, u64, u64)> {
    let value = response
        .headers()
        .get(header::CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::Range)?;
    parse_range(value)
}
fn parse_range(value: &str) -> Result<(u64, u64, u64)> {
    let (range, total) = value
        .strip_prefix("bytes ")
        .and_then(|v| v.split_once('/'))
        .ok_or(Error::Range)?;
    let (start, end) = range.split_once('-').ok_or(Error::Range)?;
    let start: u64 = start.parse().map_err(|_| Error::Range)?;
    let end: u64 = end.parse().map_err(|_| Error::Range)?;
    let total: u64 = total.parse().map_err(|_| Error::Range)?;
    if start > end || end >= total {
        Err(Error::Range)
    } else {
        Ok((start, end, total))
    }
}
fn split_ranges(total: Option<u64>, count: usize) -> Vec<(u64, Option<u64>)> {
    match total {
        Some(0) | None => vec![(0, None)],
        Some(total) => {
            let count = (count as u64).min(total).max(1);
            let boundary = |i| (total / count) * i + ((total % count) * i) / count;
            (0..count)
                .map(|i| (boundary(i), Some(boundary(i + 1) - 1)))
                .collect()
        }
    }
}
async fn read_checkpoint(path: &Path) -> Result<u64> {
    match fs::read_to_string(path).await {
        Ok(value) => value
            .parse()
            .map_err(|_| Error::Resume("a checkpoint is invalid".into())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e.into()),
    }
}
async fn saved_bytes(path: &Path, count: usize) -> Result<u64> {
    let mut saved = 0;
    for index in 0..count {
        saved += read_checkpoint(&path.join(format!("{index}.checkpoint"))).await?;
    }
    Ok(saved)
}
async fn clear_segments(directory: &Path, count: usize) -> Result<()> {
    for index in 0..count {
        for suffix in ["part", "checkpoint"] {
            match fs::remove_file(directory.join(format!("{index}.{suffix}"))).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(())
}
async fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension("tmp");
    let mut file = File::create(&temporary).await?;
    file.write_all(bytes).await?;
    file.flush().await?;
    file.sync_all().await?;
    drop(file);
    fs::rename(temporary, path).await?;
    Ok(())
}
struct WorkLock(std::fs::File);
impl WorkLock {
    async fn acquire(path: PathBuf) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(path)?;
            fs2::FileExt::try_lock_exclusive(&file)?;
            Ok(Self(file))
        })
        .await
        .map_err(|e| Error::Invalid(e.to_string()))?
    }
}
impl Drop for WorkLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ranges_cover_every_byte_exactly_once() {
        let ranges = split_ranges(Some(10), 3);
        assert_eq!(ranges, vec![(0, Some(2)), (3, Some(5)), (6, Some(9))]);
        assert!(parse_range("bytes 0-1/1").is_err());
        assert!(parse_range("bytes */100").is_err());
        assert_eq!(
            parse_range("bytes 0-0/10").expect("valid range"),
            (0, 0, 10)
        );
    }
}

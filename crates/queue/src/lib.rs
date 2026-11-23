use chrono::{Local, Timelike};
use ndm_core::{
    validate_url, within_schedule, Download, Engine, EngineEvent, NetworkLimits, Settings,
    Snapshot, Status,
};
use ndm_storage::Store;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Instant,
};
use tokio::sync::{mpsc, oneshot, watch};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    BrowserAdd {
        urls: Vec<String>,
        start: bool,
    },
    Add {
        urls: Vec<String>,
        directory: Option<PathBuf>,
        name: Option<String>,
        start: bool,
        sha256: Option<String>,
    },
    StartQueue,
    PauseQueue,
    StopQueue,
    Drain,
    Pause {
        id: String,
        continue_queue: bool,
    },
    Stop {
        id: String,
    },
    Resume {
        id: String,
    },
    Restart {
        id: String,
    },
    Remove {
        id: String,
        delete_files: bool,
    },
    Move {
        id: String,
        position: String,
    },
    Skip {
        id: String,
    },
    Reorder {
        ids: Vec<String>,
    },
    Settings {
        settings: Settings,
    },
    Shutdown,
}

type Reply = oneshot::Sender<Result<Snapshot, String>>;
#[derive(Clone)]
pub struct Manager {
    commands: mpsc::Sender<(Action, Reply)>,
    snapshots: watch::Receiver<Snapshot>,
}
struct Active {
    cancel: CancellationToken,
    after: Status,
    remove: Option<bool>,
    restart: bool,
    last_bytes: u64,
    sampled: Instant,
}
enum WorkerEvent {
    Update(String, EngineEvent),
    Done(String, ndm_core::Result<(PathBuf, String)>),
}

impl Manager {
    pub async fn open(data: PathBuf, defaults: Settings) -> Result<Self, String> {
        defaults.validate().map_err(|e| e.to_string())?;
        let store = Store::open(data.join("ndm.db")).await?;
        let mut snapshot = store.load().await?.unwrap_or(Snapshot {
            user_paused: false,
            downloads: vec![],
            settings: defaults,
            running: false,
            draining: false,
        });
        let resume = snapshot.settings.resume_on_launch && snapshot.running;
        for job in &mut snapshot.downloads {
            if job.status.active() {
                job.status = if resume {
                    Status::Queued
                } else {
                    Status::Paused
                };
                job.resume_with_queue = true;
            }
            job.speed = 0.0;
        }
        snapshot.running = resume;
        snapshot.draining = false;
        let limits = NetworkLimits::new(
            snapshot.settings.global_connections,
            snapshot.settings.host_connections,
            snapshot.settings.speed_limit,
        );
        let engine = Engine::new(data.join("parts"), limits).map_err(|e| e.to_string())?;
        store.save(&snapshot).await?;
        let (commands, command_rx) = mpsc::channel(64);
        let (snapshot_tx, snapshots) = watch::channel(snapshot.clone());
        let (worker_tx, worker_rx) = mpsc::channel(512);
        tokio::spawn(
            Actor {
                snapshot,
                engine,
                store,
                active: HashMap::new(),
                command_rx,
                snapshot_tx,
                worker_rx,
                worker_tx,
                shutdown: None,
                dirty: false,
            }
            .run(),
        );
        Ok(Self {
            commands,
            snapshots,
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshots.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<Snapshot> {
        self.snapshots.clone()
    }
    pub async fn execute(&self, action: Action) -> Result<Snapshot, String> {
        let (reply, response) = oneshot::channel();
        self.commands
            .send((action, reply))
            .await
            .map_err(|_| "The queue has shut down.".to_string())?;
        response
            .await
            .map_err(|_| "The queue has shut down.".to_string())?
    }
}

struct Actor {
    snapshot: Snapshot,
    engine: Engine,
    store: Store,
    active: HashMap<String, Active>,
    command_rx: mpsc::Receiver<(Action, Reply)>,
    snapshot_tx: watch::Sender<Snapshot>,
    worker_rx: mpsc::Receiver<WorkerEvent>,
    worker_tx: mpsc::Sender<WorkerEvent>,
    shutdown: Option<Reply>,
    dirty: bool,
}

impl Actor {
    async fn run(mut self) {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            tokio::select! {
                command = self.command_rx.recv(), if self.shutdown.is_none() => {
                    let Some((action, reply)) = command else { self.cancel_all(Status::Stopped); break; };
                    if matches!(action, Action::Shutdown) {
                        self.snapshot.running = false; self.cancel_all(Status::Stopped); self.shutdown = Some(reply);
                    } else {
                        let before = self.snapshot.clone();
                        let user_paused = match &action {
                            Action::PauseQueue | Action::StopQueue | Action::Stop { .. }
                            | Action::Pause { continue_queue: false, .. } | Action::Drain => Some(true),
                            Action::StartQueue | Action::Resume { .. } | Action::Restart { .. }
                            | Action::Add { start: true, .. } => Some(false),
                            _ => None,
                        };
                        let result = self.apply(action).await;
                        if result.is_ok() {
                            if let Some(paused) = user_paused { self.snapshot.user_paused = paused; }
                        }
                        let result = match result {
                            Ok(()) => match self.store.save(&self.snapshot).await {
                                Ok(()) => Ok(self.snapshot.clone()),
                                Err(e) => { self.snapshot = before; self.snapshot.running = false; self.cancel_all(Status::Paused); Err(format!("Could not save the queue: {e}")) }
                            },
                            Err(e) => Err(e),
                        };
                        let _ = reply.send(result);
                    }
                    self.publish();
                },
                event = self.worker_rx.recv() => { if let Some(event) = event { self.worker(event).await; self.dirty = true; } },
                _ = tick.tick() => {
                    for job in &mut self.snapshot.downloads {
                        if let Some(active) = self.active.get_mut(&job.id) {
                            let elapsed = active.sampled.elapsed().as_secs_f64();
                            let rate = job.downloaded.saturating_sub(active.last_bytes) as f64 / elapsed.max(0.001);
                            job.speed = if job.speed == 0.0 { rate } else { job.speed * 0.7 + rate * 0.3 };
                            active.last_bytes = job.downloaded; active.sampled = Instant::now();
                        }
                    }
                    if self.dirty {
                        if let Err(e) = self.store.save(&self.snapshot).await {
                            self.snapshot.running = false; self.cancel_all(Status::Paused);
                            for job in &mut self.snapshot.downloads { if job.status.active() { job.error = Some(format!("Queue checkpoint failed: {e}")); } }
                        }
                        self.dirty = false;
                    }
                    self.publish();
                }
            }
            if self.shutdown.is_some() && self.active.is_empty() {
                let result = self
                    .store
                    .save(&self.snapshot)
                    .await
                    .map(|_| self.snapshot.clone());
                if let Some(reply) = self.shutdown.take() {
                    let _ = reply.send(result);
                }
                self.publish();
                break;
            }
            self.schedule();
        }
    }

    fn publish(&self) {
        self.snapshot_tx.send_replace(self.snapshot.clone());
    }
    fn cancel_all(&mut self, status: Status) {
        for (id, active) in &mut self.active {
            active.after = status;
            active.cancel.cancel();
            if let Some(job) = self.snapshot.downloads.iter_mut().find(|job| job.id == *id) {
                job.resume_with_queue = status == Status::Paused;
            }
        }
    }
    fn job_index(&self, id: &str) -> Result<usize, String> {
        self.snapshot
            .downloads
            .iter()
            .position(|job| job.id == id)
            .ok_or_else(|| "This download no longer exists.".into())
    }
    async fn apply(&mut self, action: Action) -> Result<(), String> {
        match action {
            Action::BrowserAdd { urls, start } => {
                if urls.is_empty() || urls.len() > 100 {
                    return Err("Add between 1 and 100 URLs at a time.".into());
                }
                let mut seen: HashSet<String> = self
                    .snapshot
                    .downloads
                    .iter()
                    .map(|job| job.url.clone())
                    .collect();
                let mut fresh = Vec::new();
                let mut requested = HashMap::new();
                for input in urls {
                    if input.len() > 16384 {
                        return Err("The URL exceeds the length limit.".into());
                    }
                    let url = validate_url(&input).map_err(|e| e.to_string())?.to_string();
                    let order = requested.len();
                    requested.entry(url.clone()).or_insert(order);
                    if seen.insert(url.clone()) {
                        fresh.push(url);
                    }
                }
                if !fresh.is_empty() {
                    // Box the shared add action rather than duplicate its validation and creation.
                    Box::pin(self.apply(Action::Add {
                        urls: fresh,
                        directory: None,
                        name: None,
                        start: false,
                        sha256: None,
                    }))
                    .await?;
                }
                if start {
                    // A new Download now can start a URL previously added to the queue.
                    let mut jobs: Vec<_> = self
                        .snapshot
                        .downloads
                        .extract_if(.., |job| {
                            job.status == Status::Queued && requested.contains_key(&job.url)
                        })
                        .collect();
                    jobs.sort_by_key(|job| requested[&job.url]);
                    if !jobs.is_empty() {
                        let next = self
                            .snapshot
                            .downloads
                            .iter()
                            .position(|job| job.status == Status::Queued)
                            .unwrap_or(self.snapshot.downloads.len());
                        self.snapshot.downloads.splice(next..next, jobs);
                        if !self.snapshot.user_paused {
                            self.snapshot.running = true;
                        }
                    }
                }
            }
            Action::Add {
                urls,
                directory,
                name,
                start,
                sha256,
            } => {
                if urls.is_empty() || urls.len() > 100 {
                    return Err("Add between 1 and 100 URLs at a time.".into());
                }
                if self.snapshot.downloads.len() + urls.len() > 10000 {
                    return Err("The queue is full. Remove old entries first.".into());
                }
                if sha256
                    .as_ref()
                    .is_some_and(|s| s.len() != 64 || !s.chars().all(|c| c.is_ascii_hexdigit()))
                {
                    return Err("SHA-256 must contain exactly 64 hexadecimal characters.".into());
                }
                if urls.len() > 1 && (name.is_some() || sha256.is_some()) {
                    return Err("A custom name or hash requires one URL.".into());
                }
                let directory =
                    directory.unwrap_or_else(|| self.snapshot.settings.directory.clone());
                if !directory.is_absolute() {
                    return Err("Choose an absolute destination folder.".into());
                }
                let mut seen: HashSet<String> = self
                    .snapshot
                    .downloads
                    .iter()
                    .map(|job| job.url.clone())
                    .collect();
                let mut validated = Vec::new();
                for input in urls {
                    if input.len() > 16384 {
                        return Err("The URL exceeds the length limit.".into());
                    }
                    let url = validate_url(&input).map_err(|e| e.to_string())?.to_string();
                    if !seen.insert(url.clone()) {
                        return Err(
                            "A URL is already in the queue or appears twice in this batch.".into(),
                        );
                    }
                    validated.push(url);
                }
                for url in validated {
                    self.snapshot.downloads.push(Download {
                        id: Uuid::new_v4().to_string(),
                        url,
                        name: name
                            .as_deref()
                            .map(ndm_core::sanitize_filename)
                            .unwrap_or_default(),
                        directory: directory.clone(),
                        status: Status::Queued,
                        downloaded: 0,
                        total: None,
                        speed: 0.0,
                        connections: 0,
                        resumable: false,
                        error: None,
                        path: None,
                        sha256: None,
                        expected_sha256: sha256.clone(),
                        resume_with_queue: false,
                    });
                }
                if start {
                    self.snapshot.running = true;
                    self.snapshot.draining = false;
                }
            }
            Action::StartQueue => {
                for job in &mut self.snapshot.downloads {
                    if job.status == Status::Paused && job.resume_with_queue {
                        job.status = Status::Queued;
                        job.resume_with_queue = false;
                    }
                }
                for active in self.active.values_mut() {
                    if active.after == Status::Paused && active.cancel.is_cancelled() {
                        active.after = Status::Queued;
                    }
                }
                self.snapshot.running = true;
                self.snapshot.draining = false;
            }
            Action::PauseQueue => {
                self.snapshot.running = false;
                self.snapshot.draining = false;
                self.cancel_all(Status::Paused);
            }
            Action::StopQueue => {
                self.snapshot.running = false;
                self.snapshot.draining = false;
                self.cancel_all(Status::Stopped);
            }
            Action::Drain => {
                self.snapshot.draining = true;
            }
            Action::Pause { id, continue_queue } => {
                let index = self.job_index(&id)?;
                if self.snapshot.downloads[index].status == Status::Completed {
                    return Err("A completed file cannot be paused.".into());
                }
                if !continue_queue {
                    self.snapshot.running = false;
                }
                if let Some(active) = self.active.get_mut(&id) {
                    active.after = Status::Paused;
                    active.cancel.cancel();
                } else {
                    self.snapshot.downloads[index].status = Status::Paused;
                }
                self.snapshot.downloads[index].resume_with_queue = !continue_queue;
            }
            Action::Stop { id } => {
                let index = self.job_index(&id)?;
                if self.snapshot.downloads[index].status == Status::Completed {
                    return Err("A completed file cannot be stopped.".into());
                }
                self.snapshot.running = false;
                if let Some(active) = self.active.get_mut(&id) {
                    active.after = Status::Stopped;
                    active.cancel.cancel();
                } else {
                    self.snapshot.downloads[index].status = Status::Stopped;
                }
                self.snapshot.downloads[index].resume_with_queue = false;
            }
            Action::Resume { id } => {
                let index = self.job_index(&id)?;
                if self.snapshot.downloads[index].status == Status::Completed {
                    return Err("This file is already complete.".into());
                }
                if let Some(active) = self.active.get_mut(&id) {
                    if active.cancel.is_cancelled() {
                        active.after = Status::Queued;
                    }
                } else {
                    self.snapshot.downloads[index].status = Status::Queued;
                    self.snapshot.downloads[index].error = None;
                }
                self.snapshot.running = true;
                self.snapshot.draining = false;
            }
            Action::Restart { id } => {
                let index = self.job_index(&id)?;
                if let Some(active) = self.active.get_mut(&id) {
                    active.restart = true;
                    active.cancel.cancel();
                } else {
                    self.engine.discard(&id).await.map_err(|e| e.to_string())?;
                    reset(&mut self.snapshot.downloads[index]);
                }
                self.snapshot.running = true;
                self.snapshot.draining = false;
            }
            Action::Remove { id, delete_files } => {
                let index = self.job_index(&id)?;
                if let Some(active) = self.active.get_mut(&id) {
                    active.remove = Some(delete_files);
                    active.cancel.cancel();
                } else {
                    self.remove(index, delete_files).await?;
                }
            }
            Action::Move { id, position } => {
                let index = self.job_index(&id)?;
                let target = match position.as_str() {
                    "top" => 0,
                    "bottom" => self.snapshot.downloads.len() - 1,
                    "up" => index.saturating_sub(1),
                    "down" => (index + 1).min(self.snapshot.downloads.len() - 1),
                    _ => return Err("Unknown queue position.".into()),
                };
                let job = self.snapshot.downloads.remove(index);
                self.snapshot.downloads.insert(target, job);
            }
            Action::Skip { id } => {
                let index = self.job_index(&id)?;
                let job = &self.snapshot.downloads[index];
                if job.downloaded > 0 && !job.resumable {
                    return Err("This server cannot resume. Pause this download or explicitly restart it before skipping.".into());
                }
                if job.status == Status::Completed {
                    return Err("This file is already complete.".into());
                }
                if let Some(active) = self.active.get_mut(&id) {
                    active.after = Status::Queued;
                    active.cancel.cancel();
                } else {
                    self.snapshot.downloads[index].status = Status::Queued;
                }
                let job = self.snapshot.downloads.remove(index);
                self.snapshot.downloads.push(job);
            }
            Action::Reorder { ids } => {
                let expected: HashSet<_> = self.snapshot.downloads.iter().map(|d| &d.id).collect();
                if ids.len() != expected.len() || ids.iter().collect::<HashSet<_>>() != expected {
                    return Err("Queue changed; try reordering again.".into());
                }
                let order: HashMap<_, _> = ids.iter().enumerate().map(|(i, id)| (id, i)).collect();
                self.snapshot
                    .downloads
                    .sort_by_key(|job| order.get(&job.id).copied().unwrap_or(usize::MAX));
            }
            Action::Settings { settings } => {
                settings.validate().map_err(|e| e.to_string())?;
                let limits_changed = settings.global_connections
                    != self.snapshot.settings.global_connections
                    || settings.host_connections != self.snapshot.settings.host_connections;
                if limits_changed && !self.active.is_empty() {
                    return Err("Pause all downloads before changing connection ceilings.".into());
                }
                if limits_changed {
                    self.engine = Engine::new(
                        self.engine_work_root(),
                        NetworkLimits::new(
                            settings.global_connections,
                            settings.host_connections,
                            settings.speed_limit,
                        ),
                    )
                    .map_err(|e| e.to_string())?;
                }
                self.engine.limits.set_rate(settings.speed_limit).await;
                self.snapshot.settings = settings;
            }
            Action::Shutdown => {}
        }
        self.dirty = true;
        Ok(())
    }
    fn engine_work_root(&self) -> PathBuf {
        self.engine.work_root().to_path_buf()
    }
    async fn remove(&mut self, index: usize, delete_files: bool) -> Result<(), String> {
        let job = &self.snapshot.downloads[index];
        if delete_files {
            if let Some(path) = &job.path {
                match tokio::fs::remove_file(path).await {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.to_string()),
                }
            }
            self.engine
                .discard(&job.id)
                .await
                .map_err(|e| e.to_string())?;
        }
        self.snapshot.downloads.remove(index);
        Ok(())
    }
    async fn worker(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Update(id, event) => {
                if let Some(job) = self.snapshot.downloads.iter_mut().find(|job| job.id == id) {
                    match event {
                        EngineEvent::Metadata {
                            name,
                            total,
                            connections,
                            resumable,
                        } => {
                            if job.name.is_empty() {
                                job.name = name;
                            }
                            job.total = total;
                            job.connections = connections;
                            job.resumable = resumable;
                            job.status = Status::Downloading;
                        }
                        EngineEvent::Progress(downloaded) => job.downloaded = downloaded,
                    }
                }
            }
            WorkerEvent::Done(id, result) => {
                let Some(active) = self.active.remove(&id) else {
                    return;
                };
                let Ok(index) = self.job_index(&id) else {
                    return;
                };
                if let Some(delete_files) = active.remove {
                    if let Err(e) = self.remove(index, delete_files).await {
                        self.snapshot.downloads[index].status = Status::Failed;
                        self.snapshot.downloads[index].error = Some(e);
                    }
                    return;
                }
                if active.restart {
                    match self.engine.discard(&id).await {
                        Ok(()) => reset(&mut self.snapshot.downloads[index]),
                        Err(e) => {
                            self.snapshot.downloads[index].status = Status::Failed;
                            self.snapshot.downloads[index].error = Some(e.to_string());
                        }
                    }
                    return;
                }
                let job = &mut self.snapshot.downloads[index];
                job.speed = 0.0;
                match result {
                    Ok((path, hash)) => {
                        job.downloaded = tokio::fs::metadata(&path)
                            .await
                            .map(|m| m.len())
                            .unwrap_or(job.downloaded);
                        job.total = Some(job.downloaded);
                        job.path = Some(path);
                        job.sha256 = Some(hash);
                        job.status = Status::Completed;
                        job.error = None;
                        let _ = self.engine.discard(&id).await;
                    }
                    Err(ndm_core::DownloadError::Cancelled) => {
                        job.status = active.after;
                    }
                    Err(e) => {
                        job.status = Status::Failed;
                        job.error = Some(e.to_string());
                        if self.snapshot.settings.stop_on_error {
                            self.snapshot.running = false;
                            self.snapshot.user_paused = true;
                        }
                    }
                }
                if self.snapshot.draining && self.active.is_empty() {
                    self.snapshot.running = false;
                    self.snapshot.draining = false;
                }
            }
        }
    }

    fn schedule(&mut self) {
        let settings = &self.snapshot.settings;
        let now = Local::now();
        let minute = (now.hour() * 60 + now.minute()) as u16;
        if !self.snapshot.running
            || self.snapshot.draining
            || self.shutdown.is_some()
            || (settings.schedule_enabled
                && !within_schedule(minute, settings.schedule_start, settings.schedule_end))
        {
            return;
        }
        for job in &mut self.snapshot.downloads {
            if self.active.len() >= settings.concurrent_downloads {
                break;
            }
            if job.status != Status::Queued || self.active.contains_key(&job.id) {
                continue;
            }
            let cancel = CancellationToken::new();
            self.active.insert(
                job.id.clone(),
                Active {
                    cancel: cancel.clone(),
                    after: Status::Paused,
                    remove: None,
                    restart: false,
                    last_bytes: job.downloaded,
                    sampled: Instant::now(),
                },
            );
            job.status = Status::Connecting;
            job.error = None;
            let job = job.clone();
            let engine = self.engine.clone();
            let tx = self.worker_tx.clone();
            let connections = settings.connections_per_file;
            tokio::spawn(async move {
                let (events, mut receiver) = mpsc::channel(128);
                let relay_tx = tx.clone();
                let id = job.id.clone();
                let relay = tokio::spawn(async move {
                    while let Some(event) = receiver.recv().await {
                        if relay_tx
                            .send(WorkerEvent::Update(id.clone(), event))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                });
                let result = engine.download(&job, connections, cancel, events).await;
                let _ = relay.await;
                let _ = tx.send(WorkerEvent::Done(job.id, result)).await;
            });
            self.dirty = true;
        }
    }
}

fn reset(job: &mut Download) {
    job.status = Status::Queued;
    job.downloaded = 0;
    job.total = None;
    job.speed = 0.0;
    job.error = None;
    job.path = None;
    job.sha256 = None;
    job.resume_with_queue = false;
}

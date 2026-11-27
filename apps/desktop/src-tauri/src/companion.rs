use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path as RoutePath, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use ndm_core::{sanitize_filename, Status, Usage};
use ndm_queue::{Action, Manager};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    path::PathBuf,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri_plugin_opener::OpenerExt;
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::Mutex,
};
use uuid::Uuid;

const PORT: u16 = 42672;
const CHUNK: usize = 512 * 1024;
const MAX_FILE: u64 = 128 * 1024 * 1024 * 1024;
const PAIR_SECONDS: u64 = 120;
const SESSION_SECONDS: u64 = 12 * 60 * 60;
const ONLINE_SECONDS: u64 = 20;

type Result<T> = std::result::Result<T, String>;
type Http<T> = std::result::Result<T, ApiError>;

struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({"error": self.1}))).into_response()
    }
}
fn bad(message: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message.into())
}
fn io_error(error: impl std::fmt::Display) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}
fn missing() -> ApiError {
    ApiError(StatusCode::NOT_FOUND, "Transfer not found.".into())
}
fn unavailable() -> ApiError {
    ApiError(
        StatusCode::CONFLICT,
        "Transfer paused or already finished.".into(),
    )
}
fn unauthorized() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "Pair with this PC again.".into())
}
fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}
fn sha(input: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(input))
}
fn matches_hash(expected: &str, actual: &str) -> bool {
    expected.len() == actual.len()
        && expected
            .bytes()
            .zip(actual.bytes())
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

#[derive(Clone, Serialize, Deserialize)]
struct Device {
    id: Uuid,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires: Option<u64>,
    token_hash: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Ready,
    Paused,
    Complete,
    Failed,
}

#[derive(Clone, Serialize, Deserialize)]
struct Transfer {
    id: Uuid,
    device: Uuid,
    to_pc: bool,
    name: String,
    total: u64,
    bytes: u64,
    phase: Phase,
    path: PathBuf,
    modified: Option<SystemTime>,
    error: Option<String>,
    #[serde(skip)]
    speed: f64,
    #[serde(skip)]
    sampled: Option<Instant>,
}

impl Transfer {
    fn partial(&self) -> PathBuf {
        self.path.with_file_name(format!(".{}.partial", self.id))
    }
    fn sample(&mut self, offset: u64) {
        if let Some(prev) = self.sampled {
            let rate =
                offset.saturating_sub(self.bytes) as f64 / prev.elapsed().as_secs_f64().max(0.001);
            self.speed = if self.speed > 0.0 {
                self.speed * 0.65 + rate * 0.35
            } else {
                rate
            };
        }
        self.bytes = offset;
        self.sampled = Some(Instant::now());
    }
    fn public(&self) -> serde_json::Value {
        let speed = if self.phase == Phase::Ready
            && self
                .sampled
                .is_some_and(|time| time.elapsed().as_secs() < 5)
        {
            self.speed
        } else {
            0.0
        };
        serde_json::json!({"id":self.id,"device":self.device,"to_pc":self.to_pc,"name":self.name,"total":self.total,
            "bytes":self.bytes,"phase":self.phase,"error":self.error,"speed":speed})
    }
    fn pause(&mut self, paused: bool) -> Http<()> {
        if self.phase == Phase::Complete {
            return Err(unavailable());
        }
        self.phase = if paused { Phase::Paused } else { Phase::Ready };
        self.error = None;
        self.speed = 0.0;
        self.sampled = None;
        Ok(())
    }
}

#[derive(Default, Deserialize, Serialize)]
struct Saved {
    enabled: bool,
    #[serde(default)]
    bluetooth: bool,
    devices: Vec<Device>,
    transfers: Vec<Transfer>,
}
#[derive(Default)]
struct Runtime {
    saved: Saved,
    online: std::collections::HashMap<Uuid, u64>,
    pair: Option<(String, u64)>,
    server: Option<axum_server::Handle<SocketAddr>>,
    fingerprint: String,
    discovery: Option<(mdns_sd::ServiceDaemon, String)>,
    port: u16,
    #[cfg(windows)]
    bluetooth: Option<crate::companion_bluetooth::Listener>,
}

#[derive(Clone)]
pub struct Companion {
    inner: Arc<Mutex<Runtime>>,
    dir: PathBuf,
    manager: Manager,
    bind_port: u16,
}

impl Companion {
    pub async fn open(dir: PathBuf, manager: Manager) -> Result<Self> {
        let saved: Saved = match fs::read(dir.join("state.json")).await {
            Ok(raw) => serde_json::from_slice(&raw).map_err(err)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Saved::default(),
            Err(e) => return Err(err(e)),
        };
        let startup = saved.enabled;
        let state = Self {
            inner: Arc::new(Mutex::new(Runtime {
                saved,
                ..Default::default()
            })),
            dir,
            manager,
            bind_port: PORT,
        };
        if startup {
            if let Err(error) = state.enable(true).await {
                eprintln!("Companion: {error}");
            }
            #[cfg(windows)]
            {
                let mut runtime = state.inner.lock().await;
                if runtime.server.is_some() && runtime.saved.bluetooth {
                    runtime.bluetooth =
                        crate::companion_bluetooth::Listener::start(runtime.port, None)
                            .await
                            .ok();
                }
            }
        }
        Ok(state)
    }
    async fn save(&self, saved: &Saved) -> Result<()> {
        fs::create_dir_all(&self.dir).await.map_err(err)?;
        let temp = self.dir.join("state.tmp");
        let bytes = serde_json::to_vec(saved).map_err(err)?;
        let mut file = fs::File::create(&temp).await.map_err(err)?;
        file.write_all(&bytes).await.map_err(err)?;
        file.sync_all().await.map_err(err)?;
        drop(file);
        fs::rename(&temp, self.dir.join("state.json"))
            .await
            .map_err(err)
    }
    async fn identity(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        #[derive(Serialize, Deserialize)]
        struct Identity {
            cert: Vec<u8>,
            key: Vec<u8>,
        }
        let path = self.dir.join("identity.json");
        let identity: Identity = match fs::read(&path).await {
            Ok(raw) => serde_json::from_slice(&raw).map_err(err)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let generated =
                    rcgen::generate_simple_self_signed(vec!["ndm.local".into()]).map_err(err)?;
                let identity = Identity {
                    cert: generated.cert.der().to_vec(),
                    key: generated.signing_key.serialize_der(),
                };
                fs::create_dir_all(&self.dir).await.map_err(err)?;
                let mut file = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)
                    .await
                    .map_err(err)?;
                file.write_all(&serde_json::to_vec(&identity).map_err(err)?)
                    .await
                    .map_err(err)?;
                file.sync_all().await.map_err(err)?;
                identity
            }
            Err(e) => return Err(err(e)),
        };
        Ok((identity.cert, identity.key))
    }
    async fn enable(&self, enabled: bool) -> Result<()> {
        let mut state = self.inner.lock().await;
        if enabled {
            if state.server.is_some() {
                return Ok(());
            }
            let (cert, key) = self.identity().await?;
            let fingerprint = sha(&cert);
            let config = rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .map_err(err)?
            .with_no_client_auth()
            .with_single_cert(
                vec![cert.into()],
                rustls::pki_types::PrivatePkcs8KeyDer::from(key).into(),
            )
            .map_err(err)?;
            let socket = std::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, self.bind_port))
                .map_err(err)?;
            let port = socket.local_addr().map_err(err)?.port();
            socket.set_nonblocking(true).map_err(err)?;
            let routes = Router::new()
                .route("/pair", post(pair))
                .route("/status", get(status))
                .route("/download", post(download))
                .route("/usage", post(sync_usage))
                .route("/transfer", post(prepare_upload))
                .route(
                    "/transfer/{id}",
                    get(transfer_info)
                        .delete(pause_transfer)
                        .post(retry_transfer),
                )
                .route("/transfer/{id}/chunk", get(read_chunk).post(write_chunk))
                .route("/transfer/{id}/ack", post(acknowledge))
                .route("/device", delete(disconnect))
                .layer(DefaultBodyLimit::max(1024 * 1024))
                .with_state(self.clone());
            let handle = axum_server::Handle::new();
            state.saved.enabled = true;
            if let Err(error) = self.save(&state.saved).await {
                state.saved.enabled = false;
                return Err(error);
            }
            if self.bind_port == PORT {
                match advertise(&fingerprint, port) {
                    Ok(discovery) => state.discovery = Some(discovery),
                    Err(error) => eprintln!("Phone discovery: {error}"),
                }
            }
            state.fingerprint = fingerprint;
            state.port = port;
            state.server = Some(handle.clone());
            tokio::spawn(async move {
                match axum_server::from_tcp_rustls(
                    socket,
                    axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(config)),
                ) {
                    Ok(server) => {
                        if let Err(error) = server
                            .handle(handle)
                            .serve(routes.into_make_service())
                            .await
                        {
                            eprintln!("Companion server: {error}");
                        }
                    }
                    Err(error) => eprintln!("Companion socket: {error}"),
                }
            });
        } else {
            state.saved.enabled = false;
            self.save(&state.saved).await?;
            state.online.clear();
            state.pair = None;
            #[cfg(windows)]
            {
                state.bluetooth = None;
            }
            if let Some((daemon, name)) = state.discovery.take() {
                let _ = daemon.unregister(&name);
                let _ = daemon.shutdown();
            }
            if let Some(handle) = state.server.take() {
                handle.shutdown();
            }
        }
        Ok(())
    }
    async fn authenticate(&self, headers: &HeaderMap) -> Http<Uuid> {
        let token = headers
            .get("authorization")
            .and_then(|header| header.to_str().ok())
            .and_then(|header| header.strip_prefix("Bearer "))
            .filter(|token| token.len() == 64)
            .ok_or_else(unauthorized)?;
        let mut state = self.inner.lock().await;
        if !state.saved.enabled {
            return Err(unauthorized());
        }
        let hash = sha(token);
        let device = state
            .saved
            .devices
            .iter()
            .find(|d| {
                matches_hash(&d.token_hash, &hash) && d.expires.is_none_or(|time| time > now())
            })
            .map(|d| d.id)
            .ok_or_else(unauthorized)?;
        state.online.insert(device, now());
        Ok(device)
    }
    async fn view(&self) -> serde_json::Value {
        let state = self.inner.lock().await;
        let devices = state.saved.devices.iter().filter(|device| device.expires.is_none_or(|time| time > now())).map(|d|
            serde_json::json!({"id":d.id,"name":d.name,"remember":d.expires.is_none(),"online":state.online.get(&d.id).is_some_and(|t| now().saturating_sub(*t)<ONLINE_SECONDS)})
        ).collect::<Vec<_>>();
        serde_json::json!({"enabled":state.server.is_some(),"devices":devices,
            "pairing":state.pair.is_some(), "windows_pairing":cfg!(windows),
            "transfers":state.saved.transfers.iter().rev().map(Transfer::public).collect::<Vec<_>>()})
    }
    async fn revoke(&self, id: Uuid) -> Result<()> {
        let mut state = self.inner.lock().await;
        let index = state
            .saved
            .devices
            .iter()
            .position(|device| device.id == id)
            .ok_or("Device not found.")?;
        let removed = state.saved.devices.remove(index);
        if let Err(error) = self.save(&state.saved).await {
            state.saved.devices.insert(index, removed);
            return Err(error);
        }
        state.online.remove(&id);
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    code: String,
    id: Uuid,
    name: String,
    remember: bool,
}
async fn pair(
    State(app): State<Companion>,
    Json(pair): Json<Pair>,
) -> Http<Json<serde_json::Value>> {
    let mut state = app.inner.lock().await;
    if pair.id.is_nil()
        || pair.name.trim().is_empty()
        || pair.name.chars().count() > 64
        || pair.name.chars().any(char::is_control)
    {
        return Err(bad("Invalid device name or ID."));
    }
    if !state.saved.enabled
        || !state
            .pair
            .as_ref()
            .is_some_and(|(hash, expires)| now() < *expires && matches_hash(hash, &sha(&pair.code)))
    {
        return Err(unauthorized());
    }
    let token = secret();
    let expires = if pair.remember {
        None
    } else {
        Some(now() + SESSION_SECONDS)
    };
    let previous = state.saved.devices.clone();
    state
        .saved
        .devices
        .retain(|d| d.id != pair.id && d.expires.is_none_or(|time| time > now()));
    if state.saved.devices.len() >= 16 {
        state.saved.devices = previous;
        return Err(bad("Remove an old paired device first."));
    }
    state.saved.devices.push(Device {
        id: pair.id,
        name: pair.name,
        expires,
        token_hash: sha(&token),
    });
    if let Err(error) = app.save(&state.saved).await {
        state.saved.devices = previous;
        return Err(io_error(error));
    }
    state.pair = None;
    state.online.insert(pair.id, now());
    Ok(Json(serde_json::json!({"token":token,"expires":expires})))
}

async fn disconnect(State(app): State<Companion>, headers: HeaderMap) -> Http<StatusCode> {
    let id = app.authenticate(&headers).await?;
    app.revoke(id).await.map_err(io_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn status(State(app): State<Companion>, headers: HeaderMap) -> Http<Json<serde_json::Value>> {
    let device = app.authenticate(&headers).await?;
    let snapshot = app.manager.snapshot();
    let downloads = snapshot.downloads.iter().map(|d| serde_json::json!({
        "id":d.id, "name":if d.name.is_empty() {"Download"} else {d.name.as_str()}, "status":d.status, "bytes":d.downloaded, "total":d.total, "speed":d.speed,
        "error":if d.status == Status::Failed {Some("See NDM on the PC for details.")} else {None}
    })).collect::<Vec<_>>();
    let state = app.inner.lock().await;
    let transfers = state
        .saved
        .transfers
        .iter()
        .rev()
        .filter(|x| x.device == device)
        .map(Transfer::public)
        .collect::<Vec<_>>();
    Ok(Json(
        serde_json::json!({"downloads":downloads,"transfers":transfers,"running":snapshot.running}),
    ))
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Control {
    Add { urls: Vec<String> },
    Resume { id: Uuid },
    Pause { id: Uuid },
    Stop { id: Uuid },
    Remove { id: Uuid },
    StartQueue,
    PauseQueue,
    StopQueue,
}
async fn download(
    State(app): State<Companion>,
    headers: HeaderMap,
    Json(control): Json<Control>,
) -> Http<StatusCode> {
    app.authenticate(&headers).await?;
    let action = match control {
        Control::Add { urls } => Action::Add {
            urls,
            directory: None,
            name: None,
            start: true,
            sha256: None,
        },
        Control::Resume { id } => Action::Resume { id: id.to_string() },
        Control::Pause { id } => Action::Pause {
            id: id.to_string(),
            continue_queue: true,
        },
        Control::Stop { id } => Action::StopOne { id: id.to_string() },
        Control::Remove { id } => Action::Remove {
            id: id.to_string(),
            delete_files: false,
        },
        Control::StartQueue => Action::StartQueue,
        Control::PauseQueue => Action::PauseQueue,
        Control::StopQueue => Action::StopQueue,
    };
    app.manager.execute(action).await.map_err(bad)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn sync_usage(
    State(app): State<Companion>,
    headers: HeaderMap,
    Json(entries): Json<Vec<Usage>>,
) -> Http<Json<Vec<Usage>>> {
    let device = app.authenticate(&headers).await?;
    let hour = (now() / 3600) as i64;
    if entries.len() > 6200
        || entries.iter().any(|row| {
            row.hour < hour - 31 * 24
                || row.hour > hour + 1
                || row.bytes > 1_000_000_000_000_000
                || ![
                    "video",
                    "audio",
                    "images",
                    "apps",
                    "games",
                    "archives",
                    "documents",
                    "other",
                ]
                .contains(&row.category.as_str())
        })
    {
        return Err(bad("Invalid download statistics."));
    }
    app.manager
        .sync_usage(device.to_string(), entries)
        .await
        .map_err(io_error)?;
    Ok(Json(app.manager.usage().await.map_err(io_error)?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Incoming {
    id: Uuid,
    name: String,
    total: u64,
}
async fn prepare_upload(
    State(app): State<Companion>,
    headers: HeaderMap,
    Json(x): Json<Incoming>,
) -> Http<Json<serde_json::Value>> {
    let device = app.authenticate(&headers).await?;
    let name = sanitize_filename(&x.name);
    if x.id.is_nil() || x.total > MAX_FILE || x.name.trim().is_empty() {
        return Err(bad("Invalid file metadata."));
    }
    let mut state = app.inner.lock().await;
    if let Some(transfer) = state.saved.transfers.iter().find(|t| t.id == x.id) {
        if transfer.device != device
            || !transfer.to_pc
            || transfer.name != name
            || transfer.total != x.total
        {
            return Err(missing());
        }
        return Ok(Json(transfer.public()));
    }
    if state.saved.transfers.len() >= 2000 {
        return Err(bad("Clear old transfers on the PC first."));
    }
    let folder = app
        .manager
        .snapshot()
        .settings
        .directory
        .join("NDM Shared")
        .join(x.id.to_string());
    fs::create_dir_all(&folder).await.map_err(io_error)?;
    let transfer = Transfer {
        id: x.id,
        device,
        to_pc: true,
        path: folder.join(&name),
        name,
        total: x.total,
        bytes: 0,
        phase: Phase::Ready,
        modified: None,
        error: None,
        speed: 0.0,
        sampled: None,
    };
    let view = transfer.public();
    state.saved.transfers.push(transfer);
    if let Err(error) = app.save(&state.saved).await {
        state.saved.transfers.pop();
        return Err(io_error(error));
    }
    Ok(Json(view))
}

fn find(state: &Runtime, id: Uuid, device: Uuid) -> Http<usize> {
    state
        .saved
        .transfers
        .iter()
        .position(|t| t.id == id && t.device == device)
        .ok_or_else(missing)
}
async fn transfer_info(
    State(app): State<Companion>,
    RoutePath(id): RoutePath<Uuid>,
    headers: HeaderMap,
) -> Http<Json<serde_json::Value>> {
    let device = app.authenticate(&headers).await?;
    let state = app.inner.lock().await;
    Ok(Json(
        state.saved.transfers[find(&state, id, device)?].public(),
    ))
}
async fn set_paused(app: Companion, device: Uuid, id: Uuid, paused: bool) -> Http<StatusCode> {
    let mut state = app.inner.lock().await;
    let index = find(&state, id, device)?;
    state.saved.transfers[index].pause(paused)?;
    app.save(&state.saved).await.map_err(io_error)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn pause_transfer(
    State(app): State<Companion>,
    RoutePath(id): RoutePath<Uuid>,
    headers: HeaderMap,
) -> Http<StatusCode> {
    let device = app.authenticate(&headers).await?;
    set_paused(app, device, id, true).await
}
async fn retry_transfer(
    State(app): State<Companion>,
    RoutePath(id): RoutePath<Uuid>,
    headers: HeaderMap,
) -> Http<StatusCode> {
    let device = app.authenticate(&headers).await?;
    set_paused(app, device, id, false).await
}

#[derive(Deserialize)]
struct Offset {
    offset: u64,
}
async fn write_chunk(
    State(app): State<Companion>,
    RoutePath(id): RoutePath<Uuid>,
    Query(Offset { offset }): Query<Offset>,
    headers: HeaderMap,
    body: Bytes,
) -> Http<Json<serde_json::Value>> {
    let device = app.authenticate(&headers).await?;
    if body.len() > CHUNK {
        return Err(bad("Chunk is too large."));
    }
    let mut state = app.inner.lock().await;
    let index = find(&state, id, device)?;
    let t = &mut state.saved.transfers[index];
    if !t.to_pc {
        return Err(missing());
    }
    if t.phase == Phase::Complete {
        return Ok(Json(t.public()));
    }
    if t.phase != Phase::Ready {
        return Err(unavailable());
    }
    let partial = t.partial();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&partial)
        .await
        .map_err(io_error)?;
    let stored = file.metadata().await.map_err(io_error)?.len();
    if stored > t.total {
        t.phase = Phase::Failed;
        t.error =
            Some("Partial file no longer matches. Clear the transfer on the PC and retry.".into());
        let response = t.public();
        app.save(&state.saved).await.map_err(io_error)?;
        return Ok(Json(response));
    }
    if offset != stored
        || offset > t.total
        || body.len() as u64 > t.total - offset
        || (body.is_empty() && offset != t.total)
    {
        t.bytes = stored;
        return Ok(Json(t.public()));
    }
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(io_error)?;
    file.write_all(&body).await.map_err(io_error)?;
    file.sync_data().await.map_err(io_error)?;
    drop(file);
    t.sample(offset + body.len() as u64);
    let mut published = None;
    if t.bytes == t.total {
        // Link first: a pre-existing file is never overwritten.
        if let Err(error) = fs::hard_link(&partial, &t.path).await {
            if error.kind() != std::io::ErrorKind::AlreadyExists
                || !same_file::is_same_file(&partial, &t.path).unwrap_or(false)
            {
                t.phase = Phase::Failed;
                t.error = Some(format!("Could not publish the received file: {error}"));
                let response = t.public();
                app.save(&state.saved).await.map_err(io_error)?;
                return Ok(Json(response));
            }
        }
        t.phase = Phase::Complete;
        published = Some(partial);
    }
    let view = t.public();
    app.save(&state.saved).await.map_err(io_error)?;
    if let Some(partial) = published {
        let _ = fs::remove_file(partial).await;
    }
    Ok(Json(view))
}

async fn read_chunk(
    State(app): State<Companion>,
    RoutePath(id): RoutePath<Uuid>,
    Query(Offset { offset }): Query<Offset>,
    headers: HeaderMap,
) -> Http<Response> {
    let device = app.authenticate(&headers).await?;
    let mut state = app.inner.lock().await;
    let index = find(&state, id, device)?;
    let t = &mut state.saved.transfers[index];
    if t.to_pc {
        return Err(missing());
    }
    if t.phase != Phase::Ready {
        return Err(unavailable());
    }
    if offset > t.total {
        return Err(bad("Invalid transfer offset."));
    }
    let mut file = fs::File::open(&t.path).await.map_err(io_error)?;
    let meta = file.metadata().await.map_err(io_error)?;
    if meta.len() != t.total || meta.modified().ok() != t.modified {
        t.phase = Phase::Failed;
        t.error = Some("Source file changed. Select it again to send the new copy.".into());
        app.save(&state.saved).await.map_err(io_error)?;
        return Err(bad("Source file changed. Select it again on the PC."));
    }
    let mut buffer = vec![0u8; (t.total - offset).min(CHUNK as u64) as usize];
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(io_error)?;
    file.read_exact(&mut buffer).await.map_err(io_error)?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        Body::from(buffer),
    )
        .into_response())
}

async fn acknowledge(
    State(app): State<Companion>,
    RoutePath(id): RoutePath<Uuid>,
    headers: HeaderMap,
    Json(Offset { offset }): Json<Offset>,
) -> Http<StatusCode> {
    let device = app.authenticate(&headers).await?;
    let mut state = app.inner.lock().await;
    let index = find(&state, id, device)?;
    let t = &mut state.saved.transfers[index];
    if t.to_pc || offset > t.total {
        return Err(bad("Invalid acknowledgement."));
    }
    if t.phase == Phase::Complete && offset == t.total {
        return Ok(StatusCode::NO_CONTENT);
    }
    if t.phase != Phase::Ready {
        return Err(unavailable());
    }
    if offset >= t.bytes {
        t.sample(offset);
    }
    if t.bytes == t.total {
        t.phase = Phase::Complete;
    }
    app.save(&state.saved).await.map_err(io_error)?;
    Ok(StatusCode::NO_CONTENT)
}

fn advertise(fingerprint: &str, port: u16) -> Result<(mdns_sd::ServiceDaemon, String)> {
    let label = format!("ndm-{}", &fingerprint[..16]);
    let info = mdns_sd::ServiceInfo::new(
        "_ndm._tcp.local.",
        &label,
        &format!("{label}.local."),
        "",
        port,
        None,
    )
    .map_err(err)?
    .enable_addr_auto();
    let fullname = info.get_fullname().to_string();
    let daemon = mdns_sd::ServiceDaemon::new().map_err(err)?;
    if let Err(error) = daemon.register(info) {
        let _ = daemon.shutdown();
        return Err(err(error));
    }
    Ok((daemon, fullname))
}

fn local_ip() -> Result<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(err)?;
    socket
        .connect((Ipv4Addr::new(192, 0, 2, 1), 9))
        .map_err(|_| "Connect this PC to Wi-Fi or Ethernet.".to_string())?;
    match socket.local_addr().map_err(err)?.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_unspecified() && !ip.is_loopback() => Ok(ip),
        _ => Err("No usable IPv4 LAN adapter found.".into()),
    }
}

fn lan_ips() -> Vec<Ipv4Addr> {
    let mut ips = Vec::new();
    let candidates = local_ip().ok().into_iter().chain(
        if_addrs::get_if_addrs()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|interface| match interface.ip() {
                std::net::IpAddr::V4(ip) => Some(ip),
                _ => None,
            }),
    );
    for ip in candidates.filter(Ipv4Addr::is_private) {
        if !ips.contains(&ip) {
            ips.push(ip);
        }
    }
    ips.truncate(3);
    ips
}

#[cfg(any(target_os = "windows", test))]
fn tailscale_ipv4(value: &str) -> Option<Ipv4Addr> {
    value.lines().find_map(|line| {
        let ip: Ipv4Addr = line.trim().parse().ok()?;
        let [a, b, ..] = ip.octets();
        (a == 100 && (64..=127).contains(&b)).then_some(ip)
    })
}

#[cfg(target_os = "windows")]
async fn tailscale_ip() -> Option<Ipv4Addr> {
    let path = PathBuf::from(std::env::var_os("ProgramFiles")?)
        .join("Tailscale")
        .join("tailscale.exe");
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::process::Command::new(path)
            .args(["ip", "-4"])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !output.status.success() {
        return None;
    }
    tailscale_ipv4(std::str::from_utf8(&output.stdout).ok()?)
}

#[cfg(not(target_os = "windows"))]
async fn tailscale_ip() -> Option<Ipv4Addr> {
    None
}

#[tauri::command]
pub async fn companion_view(companion: tauri::State<'_, Companion>) -> Result<serde_json::Value> {
    Ok(companion.view().await)
}
#[tauri::command]
pub async fn companion_enable(companion: tauri::State<'_, Companion>, enabled: bool) -> Result<()> {
    companion.enable(enabled).await
}
#[tauri::command]
pub async fn companion_pair(
    companion: tauri::State<'_, Companion>,
    remote: Option<bool>,
    bluetooth: Option<bool>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value> {
    use qrcode::{types::Color, QrCode};
    let bluetooth = bluetooth.unwrap_or(false);
    let remote = remote.unwrap_or(false);
    let tunnel = if bluetooth {
        None
    } else {
        tailscale_ip().await
    };
    if remote && tunnel.is_none() {
        return Err("Open Tailscale and sign in on the PC before making a remote QR code.".into());
    }
    let mut ips = if bluetooth { Vec::new() } else { lan_ips() };
    if let Some(tunnel) = tunnel {
        if remote {
            ips.insert(0, tunnel);
        } else {
            ips.push(tunnel);
        }
    }
    let mut state = companion.inner.lock().await;
    if state.server.is_none() {
        return Err("Enable phone connection first.".into());
    }
    let hosts: Vec<_> = ips
        .iter()
        .map(|ip| format!("{ip}:{}", state.port))
        .collect();
    #[cfg(not(windows))]
    let _ = app;
    let host = if bluetooth {
        #[cfg(windows)]
        {
            state.bluetooth = None;
            let listener = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                crate::companion_bluetooth::Listener::start(state.port, Some(app)),
            )
            .await
            .map_err(|_| "Bluetooth took too long. Turn it on in Windows and try again.")??;
            if !state.saved.bluetooth {
                state.saved.bluetooth = true;
                if let Err(error) = companion.save(&state.saved).await {
                    state.saved.bluetooth = false;
                    return Err(error);
                }
            }
            let address = listener.address.clone();
            state.bluetooth = Some(listener);
            address
        }
        #[cfg(not(windows))]
        return Err("Bluetooth pairing is available on Windows.".into());
    } else {
        hosts
            .first()
            .ok_or("Connect the PC to Wi-Fi, Ethernet or Tailscale first.")?
            .clone()
    };
    let expires = now() + PAIR_SECONDS;
    let code = secret();
    let data = format!(
        "NDM1:{}",
        serde_json::json!({"host":host,"hosts":hosts,"bluetooth":bluetooth,"pin":state.fingerprint,"code":code,"expires":expires})
    );
    let qr = QrCode::new(data.as_bytes()).map_err(err)?;
    let size = qr.width();
    let mut path = String::new();
    use std::fmt::Write;
    for y in 0..size {
        for x in 0..size {
            if qr[(x, y)] == Color::Dark {
                let _ = write!(path, "M{} {}h1v1h-1z", x + 4, y + 4);
            }
        }
    }
    state.pair = Some((sha(code), expires));
    Ok(
        serde_json::json!({"path":path,"size":size+8,"expires":expires,"host":host,"remote":remote,"bluetooth":bluetooth}),
    )
}
#[tauri::command]
pub async fn companion_revoke(companion: tauri::State<'_, Companion>, id: Uuid) -> Result<()> {
    companion.revoke(id).await
}
#[tauri::command]
pub async fn companion_send(
    companion: tauri::State<'_, Companion>,
    device: Uuid,
    files: Vec<PathBuf>,
) -> Result<()> {
    if files.is_empty() || files.len() > 32 {
        return Err("Choose between 1 and 32 files.".into());
    }
    let mut state = companion.inner.lock().await;
    if state.server.is_none()
        || !state
            .saved
            .devices
            .iter()
            .any(|d| d.id == device && d.expires.is_none_or(|t| t > now()))
    {
        return Err("Pair a phone and enable sharing first.".into());
    }
    let original = state.saved.transfers.len();
    if original + files.len() > 2000 {
        return Err("Clear old transfers first.".into());
    }
    let mut transfers = Vec::new();
    for path in files {
        if !path.is_absolute() {
            return Err("Choose local files.".into());
        }
        let meta = fs::metadata(&path).await.map_err(err)?;
        if !meta.is_file() || meta.len() > MAX_FILE {
            return Err("File is not readable or is too large.".into());
        }
        let name = sanitize_filename(&path.file_name().ok_or("Missing name.")?.to_string_lossy());
        transfers.push(Transfer {
            id: Uuid::new_v4(),
            device,
            to_pc: false,
            name,
            path,
            modified: meta.modified().ok(),
            total: meta.len(),
            bytes: 0,
            phase: Phase::Ready,
            error: None,
            speed: 0.0,
            sampled: None,
        });
    }
    state.saved.transfers.extend(transfers);
    if let Err(error) = companion.save(&state.saved).await {
        state.saved.transfers.truncate(original);
        return Err(error);
    }
    Ok(())
}
#[tauri::command]
pub async fn companion_transfer(
    companion: tauri::State<'_, Companion>,
    app: tauri::AppHandle,
    id: Uuid,
    action: String,
) -> Result<()> {
    let mut state = companion.inner.lock().await;
    let index = state
        .saved
        .transfers
        .iter()
        .position(|t| t.id == id)
        .ok_or("Transfer not found.")?;
    let transfer = &mut state.saved.transfers[index];
    match action.as_str() {
        "open" | "reveal" if transfer.to_pc && transfer.phase == Phase::Complete => {
            let path = transfer.path.clone();
            drop(state);
            if action == "reveal" {
                app.opener().reveal_item_in_dir(path).map_err(err)
            } else {
                app.opener()
                    .open_path(path.to_string_lossy(), None::<&str>)
                    .map_err(err)
            }
        }
        "cancel" | "retry" => {
            transfer.pause(action == "cancel").map_err(|e| e.1)?;
            companion.save(&state.saved).await
        }
        "clear" => {
            let path =
                (transfer.to_pc && transfer.phase != Phase::Complete).then(|| transfer.partial());
            state.saved.transfers.remove(index);
            companion.save(&state.saved).await?;
            if let Some(path) = path {
                let _ = fs::remove_file(path).await;
            }
            Ok(())
        }
        _ => Err("This action is not available.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn selects_only_tailscale_ipv4() {
        assert_eq!(
            tailscale_ipv4("2001:db8::1\n192.168.0.1\n100.72.0.12\r\n"),
            Some(Ipv4Addr::new(100, 72, 0, 12))
        );
        assert_eq!(tailscale_ipv4("100.63.0.1\n100.128.0.1"), None);
    }

    #[tokio::test]
    async fn pinned_pairing_transfer_resume_stats_and_revocation() {
        let root = tempfile::tempdir().unwrap();
        let download_root = root.path().join("downloads");
        let manager = Manager::open(
            root.path().join("queue"),
            ndm_core::Settings::new(download_root.clone()),
        )
        .await
        .unwrap();
        let state = Companion {
            inner: Arc::new(Mutex::new(Runtime::default())),
            dir: root.path().join("companion"),
            manager,
            bind_port: 0,
        };
        state.enable(true).await.unwrap();
        let code = secret();
        let port = {
            let mut current = state.inner.lock().await;
            current.pair = Some((sha(&code), now() + 120));
            current.port
        };
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let (cert, _) = state.identity().await.unwrap();
        let client = reqwest::Client::builder()
            .no_proxy()
            .resolve("ndm.local", address)
            .add_root_certificate(reqwest::Certificate::from_der(&cert).unwrap())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let url = format!("https://ndm.local:{port}");
        let device = Uuid::new_v4();
        let pairing = json!({"code":code,"name":"Test device","id":device,"remember":true});
        assert_eq!(
            client
                .get(format!("{url}/status"))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        let reply: Value = client
            .post(format!("{url}/pair"))
            .json(&pairing)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        let token = reply["token"].as_str().unwrap();
        assert_eq!(
            client
                .post(format!("{url}/pair"))
                .json(&pairing)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        let id = Uuid::new_v4();
        let upload = json!({"id":id,"name":"../../clip.txt","total":6});
        client
            .post(format!("{url}/transfer"))
            .bearer_auth(token)
            .json(&upload)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let send = |offset: usize, bytes: &'static str| {
            client
                .post(format!("{url}/transfer/{id}/chunk?offset={offset}"))
                .bearer_auth(token)
                .body(bytes)
        };
        let first: Value = send(0, "abc").send().await.unwrap().json().await.unwrap();
        assert_eq!(first["bytes"], 3);
        let duplicate: Value = send(0, "abc").send().await.unwrap().json().await.unwrap();
        assert_eq!(duplicate["bytes"], 3);
        let finished: Value = send(3, "def").send().await.unwrap().json().await.unwrap();
        assert_eq!(finished["phase"], "complete");
        let target = download_root
            .join("NDM Shared")
            .join(id.to_string())
            .join(sanitize_filename("../../clip.txt"));
        assert_eq!(fs::read(&target).await.unwrap(), b"abcdef");

        let outbound = Uuid::new_v4();
        let source = root.path().join("image.png");
        fs::write(&source, vec![42; CHUNK + 99]).await.unwrap();
        let modified = fs::metadata(&source).await.unwrap().modified().ok();
        state.inner.lock().await.saved.transfers.push(Transfer {
            id: outbound,
            device,
            to_pc: false,
            name: "image.png".into(),
            total: (CHUNK + 99) as u64,
            bytes: 0,
            phase: Phase::Ready,
            path: source,
            modified,
            error: None,
            speed: 0.0,
            sampled: None,
        });
        let chunk = |offset: usize| {
            client
                .get(format!("{url}/transfer/{outbound}/chunk?offset={offset}"))
                .bearer_auth(token)
        };
        let initial = chunk(0)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(initial.len(), CHUNK);
        let ack = |offset: usize| {
            client
                .post(format!("{url}/transfer/{outbound}/ack"))
                .bearer_auth(token)
                .json(&json!({"offset":offset}))
        };
        ack(CHUNK).send().await.unwrap().error_for_status().unwrap();
        let tail = chunk(CHUNK)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(tail.as_ref(), &[42; 99]);
        ack(CHUNK + 99)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
        let report = json!([{"hour":now()/3600,"bytes":57,"category":"images","device":"mobile"}]);
        for _ in 0..2 {
            let reply: Vec<Usage> = client
                .post(format!("{url}/usage"))
                .bearer_auth(token)
                .json(&report)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            assert_eq!(
                reply.iter().find(|x| x.device == "mobile").unwrap().bytes,
                57
            );
        }
        state.revoke(device).await.unwrap();
        assert_eq!(
            client
                .get(format!("{url}/status"))
                .bearer_auth(token)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            client
                .get(format!("{url}/transfer/{id}"))
                .bearer_auth(token)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        state.enable(false).await.unwrap();
    }
}

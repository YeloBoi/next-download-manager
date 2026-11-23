use ndm_core::validate_url;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const MAX_MESSAGE: usize = 256 * 1024;
pub const HOST: &str = "local.ndm.bridge";
pub const FIREFOX_ID: &str = "ndm@yeloboi.github.io";
pub const FIREFOX_MANIFEST: &str = "local.ndm.bridge.firefox.json";
pub const CHROMIUM_MANIFEST: &str = "local.ndm.bridge.json";
pub const CHROMIUM_ORIGIN: &str = "chrome-extension://llccemgchkhblimpddegikfekglcjoen/";

#[cfg(windows)]
mod registration;
#[cfg(windows)]
pub use registration::{register_hosts, unregister_hosts};

#[derive(Clone, Default)]
pub struct ConnectionState(std::sync::Arc<std::sync::Mutex<Option<std::time::Instant>>>);

#[derive(Serialize)]
pub struct ConnectionStatus {
    pub seen: bool,
    pub connected: bool,
}

impl ConnectionState {
    fn record(&self) {
        if let Ok(mut seen) = self.0.lock() {
            *seen = Some(std::time::Instant::now());
        }
    }

    pub fn status(&self) -> ConnectionStatus {
        let seen = self.0.lock().ok().and_then(|seen| *seen);
        ConnectionStatus {
            seen: seen.is_some(),
            connected: seen.is_some_and(|seen| seen.elapsed().as_secs() < 15),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u8,
    pub request_id: String,
    pub kind: String,
    pub urls: Vec<String>,
    pub start: bool,
    pub created_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub version: u8,
    pub request_id: String,
    pub accepted: bool,
    pub error: Option<String>,
}

impl Request {
    pub fn validate(&self) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if self.version != 1
            || self.request_id.len() > 64
            || self.request_id.is_empty()
            || self.urls.len() > 100
            || !["add", "ping"].contains(&self.kind.as_str())
            || (self.kind == "add" && self.urls.is_empty())
            || (self.kind == "ping" && (!self.urls.is_empty() || self.start))
            || self.created_at > now + 30
            || now.saturating_sub(self.created_at) > 60
        {
            return Err("Invalid bridge message.".into());
        }
        for url in &self.urls {
            if url.len() > 16384 {
                return Err("URL too long.".into());
            }
            validate_url(url).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut length = [0; 4];
    reader.read_exact(&mut length).map_err(|e| e.to_string())?;
    let length = u32::from_le_bytes(length) as usize;
    if length == 0 || length > MAX_MESSAGE {
        return Err("Native message exceeds the allowed size.".into());
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}
pub fn write_frame<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_MESSAGE {
        return Err("Native response exceeds the allowed size.".into());
    }
    writer
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(bytes).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

pub fn bridge_directory() -> Result<std::path::PathBuf, String> {
    dirs::data_local_dir()
        .map(|path| path.join("NDM").join("bridge"))
        .ok_or_else(|| "Cannot find local application data.".into())
}

// Local spool files are scoped to the current Windows user's application-data directory.
// A browser acknowledgement is written only after the queue's SQLite transaction commits.
pub async fn serve(manager: ndm_queue::Manager, connection: ConnectionState) -> Result<(), String> {
    let directory = bridge_directory()?;
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|e| e.to_string())?;
    loop {
        let mut files = tokio::fs::read_dir(&directory)
            .await
            .map_err(|e| e.to_string())?;
        while let Some(entry) = files.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("request")
                || !entry
                    .file_type()
                    .await
                    .map_err(|e| e.to_string())?
                    .is_file()
            {
                continue;
            }
            let result = async {
                if entry.metadata().await.map_err(|e| e.to_string())?.len() > MAX_MESSAGE as u64 {
                    return Err("Bridge request is too large.".into());
                }
                let bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
                let request: Request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                request.validate()?;
                connection.record();
                if request.kind == "add" {
                    manager
                        .execute(ndm_queue::Action::BrowserAdd {
                            urls: request.urls,
                            start: request.start,
                        })
                        .await?;
                }
                Ok(())
            }
            .await;
            let response = Response {
                version: 1,
                request_id: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into(),
                accepted: result.is_ok(),
                error: result.err(),
            };
            let bytes = serde_json::to_vec(&response).map_err(|e| e.to_string())?;
            tokio::fs::write(path.with_extension("response.tmp"), bytes)
                .await
                .map_err(|e| e.to_string())?;
            tokio::fs::rename(
                path.with_extension("response.tmp"),
                path.with_extension("response"),
            )
            .await
            .map_err(|e| e.to_string())?;
            let _ = tokio::fs::remove_file(&path).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connection_status_requires_a_recent_valid_handshake() {
        let state = ConnectionState::default();
        assert!(!state.status().seen);
        assert!(!state.status().connected);
        state.record();
        assert!(state.status().seen);
        assert!(state.status().connected);
        *state.0.lock().expect("state") =
            Some(std::time::Instant::now() - std::time::Duration::from_secs(16));
        assert!(state.status().seen);
        assert!(!state.status().connected);
    }

    #[test]
    fn native_framing_and_invalid_requests() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, b"{}").expect("frame");
        assert_eq!(read_frame(&mut bytes.as_slice()).expect("read"), b"{}");
        assert!(read_frame(&mut u32::MAX.to_le_bytes().as_slice()).is_err());
        assert!(read_frame(&mut [2, 0, 0, 0, 1].as_slice()).is_err());
        let request = Request {
            version: 2,
            request_id: "test".into(),
            kind: "add".into(),
            urls: vec!["https://example.com/a".into()],
            start: false,
            created_at: 0,
        };
        assert!(request.validate().is_err());
        let ping = Request {
            version: 1,
            request_id: "ping".into(),
            kind: "ping".into(),
            urls: vec!["https://example.com/a".into()],
            start: false,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_secs(),
        };
        assert!(
            ping.validate().is_err(),
            "a ping cannot carry download URLs"
        );
        assert!(serde_json::from_str::<Request>(r#"{"version":1,"request_id":"a","urls":["https://example.com"],"start":false,"command":"calc"}"#).is_err());
    }
}

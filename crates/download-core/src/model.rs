use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("{0}")]
    Invalid(String),
    #[error("Network request failed: {0}")]
    Network(String),
    #[error("HTTP {0}")]
    Http(u16),
    #[error("The server returned a web page. Use a direct file URL.")]
    Html,
    #[error("Cannot safely resume: {0}. Restart explicitly to discard saved data.")]
    Resume(String),
    #[error("The server did not honor the requested byte range.")]
    Range,
    #[error("The transfer ended before all expected bytes arrived.")]
    Incomplete,
    #[error("The SHA-256 does not match. The file has not been published.")]
    HashMismatch,
    #[error("Download interrupted; saved bytes have been retained.")]
    Cancelled,
    #[error("File operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Saved state is invalid: {0}")]
    State(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DownloadError>;

pub fn validate_url(input: &str) -> Result<Url> {
    let url = Url::parse(input.trim())
        .map_err(|_| DownloadError::Invalid("Enter a valid HTTP or HTTPS URL.".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(DownloadError::Invalid(
            "Only HTTP(S) URLs without embedded credentials are supported.".into(),
        ));
    }
    Ok(url)
}

pub fn redacted_source(input: &str) -> String {
    match validate_url(input) {
        Ok(mut url) => {
            url.set_query(None);
            url.set_fragment(None);
            // A signed path can also be sensitive; expose only the origin in UI/logs.
            url.set_path("/");
            url.to_string()
        }
        Err(_) => "Invalid URL".into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Queued,
    Connecting,
    Downloading,
    Paused,
    Stopped,
    Completed,
    Failed,
}

impl Status {
    pub fn active(self) -> bool {
        matches!(self, Self::Connecting | Self::Downloading)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub id: String,
    pub url: String,
    pub name: String,
    pub directory: PathBuf,
    pub status: Status,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: f64,
    pub connections: usize,
    pub resumable: bool,
    pub error: Option<String>,
    pub path: Option<PathBuf>,
    pub sha256: Option<String>,
    pub expected_sha256: Option<String>,
    #[serde(default)]
    pub resume_with_queue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub directory: PathBuf,
    pub language: String,
    pub theme: String,
    pub compact: bool,
    pub concurrent_downloads: usize,
    pub connections_per_file: usize,
    pub global_connections: usize,
    pub host_connections: usize,
    pub speed_limit: u64,
    pub resume_on_launch: bool,
    pub stop_on_error: bool,
    pub schedule_enabled: bool,
    pub schedule_start: u16,
    pub schedule_end: u16,
    pub close_to_tray: bool,
    #[serde(default)]
    pub confirmless_add: bool,
}

impl Settings {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            language: "en".into(),
            theme: "dark".into(),
            compact: false,
            concurrent_downloads: 1,
            connections_per_file: 1,
            global_connections: 16,
            host_connections: 8,
            speed_limit: 0,
            resume_on_launch: false,
            stop_on_error: false,
            schedule_enabled: false,
            schedule_start: 0,
            schedule_end: 0,
            close_to_tray: false,
            confirmless_add: false,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if !(1..=8).contains(&self.concurrent_downloads)
            || !(1..=8).contains(&self.connections_per_file)
            || !(1..=64).contains(&self.global_connections)
            || !(1..=32).contains(&self.host_connections)
            || self.schedule_start >= 1440
            || self.schedule_end >= 1440
            || !["en", "fa"].contains(&self.language.as_str())
            || !["dark", "light", "system"].contains(&self.theme.as_str())
            || !self.directory.is_absolute()
        {
            return Err(DownloadError::Invalid(
                "One or more settings are outside their allowed range.".into(),
            ));
        }
        Ok(())
    }
}

pub fn file_category(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()).as_deref() {
        Some("mp4" | "mkv" | "webm" | "avi" | "mov" | "m4v") => "video",
        Some("mp3" | "wav" | "ogg" | "m4a" | "flac" | "aac") => "audio",
        Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "svg") => "images",
        Some("apk" | "exe" | "msi" | "msix" | "dmg" | "deb" | "rpm" | "appimage") => "apps",
        Some("iso" | "rom" | "xci" | "nsp" | "gba" | "nds") => "games",
        Some("zip" | "rar" | "7z" | "tar" | "gz" | "xz") => "archives",
        Some("pdf" | "doc" | "docx" | "txt" | "xls" | "xlsx" | "ppt" | "pptx" | "epub") => "documents",
        _ => "other",
    }
}

pub fn suggested_directory(root: &std::path::Path, name: &str) -> PathBuf {
    let folder = match file_category(name) {
        "video" => "Videos", "audio" => "Music", "images" => "Images",
        "apps" => "Programs", "games" => "Games", "archives" => "Archives",
        "documents" => "Documents", _ => return root.to_path_buf(),
    };
    root.join(folder)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    pub hour: i64,
    pub category: String,
    pub bytes: u64,
    pub device: String,
}

pub fn within_schedule(minute: u16, start: u16, end: u16) -> bool {
    if start == end {
        true
    } else if start < end {
        minute >= start && minute < end
    } else {
        minute >= start || minute < end
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default = "legacy_user_paused")]
    pub user_paused: bool,
    pub downloads: Vec<Download>,
    pub settings: Settings,
    pub running: bool,
    pub draining: bool,
}

fn legacy_user_paused() -> bool {
    // Older queues did not record stop intent; require an explicit start after migration.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_queue_does_not_lose_an_explicit_user_stop() {
        let value = serde_json::json!({
            "downloads": [],
            "settings": Settings::new(std::env::temp_dir()),
            "running": false,
            "draining": false,
        });
        let snapshot: Snapshot = serde_json::from_value(value).expect("legacy queue");
        assert!(snapshot.user_paused);
    }
    #[test]
    fn schedule_across_midnight() {
        assert!(within_schedule(30, 1380, 120));
        assert!(within_schedule(1400, 1380, 120));
        assert!(!within_schedule(120, 1380, 120));
        assert!(!within_schedule(600, 1380, 120));
    }
    #[test]
    fn credentials_and_schemes_rejected() {
        for input in [
            "file:///C:/Windows",
            "javascript:alert(1)",
            "https://user:pass@example.com/a",
        ] {
            assert!(validate_url(input).is_err());
        }
        assert_eq!(
            redacted_source("https://example.com/secret?token=hidden"),
            "https://example.com/"
        );
    }
}

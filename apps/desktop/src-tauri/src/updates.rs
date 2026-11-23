use ndm_queue::{Action, Manager};
use reqwest::Client;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

const REPO: &str = "YeloBoi/next-download-manager";
const MAX_INSTALLER: u64 = 200 * 1024 * 1024;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    digest: Option<String>,
}
#[derive(Serialize)]
pub struct UpdateInfo {
    version: String,
    notes: String,
    installable: bool,
    portable: bool,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent(concat!("NDM/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::custom(|redirect| {
            if redirect.previous().len() < 5
                && redirect.url().scheme() == "https"
                && matches!(
                    redirect.url().host_str(),
                    Some(
                        "github.com"
                            | "api.github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                )
            {
                redirect.follow()
            } else {
                redirect.error("Untrusted update redirect")
            }
        }))
        .build()
        .map_err(error)
}

async fn latest(client: &Client) -> Result<Option<(Version, Release)>, String> {
    let response = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(error)?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let release: Release = response
        .error_for_status()
        .map_err(error)?
        .json()
        .await
        .map_err(error)?;
    let version = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .ok_or("Invalid release tag.")?,
    )
    .map_err(error)?;
    let current = Version::parse(env!("CARGO_PKG_VERSION")).map_err(error)?;
    Ok((version > current).then_some((version, release)))
}

fn installed() -> bool {
    cfg!(all(windows, target_arch = "x86_64"))
        && std::env::current_exe().is_ok_and(|path| path.with_file_name("uninstall.exe").is_file())
}

fn installer<'a>(release: &'a Release, version: &Version) -> Option<(&'a Asset, &'a str)> {
    let name = format!("NDM_{version}_x64-setup.exe");
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == name && (1..=MAX_INSTALLER).contains(&asset.size))?;
    let digest = asset.digest.as_deref()?.strip_prefix("sha256:")?;
    (digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())).then_some((asset, digest))
}

#[tauri::command]
pub async fn check_update() -> Result<Option<UpdateInfo>, String> {
    let Some((version, release)) = latest(&client()?).await? else {
        return Ok(None);
    };
    let portable = !installed();
    Ok(Some(UpdateInfo {
        version: version.to_string(),
        installable: !portable && installer(&release, &version).is_some(),
        notes: release
            .body
            .unwrap_or_default()
            .chars()
            .take(24000)
            .collect(),
        portable,
    }))
}

#[tauri::command]
pub fn open_release(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(
            format!("https://github.com/{REPO}/releases/latest"),
            None::<&str>,
        )
        .map_err(error)
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    manager: State<'_, Manager>,
    version: String,
) -> Result<(), String> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static BUSY: AtomicBool = AtomicBool::new(false);
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            BUSY.store(false, Ordering::Release);
        }
    }
    if !installed() {
        return Err("Use the portable archive from GitHub Releases.".into());
    }
    if BUSY.swap(true, Ordering::AcqRel) {
        return Err("An update is already downloading.".into());
    }
    let _reset = Reset;
    let client = client()?;
    let (actual, release) = latest(&client).await?.ok_or("No update available.")?;
    if actual.to_string() != version {
        return Err("A different update is now available. Check again.".into());
    }
    let (asset, expected) =
        installer(&release, &actual).ok_or("The release has no verifiable installer.")?;
    let url = format!(
        "https://github.com/{REPO}/releases/download/{}/{}",
        release.tag_name, asset.name
    );
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(error)?
        .error_for_status()
        .map_err(error)?;
    let folder = dirs::data_local_dir()
        .ok_or("Cannot find application data.")?
        .join("NDM")
        .join("updates")
        .join(Uuid::new_v4().to_string());
    tokio::fs::create_dir_all(&folder).await.map_err(error)?;
    let path = folder.join(&asset.name);
    let result = async {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
            .map_err(error)?;
        let mut hash = Sha256::new();
        let mut received = 0;
        while let Some(chunk) = response.chunk().await.map_err(error)? {
            received += chunk.len() as u64;
            if received > asset.size {
                return Err("Update size does not match GitHub.".into());
            }
            hash.update(&chunk);
            file.write_all(&chunk).await.map_err(error)?;
            let _ = app.emit("ndm://update-progress", (received, asset.size));
        }
        if received != asset.size
            || !format!("{:x}", hash.finalize()).eq_ignore_ascii_case(expected)
        {
            return Err("Update integrity verification failed.".into());
        }
        file.sync_all().await.map_err(error)?;
        drop(file);
        // NSIS may require the running executable to close before replacing it.
        manager.execute(Action::Shutdown).await?;
        let launch = std::process::Command::new(&path)
            .args(["/P", "/R", "/UPDATE"])
            .spawn();
        if let Err(error) = launch {
            let _ = app.emit("ndm://error", error.to_string());
            let _ = tokio::fs::remove_dir_all(&folder).await;
            app.restart();
        }
        app.exit(0);
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_dir_all(folder).await;
    }
    result
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_exact_asset_and_digest() {
        let version = Version::parse("0.1.0-alpha.10").unwrap();
        let mut release = Release {
            tag_name: format!("v{version}"),
            body: None,
            assets: vec![Asset {
                name: format!("NDM_{version}_x64-setup.exe"),
                size: 25,
                digest: None,
            }],
        };
        assert!(installer(&release, &version).is_none());
        release.assets[0].digest = Some(format!("sha256:{}", "a".repeat(64)));
        assert!(installer(&release, &version).is_some());
        assert!(version > Version::parse("0.1.0-alpha.9").unwrap());
    }
}

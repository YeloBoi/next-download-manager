use crate::{
    bridge_directory, CHROMIUM_MANIFEST, CHROMIUM_ORIGIN, FIREFOX_ID, FIREFOX_MANIFEST, HOST,
};
use std::{os::windows::process::CommandExt, path::Path, process::Command};

const CHROMIUM_KEYS: [&str; 2] = ["Google\\Chrome", "Microsoft\\Edge"];
const FIREFOX_KEY: &str = "Mozilla";

fn registry(browser: &str) -> String {
    format!("HKCU\\Software\\{browser}\\NativeMessagingHosts\\{HOST}")
}

fn reg() -> Result<Command, String> {
    let windows = std::env::var_os("SystemRoot").ok_or("Cannot locate Windows.")?;
    let mut command = Command::new(Path::new(&windows).join("System32").join("reg.exe"));
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    Ok(command)
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if std::fs::read(path).is_ok_and(|previous| previous == bytes) {
        return Ok(());
    }
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = std::fs::write(&temporary, bytes).and_then(|_| std::fs::rename(&temporary, path));
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result.map_err(|e| e.to_string())
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|c| (b'a'..=b'p').contains(&c))
}

pub fn register_hosts(host: &Path, additional_id: Option<&str>) -> Result<(), String> {
    if !host.is_absolute() || !host.is_file() {
        return Err("NDM native host is missing. Reinstall the current NDM release.".into());
    }
    if additional_id.is_some_and(|id| !valid_id(id)) {
        return Err("Invalid Chrome or Edge extension ID.".into());
    }
    let directory = bridge_directory()?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let legacy = directory.join("allowed-origins.json");
    // Carry forward only origins explicitly registered by older installers or the setup script.
    let mut origins: Vec<String> = std::fs::read(&legacy)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<String>>(&bytes).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|origin| {
            origin
                .strip_prefix("chrome-extension://")
                .and_then(|id| id.strip_suffix('/'))
                .is_some_and(valid_id)
        })
        .collect();
    if let Some(id) = additional_id {
        origins.push(format!("chrome-extension://{id}/"));
    }
    origins.push(CHROMIUM_ORIGIN.to_owned());
    origins.sort_unstable();
    origins.dedup();
    if additional_id.is_some() {
        write_json(&legacy, &serde_json::json!(origins))?;
    }
    let chromium = serde_json::json!({"name": HOST, "description": "NDM native messaging host",
        "path": host, "type": "stdio", "allowed_origins": origins});
    let firefox = serde_json::json!({"name": HOST, "description": "NDM native messaging host",
        "path": host, "type": "stdio", "allowed_extensions": [FIREFOX_ID]});
    let mut failed = Vec::new();
    for (filename, manifest, browsers) in [
        (CHROMIUM_MANIFEST, chromium, CHROMIUM_KEYS.as_slice()),
        (
            FIREFOX_MANIFEST,
            firefox,
            std::slice::from_ref(&FIREFOX_KEY),
        ),
    ] {
        let path = directory.join(filename);
        write_json(&path, &manifest)?;
        for browser in browsers {
            let output = reg()?
                .args(["ADD", &registry(browser), "/ve", "/t", "REG_SZ", "/d"])
                .arg(&path)
                .arg("/f")
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                failed.push(*browser);
            }
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Windows could not register the NDM native host for {}.",
            failed.join(", ")
        ))
    }
}

pub fn unregister_hosts(host: &Path) -> Result<(), String> {
    let directory = bridge_directory()?;
    for (filename, browsers) in [
        (CHROMIUM_MANIFEST, CHROMIUM_KEYS.as_slice()),
        (FIREFOX_MANIFEST, std::slice::from_ref(&FIREFOX_KEY)),
    ] {
        let path = directory.join(filename);
        let Some(manifest) = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        else {
            continue;
        };
        // A portable copy or another install may currently own the registration.
        if !manifest["path"]
            .as_str()
            .is_some_and(|owner| Path::new(owner) == host)
        {
            continue;
        }
        for browser in browsers {
            let key = registry(browser);
            let existing = reg()?
                .args(["QUERY", &key, "/ve"])
                .output()
                .map_err(|e| e.to_string())?;
            if existing.status.success()
                && !reg()?
                    .args(["DELETE", &key, "/f"])
                    .output()
                    .map_err(|e| e.to_string())?
                    .status
                    .success()
            {
                return Err(format!("Windows could not unregister NDM for {browser}."));
            }
        }
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

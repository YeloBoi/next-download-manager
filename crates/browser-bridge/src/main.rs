use ndm_browser_bridge::{
    bridge_directory, read_frame, write_frame, Request, Response, MAX_MESSAGE,
};

#[tokio::main]
async fn main() {
    // Each browser supplies its caller identity differently.
    loop {
        if let Err(error) = run().await {
            let response = Response {
                version: 1,
                request_id: String::new(),
                accepted: false,
                error: Some(error),
            };
            if let Ok(bytes) = serde_json::to_vec(&response) {
                let _ = write_frame(&mut std::io::stdout().lock(), &bytes);
            }
            break;
        }
    }
}

async fn run() -> Result<(), String> {
    let directory = bridge_directory()?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    if firefox_caller(&args, &directory) {
        let manifest: serde_json::Value = serde_json::from_slice(
            &tokio::fs::read(directory.join("local.ndm.bridge.firefox.json"))
                .await
                .map_err(|_| "Register the Firefox native host first.".to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let allowed = manifest["allowed_extensions"]
            .as_array()
            .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(args[1].as_str())));
        if !allowed {
            return Err("This Firefox extension is not allowed to connect.".into());
        }
    } else {
        let allowed: Vec<String> = serde_json::from_slice(
            &tokio::fs::read(directory.join("allowed-origins.json"))
                .await
                .map_err(|_| {
                    "Native host is not registered. Follow the extension setup guide.".to_string()
                })?,
        )
        .map_err(|e| e.to_string())?;
        let origin = args.first().ok_or("Missing browser origin.")?;
        if !allowed.contains(origin) {
            return Err("This extension is not allowed to connect.".into());
        }
    }
    let bytes = read_frame(&mut std::io::stdin().lock())?;
    let request: Request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    request.validate()?;
    let id = uuid::Uuid::new_v4().to_string();
    let path = directory.join(&id);
    tokio::fs::write(path.with_extension("request.tmp"), &bytes)
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::rename(
        path.with_extension("request.tmp"),
        path.with_extension("request"),
    )
    .await
    .map_err(|e| e.to_string())?;
    // Launch only the sibling desktop executable, never a URL or a shell command.
    // Pings do not start the application; only an explicit download request does.
    if request.kind == "add" {
        let desktop = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("ndm.exe");
        if let Err(error) = tokio::process::Command::new(desktop)
            .arg("--browser-request")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            let _ = tokio::fs::remove_file(path.with_extension("request")).await;
            return Err(format!(
                "Could not open NDM: {error}. Keep ndm.exe beside the native host."
            ));
        }
    }
    let started = std::time::Instant::now();
    loop {
        if let Ok(bytes) = tokio::fs::read(path.with_extension("response")).await {
            if bytes.len() > MAX_MESSAGE {
                return Err("Invalid desktop response.".into());
            }
            let mut response: Response =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            response.request_id = request.request_id;
            let _ = tokio::fs::remove_file(path.with_extension("response")).await;
            return write_frame(
                &mut std::io::stdout().lock(),
                &serde_json::to_vec(&response).map_err(|e| e.to_string())?,
            );
        }
        if started.elapsed()
            > std::time::Duration::from_secs(if request.kind == "ping" { 2 } else { 30 })
        {
            let _ = tokio::fs::remove_file(path.with_extension("request")).await;
            if request.kind == "ping" {
                let response = Response {
                    version: 1,
                    request_id: request.request_id,
                    accepted: false,
                    error: Some("Open NDM to connect automatically.".into()),
                };
                return write_frame(
                    &mut std::io::stdout().lock(),
                    &serde_json::to_vec(&response).map_err(|e| e.to_string())?,
                );
            }
            return Err("NDM did not acknowledge the request. Open the desktop app and try again; the browser download has been retained.".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

fn firefox_caller(args: &[String], directory: &std::path::Path) -> bool {
    args.len() == 2
        && std::path::Path::new(&args[0]) == directory.join("local.ndm.bridge.firefox.json")
        && args[1] == "ndm@yeloboi.github.io"
}

#[cfg(test)]
mod tests {
    use super::firefox_caller;

    #[test]
    fn firefox_requires_registered_manifest_and_exact_addon_id() {
        let dir = std::path::Path::new("bridge");
        let manifest = dir
            .join("local.ndm.bridge.firefox.json")
            .to_string_lossy()
            .into_owned();
        assert!(firefox_caller(
            &[manifest.clone(), "ndm@yeloboi.github.io".into()],
            dir
        ));
        assert!(!firefox_caller(
            &[manifest, "other@example.com".into()],
            dir
        ));
        assert!(!firefox_caller(
            &["unregistered.json".into(), "ndm@yeloboi.github.io".into()],
            dir
        ));
        assert!(!firefox_caller(&[], dir));
    }
}

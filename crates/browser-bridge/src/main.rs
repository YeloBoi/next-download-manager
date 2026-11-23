use ndm_browser_bridge::{
    bridge_directory, read_frame, write_frame, Request, Response, CHROMIUM_MANIFEST, FIREFOX_ID,
    FIREFOX_MANIFEST, MAX_MESSAGE,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(windows)]
    if let [mode, rest @ ..] = args.as_slice() {
        if matches!(mode.as_str(), "--register" | "--unregister") {
            let result = configure(mode, rest);
            if let Err(error) = result {
                eprintln!("{error}");
                std::process::exit(1);
            }
            return;
        }
    }
    if let Err(error) = run(&args).await {
        let _ = reply(&failure(String::new(), error));
    }
}

#[cfg(windows)]
fn configure(mode: &str, args: &[String]) -> Result<(), String> {
    let host = std::env::current_exe().map_err(|e| e.to_string())?;
    match (mode, args) {
        ("--unregister", []) => ndm_browser_bridge::unregister_hosts(&host),
        ("--register", []) => ndm_browser_bridge::register_hosts(&host, None),
        ("--register", [id]) => ndm_browser_bridge::register_hosts(&host, Some(id)),
        _ => Err(
            "Use --register with an optional extension ID, or --unregister without arguments."
                .into(),
        ),
    }
}

fn failure(request_id: String, error: String) -> Response {
    Response {
        version: 1,
        request_id,
        accepted: false,
        error: Some(error),
    }
}

fn reply(response: &Response) -> Result<(), String> {
    write_frame(
        &mut std::io::stdout().lock(),
        &serde_json::to_vec(response).map_err(|e| e.to_string())?,
    )
}

async fn run(args: &[String]) -> Result<(), String> {
    let directory = bridge_directory()?;
    let (filename, field, caller) = if firefox_caller(args, &directory) {
        (FIREFOX_MANIFEST, "allowed_extensions", FIREFOX_ID)
    } else {
        (
            CHROMIUM_MANIFEST,
            "allowed_origins",
            args.first().ok_or("Missing browser origin.")?.as_str(),
        )
    };
    let manifest: serde_json::Value = serde_json::from_slice(
        &tokio::fs::read(directory.join(filename))
            .await
            .map_err(|_| {
                "Native host is not registered. Open NDM and repair the browser connection."
                    .to_string()
            })?,
    )
    .map_err(|e| e.to_string())?;
    if !manifest[field]
        .as_array()
        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(caller)))
    {
        return Err("This extension is not allowed to connect.".into());
    }
    while let Ok(bytes) = read_frame(&mut std::io::stdin().lock()) {
        let request: Request = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let response = match request.validate() {
            Ok(()) => forward(&directory, &request, &bytes).await,
            Err(error) => Err(error),
        };
        let response = response.unwrap_or_else(|error| failure(request.request_id, error));
        reply(&response)?;
    }
    Ok(())
}

async fn forward(
    directory: &std::path::Path,
    request: &Request,
    bytes: &[u8],
) -> Result<Response, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let path = directory.join(&id);
    // Launch only the sibling desktop executable, never a URL or a shell command.
    // Pings do not start the application; only an explicit download request does.
    if request.kind == "add" {
        let desktop = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("ndm.exe");
        let mut command = tokio::process::Command::new(desktop);
        // Firefox kills its native-host job after a one-shot reply; NDM must outlive it.
        #[cfg(windows)]
        command.creation_flags(0x01000000); // CREATE_BREAKAWAY_FROM_JOB
        if let Err(error) = command
            .arg("--browser-request")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            return Err(format!(
                "Could not open NDM: {error}. Keep ndm.exe beside the native host."
            ));
        }
    }
    tokio::fs::write(path.with_extension("request.tmp"), bytes)
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::rename(
        path.with_extension("request.tmp"),
        path.with_extension("request"),
    )
    .await
    .map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    loop {
        if let Ok(bytes) = tokio::fs::read(path.with_extension("response")).await {
            if bytes.len() > MAX_MESSAGE {
                return Err("Invalid desktop response.".into());
            }
            let mut response: Response =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            response.request_id.clone_from(&request.request_id);
            let _ = tokio::fs::remove_file(path.with_extension("response")).await;
            return Ok(response);
        }
        if started.elapsed()
            > std::time::Duration::from_secs(if request.kind == "ping" { 2 } else { 30 })
        {
            let _ = tokio::fs::remove_file(path.with_extension("request")).await;
            if request.kind == "ping" {
                return Err("Open NDM to connect automatically.".into());
            }
            return Err("NDM did not acknowledge the request. Open the desktop app and try again; the browser download has been retained.".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

fn firefox_caller(args: &[String], directory: &std::path::Path) -> bool {
    args.len() == 2
        && std::path::Path::new(&args[0]) == directory.join(FIREFOX_MANIFEST)
        && args[1] == FIREFOX_ID
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

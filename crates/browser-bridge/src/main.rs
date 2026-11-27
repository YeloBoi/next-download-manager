use ndm_browser_bridge::{
    bridge_directory, read_frame, write_frame, Request, Response, CHROMIUM_MANIFEST, FIREFOX_ID,
    FIREFOX_MANIFEST, MAX_MESSAGE,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(any(windows, target_os = "macos"))]
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

#[cfg(any(windows, target_os = "macos"))]
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

#[cfg(windows)]
async fn relaunch_in_desktop_session() -> Result<(), String> {
    let desktop = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .with_file_name("ndm.exe");
    let windows = std::env::var_os("SystemRoot").ok_or("Cannot locate Windows.")?;
    let powershell = std::path::Path::new(&windows)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let mut command = tokio::process::Command::new(powershell);
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", r#"
$ErrorActionPreference = 'Stop'
$service = New-Object -ComObject 'Schedule.Service'
$service.Connect()
$root = $service.GetFolder('\')
$definition = $service.NewTask(0)
$definition.RegistrationInfo.Description = 'Open NDM in the active desktop session after an explicit browser request.'
$definition.Principal.LogonType = 3
$definition.Principal.RunLevel = 0
$definition.Settings.AllowDemandStart = $true
$definition.Settings.DisallowStartIfOnBatteries = $false
$definition.Settings.StopIfGoingOnBatteries = $false
$definition.Settings.ExecutionTimeLimit = 'PT0S'
$action = $definition.Actions.Create(0)
$action.Path = $env:NDM_DESKTOP_EXECUTABLE
$action.Arguments = '--browser-request'
$name = 'NDM Browser Launch ' + [Guid]::NewGuid().ToString()
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value
# 2 = create new; 3 = use the signed-in user's existing interactive token.
$task = $root.RegisterTaskDefinition($name, $definition, 2, $user, $null, 3, $null)
try {
    $instance = $task.Run($null)
    for ($i=0; $i -lt 20 -and $instance.State -eq 2; $i++) {
        Start-Sleep -Milliseconds 150
        $instance.Refresh()
    }
    if ($instance.State -eq 2) { throw 'NDM did not start in the current Windows session.' }
} finally {
    $root.DeleteTask($name, 0)
}
"#])
        .env("NDM_DESKTOP_EXECUTABLE", desktop)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null());
    let output = tokio::time::timeout(std::time::Duration::from_secs(10), command.output())
        .await
        .map_err(|_| "Could not open NDM: Windows desktop launch timed out.".to_owned())?
        .map_err(|error| format!("Could not open NDM in the Windows desktop session: {error}."))?;
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Could not open NDM in the Windows desktop session: {}",
            error.trim()
        ))
    }
}

async fn run(args: &[String]) -> Result<(), String> {
    let directory = bridge_directory()?;
    let firefox = firefox_caller(args, &directory);
    let (filename, field, caller) = if firefox {
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
            Ok(()) => forward(&directory, &request, &bytes, firefox).await,
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
    _firefox: bool,
) -> Result<Response, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let path = directory.join(&id);
    // Launch the installed desktop executable or its enclosing macOS app bundle.
    // Pings do not start the application; only an explicit download request does.
    let mut desktop = if request.kind == "add" {
        let desktop = std::env::current_exe().map_err(|e| e.to_string())?;
        #[cfg(target_os = "macos")]
        let mut command = {
            let bundle = desktop
                .ancestors()
                .nth(3)
                .filter(|path| path.extension().is_some_and(|ext| ext == "app"))
                .ok_or("Install NDM.app before connecting the extension.")?;
            let mut command = tokio::process::Command::new("/usr/bin/open");
            command.arg("-a").arg(bundle).arg("--args");
            command
        };
        #[cfg(not(target_os = "macos"))]
        let mut command = tokio::process::Command::new(desktop.with_file_name("ndm.exe"));
        // Firefox kills its native-host job after a one-shot reply; NDM must outlive it.
        #[cfg(windows)]
        if _firefox {
            command.creation_flags(0x01000000); // CREATE_BREAKAWAY_FROM_JOB
        }
        let child = command
            .arg("--browser-request")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|error| {
                format!("Could not open NDM: {error}. Check your desktop installation.")
            })?;
        Some(child)
    } else {
        None
    };
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
        // An existing instance makes the new process exit successfully after restoring it.
        // A failed startup must report its exit status instead of leaving the browser waiting.
        if let Some(status) = desktop
            .as_mut()
            .and_then(|child| child.try_wait().ok().flatten())
        {
            if !status.success() {
                #[cfg(windows)]
                if _firefox && status.code() == Some(101) {
                    // WebView2 can disconnect during a Firefox-started cold launch.
                    // Start outside Firefox's process tree and keep waiting for the queue.
                    if let Err(error) = relaunch_in_desktop_session().await {
                        let _ = tokio::fs::remove_file(path.with_extension("request")).await;
                        return Err(error);
                    }
                    desktop = None;
                    continue;
                }
                let _ = tokio::fs::remove_file(path.with_extension("request")).await;
                return Err(format!("Could not open NDM: {status}."));
            }
            desktop = None;
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
    let [manifest, id] = args else { return false };
    if id != FIREFOX_ID {
        return false;
    }
    let path = std::path::Path::new(manifest);
    #[cfg(target_os = "macos")]
    if ndm_browser_bridge::mac_firefox_manifest().is_ok_and(|registered| path == registered) {
        return true;
    }
    path == directory.join(FIREFOX_MANIFEST)
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

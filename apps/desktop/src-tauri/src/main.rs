#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use ndm_core::{Settings, Snapshot, Status};
use ndm_queue::{Action, Manager};
use tauri::{Emitter, Manager as TauriManager, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
fn browser_connection(
    connection: State<'_, ndm_browser_bridge::ConnectionState>,
    repair: Option<bool>,
) -> Result<ndm_browser_bridge::ConnectionStatus, String> {
    if repair == Some(true) {
        setup_browser_hosts()?;
    }
    Ok(connection.status())
}

fn setup_browser_hosts() -> Result<(), String> {
    #[cfg(windows)]
    ndm_browser_bridge::register_hosts(
        &std::env::current_exe()
            .map_err(|e| e.to_string())?
            .with_file_name("ndm-native-host.exe"),
        None,
    )?;
    Ok(())
}

#[tauri::command]
fn extension_guide(app: tauri::AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(
            "https://github.com/YeloBoi/next-download-manager/blob/main/docs/EXTENSION.md",
            None::<&str>,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn snapshot(manager: State<'_, Manager>) -> Snapshot {
    public_snapshot(manager.snapshot())
}

#[tauri::command]
async fn command(action: Action, manager: State<'_, Manager>) -> Result<Snapshot, String> {
    manager.execute(action).await.map(public_snapshot)
}

// Browser UI receives only origins; signed URLs remain in the local database.
fn public_snapshot(mut snapshot: Snapshot) -> Snapshot {
    for job in &mut snapshot.downloads {
        job.url = ndm_core::redacted_source(&job.url);
    }
    snapshot
}

#[tauri::command]
async fn open_download(
    id: String,
    folder: bool,
    app: tauri::AppHandle,
    manager: State<'_, Manager>,
) -> Result<(), String> {
    let snapshot = manager.snapshot();
    let job = snapshot
        .downloads
        .iter()
        .find(|job| job.id == id)
        .ok_or("Download not found.")?;
    let path = job
        .path
        .as_ref()
        .ok_or("The download has no completed file.")?;
    if folder {
        app.opener()
            .reveal_item_in_dir(path)
            .map_err(|e| e.to_string())
    } else {
        if job.status != Status::Completed {
            return Err("This download is incomplete.".into());
        }
        app.opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
async fn import_links(
    app: tauri::AppHandle,
    manager: State<'_, Manager>,
) -> Result<Snapshot, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .add_filter("Text", &["txt"])
        .blocking_pick_file()
        .ok_or("No file selected.")?
        .into_path()
        .map_err(|e| e.to_string())?;
    let metadata = tokio::fs::metadata(&path)
        .await
        .map_err(|e| e.to_string())?;
    if metadata.len() > 1024 * 1024 {
        return Err("Link file must be smaller than 1 MiB.".into());
    }
    let text = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| e.to_string())?;
    let urls = text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    manager
        .execute(Action::Add {
            urls,
            directory: None,
            name: None,
            start: false,
            sha256: None,
        })
        .await
        .map(public_snapshot)
}

#[tauri::command]
async fn export_links(app: tauri::AppHandle, manager: State<'_, Manager>) -> Result<(), String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .set_file_name("ndm-links.txt")
        .blocking_save_file()
        .ok_or("No file selected.")?
        .into_path()
        .map_err(|e| e.to_string())?;
    let text = manager
        .snapshot()
        .downloads
        .iter()
        .map(|d| d.url.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    tokio::fs::write(path, text)
        .await
        .map_err(|e| e.to_string())
}

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            snapshot,
            command,
            open_download,
            import_links,
            export_links,
            browser_connection,
            extension_guide
        ])
        .setup(|app| {
            let data = dirs::data_local_dir()
                .ok_or("Cannot locate application data.")?
                .join("NDM");
            let downloads =
                dirs::download_dir().ok_or("Cannot locate the Windows Downloads folder.")?;
            let manager =
                tauri::async_runtime::block_on(Manager::open(data, Settings::new(downloads)))
                    .map_err(std::io::Error::other)?;
            app.manage(manager.clone());
            let handle = app.handle().clone();
            let mut receiver = manager.subscribe();
            tauri::async_runtime::spawn(async move {
                while receiver.changed().await.is_ok() {
                    let snapshot = receiver.borrow_and_update().clone();
                    let _ = handle.emit("ndm://snapshot", public_snapshot(snapshot));
                }
            });
            let bridge_manager = manager.clone();
            let connection = ndm_browser_bridge::ConnectionState::default();
            app.manage(connection.clone());
            if let Err(error) = setup_browser_hosts() {
                eprintln!("Browser setup: {error}");
            }
            tauri::async_runtime::spawn(async move {
                if let Err(error) = ndm_browser_bridge::serve(bridge_manager, connection).await {
                    eprintln!("Bridge stopped: {error}");
                }
            });
            use tauri::{
                menu::{Menu, MenuItem},
                tray::TrayIconBuilder,
            };
            let show = MenuItem::with_id(app, "show", "Open NDM", true, None::<&str>)?;
            let start = MenuItem::with_id(app, "start", "Start queue", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pause queue", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit NDM", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &start, &pause, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("NDM — Next Download Manager");
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "start" | "pause" => {
                    let manager = app.state::<Manager>().inner().clone();
                    let action = if event.id.as_ref() == "start" {
                        Action::StartQueue
                    } else {
                        Action::PauseQueue
                    };
                    tauri::async_runtime::spawn(async move {
                        let _ = manager.execute(action).await;
                    });
                }
                "quit" => shutdown(app.clone()),
                _ => {}
            })
            .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if window.state::<Manager>().snapshot().settings.close_to_tray {
                    let _ = window.hide();
                } else {
                    shutdown(window.app_handle().clone());
                }
            }
        })
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|_, _| {}),
        Err(error) => {
            eprintln!("NDM could not start: {error}");
            std::process::exit(1);
        }
    }
}

fn shutdown(app: tauri::AppHandle) {
    let manager = app.state::<Manager>().inner().clone();
    tauri::async_runtime::spawn(async move {
        match manager.execute(Action::Shutdown).await {
            Ok(_) => app.exit(0),
            Err(error) => {
                let _ = app.emit("ndm://error", error);
            }
        }
    });
}

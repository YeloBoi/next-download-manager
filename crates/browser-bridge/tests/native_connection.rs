#![cfg(windows)]

use ndm_browser_bridge::{
    bridge_directory, register_hosts, ConnectionState, Request, Response, CHROMIUM_MANIFEST,
    CHROMIUM_ORIGIN, FIREFOX_ID, FIREFOX_MANIFEST,
};
use ndm_core::Settings;
use ndm_queue::Manager;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
#[ignore = "uses only an isolated GitHub Windows runner's native-host registration"]
async fn automatic_registration_and_background_reconnection() {
    assert_eq!(std::env::var("GITHUB_ACTIONS").as_deref(), Ok("true"));
    let host = std::path::Path::new(env!("CARGO_BIN_EXE_ndm-native-host"));
    let directory = bridge_directory().expect("bridge folder");
    let other_id = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    register_hosts(host, Some(other_id)).expect("manual compatibility registration");
    register_hosts(host, None).expect("automatic registration");
    let chromium: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join(CHROMIUM_MANIFEST)).unwrap()).unwrap();
    assert_eq!(
        chromium["allowed_origins"],
        serde_json::json!([format!("chrome-extension://{other_id}/"), CHROMIUM_ORIGIN])
    );
    let manifest = directory.join(FIREFOX_MANIFEST);
    let registration: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(
        registration["allowed_extensions"],
        serde_json::json!([FIREFOX_ID])
    );
    assert_eq!(registration["path"], serde_json::json!(host));
    for (browser, expected) in [
        ("Mozilla", FIREFOX_MANIFEST),
        ("Google\\Chrome", CHROMIUM_MANIFEST),
        ("Microsoft\\Edge", CHROMIUM_MANIFEST),
    ] {
        let key = format!("HKCU\\Software\\{browser}\\NativeMessagingHosts\\local.ndm.bridge");
        let registry = std::process::Command::new("reg.exe")
            .args(["QUERY", &key, "/ve"])
            .output()
            .unwrap();
        assert!(registry.status.success());
        assert!(String::from_utf8_lossy(&registry.stdout).contains(expected));
    }
    let mut child = tokio::process::Command::new(host)
        .arg(&manifest)
        .arg(FIREFOX_ID)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    async fn ping(
        input: &mut tokio::process::ChildStdin,
        output: &mut tokio::process::ChildStdout,
    ) -> Response {
        let request = Request {
            version: 1,
            request_id: uuid::Uuid::new_v4().to_string(),
            kind: "ping".into(),
            urls: vec![],
            start: false,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        input
            .write_all(&(bytes.len() as u32).to_le_bytes())
            .await
            .unwrap();
        input.write_all(&bytes).await.unwrap();
        let mut length = [0; 4];
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            output.read_exact(&mut length),
        )
        .await
        .unwrap()
        .unwrap();
        let size = u32::from_le_bytes(length) as usize;
        assert!(size <= ndm_browser_bridge::MAX_MESSAGE);
        let mut reply = vec![0; size];
        output.read_exact(&mut reply).await.unwrap();
        let response: Response = serde_json::from_slice(&reply).unwrap();
        assert_eq!(response.request_id, request.request_id);
        response
    }
    assert!(
        !ping(&mut input, &mut output).await.accepted,
        "closed app is offline, not uninstalled"
    );
    let root = std::env::temp_dir().join(format!("ndm-connection-{}", uuid::Uuid::new_v4()));
    let manager = Manager::open(root.join("state"), Settings::new(root.join("downloads")))
        .await
        .unwrap();
    let state = ConnectionState::default();
    let server = tokio::spawn(ndm_browser_bridge::serve(manager.clone(), state.clone()));
    assert!(
        ping(&mut input, &mut output).await.accepted,
        "same port recovers when desktop starts"
    );
    assert!(state.status().connected);
    assert!(
        ping(&mut input, &mut output).await.accepted,
        "native host supports repeated messages"
    );
    child.kill().await.unwrap();
    server.abort();
}

use ndm_browser_bridge::{
    bridge_directory, read_frame, write_frame, Request, Response, MAX_MESSAGE,
};

#[tokio::main]
async fn main() {
    // Chrome provides the caller origin. Do not accept invocations from unregistered IDs.
    let result = run().await;
    if let Err(error) = result {
        let response = Response {
            version: 1,
            request_id: String::new(),
            accepted: false,
            error: Some(error),
        };
        if let Ok(bytes) = serde_json::to_vec(&response) {
            let _ = write_frame(&mut std::io::stdout().lock(), &bytes);
        }
    }
}

async fn run() -> Result<(), String> {
    let directory = bridge_directory()?;
    let allowed: Vec<String> = serde_json::from_slice(
        &tokio::fs::read(directory.join("allowed-origins.json"))
            .await
            .map_err(|_| {
                "Native host is not registered. Follow the extension setup guide.".to_string()
            })?,
    )
    .map_err(|e| e.to_string())?;
    let origin = std::env::args().nth(1).ok_or("Missing browser origin.")?;
    if !allowed.contains(&origin) {
        return Err("This extension is not allowed to connect.".into());
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
        if started.elapsed().as_secs() > 15 {
            let _ = tokio::fs::remove_file(path.with_extension("request")).await;
            return Err("NDM did not acknowledge the request. Open the desktop app and try again; the browser download has been retained.".into());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

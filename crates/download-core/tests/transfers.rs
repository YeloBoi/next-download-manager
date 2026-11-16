#[path = "../../../tests/fixtures/http.rs"]
mod support;
use ndm_core::{Download, DownloadError, Engine, EngineEvent, NetworkLimits, Status};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::atomic::Ordering};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

fn job(url: String, destination: &Path) -> Download {
    Download {
        id: "aabb-0011".into(),
        url,
        name: String::new(),
        directory: destination.to_path_buf(),
        status: Status::Queued,
        downloaded: 0,
        total: None,
        speed: 0.0,
        connections: 0,
        resumable: false,
        error: None,
        path: None,
        sha256: None,
        expected_sha256: None,
        resume_with_queue: false,
    }
}
fn engine(root: &Path) -> Engine {
    Engine::new(root.join("parts"), NetworkLimits::new(4, 4, 0)).expect("engine")
}
async fn run(engine: &Engine, job: &Download) -> ndm_core::Result<(std::path::PathBuf, String)> {
    let (tx, mut rx) = mpsc::channel(128);
    let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });
    let result = engine.download(job, 4, CancellationToken::new(), tx).await;
    drain.await.expect("events");
    result
}
async fn interrupt(engine: &Engine, job: &Download) {
    let (tx, mut rx) = mpsc::channel(128);
    let token = CancellationToken::new();
    let cancel = token.clone();
    let monitor = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            if matches!(event,EngineEvent::Progress(n) if n>32768) {
                cancel.cancel();
            }
        }
    });
    assert!(matches!(
        engine.download(job, 4, token, tx).await,
        Err(DownloadError::Cancelled)
    ));
    monitor.await.expect("monitor");
}
#[tokio::test]
async fn multipart_hash_and_existing_file_are_preserved() {
    let server = support::Server::start().await;
    let dir = tempfile::tempdir().expect("dir");
    tokio::fs::write(dir.path().join("fixture.bin"), b"keep")
        .await
        .expect("existing");
    let (path, hash) = run(&engine(dir.path()), &job(server.url("range"), dir.path()))
        .await
        .expect("download");
    assert_eq!(tokio::fs::read(path).await.expect("file"), *server.data);
    assert_eq!(
        hash,
        format!("{:x}", Sha256::digest(server.data.as_slice()))
    );
    assert_eq!(
        tokio::fs::read(dir.path().join("fixture.bin"))
            .await
            .expect("existing"),
        b"keep"
    );
    assert!(server.requests.lock().expect("requests").len() >= 5);
}
#[tokio::test]
async fn fresh_fallback_unknown_size_redirect_and_retry_preserve_bytes() {
    let server = support::Server::start().await;
    for mode in ["plain", "ignore", "reject", "unknown", "redirect", "retry"] {
        let dir = tempfile::tempdir().expect("dir");
        let (path, _) = run(&engine(dir.path()), &job(server.url(mode), dir.path()))
            .await
            .unwrap_or_else(|e| panic!("{mode}: {e}"));
        assert_eq!(
            tokio::fs::read(path).await.expect("file"),
            *server.data,
            "{mode}"
        );
    }
}
#[tokio::test]
async fn resume_matches_hash_and_changed_sources_are_rejected() {
    let server = support::Server::start().await;
    let dir = tempfile::tempdir().expect("dir");
    let engine = engine(dir.path());
    let job = job(server.url("range"), dir.path());
    interrupt(&engine, &job).await;
    server.version.store(2, Ordering::SeqCst);
    assert!(matches!(
        run(&engine, &job).await,
        Err(DownloadError::Resume(_))
    ));
    server.version.store(1, Ordering::SeqCst);
    let (path, _) = run(&engine, &job).await.expect("resume");
    assert_eq!(tokio::fs::read(path).await.expect("file"), *server.data);
}
#[tokio::test]
async fn html_short_body_hash_mismatch_and_empty_file() {
    let server = support::Server::start().await;
    for mode in ["html", "short"] {
        let dir = tempfile::tempdir().expect("dir");
        assert!(
            run(&engine(dir.path()), &job(server.url(mode), dir.path()))
                .await
                .is_err(),
            "{mode}"
        );
        assert!(!dir.path().join("fixture.bin").exists());
    }
    let dir = tempfile::tempdir().expect("dir");
    let mut wrong = job(server.url("range"), dir.path());
    wrong.expected_sha256 = Some("0".repeat(64));
    assert!(matches!(
        run(&engine(dir.path()), &wrong).await,
        Err(DownloadError::HashMismatch)
    ));
    assert!(!dir.path().join("fixture.bin").exists());
    let empty = tempfile::tempdir().expect("dir");
    let (path, hash) = run(
        &engine(empty.path()),
        &job(server.url("empty"), empty.path()),
    )
    .await
    .expect("empty file");
    assert_eq!(tokio::fs::metadata(path).await.expect("metadata").len(), 0);
    assert_eq!(hash, format!("{:x}", Sha256::digest([])));
}

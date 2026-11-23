#[allow(dead_code)]
#[path = "../../../tests/fixtures/http.rs"]
mod support;
use ndm_core::{Settings, Snapshot, Status};
use ndm_queue::{Action, Manager};
use std::time::Duration;

async fn wait(manager: &Manager, predicate: impl Fn(&Snapshot) -> bool) -> Snapshot {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let snapshot = manager.snapshot();
            if predicate(&snapshot) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("queue condition timed out")
}
fn add(urls: Vec<String>, start: bool) -> Action {
    Action::Add {
        urls,
        directory: None,
        name: None,
        start,
        sha256: None,
    }
}

#[tokio::test]
async fn manual_start_precedes_the_waiting_queue_and_persists_usage() {
    let server = support::Server::start().await;
    let dir = tempfile::tempdir().expect("dir");
    let settings = Settings::new(dir.path().join("downloads"));
    let data = dir.path().join("state");
    let manager = Manager::open(data.clone(), settings.clone()).await.unwrap();
    manager
        .execute(add(vec![server.url("plain")], false))
        .await
        .unwrap();
    let start = server.url("range");
    let response = manager
        .execute(add(vec![start.clone()], true))
        .await
        .unwrap();
    assert_eq!(response.downloads[0].url, start);
    let completed = wait(&manager, |s| s.downloads[0].status == Status::Completed).await;
    manager
        .execute(Action::Remove {
            id: completed.downloads[0].id.clone(),
            delete_files: false,
        })
        .await
        .unwrap();
    manager.execute(Action::Shutdown).await.unwrap();
    let reopened = Manager::open(data, settings).await.unwrap();
    let usage = reopened.usage().await.unwrap();
    assert!(usage
        .iter()
        .any(|row| row.device == "pc" && row.bytes >= server.data.len() as u64));
    reopened.execute(Action::Shutdown).await.unwrap();
}

#[tokio::test]
async fn browser_batches_deduplicate_preserve_order_and_respect_durable_user_stop() {
    let server = support::Server::start().await;
    let dir = tempfile::tempdir().expect("dir");
    let data = dir.path().join("state");
    let settings = Settings::new(dir.path().join("downloads"));
    let manager = Manager::open(data.clone(), settings.clone())
        .await
        .expect("open");
    manager
        .execute(Action::PauseQueue)
        .await
        .expect("pause empty queue");
    let urls = vec![server.url("range"), server.url("plain")];
    manager
        .execute(Action::BrowserAdd {
            urls: urls.clone(),
            start: true,
        })
        .await
        .expect("browser batch");
    let initial = manager.snapshot();
    assert!(
        !initial.running,
        "browser must not override an explicit pause"
    );
    assert_eq!(
        initial
            .downloads
            .iter()
            .map(|job| job.url.clone())
            .collect::<Vec<_>>(),
        urls
    );
    manager
        .execute(Action::BrowserAdd {
            urls: urls.clone(),
            start: true,
        })
        .await
        .expect("replayed batch");
    assert_eq!(manager.snapshot().downloads.len(), 2);
    assert!(manager
        .execute(Action::BrowserAdd {
            urls: vec![server.url("unknown"), "file:///invalid".into()],
            start: false
        })
        .await
        .is_err());
    assert_eq!(
        manager.snapshot().downloads.len(),
        2,
        "validation must be atomic"
    );
    manager.execute(Action::Shutdown).await.expect("shutdown");
    let reopened = Manager::open(data, settings).await.expect("reopen");
    reopened
        .execute(Action::BrowserAdd {
            urls: vec![server.url("unknown")],
            start: true,
        })
        .await
        .expect("new browser batch");
    assert!(!reopened.snapshot().running);
    assert_eq!(
        reopened.snapshot().downloads[0].url,
        server.url("unknown"),
        "download now goes before waiting files"
    );
    reopened
        .execute(Action::StartQueue)
        .await
        .expect("explicit start");
    assert!(!reopened.snapshot().user_paused);
    reopened.execute(Action::Shutdown).await.expect("shutdown");
}
#[tokio::test]
async fn browser_download_now_starts_an_existing_queued_url_without_duplicating_it() {
    let server = support::Server::start().await;
    let dir = tempfile::tempdir().expect("dir");
    let manager = Manager::open(
        dir.path().join("state"),
        Settings::new(dir.path().join("downloads")),
    )
    .await
    .expect("open");
    let url = server.url("plain");
    manager
        .execute(Action::BrowserAdd {
            urls: vec![url.clone()],
            start: false,
        })
        .await
        .unwrap();
    assert!(!manager.snapshot().running);
    manager
        .execute(Action::BrowserAdd {
            urls: vec![url],
            start: true,
        })
        .await
        .unwrap();
    let done = wait(&manager, |state| {
        state.downloads[0].status == Status::Completed
    })
    .await;
    assert_eq!(done.downloads.len(), 1);
}

#[tokio::test]
async fn fifo_add_while_active_drain_and_persistent_reopen() {
    let server = support::Server::start().await;
    let dir = tempfile::tempdir().expect("dir");
    let data = dir.path().join("state");
    let settings = Settings::new(dir.path().join("downloads"));
    let manager = Manager::open(data.clone(), settings.clone())
        .await
        .expect("open");
    manager
        .execute(add(vec![server.url("range"), server.url("plain")], true))
        .await
        .expect("add");
    let first = wait(&manager, |s| s.downloads[0].downloaded > 0)
        .await
        .downloads[0]
        .id
        .clone();
    manager
        .execute(add(vec![server.url("unknown")], false))
        .await
        .expect("add while active");
    let snapshot = manager.snapshot();
    assert_eq!(snapshot.downloads[0].id, first);
    assert!(snapshot.downloads[0].status.active());
    assert_eq!(snapshot.downloads[1].status, Status::Queued);
    manager.execute(Action::Drain).await.expect("drain");
    let stopped = wait(&manager, |s| {
        !s.running && s.downloads[0].status == Status::Completed
    })
    .await;
    assert_eq!(stopped.downloads[1].status, Status::Queued);
    assert_eq!(stopped.downloads[2].status, Status::Queued);
    manager.execute(Action::Shutdown).await.expect("shutdown");
    let reopened = Manager::open(data, settings).await.expect("reopen");
    assert_eq!(reopened.snapshot().downloads.len(), 3);
    assert!(!reopened.snapshot().running);
    assert_eq!(reopened.snapshot().downloads[0].status, Status::Completed);
    reopened.execute(Action::Shutdown).await.expect("shutdown");
}
#[tokio::test]
async fn pause_and_stop_have_distinct_queue_resume_behavior() {
    let server = support::Server::start().await;
    let dir = tempfile::tempdir().expect("dir");
    let manager = Manager::open(
        dir.path().join("state"),
        Settings::new(dir.path().join("downloads")),
    )
    .await
    .expect("open");
    manager
        .execute(add(vec![server.url("range"), server.url("plain")], true))
        .await
        .expect("add");
    let id = wait(&manager, |s| s.downloads[0].downloaded > 0)
        .await
        .downloads[0]
        .id
        .clone();
    manager
        .execute(Action::Pause {
            id: id.clone(),
            continue_queue: false,
        })
        .await
        .expect("pause");
    wait(&manager, |s| s.downloads[0].status == Status::Paused).await;
    assert!(!manager.snapshot().running);
    manager
        .execute(Action::Resume { id: id.clone() })
        .await
        .expect("resume");
    wait(&manager, |s| s.downloads[0].status.active()).await;
    manager
        .execute(Action::Stop { id: id.clone() })
        .await
        .expect("stop");
    wait(&manager, |s| s.downloads[0].status == Status::Stopped).await;
    manager
        .execute(Action::StartQueue)
        .await
        .expect("start queue");
    assert_eq!(manager.snapshot().downloads[0].status, Status::Stopped);
    manager
        .execute(Action::Remove {
            id,
            delete_files: true,
        })
        .await
        .expect("remove");
    assert_eq!(manager.snapshot().downloads.len(), 1);
    manager.execute(Action::Shutdown).await.expect("shutdown");
}

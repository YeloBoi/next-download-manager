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

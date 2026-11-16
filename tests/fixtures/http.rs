use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::get,
    Router,
};
use std::{
    convert::Infallible,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Clone)]
struct Fixture {
    data: Arc<Vec<u8>>,
    version: Arc<AtomicUsize>,
    retries: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<String>>>,
}
pub struct Server {
    pub base: String,
    pub data: Arc<Vec<u8>>,
    pub version: Arc<AtomicUsize>,
    pub requests: Arc<Mutex<Vec<String>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    pub async fn start() -> Self {
        let data = Arc::new((0..2 * 1024 * 1024).map(|n| (n % 251) as u8).collect());
        let version = Arc::new(AtomicUsize::new(1));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let fixture = Fixture {
            data: data.clone(),
            version: version.clone(),
            retries: Arc::default(),
            requests: requests.clone(),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fixture");
        let base = format!("http://{}", listener.local_addr().expect("address"));
        let router = Router::new()
            .route("/{mode}", get(serve))
            .with_state(fixture);
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.expect("serve fixture");
        });
        Self {
            base,
            data,
            version,
            requests,
            task,
        }
    }
    pub fn url(&self, mode: &str) -> String {
        format!("{}/{mode}", self.base)
    }
}
async fn serve(
    State(state): State<Fixture>,
    Path(mode): Path<String>,
    headers: HeaderMap,
) -> Response {
    let range = headers
        .get(header::RANGE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    state
        .requests
        .lock()
        .expect("requests")
        .push(format!("{mode}:{range}"));
    if mode == "redirect" {
        return Response::builder()
            .status(302)
            .header(header::LOCATION, "/range")
            .body(Body::empty())
            .expect("redirect");
    }
    if mode == "html" {
        return Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from("<html>login</html>"))
            .expect("html");
    }
    if mode == "empty" {
        return Response::builder()
            .status(416)
            .header(header::CONTENT_RANGE, "bytes */0")
            .body(Body::empty())
            .expect("empty");
    }
    let probe = range == "bytes=0-0";
    if mode == "retry" && !probe && state.retries.fetch_add(1, Ordering::SeqCst) < 2 {
        return Response::builder()
            .status(503)
            .header(header::RETRY_AFTER, "0")
            .body(Body::empty())
            .expect("retry");
    }
    let ranged =
        !range.is_empty() && mode != "plain" && mode != "unknown" && (probe || mode != "ignore");
    if mode == "reject" && ranged && !probe {
        return Response::builder()
            .status(416)
            .body(Body::empty())
            .expect("416");
    }
    let size = state.data.len();
    let (start, end) = if ranged {
        let (start, end) = range
            .strip_prefix("bytes=")
            .expect("range prefix")
            .split_once('-')
            .expect("range separator");
        (
            start.parse::<usize>().expect("start"),
            end.parse::<usize>().unwrap_or(size - 1),
        )
    } else {
        (0, size - 1)
    };
    let mut builder = Response::builder()
        .status(if ranged {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::ETAG,
            format!("\"fixture-{}\"", state.version.load(Ordering::SeqCst)),
        )
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename*=UTF-8''fixture.bin",
        );
    if ranged {
        builder = builder.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
    }
    if mode != "unknown" {
        builder = builder.header(header::CONTENT_LENGTH, end - start + 1);
    }
    let end = if mode == "short" && !probe {
        start + (end - start) / 2
    } else {
        end
    };
    let stream =
        futures_util::stream::unfold((state.data, start, end), |(data, cursor, end)| async move {
            if cursor > end {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(4)).await;
            let next = (cursor + 16 * 1024).min(end + 1);
            let bytes = Bytes::copy_from_slice(&data[cursor..next]);
            Some((Ok::<_, Infallible>(bytes), (data, next, end)))
        });
    builder.body(Body::from_stream(stream)).expect("response")
}

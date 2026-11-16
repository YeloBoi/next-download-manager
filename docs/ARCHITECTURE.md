# Architecture and UI decision

`ndm-core`: Tokio/reqwest transfer engine, HTTP range validation, per-part durable
checkpoints, bounded retries, SHA-256, global/host permits and bandwidth limiter.
`ndm-storage`: SQLite WAL, full synchronous transactions, blocking I/O off runtime
workers. `ndm-queue`: one actor owns mutable state and sends snapshots over a watch
channel. Cancellation tokens and completion messages prevent duplicate active
jobs. `ndm-browser-bridge`: bounded native framing and a per-user spool. Desktop:
Tauri invokes actor commands and renders public snapshots containing source origins
only. Signed URLs remain in the local database and part manifests; protect those
files with your Windows account and avoid sharing them.

Parts and checkpoints are flushed before committed offsets are recorded. Resume
checks file identity and part layout, truncates uncheckpointed tails, and refuses
changed sources. A final SHA-256 is computed during assembly. Publication uses a
hard link to avoid replacing a file created concurrently; destinations must support
hard links (for example NTFS). Failure preserves parts rather than overwriting.
Parts are kept under app data; final assembly temporarily doubles the required
storage. Cross-volume free-space accounting and non-NTFS publication still need
improvement before release.

The existing implementation selects **Tauri 2 + TypeScript** rather than Slint.
WebView2 provides browser text shaping, bidirectional layout, HTML keyboard
semantics and a font pipeline for English and Persian. Tauri is licensed under
MIT or Apache-2.0. Slint has multiple licensing routes with conditions and needs
separate license/accessibility/RTL evaluation; this project does not assert it is
incapable of Persian. No unverified Slint capability or license claim is used as
a release guarantee. Dependency notices and actual redistribution obligations
must still be reviewed. Real Persian layout and assistive technology tests remain
required; choosing a webview alone is not such a test.

The existing dark app direction uses a neutral charcoal canvas, blue transfer
data, a distinct N/down-arrow mark, Inter and Vazirmatn. It exposes real queue
state, with no seeded downloads. Design source: `.taste-skill/direction.json`.

The owner requested on 2026-11-17 that all builds and dependency downloads take
place on GitHub hosted runners. Do not run cargo/npm dependency installation,
builds or artifact downloads on the owner's computer without a new request.

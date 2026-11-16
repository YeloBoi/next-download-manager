# Third-party notices (development inventory)

Original NDM code is governed by LICENSE. Dependencies retain their own licenses.
This inventory is not a completed redistribution audit.

Direct dependencies include Rust, Tokio, reqwest, serde/serde_json, thiserror,
url, uuid, sha2, percent-encoding, fs2, chrono, dirs, rusqlite/SQLite, Tauri and
its dialog, opener and single-instance plugins. Consult the exact packages in
Cargo.lock and their LICENSE/NOTICE files for authoritative terms.

Frontend: Tauri API/plugins (MIT OR Apache-2.0), TypeScript (Apache-2.0), Vite
(MIT), Lucide icons (ISC), Inter font (SIL OFL-1.1), Vazirmatn font (SIL OFL-1.1),
Fontsource packaging and their transitive dependencies. Font files are bundled
from @fontsource-variable packages; no font download occurs during app use.

Before external distribution, collect the actual complete license texts and
attribution notices for the locked dependency graph, fonts, assets, installer
runtime and SQLite configuration. Preserve copyright statements and any required
notices. Do not treat this list alone as sufficient license compliance evidence.

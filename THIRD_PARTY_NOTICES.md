# Third-party notices (development inventory)

The owner requested removal of the project's draft LICENSE. Dependencies retain
their own licenses and notices independently of that decision.
This inventory is not a completed redistribution audit.

Direct dependencies include Rust, Tokio, reqwest, serde/serde_json, thiserror,
url, uuid, sha2, percent-encoding, fs2, chrono, dirs, rusqlite/SQLite, Tauri and
its dialog, opener and single-instance plugins. Consult the exact packages in
Cargo.lock and their LICENSE/NOTICE files for authoritative terms.

Android uses AndroidX Core and ZXing Android Embedded (Apache-2.0). The scanning
library includes ZXing Core (Apache-2.0). These dependencies are independent of
the project’s own licensing status.

Frontend: Tauri API/plugins (MIT OR Apache-2.0), TypeScript (Apache-2.0), Vite
(MIT), Lucide icons (ISC), Inter font (SIL OFL-1.1), Vazirmatn font (SIL OFL-1.1),
Fontsource packaging and their transitive dependencies. Font files are bundled
from @fontsource-variable packages; no font download occurs during app use.

Hosted packaging collects the supplied package LICENSE, COPYING, NOTICE and OFL
files, plus Cargo license-file entries, into THIRD_PARTY_LICENSES with a dependency
inventory. The installer and portable archive include that directory. The bundled
Inter and Vazirmatn font licenses must be present or packaging fails.
This automated collection preserves available texts; it is not a complete legal
audit of every transitive dependency, runtime or SQLite configuration.

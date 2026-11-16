# NDM 0.1.0-alpha.1 — development build

Direct-download queue with Rust core, SQLite state, Tauri desktop interface and
Chromium native-messaging extension. English default, optional Persian RTL.

Development artifacts are unsigned and unvalidated for stable use. No digital
signing certificate is configured. The source and binary license remains a draft;
private CI artifacts are for owner evaluation. See README and docs/STATUS.md for
current behavior, installation instructions and unverified acceptance criteria.

Portable mode still stores data in LocalAppData. WebView2 is required. Native-host
registration needs an explicit extension ID after loading the unpacked extension.
Existing downloads are retained by default when removing queue records.

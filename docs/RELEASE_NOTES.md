# NDM 0.1.0-alpha.3 — browser integration milestone

Development alpha, not stable; this milestone does not complete the full product brief.

- Ordered multi-tab link selection saved locally across popup closure and service-worker suspension, with count badge and move/remove controls.
- Separate right-click actions for immediate download and collecting without downloading.
- Failed sends retained for retry; only acknowledged URLs removed from selection.
- Native host opens only sibling ndm.exe for an explicit add. Background pings never launch the app, and existing windows are not focused by browser delivery.
- Queue actor deduplicates URLs still present, preserves batch order and respects a user's persisted pause/stop/drain choice. Use Start queue in NDM when paused.
- Real connection card in English/Persian, based on a recent valid native handshake. A delayed heartbeat can show disconnected; this does not mean uninstalled.
- Existing logo crops, themed rounded titlebar, direct/segmented engine and safe resume remain.

Install the Windows x64 setup EXE, or extract the portable ZIP and run ndm.exe.
WebView2 is required and the installer may fetch it if missing. Portable state is
stored in LocalAppData. Load the extension ZIP unpacked, then register the actual
extension ID with the supplied Register-NativeHost.ps1. Keep the native host beside
the desktop executable. See docs/EXTENSION.md. Store publishing and automatic
native-host registration are not implemented.

Unsigned alpha. Hosted checks do not establish clean-machine Windows 10/11
installation, native cold launch/focus behavior, tray shutdown, accessibility,
DPI or installed browser-to-native correctness. Manual acceptance remains open.
There is no durable request-ID ledger after history removal. Visual/Shift selection,
groups, Download Later, startup settings, complete profiles, usage budget, safe
archive extraction, mini window, activity/Undo and other advanced requirements
remain unfinished; see docs/IMPLEMENTATION_STATUS.md for every brief section.

The owner previously requested removal of the project LICENSE. No replacement
license is added. Third-party notices and bundled dependency licenses remain.

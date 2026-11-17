# NDM 0.1.0-alpha.5 - Automatic Firefox connection

- Open the updated NDM app once: Firefox native-host registration is automatic. No manual PowerShell command or extension ID is needed.
- A background-owned persistent native port replaces popup-owned checks. Closing the popup no longer disconnects the browser; five-second probes recover after desktop/host restarts.
- Check connection repairs Firefox registration and keeps its button stable. Desktop status refreshes automatically; no repeated popup-opening instruction.
- Update both the desktop app and Firefox extension. Keep ndm-native-host.exe beside ndm.exe. Chrome/Edge still need their explicit extension-ID registration.
- Hosted tests exercise real Firefox background recovery without a popup, and real Windows registration/native protocol with the queue service.
- Firefox remains an unsigned temporary add-on; permanent Mozilla signing is pending.

Previous release notes:

# NDM 0.1.0-alpha.4 - Firefox compatibility fix

- Separate **firefox.zip** with module background scripts, fixing the disabled background.service_worker error.
- Native Firefox Promise APIs, fixed add-on identity and native-host registration via -Browser Firefox.
- Update the desktop/native host to alpha.4 for Firefox delivery. Chrome and Edge remain supported.
- Firefox 140 or later. Unsigned development extension: use Load Temporary Add-on in about:debugging and reload after restarting Firefox. Permanent Mozilla signing is pending.
- Hosted Firefox manifest/runtime checks gate publication alongside Windows checks.

The browser-integration features from alpha.3 remain:

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

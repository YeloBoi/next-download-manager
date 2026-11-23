# NDM 0.1.0-alpha.12 — Bluetooth or Wi-Fi / Internet

- Choose **Bluetooth** in Windows Settings → Your phone, and in Android under PC → Scan QR code. NDM requests Bluetooth if it is off. Scan the new QR code and confirm Bluetooth pairing if the OS asks. No internet or Wi-Fi is required.
- Choose **Wi-Fi / Internet** for a shared Wi-Fi network or phone hotspot. For separate internet connections, use **Remote QR (Tailscale)** with both devices signed into the same Tailscale network.

- Update both Windows and Android, connect both devices to the same Tailscale network, then use **Settings → Your phone → Remote QR (Tailscale)** on Windows and scan on Android. The PC can stay on Wi-Fi while the phone uses mobile data. Keep NDM open on Windows.
- For local pairing, **Show pairing QR** can include multiple PC addresses. New remembered pairings try the other addresses if the current route becomes unavailable.
- Android checks for updates, verifies the release checksum and the existing app’s signature, then shows a preview before you confirm installation. Install this first updater release from GitHub once if you are on alpha.8 or earlier.
- The Chrome, Edge and Firefox popups can find newer matching add-on downloads. ZIP installs still require manual replacement. Native Firefox automatic updates require a Mozilla-signed XPI.

# NDM 0.1.0-alpha.8 — Android companion and file sharing

- Review Add links before starting. NDM checks the name, exact size, extension and safe resume via HEAD when the server provides them, and suggests a folder by file type. Start takes the next free slot even outside the automatic queue schedule.
- The installed Windows app checks GitHub Releases for newer versions, shows release notes and download progress, and verifies the installer against GitHub’s asset SHA-256 before installing and restarting. Later snoozes that version for 24 hours. Portable users update by replacing their archive.
- The signed Android app downloads files locally and shows APK app icon, name, package, version and size ahead of Android’s own installation confirmation.
- Pair Android and Windows on the same Wi-Fi. In Windows Settings, enable **Your phone** and show its one-use QR code. Android can watch PC downloads, add links, pause, resume, stop, and remove queue entries. Remembered phones rejoin using their saved credential, and access can be revoked on the PC.
- Local file sharing works in both directions, supports multiple files and resumes from its last saved offset. iOS and cloud relay are outside this release. PC and Android usage charts report completed internet downloads separately from local shares.
- The built-in updater first ships here: install this version from Releases if you are upgrading from alpha.7.

## NDM 0.1.0-alpha.7 — Automatic connection for Chrome, Edge, and Firefox

- The Windows installer and NDM startup register native messaging for all three browsers. The release Chrome/Edge extension keeps one ID regardless of its unpacked folder; no manual PowerShell command is needed.
- Use the new extension and app together. Replace the old unpacked Chrome/Edge extension to avoid two NDM toolbar icons. Firefox still requires Load Temporary Add-on after a browser restart.
- Download all can start the same link after Add all to queue without creating a duplicate. Explicit queue pause and scheduling settings still apply.
- The popup preserves failed links and distinguishes registration, launch, and version problems. The desktop connection card stays hidden while connected.
- Release gates include the installed NDM executable and packaged extensions in actual Windows Chrome, Edge, and Firefox, with HTTP file hashes checked after download.

Older release notes (historical behavior):

# NDM 0.1.0-alpha.6 - Browser downloads open and survive Firefox replies

- Fix Windows native-host child lifetime: the desktop uses CREATE_BREAKAWAY_FROM_JOB so Firefox closing a one-shot native message cannot kill NDM and its download.
- Explicit browser download requests show and restore the desktop, including an already running hidden/minimized instance. Background probes still never launch NDM.
- Hide the entire browser connection card while connected; show setup/reconnect controls again when disconnected.
- Hosted Windows acceptance now launches the actual packaged desktop/native executables under a Firefox-style job, sends cold/warm downloads, kills the native job immediately after acknowledgment, verifies a visible single desktop and SHA-256 of real HTTP downloads.
- Hosted Edge checks connection-card hide/reappearance across polling and settings redraw. Firefox checks the actual Download all button sends start=true and clears acknowledged links.
- Update the desktop app to alpha.6. Extension 0.1.5 is included; Firefox remains an unsigned temporary add-on.
- An explicitly paused/stopped queue still requires Start queue. Clean-machine installer and combined installed Windows Firefox acceptance remain manual.

Previous release notes (historical behavior):

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

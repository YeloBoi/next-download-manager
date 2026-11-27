# NDM — Next Download Manager

![NDM logo](apps/desktop/public/brand/wordmark-color.png)

Direct-download manager for Windows 10/11 x64 and macOS 12 or later (Apple Silicon and Intel), with an Android 8+ companion. Development alpha.
The Rust engine and SQLite queue are separate from the Tauri 2 / TypeScript UI.
English is the default; Persian changes the layout to RTL and uses Vazirmatn.
The successful hosted build and downloadable artifacts are recorded in
[build evidence](docs/BUILD_REPORT.md).

## Builds on GitHub

Download the [latest release](https://github.com/YeloBoi/next-download-manager/releases/latest)
from **Releases → Assets**. For installation, choose
the Windows x64 `*-setup.exe` installer, the macOS DMG for your chip when available, or
`*-android.apk` for Android. The Windows portable ZIP, browser extensions, checksums
and release notes are separate assets.

All dependency installation, Rust compilation, tests and packaging run in
**GitHub Actions → Desktop checks and build**. No local build is required.
After a successful run, choose only the artifact you need: `ndm-installer-windows-x64`,
`ndm-portable-windows-x64`, `ndm-chromium-extension`, `ndm-firefox-extension` or `ndm-checksums-and-notes`.
The macOS CI artifacts are `ndm-macos-arm64` and `ndm-macos-x64`. Tagged releases attach macOS DMGs after Developer ID signing and notarization.
Unsigned alpha binaries may trigger Windows reputation warnings; no code-signing
certificate is configured. Keep Windows security protections enabled.

Extract the portable ZIP and run `ndm.exe`, or run the per-user installer.
Microsoft Edge WebView2 Runtime is required; the installer can fetch it if missing.
Portable means the executables are carried together; settings and partial files
still live in `%LOCALAPPDATA%\NDM`, not beside the executable.

On macOS, mount the DMG, drag **NDM** to **Applications**, then open it from there
once to register Chrome, Edge and Firefox. The queue and settings live in
`~/Library/Application Support/NDM`. Self-built and CI-only DMGs are for testing.
See [macOS build and release instructions](docs/MACOS.md).

## Use

Add an HTTP(S) direct file URL, review the file and suggested destination, then select **Download now**
or **Add to queue**. Add multiple URLs one per line. The default queue is FIFO
with one file and one connection. Settings control concurrent files, connections
per file, host/global ceilings, speed, recovery and a local-time schedule.

Pause keeps partial bytes and pauses queue scheduling by default. **Pause &
continue queue** pauses only the selected file. Stop retains data and requires
explicit resume. **Stop after current** lets active jobs finish. **Download next**
reorders waiting work without interrupting active downloads. Restart and file
deletion require confirmation. Existing destination files are not overwritten.

Resume requires actual validated byte ranges plus a matching strong ETag or
Last-Modified. If safe resume is unavailable, NDM preserves the bytes and asks
for an explicit restart. Unknown sizes show no invented percentage or ETA.
Downloaded programs are never run automatically.

In **Settings → Your phone**, enable the LAN connection and show a one-time QR.
Open **PC → Scan QR code** in NDM on Android, with both devices on the same network.
Allow NDM through Windows Firewall on private networks or allow macOS local network access if prompted. Bluetooth and Tailscale pairing use Windows integrations. Use **PC** for remote
control and **Files** to accept and send files. Android’s share menu also has **Share to PC**
for links and files. The PC saves received files under **NDM Shared** in the configured
downloads destination. Use **Watch in background** on Android for completion notifications
while the app is not on screen.

**Download usage** compares completed PC and phone internet downloads. It keeps local
file transfers out of these totals and starts collecting statistics with this version.
The update banner checks GitHub for a newer release. macOS and Windows portable users download a new package from Releases. Installed Windows copies can download a verified installer directly.
See the [Persian companion guide](docs/COMPANION.fa.md).

## Browser extension

See [extension setup](docs/EXTENSION.md). Load `chromium.zip` unpacked in Chrome
or Edge, or load `firefox.zip` as a temporary add-on in Firefox. Open NDM once
to register its native host. Only links you choose to send are shared with NDM.
[Store submission instructions](docs/EXTENSION_STORES.md) cover the separate
Chrome Web Store and Firefox Add-ons packages.

[راهنمای فارسی نصب و اتصال افزونه](docs/EXTENSION.fa.md)

## Development (optional, not run on the owner's computer)

On Windows, requires stable Rust with MSVC, Visual Studio C++ Build Tools and Windows SDK,
Node 24, and WebView2. CI installs these on a hosted Windows runner.

```powershell
npm ci --include=optional
npm run check
npm run build
cargo fmt --all --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
npm run desktop
```

`scripts/Build-Artifacts.ps1` builds Windows packages. On macOS, use `bash scripts/Build-Mac.sh` after `npm ci --include=optional`. Rust and npm
lockfiles are committed. `npm run dev` is a UI preview only; it does not simulate
downloads or insert sample files. No screenshots are claimed until the real
desktop app has been exercised and captured.

## Status and limitations

See the [complete implementation ledger and milestones](docs/IMPLEMENTATION_STATUS.md),
[acceptance status](docs/STATUS.md), [architecture](docs/ARCHITECTURE.md) and
[release notes](docs/RELEASE_NOTES.md). CI success is not proof of interactive
desktop behavior, correct RTL rendering, clean-machine installation or complete
crash resilience. These remain unverified acceptance criteria for this development alpha.

There is no telemetry, DRM bypass, page-video extraction or support for arbitrary
authenticated browser sessions. Auto connection tuning, sleep
prevention, expired-link replacement, background clipboard watching
and shutdown-after-queue are not implemented. The mobile app is Android only. PC
control and file sharing require the PC application to be running on the same local network.

The owner requested removal of the draft project LICENSE and authorized public
source and alpha binaries. No replacement license is added. Third-party licenses
and notices remain applicable; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
The NDM name is provisional; trademark and product-name availability have not
been established.

The supplied logo variants are cropped for their actual contexts: desktop and
installer icons, theme-aware sidebar, custom titlebar, browser extension and docs.
See [brand assets](docs/BRAND.md) for the crop map.

![NDM monochrome logo](apps/desktop/public/brand/wordmark-mono.png)

# Hosted Windows build evidence

## 0.1.0-alpha.7 automatic browser registration and real browser downloads

- CI: https://github.com/YeloBoi/next-download-manager/actions/runs/45174575936
- Release: https://github.com/YeloBoi/next-download-manager/actions/runs/45175978317
- Hosted Windows tested the packaged NSIS installer, then loaded the real release extension in Chrome, Edge and Firefox. Checked registration without manual setup, cold launch, automatic connection, queue-only, starting the same queued URL, and the SHA-256 of downloaded files.
- Hosted Windows verified cold/warm window restoration and app lifetime after the native host exits. Hosted Edge checked card hide/restore across connect/disconnect. Hosted Linux Firefox checked background/native-port recovery and retention of rejected links.
- Installer and portable app register Chrome, Edge and Firefox using stable IDs. Chromium is unpacked; Firefox is temporary and unsigned. No store publication.
- Published as Latest with 8 assets after validating SHA256SUMS.

| Release asset | Bytes |
| --- | ---: |
| NDM-0.1.0-alpha.7-chromium.zip | 40,420 |
| NDM-0.1.0-alpha.7-firefox.zip | 40,126 |
| NDM-0.1.0-alpha.7-windows-x64-portable.zip | 10,282,255 |
| NDM_0.1.0-alpha.7_x64-setup.exe | 6,117,396 |
| preview-en-dark.png | 81,313 |
| preview-fa-light.png | 76,035 |
| RELEASE_NOTES.md | 6,051 |
| SHA256SUMS | 663 |

## 0.1.0-alpha.5 automatic Firefox connection

- Source/tag: `a871252dc7908d160f5b2d5783781032f637dd44` / `v0.1.0-alpha.5`.
- Successful CI: https://github.com/YeloBoi/next-download-manager/actions/runs/44110216010
- Successful tagged checks/build/publication: https://github.com/YeloBoi/next-download-manager/actions/runs/44111011205
- Passed: TypeScript, 13 Node tests, frontend build, bilingual hosted Edge UI smoke,
  Rust fmt/check/warnings-denied all-target Clippy and 17 existing Rust tests.
- A separate hosted Windows integration test also passed: automatic HKCU Firefox
  registration, real native-host executable, offline reply without a desktop,
  reconnection to the real queue/bridge service on the same port and repeated pings.
  It is ignored in ordinary cargo test and explicitly run only on GitHub Windows.
- Actual hosted Firefox passed: initial automatic handshake without any popup,
  popup unload for 35 seconds, continued same native process, offline/online recovery,
  killed-host reconnection, persisted selection, failed send retention and Persian RTL.
  This browser test uses a native-host fixture; the Windows test uses the real host
  and queue service. The combined installed Windows Firefox/Tauri UI remains manual.
- Firefox auto registration runs on app startup. Check repairs it. A background-owned
  native port probes every five seconds; desktop freshness expires after 15 seconds.
- Both desktop and extension must be updated together; Mozilla signing remains pending.
- Windows installer/portable and both extension ZIPs built; release publisher verified
  SHA256SUMS before upload. Latest release ID `448814565` has 8 assets.

| Alpha 5 release asset | Bytes |
| --- | ---: |
| NDM-0.1.0-alpha.5-chromium.zip | 38,831 |
| NDM-0.1.0-alpha.5-firefox.zip | 38,893 |
| NDM-0.1.0-alpha.5-windows-x64-portable.zip | 9,313,871 |
| NDM_0.1.0-alpha.5_x64-setup.exe | 5,414,997 |
| preview-en-dark.png | 81,313 |
| preview-fa-light.png | 76,035 |
| RELEASE_NOTES.md | 3,771 |
| SHA256SUMS | 663 |


## 0.1.0-alpha.4 Firefox compatibility

- Source/tag: `574cd20ce1244e5e790eb097ef0c82ef2783ea0c` / `v0.1.0-alpha.4`.
- Successful Windows and Firefox CI: https://github.com/YeloBoi/next-download-manager/actions/runs/44104288158
- Successful tagged build/publication: https://github.com/YeloBoi/next-download-manager/actions/runs/44104487055
- Windows: TypeScript, 10 Node tests, frontend build, hosted Edge UI smoke checks,
  Rust fmt/check/warnings-denied Clippy, 17 Rust tests, native host, desktop,
  NSIS installer, portable ZIP and both browser extension ZIPs passed.
- Hosted Linux Firefox: Mozilla manifest lint had zero errors; one warning is
  Android-only minimum-version compatibility (NDM targets Windows desktop).
  Actual temporary installation, module background startup, saved selection across
  popup reload, retention after failed native delivery and Persian RTL passed.
- Native-host caller tests reject wrong Firefox manifest paths and add-on IDs.
  An installed Windows Firefox-to-native connection, cold launch, clean installer
  and permanent Mozilla signing remain unverified/not performed.
- Latest release ID: `448765943`; 8 published assets.

| Alpha 4 release asset | Bytes |
| --- | ---: |
| NDM-0.1.0-alpha.4-chromium.zip | 37,800 |
| NDM-0.1.0-alpha.4-firefox.zip | 37,862 |
| NDM-0.1.0-alpha.4-windows-x64-portable.zip | 9,294,040 |
| NDM_0.1.0-alpha.4_x64-setup.exe | 5,409,475 |
| preview-en-dark.png | 80,764 |
| preview-fa-light.png | 74,979 |
| RELEASE_NOTES.md | 2,830 |
| SHA256SUMS | 663 |


## 0.1.0-alpha.3 browser integration milestone

- Source/tag: `bf04b0262daa07f971e5bf06261f394dfa5c73ed` / `v0.1.0-alpha.3`.
- Successful Windows CI: https://github.com/YeloBoi/next-download-manager/actions/runs/44094938974
- Tagged release workflow: https://github.com/YeloBoi/next-download-manager/actions/runs/44095875811
- CI passed TypeScript, 8 Node tests, frontend production build, hosted Edge smoke
  checks, Rust formatting, workspace check, warnings-denied all-target Clippy and
  16 Rust tests. Native host, desktop release, NSIS installer, portable ZIP,
  extension ZIP, checksums/notes and all five artifact uploads succeeded.
- Tagged Windows build and the Ubuntu publish job both completed successfully.
  The publish job verified every SHA256SUMS entry before uploading seven assets.
- Public Latest endpoint verified as `v0.1.0-alpha.3`, release ID `448706071`:
  https://github.com/YeloBoi/next-download-manager/releases/latest

| Alpha 3 release asset | Bytes |
| --- | ---: |
| NDM_0.1.0-alpha.3_x64-setup.exe | 5,405,029 |
| NDM-0.1.0-alpha.3-windows-x64-portable.zip | 9,285,569 |
| NDM-0.1.0-alpha.3-chromium.zip | 37,574 |
| preview-en-dark.png | 80,764 |
| preview-fa-light.png | 74,979 |
| SHA256SUMS | 566 |
| RELEASE_NOTES.md | 2,207 |

New tests cover canonical selection ordering/limits, serialized cross-tab mutations,
service-worker restart with persistent storage, failed-send retention, context-menu
dispatch, sender validation, native message size/correlation, handshake expiry,
invalid ping payloads, legacy queue migration, atomic browser batches, deduplication
and persisted explicit user pause across reopening. Worker tests use a mocked
Chrome API; they are not evidence of an installed Chrome/Edge native connection.

Hosted Edge checked the actual built desktop interface, including the bilingual
unknown-connection card, existing brand assets, titlebar controls, theme, dialogs
and overflow at 1440, 768 and 390px. This is browser rendering, not native window
interaction. Native cold launch/focus, installed browser communication, extension
registration and clean-machine Windows 10/11 installation remain unverified.

All dependency installation, checks, compilation and packaging ran on GitHub.
No release binaries, build dependencies or hosted preview images were downloaded
onto the owner's machine. Local Rust formatting used the already installed formatter.
Binaries remain unsigned. Full outstanding requirements: [implementation ledger](IMPLEMENTATION_STATUS.md).

## 0.1.0-alpha.2

- Source/tag: `990526af88165f06840d8b21555b3e175c780c5a` / `v0.1.0-alpha.2`.
- Successful Windows CI: https://github.com/YeloBoi/next-download-manager/actions/runs/44080342849
- Tagged release workflow: https://github.com/YeloBoi/next-download-manager/actions/runs/44081589198
- Release: https://github.com/YeloBoi/next-download-manager/releases/tag/v0.1.0-alpha.2
- Tagged workflow completed successfully; its publish job verified checksums and
  uploaded all seven release assets. The repository was then made public at the
  owner's explicit request.

| Alpha 2 release asset | Bytes |
| --- | ---: |
| NDM_0.1.0-alpha.2_x64-setup.exe | 5,359,916 |
| NDM-0.1.0-alpha.2-windows-x64-portable.zip | 9,200,093 |
| NDM-0.1.0-alpha.2-chromium.zip | 34,843 |
| preview-en-dark.png | 76,862 |
| preview-fa-light.png | 71,533 |
| SHA256SUMS | 566 |
| RELEASE_NOTES.md | 1,119 |

Windows CI passed TypeScript, 5 Node tests, production frontend build, formatting,
workspace check, Clippy with warnings denied, 13 Rust tests, native host and Tauri
release builds, NSIS installer, portable ZIP, extension ZIP and all artifact uploads.
Hosted Edge smoke checks also passed English/dark and Persian/light UI, real crop
loading, a rounded frame, right-side window controls, the add-link dialog and
overflow checks at 1440, 768 and 390px. The two PNG previews show the actual built
frontend in Edge; they are not native Windows installer or desktop screenshots.

The supplied composite logo was cropped, with original interior colors retained.
The draft project LICENSE was removed at the owner's request. Dependency license
texts supplied by installed packages are collected into THIRD_PARTY_LICENSES for
the installer and portable archive. This is not a complete legal audit.

All builds and UI test execution took place on GitHub. No dependency installation,
local compilation or release artifact download occurred on the owner's machine.
Native window drag/resize/control behavior, clean-machine Windows installation and
browser-to-native integration remain unverified manually. Binaries are unsigned.

## Historical 0.1.0-alpha.1 evidence

- Repository: https://github.com/YeloBoi/next-download-manager (private pending legal metadata).
- Successful workflow: https://github.com/YeloBoi/next-download-manager/actions/runs/43959110412
- Built source: `6bb723248160f921d2fa4fd3ac2964351b25ce09`.
- Artifacts uploaded on 2026-11-16 UTC / 2026-11-17 Asia/Tehran.
- Runner: `windows-2022`, x64. No post-request local compilation or dependency installation.

All stages succeeded: npm clean install, TypeScript check, 5 Node tests, Vite
production build, Rust formatting, workspace check, all-target Clippy with warnings
denied, workspace tests (13 tests), native host release build, Tauri release build,
NSIS installer, portable and extension packaging, SHA256SUMS and artifact upload.

| GitHub artifact | Compressed bytes | Artifact ID |
| --- | ---: | ---: |
| ndm-installer-windows-x64 | 4,520,987 | 14274665335 |
| ndm-portable-windows-x64 | 6,687,280 | 14273541312 |
| ndm-chromium-extension | 5,112 | 14273401603 |
| ndm-checksums-and-notes | 990 | 14274610505 |

CI artifacts expire on 2026-11-30 UTC. These exact files are also attached to the
private owner-evaluation [v0.1.0-alpha.1 pre-release](https://github.com/YeloBoi/next-download-manager/releases/tag/v0.1.0-alpha.1),
where they are independent of Actions artifact retention. The promotion
[workflow](https://github.com/YeloBoi/next-download-manager/actions/runs/43961483131)
verified all four SHA256SUMS entries and uploaded five release assets on GitHub.
The source tag points to the built commit above. No rebuild or artifact download
on the owner's computer was needed.

| Release asset | Bytes |
| --- | ---: |
| NDM_0.1.0-alpha.1_x64-setup.exe | 4,538,766 |
| NDM-0.1.0-alpha.1-windows-x64-portable.zip | 6,709,659 |
| NDM-0.1.0-alpha.1-chromium.zip | 5,393 |
| SHA256SUMS | 391 |
| RELEASE_NOTES.md | 759 |

Choose the installer or the portable ZIP instead of downloading both.
The repository remains private pending rights-holder identity and binary-use terms.

The HTTP tests use a local controlled Axum server. They cover exact multipart
bytes and SHA-256, no overwrite, fresh fallback for ignored/rejected ranges,
unknown size, redirect, bounded retry, pause/resume, changed validators, HTML,
short bodies, hash mismatch and zero-byte files. Queue tests cover adding while
active, FIFO, drain, persistence, distinct pause/stop semantics and removal.
Unit tests cover filenames, schedule crossing midnight, URL validation, SQLite
state and invalid native message framing. This is a focused suite, not full
adversarial coverage of every acceptance criterion.

No interactive Windows 10/11 clean-machine installer test, accessibility/DPI/RTL
visual test, browser-to-native end-to-end test or digital signature validation was
performed. Binaries are unsigned. No public distribution was made.
See STATUS.md for remaining work. Documentation-only follow-up commits do not
change the source commit associated with the artifacts listed above.

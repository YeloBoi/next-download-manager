# Hosted Windows build evidence

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

# Hosted Windows build evidence

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

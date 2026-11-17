# Acceptance status

Development alpha. Version 0.1.0-alpha.2 adds the supplied brand assets, a custom
rounded titlebar and no draft project LICENSE, as requested by the owner.
Public source and pre-release distribution are authorized and published:
[repository](https://github.com/YeloBoi/next-download-manager) ·
[alpha 2 release](https://github.com/YeloBoi/next-download-manager/releases/tag/v0.1.0-alpha.2).

Implemented in source: direct HTTP(S), validated segmented ranges, fallback for
fresh downloads, strict resume identity, durable checkpoints, SQLite recovery,
FIFO actor queue, pause/stop/resume, reorder and batch add, concurrency ceilings,
live rate limit, SHA-256 comparison, unique destination names, English/Persian
resources, themes/density, schedule for starting work, tray menu, explicit native
messaging and unpacked extension, artifact and tag-gated release workflows.

Hosted Windows validation on commit `990526af88165f06840d8b21555b3e175c780c5a`
passed frontend checks/build, `cargo fmt --all --check`,
`cargo check --workspace --locked`, all-target Clippy with warnings denied, and
`cargo test --workspace --locked`: 13 Rust tests plus 5 Node tests. The release
executable, NSIS installer, portable ZIP and extension ZIP were built and all
five artifact uploads succeeded. Hosted Edge UI smoke checks passed English/dark,
Persian/light, asset loading, rounded frame, right-side controls, the add dialog
and overflow at 1440, 768 and 390px. See [build evidence](BUILD_REPORT.md).

All ongoing dependency installation, compilation and packaging must take place
on GitHub hosted runners at the owner's request. Do not build, install dependencies
or download build artifacts on the owner's computer without a new request.
Interactive desktop use, installation and real UI screenshots have not been
validated by the hosted build.

Remaining validation and limitations:

- Keep hosted Windows checks and packaging passing for future code changes.
- Actual desktop English/Persian shaping, keyboard/accessibility, DPI, light/dark,
  resized windows, dialogs, tray and application shutdown must be exercised.
- Clean-machine Windows 10/11 installer and uninstall tests, including extension
  registration and lack of developer tools, remain manual requirements.
- Disk-full/permission failures, process-kill crash recovery, cancellation races,
  redirects across hosts and source changes need wider adversarial coverage.
- Final-file publication currently requires hard links; non-NTFS destinations are
  not supported. Parts and destination may be on different volumes; free-space
  checks need refinement. There is no adaptive Auto connection mode.
- Notifications, clipboard monitoring, automatic interception, automatic host
  registration, sleep prevention, link replacement and shutdown actions are absent.
- Digital signing and a complete transitive dependency license audit remain open.

Next milestone: verify the actual installer and real UI, widen fault/recovery
coverage and continue the dependency inventory. Do not label a passing build as
proof of the manual acceptance checks.

# Acceptance status

Development alpha. No GitHub pre-release is asserted until it actually exists.

Implemented in source: direct HTTP(S), validated segmented ranges, fallback for
fresh downloads, strict resume identity, durable checkpoints, SQLite recovery,
FIFO actor queue, pause/stop/resume, reorder and batch add, concurrency ceilings,
live rate limit, SHA-256 comparison, unique destination names, English/Persian
resources, themes/density, schedule for starting work, tray menu, explicit native
messaging and unpacked extension, artifact and tag-gated release workflows.

Local verification before owner stopped local builds: TypeScript and five Node
tests passed; frontend production build passed after repairing npm optional
dependencies. Rust workspace build failed because MSVC `link.exe` was unavailable.
No Rust test pass, full desktop build, installer success or UI screenshot was
obtained locally. No more local builds/downloads are authorized; GitHub Actions
is the execution environment going forward.

The first hosted Windows run passed frontend checks/build, formatting and
`cargo check --workspace --locked`. Its all-target Clippy step caught a missing
type annotation in the shared HTTP fixture; that fixture was corrected for the
next run. This is not yet a Rust test pass or a packaged desktop build.

Release gates still open:

- Hosted Windows checks and packaging must pass for the exact source commit.
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
- Rights-holder identity, binary-use terms, contribution terms, dependency license
  audit, provisional product-name check and digital signing remain unresolved.

Next milestone: pass hosted checks, fix failures from runner logs, then verify the
actual installer and real UI before creating a public pre-release. Do not label
source implementation or a passing build as proof of those manual acceptance gates.

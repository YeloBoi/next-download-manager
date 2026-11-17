# Implementation status

Alpha.6 changes browser delivery to show/restore NDM and separates desktop process lifetime from Firefox's native-host job. The connection card is hidden while connected and returns on disconnect. New hosted acceptance covers real release-executable downloads after native job teardown; build evidence is recorded after CI/publication. Older sections below describe historical milestones.

## Automatic Firefox connection follow-up (alpha.5)

Verified in hosted component/integration checks: app-startup Firefox registration,
repair on Check, background-owned persistent native port, five-second retry/probes,
popup-independent status, fifteen-second desktop expiry and stable Check button.
Actual hosted Firefox tested no-popup startup, popup unload, desktop availability
changes and a killed native host. Hosted Windows tested automatic registry setup
and the real host exchanging repeated messages with the queue/bridge service.
See [build evidence](BUILD_REPORT.md). Chrome/Edge ID registration remains explicit;
combined installed Windows Firefox/Tauri acceptance and Mozilla signing remain open.
The older milestones below are historical baselines, not the current connection design.


## Firefox compatibility follow-up (alpha.4)

Verified within the hosted checks: separate Firefox manifest/package, fixed add-on
ID, native Firefox Promise APIs and Firefox caller validation. Hosted Firefox
installed the unsigned extension temporarily and exercised selection persistence,
failed-send retention and Persian RTL. Windows checks/build and checksum-verified
Latest publication also passed. See [build evidence](BUILD_REPORT.md).
Native-host registration is explicit via -Browser Firefox. Windows Firefox/native
end-to-end acceptance and permanent Mozilla signing remain open. The alpha.3
milestone below describes the previous baseline; advanced product work is unchanged.


The full Persian brief is preserved in [PRODUCT_REQUIREMENTS.fa.md](PRODUCT_REQUIREMENTS.fa.md).
This is an incremental implementation of that brief, not a claim that NDM is complete.
Status vocabulary: **Not started**, **In progress**, **Implemented** (source exists),
**Verified** (the stated check actually ran). Verification scope is recorded separately;
passing hosted tests does not establish installed Windows or real browser integration behavior.

Session constraints: compile, install dependencies, test and package on GitHub hosted
runners only. Keep the supplied cropped branding and custom rounded titlebar. Preserve
the owner's prior explicit removal of the project LICENSE; third-party notices remain.
Published versions use GitHub Latest; alpha remains in the version and release notes.
The brief's original alpha.1 target has already been superseded by alpha.2.

## Current milestone: browser connection and persistent selection (alpha.3)

Implemented. Hosted Windows CI verified the added automated checks on source
`bf04b0262daa07f971e5bf06261f394dfa5c73ed`: 16 Rust tests, 8 Node tests,
TypeScript, Edge desktop UI smoke checks, fmt/check/Clippy and all Windows packages.
Tagged build and checksum-verified publication also succeeded. Alpha.3 is public
and verified as GitHub Latest. See [build evidence](BUILD_REPORT.md).
Real native/browser/installer acceptance is not verified.

- Genuine native-message heartbeat; desktop displays unknown, recent connection or
  expired connection. No inference that an extension is uninstalled. Last valid
  message expires after 90 seconds; browser heartbeat every minute. Browser alarms
  can be delayed during sleep, so this is recent communication, not continuous presence.
- Real repository setup guide, no fabricated store URLs; English/Persian card and popup.
- Right-click download sends immediately. A second context action collects without
  downloading. Selection is ordered, deduplicated, stored in `chrome.storage.local`,
  numbered, removable and reorderable, with a count badge across tabs and popup closure.
- Serialized storage operations survive worker suspension. Failed sends remain for
  retry. Only acknowledged URLs are removed; batch acceptance is all-or-nothing.
- Native host opens only sibling `ndm.exe` on explicit add, never on background ping.
  Spool request is written before launch. Subsequent instances do not steal focus.
- Browser batches validate atomically in the queue actor, preserve internal order,
  insert Download now before waiting work, retain active transfers and respect an
  explicitly paused/stopped/draining queue, persisted across restart.
- Repeated URLs still present in the database are accepted without another download.
  This is URL deduplication, not a durable request-ID ledger after removal from history.

Added checks: selection ordering/canonicalization/limits, mocked worker suspension,
concurrent cross-tab mutation, failed retry retention, right-click behavior, foreign
sender rejection, native response correlation/size limits, heartbeat expiry, invalid
ping payload, queue restart/explicit pause/replay/atomic invalid batch, and bilingual
desktop card smoke checks. Native-host launch and installed extension remain manual.

## Coverage of all brief sections

| Brief | Status | Source / remaining work |
| --- | --- | --- |
| 1. Identity, priorities, honest capabilities | Implemented | Supplied brand, development alpha labels; no speed multiplier claims. |
| 2. Architecture, independent Rust engine | Implemented | Tokio/reqwest/SQLite/Tauri; [architecture](ARCHITECTURE.md). Slint decision documented. Single instance exists; cold browser launch added in this milestone, manual validation pending. |
| 3. Direct, segmented transfer, validated Range, resume | Implemented | Existing engine with hosted fixture tests in alpha.2. Adaptive Auto mode absent. More hostile Range/encoding/HEAD/redirect/source-change scenarios remain. |
| 3. Rate, retry, Retry-After, unknown size, timeouts | Implemented | Existing source; prior hosted tests cover a subset. Extend retry and cancellation fault matrix. |
| 3. TLS, redirect credentials, sensitive logs | In progress | TLS enabled; HTTP(S) validation and source redaction exist. Cross-origin credentials and adversarial redirects need explicit evidence. |
| 4. FIFO, add while active, pause/stop/drain/resume/reorder/skip | Implemented | SQLite actor, drag/drop and controls. Prior fixture tests; browser stop persistence covered by new test. Separate partial/final-file deletion choices still needed. |
| 5. Paths, names, .part, no overwrite, open/copy | Implemented | Sanitization and unique names. Disk space, non-NTFS publication and independent volume failures need completion. No arbitrary page/video extraction claim. |
| 6. Desktop layout, themes, density, details, charts, menus, keyboard | Implemented | Existing app; hosted bilingual smoke evidence in alpha.2. New card added. Real DPI/accessibility/keyboard and installer acceptance remain manual. Groups/Later fields absent. |
| 7. English/Persian and RTL | In progress | Central desktop translations, popup translations and [Persian extension setup](EXTENSION.fa.md); technical data LTR. Backend errors and remaining documentation still need full translation; native shaping/manual checks pending. |
| 8. Native protocol and minimal browser permissions | In progress | Bounded versioned messages, allowed origins, queue-commit acknowledgment. Separate accepted/started/completed per-link events, durable request-ID ledger, compatibility UX and automatic installer registration absent. No interception/cookie/history access. |
| 9. Smart extension card and setup | Implemented | Real recent handshake and actual guide. Store publication absent; guide uses Load unpacked and explicit registration. No invented store links. |
| 10. Context-menu download and collect | Implemented | Immediate send/cold sibling launch and collect action. Duplicate Show existing / Download again and browser/desktop notifications still needed; app-closed end-to-end test pending. |
| 11. Multi-tab selection, persistence, reordering, retry | In progress | Ordered durable selection, badge, remove/move, batch send now/queue. Named group, shared directory preview, per-item result, Save for later and type filters absent. |
| 12. Visual link selection, Shift range, cleanup, shortcut | Not started | Temporary activeTab/scripting mode, DOM order and restricted-page recovery. |
| 13. Groups, folders, group ceilings, priority/fairness | Not started | Persisted group model and scheduler constraints before UI. |
| 13. Download Later, bulk rename preview | Not started | Separate durable deferred state, natural-order proposal and safe rename transaction. |
| 14. Source page and expired-link replacement | Not started | Consent-based page metadata and validated identity before resume. |
| 14. SHA-256 comparison | Implemented | Reference input, computed hash, mismatch blocks publication. UI still needs separate transfer/size/hash labels. |
| 15. Startup, minimized, installer-owned entries | Not started | Per-user opt-in registration, real Windows status, portable move and uninstall ownership. |
| 15. Tray and independent resume setting | Implemented | Open/start/pause/quit and close-to-tray; resume default off. Manual tray/quit validation pending. |
| 16. Schedule across midnight | Implemented | Local-time start window and existing midnight unit test. Sleep/wake/time-change acceptance pending. |
| 16. Editable profiles, byte budget, sleep prevention, shutdown | In progress | Existing speed presets only; complete profiles, durable retry-inclusive budget, opt-in power actions absent. |
| 17. Quiet mode and Mini Window | In progress | Browser delivery does not focus existing window; cold launch hides it. Full preference, summary notifications, optional floating window and multi-monitor bounds absent. |
| 18. Combined search, Recent Activity, Undo | In progress | Existing name/source search, status filters. Persian normalization, type/group filters, durable activity, retention, clear and non-starting Undo absent. |
| 19. Safe optional archive extraction | Not started | Format/license review, path/symlink/bomb limits, multipart readiness, passwords, disk space and cancellation before enabling. |
| 20. Import/export and privacy | In progress | Text import/export with private-token confirmation, no telemetry. Group export schema, optional clipboard, data clearing and OS-protected secrets absent. |
| 21. Public source, documents, rights, naming | In progress | Public GitHub, tagged source and third-party inventory. Prior LICENSE removal retained. No open-license grant or trademark clearance is claimed. Store publishing, rights review and full dependency audit remain. |
| 22. Tests and CI | In progress | Hosted Windows fmt/check/clippy/tests/UI/build. Broader protocol, cancellation, disk-full, crash-kill, browser registration and clean Windows 10/11 tests pending. |
| 23. Installer, portable, extension, notes, SHA256SUMS | Verified | Alpha.3 checks, Windows packaging, checksum verification and seven release assets published; Latest verified. See [build evidence](BUILD_REPORT.md). Binaries unsigned; installed behavior still unverified. |
| 24. Execution and truthful handoff | In progress | This ledger and the complete brief retain all outstanding requirements. No unexecuted test is marked passing. |

## Next executable milestones

1. Completed: browser milestone source, hosted Windows checks and alpha.3 Latest
   publication. Complete its actual browser/native/installer manual acceptance next.
2. Persistent Download Later/groups, actor-level group limits, priorities/fairness,
   per-item results and request-ID ledger, with controlled HTTP queue tests.
3. Opt-in Windows startup/minimized and installer-owned registration/removal;
   editable network profiles and retry-inclusive durable usage budgets.
4. Visual selection/Shift, type filters, source page/link replacement, bulk rename,
   combined Persian-aware search, activity/Undo and the optional mini window.
5. Optional safe extraction, clipboard and power actions, broader failure recovery,
   actual Windows/browser/installer acceptance and final distribution review.

Developer commands, installation and extension registration are in [README](../README.md)
and [EXTENSION](EXTENSION.md). Do not run builds on the owner's computer.

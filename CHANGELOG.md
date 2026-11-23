# Changelog

## 0.1.0-alpha.13

- Add Bluetooth pairing for Android and Windows without an internet connection, alongside Wi-Fi / Internet pairing.
- Guide both devices through turning on Bluetooth and keep PC controls and file sharing on the existing encrypted connection.

## 0.1.0-alpha.11

- Pair through Tailscale across Wi-Fi and mobile data. Local pairing and remembered sessions can try more than one address.
- Check, download and verify Android updates from the latest GitHub Release before opening Android’s installer.
- Show browser extension updates in the popup; support Firefox’s native update feed for Mozilla-signed builds.

## 0.1.0-alpha.8

- Preview direct links with HEAD; suggest a destination and give confirmed downloads priority over the regular queue.
- Check for Windows updates through GitHub Releases, with release notes, a 24-hour reminder, download progress and SHA-256 verification.
- Add NDM for Android with local downloads, APK review, QR pairing with Windows, remote controls and completion notifications.
- Share multiple files in either direction on the local network and resume interrupted transfers.
- Compare completed internet traffic from Windows and Android by date and file type without counting local file shares.

## 0.1.0-alpha.3

- Persist ordered multi-tab extension selections, with badges, move/remove, retained failed sends and serialized storage mutations.
- Send context-menu downloads directly and launch only the sibling desktop executable on explicit browser requests.
- Deduplicate browser batches in the queue actor, preserve batch order and respect durable user pause/stop/drain decisions.
- Replace the fixed extension note with a bilingual card based on recent validated native handshakes and a real setup guide.
- Preserve the full product brief and track all incomplete requirements in docs/IMPLEMENTATION_STATUS.md.
- Add hosted queue, protocol, worker persistence and interface checks. Native installation/browser acceptance remains unverified.

## 0.1.0-alpha.2

- Crop and apply the owner-supplied NDM logo variants across desktop, installer, tray, extension and documentation.
- Replace native Windows decorations with a theme-aware rounded custom titlebar.
- Preserve close-to-tray and graceful download shutdown behind the custom close control.
- Remove the draft project LICENSE at the owner’s request; retain third-party notices.
- Publish hosted Windows binaries and source on GitHub.

## 0.1.0-alpha.1

- Rust transfer engine and persistent queue; desktop English/Persian UI.
- Chromium native host and explicit link submission.
- Hosted Windows validation, installer and portable artifact workflows.
- No stable release or completed manual acceptance testing is claimed.

# Build and release NDM on macOS

NDM targets macOS 12 or later. Build on Apple Silicon for an arm64 DMG and on
Intel for an x64 DMG. The desktop app uses the system WKWebView. Chrome, Edge
and Firefox communicate with the bundled native host. Android companion pairing
over the local network is available. Bluetooth and Tailscale pairing use Windows
integrations.

## Build and test

Install Xcode Command Line Tools, Node 24, Python 3, and Rust with the components
in `rust-toolchain.toml`. From the repository root run:

```sh
npm ci --include=optional
bash scripts/Build-Mac.sh
cargo test --workspace --locked --target "$(rustc --print host-tuple)"
```

The DMG is in `.artifacts`. Drag NDM to Applications and launch it from there.
Open Chrome or Firefox and try a known direct file link using the
[extension setup guide](EXTENSION.md). Quit NDM and repeat **Download with NDM**
to check that the browser can cold-start it. Test the Downloads path and, if
you use the phone companion, grant Local Network access and pair over Wi-Fi.

The **macOS desktop** workflow builds separate Apple Silicon and Intel DMGs
and checks that a native request cold-starts NDM, adds a link to the queue, and
lets Firefox establish a heartbeat. Run manual browser checks before distributing
the first public macOS build.

## Sign and notarize a tagged release

In GitHub Actions secrets, set `APPLE_CERTIFICATE` (base64 Developer ID
Application .p12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`,
`APPLE_ID`, `APPLE_PASSWORD` (an app-specific password), and `APPLE_TEAM_ID`.
Use the same settings when signing locally with Tauri. The tagged release
workflow attaches macOS DMGs after the app passes signature and stapled
notarization checks. Without a signing identity the workflow retains the DMGs
as CI artifacts for local testing. To offer those tested builds directly from an
existing GitHub Release, run **Attach tested macOS DMGs** with its release tag
and successful tagged-release workflow run ID. The attached filenames say
`unsigned` and the separate `SHA256SUMS-macos` covers both DMGs.

[Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) · [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)

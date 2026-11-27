#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

target="$(rustc --print host-tuple)"
case "$target" in
  aarch64-apple-darwin) architecture=arm64 ;;
  x86_64-apple-darwin) architecture=x64 ;;
  *) echo "Run this script on an Apple Silicon or Intel Mac." >&2; exit 1 ;;
esac

root="$PWD"
target_dir="$root/target"
export CARGO_TARGET_DIR="$target_dir"
export MACOSX_DEPLOYMENT_TARGET="$(node -p "require('./apps/desktop/src-tauri/tauri.macos.conf.json').bundle.macOS.minimumSystemVersion")"
cargo build --release --locked --target "$target" -p ndm-browser-bridge --bin ndm-native-host
mkdir -p apps/desktop/src-tauri/binaries .artifacts
install -m 755 "$target_dir/$target/release/ndm-native-host" "apps/desktop/src-tauri/binaries/ndm-native-host-$target"
node scripts/Collect-Notices.mjs
config='{"bundle":{"resources":{"../../../.artifacts/THIRD_PARTY_LICENSES":"THIRD_PARTY_LICENSES"}}}'
(
  cd apps/desktop
  ../../node_modules/.bin/tauri build --target "$target" --config "$config" -- --locked
)
bundle="$target_dir/$target/release/bundle"
test -x "$bundle/macos/NDM.app/Contents/MacOS/ndm-native-host"
if [[ -n "${APPLE_SIGNING_IDENTITY:-}" ]]; then
  codesign --verify --deep --strict "$bundle/macos/NDM.app"
  xcrun stapler validate "$bundle/macos/NDM.app"
fi
version="$(node -p "require('./apps/desktop/src-tauri/tauri.conf.json').version")"
dmgs=("$bundle"/dmg/*.dmg)
if [[ "${#dmgs[@]}" -ne 1 || ! -f "${dmgs[0]}" ]]; then
  echo "Expected one DMG in $bundle/dmg" >&2
  exit 1
fi
cp "${dmgs[0]}" ".artifacts/NDM-$version-macos-$architecture.dmg"

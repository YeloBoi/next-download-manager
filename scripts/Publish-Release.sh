#!/usr/bin/env bash
set -euo pipefail
cd "$1"
sha256sum --check --strict SHA256SUMS
shopt -s nullglob
extra=(./*.apk ./*.json ./*.xpi)
sha256sum -- "${extra[@]}" | sed 's#  \./#  #' >> SHA256SUMS
sha256sum --check --strict SHA256SUMS
gh release create "$RELEASE_TAG" ./*.exe ./*.zip ./*.png "${extra[@]}" SHA256SUMS RELEASE_NOTES.md --verify-tag --draft --prerelease --title "NDM $RELEASE_TAG" --notes-file RELEASE_NOTES.md
gh release edit "$RELEASE_TAG" --draft=false --prerelease=false --latest

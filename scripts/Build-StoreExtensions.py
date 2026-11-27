"""Build Chrome Web Store and listed Firefox packages from the shared source."""
import argparse
import json
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

ROOT = Path(__file__).resolve().parent.parent
SHARED = ROOT / "extensions" / "chromium"
FILES = (
    ["api.js", "background.js", "connection.js", "popup.css", "popup.html",
     "popup.js", "protocol.js", "selection.js", "updates.js"]
    + [f"icons/{size}.png" for size in (16, 32, 48, 128)]
    + [f"_locales/{locale}/messages.json" for locale in ("en", "fa")]
)


def store_manifest(browser):
    manifest = json.loads((ROOT / "extensions" / browser / "manifest.json").read_text())
    manifest.pop("host_permissions", None)
    if browser == "chromium":
        manifest.pop("key", None)
    else:
        settings = manifest["browser_specific_settings"]
        del settings["gecko"]["update_url"]
        settings["gecko_android"] = {"strict_min_version": "142.0"}
    return manifest


def build(browser, store, output):
    manifest = store_manifest(browser)
    filename = output / f"NDM-{manifest['version']}-{store}.zip"
    with ZipFile(filename, "w", ZIP_DEFLATED) as archive:
        archive.writestr(ZipInfo("manifest.json"), json.dumps(manifest, indent=2) + "\n", ZIP_DEFLATED)
        for name in sorted(FILES):
            archive.writestr(ZipInfo(name), (SHARED / name).read_bytes(), ZIP_DEFLATED)
    print(filename)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", nargs="?", type=Path, default=ROOT / ".artifacts" / "stores")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    build("chromium", "chrome-web-store", args.output)
    build("firefox", "firefox-amo", args.output)


if __name__ == "__main__":
    main()

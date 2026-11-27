"""Exercise the bundled macOS host's cold launch and both browser identities."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import time
import uuid

HOST = "local.ndm.bridge"
CHROMIUM_ORIGIN = "chrome-extension://llccemgchkhblimpddegikfekglcjoen/"
FIREFOX_ID = "ndm@yeloboi.github.io"


def send(host, arguments, kind):
    request = {
        "version": 1,
        "request_id": str(uuid.uuid4()),
        "kind": kind,
        "urls": ["https://example.com/ndm-mac-smoke.zip"] if kind == "add" else [],
        "start": False,
        "created_at": int(time.time()),
    }
    payload = json.dumps(request).encode()
    result = subprocess.run(
        [str(host), *arguments],
        input=struct.pack("<I", len(payload)) + payload,
        capture_output=True,
        check=True,
        timeout=45,
    )
    assert len(result.stdout) >= 4, result.stderr.decode()
    (length,) = struct.unpack_from("<I", result.stdout)
    assert length == len(result.stdout) - 4
    response = json.loads(result.stdout[4:])
    assert response["request_id"] == request["request_id"], response
    assert response["accepted"], response


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    args = parser.parse_args()
    host = args.bundle.resolve() / "Contents/MacOS/ndm-native-host"
    support = Path.home() / "Library/Application Support"
    chrome, firefox = [
        support / browser / "NativeMessagingHosts" / f"{HOST}.json"
        for browser in ("Google/Chrome", "Mozilla")
    ]
    subprocess.run([str(host), "--register"], check=True)
    try:
        for manifest, field, identity in (
            (chrome, "allowed_origins", CHROMIUM_ORIGIN),
            (firefox, "allowed_extensions", FIREFOX_ID),
        ):
            data = json.loads(manifest.read_bytes())
            assert Path(data["path"]) == host, data
            assert identity in data[field], data
        send(host, [CHROMIUM_ORIGIN], "add")
        send(host, [str(firefox), FIREFOX_ID], "ping")
        print("NDM cold-started, accepted a queue request, and answered a Firefox heartbeat.")
    finally:
        subprocess.run([str(host), "--unregister"], check=True)


if __name__ == "__main__":
    main()

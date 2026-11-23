#!/usr/bin/env bash
set -euo pipefail
adb shell input keyevent 82
adb install -r apps/android/app/build/outputs/apk/debug/app-debug.apk
adb install -r -t apps/android/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk
fixture="$(mktemp -d)"
openssl genpkey -algorithm EC -pkeyopt 'ec_paramgen_curve:P-256' -out "$fixture/key.pem"
openssl req -x509 -new -key "$fixture/key.pem" -subj '/CN=NDM' -days 1 -out "$fixture/cert.pem"
python3 scripts/android-fixture.py "$fixture" &
server=$!
trap 'kill "$server"; rm -rf -- "$fixture"' EXIT
for _ in $(seq 1 30); do test -s "$fixture/qr" && break; sleep 0.3; done
test -s "$fixture/qr"
adb reverse tcp:42672 tcp:42672
adb shell am instrument -w -e ndm_qr "$(cat "$fixture/qr")" io.github.yeloboi.ndm.test/io.github.yeloboi.ndm.AndroidSmoke | tee "$fixture/results"
grep -F NDM_ANDROID_SMOKE_OK "$fixture/results"

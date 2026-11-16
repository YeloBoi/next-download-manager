# Chrome / Edge native messaging

1. Download a successful GitHub build artifact and extract the portable ZIP and
   extension ZIP. Keep `ndm-native-host.exe` beside `ndm.exe`.
2. Open `chrome://extensions` or `edge://extensions`, enable Developer mode and
   choose **Load unpacked** for the extracted extension directory.
3. Copy the displayed extension ID. Run the following yourself, replacing the ID:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\Register-NativeHost.ps1 -ExtensionId YOUR_EXTENSION_ID -HostPath "$PWD\ndm-native-host.exe"
```

The process-scoped ExecutionPolicy option applies to this supplied setup script;
it does not permanently change the system policy. The script registers only
the current user's Chrome and Edge native-host keys. It allows exactly the
provided extension origin. Register again if the unpacked extension ID changes.
Each registration currently replaces the previous allowed ID.

4. Open NDM. Choose **Download with NDM** from a link's context menu, review the
   URLs, and choose **Download now** or **Add to queue** in the extension.

The desktop does not launch automatically from the native host. A failure leaves
normal browser downloads intact. Native messages have version, freshness, size,
URL and batch validation. The native host exchanges bounded spool files with the
desktop in the current user's `%LOCALAPPDATA%\NDM\bridge` directory; it opens no
TCP listener. Same-user local processes are within the trust boundary. No arbitrary
commands or user-selected destination paths are accepted through the protocol.

The installer includes the native host and registration scripts. Registration
requires the actual unpacked extension ID and is therefore an explicit post-install
step, not automatic. Installer uninstall removes the native-host registry keys;
run `Unregister-NativeHost.ps1` to unregister a portable installation. Queue data
and downloaded files are retained.

If connection fails, check NDM is open, the extension ID matches, the host exists,
and the HKCU native-host manifest points to the installed executable. Neither
Chrome Web Store nor Edge Add-ons publication has been performed.

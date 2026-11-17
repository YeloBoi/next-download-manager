# Firefox / Chrome / Edge native messaging

## Firefox (140 or later)

Download the **firefox.zip** asset from the latest release, not chromium.zip.
In Firefox open `about:debugging#/runtime/this-firefox`, select **Load Temporary
Add-on**, and select the downloaded ZIP (or its extracted manifest.json).
This development extension is unsigned: it remains installed until Firefox
restarts. Permanent installation needs Mozilla signing, which has not been done.
Do not disable signature enforcement or change Firefox security settings.

For desktop delivery, update NDM to **0.1.0-alpha.4 or later**, then run this
command from its installed/portable folder with the supplied registration script:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\Register-NativeHost.ps1 -Browser Firefox -HostPath "$PWD\ndm-native-host.exe"
```

Firefox uses the fixed ID `ndm@yeloboi.github.io` and the current user's Mozilla
native-host registry key. Registering Firefox preserves Chrome/Edge registration.
The Chromium-only `background.service_worker` manifest cannot load in Firefox;
the Firefox package instead supplies `background.scripts` with ES modules.

## Chrome / Edge

[راهنمای فارسی نصب و اتصال](EXTENSION.fa.md)

1. Open the [latest release](https://github.com/YeloBoi/next-download-manager/releases/latest).
   Install the Windows x64 `*-setup.exe`, or extract the portable ZIP. Also extract
   the `*-chromium.zip` extension. Keep `ndm-native-host.exe` beside `ndm.exe`;
   run the registration command from that installed or portable app folder.
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

4. Choose **Download with NDM** from a link's context menu to send that link
   immediately. The registered host opens the sibling `ndm.exe` if necessary.
   An explicitly paused/stopped queue remains paused; use **Start queue** in NDM.
   **Add link to NDM selection** collects a link without starting a download.
5. Collect from several tabs, then open the extension popup. Review the numbered
   selection, move/remove links, or find links on the current page and explicitly
   **Add to selection**. Choose **Download all** or **Add all to queue**.
   Selections are saved locally across popup closure and worker suspension.
   Failed sends remain for retry; only acknowledged links leave the selection.

The host launches only the sibling desktop executable on an explicit add request;
background connection checks never launch it. Browser delivery avoids focusing an
already running instance. A failure leaves normal browser downloads intact.
Native messages have version, freshness, size,
URL and batch validation. The native host exchanges bounded spool files with the
desktop in the current user's `%LOCALAPPDATA%\NDM\bridge` directory; it opens no
TCP listener. Same-user local processes are within the trust boundary. No arbitrary
commands or user-selected destination paths are accepted through the protocol.

The installer includes the native host and registration scripts. Registration
requires the actual unpacked extension ID and is therefore an explicit post-install
step, not automatic. Installer uninstall removes the native-host registry keys;
run `Unregister-NativeHost.ps1` to unregister a portable installation. Queue data
and downloaded files are retained.

The desktop card uses the most recent valid native-message handshake, expiring
after 90 seconds. A browser alarm pings once per minute; an open popup also checks
periodically. Sleep or delayed alarms can temporarily show disconnected. This
does not prove the extension is uninstalled. Open the popup to check immediately.
The card's setup/manage action opens this real guide; automatic browser installation
is not performed. The extension adds only the `alarms` permission for these checks.

If connection fails, check the extension ID matches, the host and sibling app exist,
and the HKCU native-host manifest points to the installed executable. Neither
Chrome Web Store nor Edge Add-ons publication has been performed.

This is an unpacked development extension. Automatic native-host registration,
visual/Shift selection, groups, filters, Save for later, source-page metadata and
per-download completion notifications remain in the [implementation ledger](IMPLEMENTATION_STATUS.md).
Retries deduplicate URLs still in the queue/history. There is no durable request-ID
ledger after an entry has been removed. Actual app-closed/native/browser installation
behavior still requires Windows acceptance testing.

# Connect NDM to Chrome, Edge, or Firefox

[راهنمای فارسی](EXTENSION.fa.md) · [Latest release](https://github.com/YeloBoi/next-download-manager/releases/latest)

Install the current NDM desktop app and open it once. The Windows installer
registers the native host for your current user. For the Windows portable ZIP,
open `ndm.exe` with `ndm-native-host.exe` beside it. On macOS, drag `NDM.app`
to Applications and open it there. **Check connection** in NDM repairs native
host registration.

## Chrome and Edge

1. Extract the **chromium.zip** asset to a folder you can keep.
2. Open `chrome://extensions` or `edge://extensions`, enable **Developer mode**, and
   choose **Load unpacked**. Select the folder that contains manifest.json.
3. Disable the old unpacked NDM extension if you see two entries. The current build
   has a fixed ID (`llccemgchkhblimpddegikfekglcjoen`) that is registered automatically
   for both browsers. No extension ID or PowerShell command is needed.

The extension popup links to newer versions. To update an unpacked install,
extract the new ZIP into the same folder, then click **Reload** on the extension
card. When the Chrome Web Store listing is available, install from the store
for automatic updates. A store copy has a different ID from the unpacked ZIP.
Disable the unpacked copy after switching. If the store copy stays offline,
check the registration steps below.

## Firefox 140 or later

Open `about:debugging#/runtime/this-firefox`, select **Load Temporary Add-on**, and
choose **firefox.zip** from the release (or its extracted manifest.json). Reload
it after restarting Firefox. If a Mozilla-signed **firefox.xpi** is available
in a release, install it once for permanent installation and automatic updates.
The ZIP is temporary. The extension uses Firefox background scripts rather than the
Chromium service worker and has the fixed ID `ndm@yeloboi.github.io`. When the
listed Firefox Add-ons version is available, install it from Firefox Add-ons
for automatic updates.

## Download a file

Choose **Download with NDM** from a direct link's context menu. To collect links
first, choose **Add link to NDM selection**, paste links into the extension, or
use **Find links on this page** on an ordinary webpage. Then choose **Download all**
or **Add all to queue**. Downloading opens NDM or restores the existing window.
An explicitly paused or stopped queue still needs **Start queue** in NDM.

The extension sends only selected HTTP(S) URLs. Links that need a browser login,
temporary blob URLs, and streamed video may not be direct downloads. Selections
stay on your computer and unacknowledged links remain available for retry.
There is no automatic interception of ordinary browser downloads.

The desktop card disappears while connected and returns after the last browser
heartbeat expires. The popup can close when focus moves to NDM; the background
keeps the connection and stored selection.

## Register a Chrome Web Store or custom Chromium ID

The GitHub ZIP's ID and Firefox ID are built in. If your desktop build predates
store registration, copy the 32-character ID from `chrome://extensions` with
Developer mode enabled. Run the command for your platform, substituting that
ID, then reload the browser extension.

- Windows PowerShell, from the NDM application folder:
  `.\ndm-native-host.exe --register <extension-id>`
- macOS Terminal:
  `/Applications/NDM.app/Contents/MacOS/ndm-native-host --register <extension-id>`

Use the exact ID for the extension installed in your browser. If you remove
NDM manually, run the same native host with `--unregister` before removing
the application. This clears the registrations owned by that copy while
preserving the download queue and files.

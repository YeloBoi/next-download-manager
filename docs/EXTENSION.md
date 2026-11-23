# Connect NDM to Chrome, Edge, or Firefox

[راهنمای فارسی](EXTENSION.fa.md) · [Latest release](https://github.com/YeloBoi/next-download-manager/releases/latest)

Update both NDM and the browser extension from the same release. The Windows
installer registers the native host for your current user. If you use the portable
ZIP, open ndm.exe once; it registers the companion ndm-native-host.exe in the same
folder. **Check connection** in NDM repairs that registration.

## Chrome and Edge

1. Extract the **chromium.zip** asset to a folder you can keep.
2. Open `chrome://extensions` or `edge://extensions`, enable **Developer mode**, and
   choose **Load unpacked**. Select the folder that contains manifest.json.
3. Disable the old unpacked NDM extension if you see two entries. The current build
   has a fixed ID (`llccemgchkhblimpddegikfekglcjoen`) that is registered automatically
   for both browsers. No extension ID or PowerShell command is needed.

The extension popup links to newer versions. To update an unpacked install,
extract the new ZIP into the same folder, then click **Reload** on the extension
card. The extension is not currently published in Chrome Web Store or Edge Add-ons.

## Firefox 140 or later

Open `about:debugging#/runtime/this-firefox`, select **Load Temporary Add-on**, and
choose **firefox.zip** from the release (or its extracted manifest.json). Reload
it after restarting Firefox. If a Mozilla-signed **firefox.xpi** is available
in a release, install it once for permanent installation and automatic updates.
The ZIP is temporary. The extension uses Firefox background scripts rather than the
Chromium service worker and has the fixed ID `ndm@yeloboi.github.io`.

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

For custom Chrome/Edge builds only, use `ndm-native-host.exe --register <extension-id>`
from the application folder. This preserves the official ID and previously
registered IDs. Uninstalling or running `Unregister-NativeHost.ps1` removes the
registration owned by that NDM copy; download and queue data remain.

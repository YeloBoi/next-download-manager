# Submit NDM to the browser stores

Build the two upload ZIPs from the repository root:

```sh
python3 scripts/Build-StoreExtensions.py
```

The ZIPs land in `.artifacts/stores/`. The **Browser store packages** Actions
workflow builds the same ZIPs, lints the Firefox package, and exercises both
packages in hosted browsers. Each ZIP uses the extension version in its manifest.
Increment that version before replacing a version the store already has.

## Listing copy

**Name** — NDM — Next Download Manager

**Single purpose** — Send direct HTTP(S) file links you choose to the local
NDM download manager.

**Description** — Right-click a direct file link to download it with NDM, or
collect several links and send them to the queue together. The extension
keeps unsent selections in your browser so you can retry. It requires the
NDM desktop app on Windows or macOS. It supports English and Persian.

Use the 128px extension icon from `extensions/chromium/icons/128.png` for
Chrome. Capture store screenshots from an installed build showing the popup
with selected links and the connected desktop app. Chrome accepts a 1280×800
or 640×400 screenshot. Use only sample public URLs when taking screenshots.

**Privacy policy URL** — Use the public GitHub URL for
[`docs/EXTENSION_PRIVACY.md`](EXTENSION_PRIVACY.md) after it is published.

| Permission | Justification |
| --- | --- |
| `nativeMessaging` | Sends selected download URLs to the locally installed NDM app and receives its acknowledgement and connection status. |
| `contextMenus` | Adds explicit actions to collect or send a link. |
| `storage` | Stores selected URLs and language locally so users can retry or resume their selection. |
| `alarms` | Reconnects to the local desktop host after the browser suspends its background process. |
| `activeTab` | Grants temporary access to the current page when the user opens the extension. |
| `scripting` | Runs bundled code to read direct URLs only when the user selects **Find links on this page**. |

Describe the selected URLs and current-page link extraction in the store's
privacy form, including any website content or browsing activity categories
the dashboard requires. Confirm the answers against the current dashboard
definitions. Store packages use bundled code, no persistent website access,
and store-provided extension updates.

## Chrome Web Store

1. Create or open the item in the Chrome Web Store Developer Dashboard. Upload
   `NDM-<version>-chrome-web-store.zip`. The dashboard assigns the published ID.
2. Save the public item ID as the repository Actions variable
   `NDM_CHROME_STORE_ID`. Build a new desktop release so its native host
   recognizes the published ID automatically. The existing developer ZIP keeps
   its own ID.
3. Verify the store-signed installation and submit the listing and privacy
   disclosures. For an earlier desktop build, [register the store ID
   manually](EXTENSION.md#register-a-chrome-web-store-or-custom-chromium-id).

[Chrome Web Store Developer Dashboard](https://chrome.google.com/webstore/devconsole/) · [Chrome Web Store program policies](https://developer.chrome.com/docs/webstore/program-policies/)

## Firefox Add-ons (AMO)

1. Choose distribution **On this site** and upload `NDM-<version>-firefox-amo.zip`.
   Select desktop Firefox. The extension uses the existing
   `ndm@yeloboi.github.io` ID so desktop registration works automatically.
2. If you already signed that ID for self-distribution, use the same AMO
   account and add a listed version to that add-on. Use a version number
   higher than any version AMO already accepted for the ID.
3. Add the listing and privacy policy. The ZIP contains the unminified extension
   source. AMO hosts and updates the listed version. The separate GitHub
   developer ZIP and any self-distributed XPI continue to use the GitHub feed.

[Firefox Add-on Developer Hub](https://addons.mozilla.org/developers/) · [Mozilla Add-on Policies](https://extensionworkshop.com/documentation/publish/add-on-policies/)

## Reviewer instructions

Install the current NDM desktop app from the public release URL. Launch
it once and choose **Check connection** if needed. If testing an early
Chrome build, include the exact store extension ID and the manual registration
command in the private review notes. Open the extension, paste a public direct
file URL, and choose **Add all to queue**. The link should appear in NDM and
the acknowledged selection should clear. Then test **Download all** with a
different direct file URL. No account or test credentials are required.

[NDM Releases](https://github.com/YeloBoi/next-download-manager/releases/latest)

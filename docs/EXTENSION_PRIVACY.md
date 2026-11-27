# NDM Browser Extension Privacy Policy

The NDM browser extension helps you send direct file links to the NDM desktop
application running on your computer.

- The extension processes links that you paste, select from a context menu, or
  collect using **Find links on this page**. When you request link collection,
  the extension reads HTTP(S) link addresses on the active page. It does not
  read browser history, cookies, form values or passwords.
- Selected URLs, language and delivery state are saved in local browser
  extension storage. Use **Clear selection** in the popup to delete the current selection.
  After NDM acknowledges a send, those links are removed from the selection.
  Removing the extension clears its locally stored selection and settings.
- When you choose to send links, the full URLs, including any query parameters,
  are sent to the local NDM desktop application. NDM stores queued URLs in its
  local queue. When a download runs, the desktop app contacts the selected
  destination to retrieve the file.
- Store-distributed builds use the browser store's update service. Separately
  distributed GitHub builds can check GitHub for extension updates.
- The extension does not send selected URLs or page content to the developer,
  sell data, use advertising, or include analytics or telemetry. NDM uses
  processed information only to provide the download and connection features
  described here. Its use of information received from Chrome APIs adheres to
  the Chrome Web Store User Data Policy, including its Limited Use requirements.

For privacy questions, contact the project maintainers through the repository.
Keep private URLs and credentials out of public issues.

[Project repository](https://github.com/YeloBoi/next-download-manager)

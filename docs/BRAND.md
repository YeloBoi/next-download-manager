# NDM brand assets

The owner supplied a 1024 × 1024 composite logo sheet. These are crops of that
image, with no generated replacement artwork. Coordinates are measured from its
top-left corner. White outside the blue mark, dark tile and monochrome lockup is
removed; original interior colors are retained.

| Crop | Rectangle (x, y, width, height) | Use |
| --- | --- | --- |
| `mark-blue.png` | 102, 202, 306, 312 | Custom titlebar and empty queue |
| `wordmark-color.png` | 102, 202, 822, 312 | README and documentation header |
| `app-dark.png` | 54, 674, 230, 228 | Windows app, installer, tray, dark sidebar and extension icons |
| `app-light.png` | 335, 673, 236, 232 | Light-theme sidebar |
| `wordmark-mono.png` | 626, 718, 363, 143 | Monochrome documentation variant |

Assets live in `apps/desktop/public/brand`. The ICO includes 16, 24, 32, 48, 64,
128 and 256px entries; Chromium icons include 16, 32, 48 and 128px.
`scripts/Crop-BrandAssets.ps1 -Source <original-sheet.png>` recreates them using
Windows drawing APIs without installing dependencies or contacting the network.

The existing quiet charcoal layout and blue product mark are retained. The new
46px custom titlebar spans both columns; its drag area excludes window controls.
The transparent, undecorated window clips the app to a 16px radius when restored,
and removes that radius while maximized. Minimize, maximize/restore and close use
Tauri window APIs; closing preserves the queue shutdown and close-to-tray setting.
Actual Windows resize, drag and window controls require desktop validation.

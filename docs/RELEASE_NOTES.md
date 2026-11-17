# NDM 0.1.0-alpha.2 — development pre-release

- Supplied NDM logo cropped into color wordmark, blue symbol, dark/light app tiles and monochrome wordmark.
- New Windows installer, taskbar/tray and Chromium extension icons.
- Custom themed titlebar replaces the Windows titlebar; rounded restored window, draggable area, minimize, maximize/restore and close controls.
- Window controls remain at the right in English and Persian. Closing preserves graceful queue shutdown and the close-to-tray setting.
- Owner-requested removal of the draft project LICENSE; no replacement license added. Third-party notices remain applicable.

Rust direct-download engine, persistent queue and English/Persian Tauri interface.
Portable mode stores state in LocalAppData. WebView2 is required and the installer
may download it if missing. Native messaging requires an explicit extension ID.

Unsigned alpha, not stable. Automated hosted checks do not establish clean-machine
installation, native window drag/resize behavior, accessibility or browser-to-native
end-to-end correctness. See docs/STATUS.md for the remaining limitations.

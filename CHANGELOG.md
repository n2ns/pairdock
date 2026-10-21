# Changelog

All notable changes to PairDock are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [0.0.2] - 2026-10-21

### Added

- macOS support: pairing, unpairing and button settings, a menu bar app without a Dock icon,
  a notice that leads to the Accessibility permission needed for shortcuts, and shortcut presets
  with Mac keys. Releases include `.dmg` files for Apple silicon and Intel Macs and update files
  for both.
- Common shortcuts for switching to the desktop on the left or right (Windows: `Win+Ctrl+Left` /
  `Win+Ctrl+Right`; macOS spaces: `Ctrl+Left` / `Ctrl+Right`), switching windows (`Alt+Tab`,
  `Cmd+Tab`), closing the window (`Alt+F4`, `Cmd+W`), the emoji panel (`Win+Period`,
  `Ctrl+Cmd+Space`), the app's windows on macOS (`Ctrl+Down`) and the previous track.
- `pairdock-cli send <shortcut>` sends a shortcut the way a mapped button does, and
  `scripts/smoke-shortcuts.ps1` checks what every Windows preset does.

### Changed

- A smaller window and interface (85 %) and a narrower sidebar, to fit laptop screens; the mouse
  picture on the button settings page is smaller, leaving more room for the settings.

### Removed

- The Copy, Paste and Undo common shortcuts; they can still be recorded.

## [0.0.1] - 2026-10-21

First public version.

### Added

- Pair and unpair devices on Logi Bolt receivers, with a guided pairing screen that highlights
  the next mouse button to click or the next digit to type.
- Detection of old slots a device leaves behind when it is paired again, with one-click cleanup.
- Button customization for Logitech mice: another button (stored on the device), keyboard
  shortcut or disabled. Settings are re-applied whenever the mouse reconnects.
- Battery level of connected devices.
- Tray app with launch at startup; buttons are restored when quitting.
- Light and dark appearance, English and Chinese interface.
- `pairdock-cli` command-line tool with protocol tracing.
- Update check on GitHub at startup (can be turned off) and in Settings; updates are signed and
  installed only after confirmation.

# Changelog

All notable changes to PairDock are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

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

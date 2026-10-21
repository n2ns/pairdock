# Privacy

[简体中文](PRIVACY.zh-CN.md)

PairDock is a local tool. It has no account, no telemetry, no crash reporting and no
advertising, and it does not connect to the internet.

## Network

PairDock makes no network requests. Its interface is bundled with the program and runs in
Microsoft Edge WebView2, which PairDock does not ask to load anything from the internet.

Two parts of Windows that PairDock relies on are maintained by Microsoft under Microsoft's own
terms:

- **WebView2** keeps itself up to date through Microsoft's update service, independently of
  PairDock.
- The **installer** downloads the WebView2 runtime from Microsoft only if it is missing. It is
  part of Windows 11 and most up-to-date Windows 10 installations.

## What PairDock accesses

- **Logitech receivers**, through the HID driver built into Windows. PairDock reads the names,
  model IDs, serial numbers, battery levels and button capabilities of the devices paired to
  them, and sends the commands to pair, unpair and configure them.
- **Your mouse buttons**, only those you set to *Shortcut* or *Disabled*. The mouse then reports
  presses of those buttons to PairDock instead of to Windows. PairDock does not see mouse
  movement or any other button.
- **Your keyboard**, not at all for reading: PairDock never reads what you type. It sees key
  presses only inside its own window while you record a shortcut. It sends key presses to
  Windows only for the shortcuts you set up, while the mapped button is held.

## What PairDock stores

Everything stays on your computer:

| What | Where |
|---|---|
| Button settings: device model IDs and the action of each button | `%APPDATA%\dev.pairdock.desktop\config.json` |
| Language and appearance | `%LOCALAPPDATA%\dev.pairdock.desktop` (WebView2 data) |
| The startup entry, if *Launch at startup* is on | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`, value `PairDock` |

PairDock writes no log files. Device serial numbers are shown, not stored.

## Protocol traces

The command-line tool (`pairdock-cli --trace <file>`) can record every message exchanged with
the receiver, for troubleshooting. This file is only written when you ask for it, and only
where you choose. It contains device addresses, serial numbers and, while pairing, the pairing
code, so review it before attaching it to a bug report.

## Removing your data

Uninstall PairDock and delete the two folders listed above. Turning off *Launch at startup*
before uninstalling removes the startup entry.

## Changes

If PairDock ever needs network access (for example, to check for updates), this document will
say so before the release that adds it, and the change will be listed in the
[changelog](CHANGELOG.md).

Questions: [open an issue](https://github.com/n2ns/pairdock/issues).

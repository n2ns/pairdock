<p align="center">
  <img src="public/icon.svg" width="104" alt="PairDock icon">
</p>

<h1 align="center">PairDock</h1>

<p align="center">
  Pair mice and keyboards to your Logitech receiver and customize their buttons —<br>
  without Logi Options+.
</p>

<p align="center">
  <a href="https://github.com/n2ns/pairdock/releases/latest"><b>Download</b></a> ·
  <a href="docs/user-guide.md">User guide</a> ·
  <a href="PRIVACY.md">Privacy</a> ·
  <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <img src="docs/images/devices-light.png" width="49%" alt="PairDock in light mode">
  <img src="docs/images/devices-dark.png" width="49%" alt="PairDock in dark mode">
</p>

## Why PairDock

Logi Options+ is a large suite with background services, an updater, an account, cloud
features and analytics. If all you need is to pair a mouse or keyboard to your receiver and
remap a couple of buttons, PairDock does that and nothing else:

- **Small** — one ~3 MB installer, one tray app.
- **No Logitech software or driver** — it talks to the receiver through the HID driver that
  comes with Windows.
- **Private** — no account, no telemetry; the only connection is an update check on GitHub,
  which you can turn off. See [PRIVACY.md](PRIVACY.md).
- **Never holds up shutdown** — it answers Windows' shutdown request immediately.

## Features

**Guided pairing.** Put the device into pairing mode, pick it from the list, and follow the
picture: PairDock highlights which mouse button to click next — left, right, and finally
both together — or which digits to type on a keyboard.

<p align="center">
  <img src="docs/images/pair-confirm.png" width="820" alt="Pairing a mouse: the next button to click is highlighted">
</p>

**Button customization.** Pick a button on the top or side view of the mouse and choose what
it does:

| Action | What it does | Needs PairDock running |
|---|---|---|
| Default | The button's original function | No |
| Another button | Performs another button's action, stored on the mouse | Yes, in the tray |
| Shortcut | Sends a key combination such as `Ctrl+C` or `Alt+Left` while held | Yes, in the tray |
| Disabled | The button does nothing | Yes, in the tray |

<p align="center">
  <img src="docs/images/buttons.png" width="820" alt="Mapping the back button to a shortcut">
</p>

**And also:** battery level of connected devices, unpairing, cleaning up the old slot a device
leaves behind when it is paired again, light and dark themes, English and Chinese.

## Requirements

- Windows 10 or 11 (64-bit) with the Microsoft Edge WebView2 runtime (included in Windows 11).
- A **Logi Bolt** USB receiver and devices with the Bolt logo.

| | Status |
|---|---|
| Logi Bolt receiver | Supported |
| Logitech Unifying receiver | Detected, not supported yet (planned) |
| Mouse pairing and button settings | Tested with a Logi M650 |
| Keyboard pairing | Implemented, not yet tested on hardware |
| macOS, Linux | Not yet; the code is structured for them |

## Install

1. Download `PairDock_x.y.z_x64-setup.exe` from the
   [latest release](https://github.com/n2ns/pairdock/releases/latest).
2. Run it. The installer is not code-signed yet, so Windows SmartScreen may show
   *"Windows protected your PC"*. Click **More info → Run anyway**.
3. If Logi Options+ is installed, quit it or uninstall it first. Both programs would try to
   handle the same buttons.

Then follow the [user guide](docs/user-guide.md).

## FAQ

**Does it work with other brands?** No. Pairing and button settings use Logitech's own HID++
protocol, which only Logitech receivers and devices understand. Bluetooth devices of any brand
are paired in the Windows Bluetooth settings.

**Do my devices stay paired if I uninstall PairDock?** Yes. Pairings are stored in the
receiver, not on the computer.

**Do shortcuts work when PairDock is closed?** Shortcuts and disabled buttons need PairDock
running in the tray. Buttons mapped to another button are stored on the mouse, but the
mouse forgets them when it reconnects and PairDock sets them again. Closing the window keeps PairDock in the tray; use **Quit** in the tray menu to
exit, which restores the buttons first.

## Contributing

Bug reports with your device model and a protocol trace help most — see
[CONTRIBUTING.md](CONTRIBUTING.md) for how to record one and how to build PairDock from source.

## License

[MIT](LICENSE). The vendored `crates/hidpp` is 0BSD, see [its notice](crates/hidpp/NOTICE.md).

PairDock is an independent project and is not affiliated with or endorsed by Logitech.
"Logitech", "Logi", "Logi Bolt", "Unifying" and "Logi Options+" are trademarks of Logitech.

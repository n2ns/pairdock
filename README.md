<p align="center">
  <img src="public/icon.svg" width="112" alt="PairDock icon">
</p>

<h1 align="center">PairDock</h1>

<p align="center">
  Pair devices to a Logitech <b>Logi Bolt</b> receiver and customize mouse buttons —<br>
  without installing Logi Options+.
</p>

---

Logi Options+ is a large suite with dozens of features, background services and an
updater. If all you need is to pair a keyboard or mouse to your receiver and remap a
couple of side buttons, PairDock does just that:

- **Pairing** — discover devices in pairing mode, pair them with an animated guide that
  highlights which mouse button to click next (or which keys to type on a keyboard),
  and unpair devices.
- **Button customization** — turn a mouse button into another button (stored on the
  device), a keyboard shortcut such as `Ctrl+C` or `Alt+Left`, or nothing at all.
- **Small and quiet** — a single tray app. It talks to the receiver through the
  operating system's built-in HID driver; no Logitech software or driver is needed.
  It answers shutdown requests immediately, so it never shows up as an app that
  "prevents shutdown".

> PairDock is an independent project and is not affiliated with or endorsed by
> Logitech. "Logitech", "Logi Bolt" and "Logi Options+" are trademarks of Logitech.

## Status

| | |
|---|---|
| Platforms | Windows 10/11. The code is structured for macOS and Linux; they are not wired up or tested yet. |
| Receivers | Logi Bolt (`046d:c548`). Unifying receivers are detected but not supported yet. |
| Tested with | Logi M650 (pairing, side buttons), Alto Keys K98M (listing). |

## How button customization works

Logitech mice expose their buttons through the HID++ feature `0x1b04`
(*REPROG_CONTROLS_V4*). Each button can be

- **remapped** on the device to another button of a compatible group (for example
  Back → Middle). This needs no software running, but the device forgets it when it
  reconnects, so PairDock re-applies it whenever the device comes online;
- **diverted**: the device stops sending the button and reports presses to PairDock,
  which then sends the configured shortcut while the button is held. This needs
  PairDock running in the tray.

Most mice (the M650 included) cannot store diversions persistently, which is why
PairDock keeps a small engine running in the tray. When you quit from the tray menu,
PairDock restores the buttons first (for up to three seconds, since a sleeping mouse
only receives the request once it wakes up). If PairDock ever exits without that,
`pairdock-cli reset` restores every button.

The button settings are stored per device model in `config.json` in the app's
config directory (`%APPDATA%\dev.pairdock.app` on Windows).

## Command-line tool

`pairdock-cli` covers the same operations and is handy for scripting and debugging:

```text
pairdock-cli list                    # receivers and paired devices
pairdock-cli pair                    # pair the first device found in pairing mode
pairdock-cli unpair <slot>
pairdock-cli info <slot>             # features and reprogrammable buttons
pairdock-cli run --config file.json  # apply button settings until Ctrl+C
pairdock-cli reset                   # restore all buttons to their defaults
pairdock-cli --trace trace.log ...   # log every HID++ report to a file
```

See [`examples/m650.json`](examples/m650.json) for the configuration format.

## Building

Requirements: Rust (stable, 2024 edition), Node.js 20+ and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform.

```bash
npm install
npm run tauri dev        # run the app
npm run tauri build      # build installers
cargo test               # unit tests
cargo run -p pairdock-cli -- list
```

`npm run dev` alone serves the UI in a browser against a simulated backend (demo
mode), which is useful for working on the interface without a receiver.

## Project layout

```text
crates/hidpp          HID++ protocol (vendored from lus/logy, 0BSD, with additions)
crates/pairdock-core   HID transport, pairing, inventory and the remapping engine
crates/pairdock-cli    command-line tool
src-tauri             desktop app backend (Tauri 2)
src                   desktop app UI (React + TypeScript)
```

## Acknowledgements

- [`hidpp`](https://github.com/lus/logy) by Lukas Schulte Pelkum — the HID++
  implementation PairDock builds on (0BSD).
- [Solaar](https://github.com/pwr-Solaar/Solaar) — the reference for Bolt pairing and
  the `0x1b04` feature.

## License

MIT, see [LICENSE](LICENSE). The vendored `crates/hidpp` is 0BSD, see
[its notice](crates/hidpp/NOTICE.md).

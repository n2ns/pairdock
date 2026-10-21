# Contributing to PairDock

Thanks for helping! The most useful contributions right now are reports from devices we have not
tested, protocol traces, Unifying receiver support and macOS/Linux ports.

## Reporting a problem

Please [open an issue](https://github.com/n2ns/pairdock/issues/new/choose) with:

- the receiver (Logi Bolt or Unifying) and the device model, e.g. *Logi M650*;
- your Windows version and the PairDock version (Settings, bottom of the page);
- what you did, what you expected and what happened;
- for pairing or button problems, a protocol trace (below).

### Recording a protocol trace

`pairdock-cli` is the command-line build of PairDock's core. Build it from source (below), quit
the PairDock app so the two don't talk to the receiver at the same time, and run, for example:

```text
pairdock-cli --trace trace.log list
pairdock-cli --trace trace.log info 2
pairdock-cli --trace trace.log pair
pairdock-cli --trace trace.log run --config examples/m650.json
```

`trace.log` lists every HID++ report sent (`->`) and received (`<-`). It contains device
addresses, serial numbers and pairing codes; see [PRIVACY.md](PRIVACY.md).

All commands:

```text
pairdock-cli list                    # receivers and paired devices, with battery
pairdock-cli pair                    # pair the first device found in pairing mode
pairdock-cli unpair <slot>
pairdock-cli info <slot>             # HID++ features and reprogrammable controls
pairdock-cli run --config <file>     # apply button settings until Ctrl+C
pairdock-cli reset                   # return every button to its default
```

## Building from source

Requirements:

- Rust (stable, edition 2024)
- Node.js 20 or newer
- The [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform (on
  Windows: the Microsoft C++ Build Tools and WebView2)

```bash
git clone git@github.com:n2ns/pairdock.git
cd pairdock
npm install
npm run tauri dev                      # run the app with hot reload
npm run tauri build                    # build the installers in target/release/bundle
cargo test --workspace                 # unit tests
cargo run -p pairdock-cli -- list      # the command-line tool
```

`npm run dev` serves the UI alone in a browser against a simulated backend ("demo mode"). It is
the quickest way to work on the interface without a receiver.

## Project layout

```text
crates/hidpp           HID++ protocol: messages, receivers, device features
                       (vendored from lus/logy, 0BSD, with additions)
crates/pairdock-core   HID transport (hidapi), receiver discovery, pairing,
                       device inventory and the button remapping engine
crates/pairdock-cli    command-line tool
src-tauri              desktop app backend (Tauri 2): commands, tray, settings
src                    desktop app UI (React + TypeScript)
docs                   user guide and screenshots
```

## How it works

- Logitech receivers expose HID++ through vendor-defined HID collections (usage page `0xFF00`,
  usage `0x0001` for short and `0x0002` for long reports). On Windows these are two separate
  HID handles; `pairdock-core::transport` pairs them into one channel.
- **Bolt pairing** follows Solaar: start discovery (register `0xC0`), pair a discovered device by
  its address (register `0xC1`), show the passkey from notification `0x4D`, count presses from
  `0x4E`, and treat only a `0x54` status with state `0x02`, no error and a slot as success. Mouse
  passkeys are ten bits, most significant first, `1` = right click.
- **Button customization** uses the `0x1B04` *ReprogControlsV4* feature: `setCidReporting`
  either remaps a control on the device or diverts it, after which the device reports presses as
  HID++ events that the engine turns into key presses (`enigo`). Diversion is not persistent, so
  the engine re-applies the configuration whenever a device reconnects.
- Windows hands every input report to every open handle. The engine therefore uses its own
  HID++ software ID, so it never takes responses meant for the UI.

[Solaar](https://github.com/pwr-Solaar/Solaar) is the best reference for receiver registers and
device features.

## Pull requests

- Keep changes focused; one topic per pull request.
- Run `cargo test --workspace` and `npm run build` before pushing; CI runs both.
- For protocol changes, describe the device you tested with and, if you can, attach a trace.
- Code comments are in English. UI text goes into both languages in `src/i18n.ts`.

By contributing you agree that your contributions are licensed under the [MIT license](LICENSE).

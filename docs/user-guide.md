# PairDock user guide

[简体中文](user-guide.zh-CN.md)

- [First start](#first-start)
- [The device list](#the-device-list)
- [Pairing a device](#pairing-a-device)
- [Unpairing a device](#unpairing-a-device)
- [Customizing buttons](#customizing-buttons)
- [Settings](#settings)
- [The tray and quitting](#the-tray-and-quitting)
- [Troubleshooting](#troubleshooting)
- [Uninstalling](#uninstalling)

## First start

1. Plug in your Logi Bolt receiver.
2. If Logi Options+ is installed, quit it from its tray icon or uninstall it. Otherwise both
   programs handle the same buttons and shortcuts may fire twice or not at all.
3. Start PairDock from the Start menu. It opens on the device list.

PairDock keeps running in the system tray (the blue mouse icon near the clock) after you close
its window. That is how button shortcuts keep working.

## The device list

<img src="images/devices-light.png" width="720" alt="The device list">

Each receiver is listed with how many of its six slots are used and its unique ID. Below it,
every paired device shows:

- **Connected**, with its battery level, when the device is awake and in range.
- **Not connected** when it is asleep, switched off, out of range — or when the slot is an old
  pairing the device no longer uses (see [Old pairings](#old-pairings)).

Wake a device by moving the mouse or pressing a key, then press **Refresh**.

## Pairing a device

You can pair up to six devices to one Logi Bolt receiver. Open **Pair a device** in the sidebar,
or click **Pair a new device** in the device list.

### 1. Get ready

<img src="images/pair-prepare.png" width="720" alt="Getting the device ready">

Put the device into pairing mode:

- **Mouse**: turn it over and hold the connect button for about 3 seconds, until its light
  blinks fast.
- **Keyboard**: hold an Easy-Switch key (usually F1–F3 or a key with a computer symbol) for
  about 3 seconds, until its light blinks fast.

If you have more than one receiver, pick the one to pair to. Then click **Start searching**.

### 2. Search

Devices in pairing mode appear within a few seconds. Click **Pair** next to yours. PairDock
keeps searching until you pick a device or leave the page.

### 3. Confirm

Bolt devices confirm the pairing with a short code, so that nobody nearby can connect a device
to your receiver.

<img src="images/pair-confirm.png" width="720" alt="Confirming a mouse with clicks">

- **Mouse**: click the highlighted button — left or right — following the sequence of ten
  steps. The picture and the row of small icons show the next click; the progress bar counts
  your clicks. After the tenth click, **press left and right together** and hold for half a
  second.
- **Keyboard**: type the digits shown on screen, then press **Enter**. The next digit is
  highlighted.

An extra click, a wrong key or waiting too long makes the receiver stop. Put the device back
into pairing mode and click **Try again**.

### 4. Done

The device is paired. For a mouse, **Customize buttons** takes you straight to its buttons.

#### Old pairings

When you pair a device that was already paired to the same receiver, the receiver keeps the old
slot as well; the device only uses the new one. PairDock recognizes this by the device's serial
number and offers **Remove old pairing** on the success screen. Removing it frees the slot.

You can also remove such a slot later from the device list: it is the copy of the device that
always shows **Not connected**.

## Unpairing a device

Click **Unpair** on the device and confirm. The device stops working with this receiver until
you pair it again.

## Customizing buttons

Click **Buttons** on a connected mouse.

<img src="images/buttons.png" width="720" alt="Button settings">

Pick a button on the top view, on the side view (for the back and forward buttons) or in the
list. The selected button lights up, and a small dot marks buttons you have changed. Then choose
what it does:

| Action | What it does |
|---|---|
| **Default** | The button's original function. |
| **Another button** | The button performs another button's action, for example *Back → Middle button*. The mouse stores this itself, so it keeps working while PairDock is closed, until the mouse reconnects. Only buttons the mouse allows are offered. |
| **Shortcut** | The button sends a key combination while you hold it. Pick one of the common shortcuts, or click **Record** and press the combination. Needs PairDock running in the tray. |
| **Disabled** | The button does nothing. Needs PairDock running in the tray. |

Changes apply immediately and are saved. Settings belong to the mouse model, so another mouse of
the same model gets the same buttons.

Windows often acts on combinations with the Windows key (such as `Win+D`) before PairDock can
record them; pick those from the common shortcuts instead.

Which buttons can be customized depends on the mouse. On a Logi M650 these are the middle
button, the back and forward buttons and the gesture button. Left and right clicks cannot be
changed.

## Settings

- **Launch at startup**: PairDock starts in the tray when you sign in, so shortcuts work
  right away.
- **Appearance**: follow the Windows theme, or always use light or dark.
- **Language**: follow Windows, or always use 中文 or English.

## The tray and quitting

- Clicking the tray icon opens the window. Closing the window keeps PairDock in the tray.
- **Quit** in the tray menu exits PairDock. Before it exits, it returns every button to its
  default, so the mouse works normally without PairDock. A sleeping mouse only receives this
  once it wakes, so PairDock waits up to three seconds.
- When Windows shuts down, PairDock exits at once; the mouse starts fresh the next time the
  receiver powers up.

## Troubleshooting

**"No Logitech receiver found"** — Plug the receiver into a USB port on the computer and
click **Refresh**. Unifying receivers are detected but cannot be used yet.

**"The device is asleep"** — Move the mouse or press a key on the keyboard, then click
**Try again**.

**The device does not show up while searching** — Make sure its light blinks fast; pairing mode
ends after a while. Hold the connect button again. Bolt devices only appear for Bolt receivers.

**Pairing fails at the last step** — Press both mouse buttons at exactly the same moment. If
one is pressed slightly earlier, it counts as an extra click.

**Shortcuts stopped working** — PairDock must be running: look for its tray icon, or start it
again. If Logi Options+ is running too, quit it.

**A button does nothing after PairDock crashed** — PairDock could not return the button to its
default. Start PairDock and quit it from the tray, or switch the mouse off and on.

**Windows SmartScreen blocks the installer** — The installer is not code-signed yet. Click
**More info → Run anyway**.

If none of this helps, please [open an issue](https://github.com/n2ns/pairdock/issues/new/choose)
with your device model. [CONTRIBUTING.md](../CONTRIBUTING.md) explains how to record a protocol
trace that lets us see what the receiver answered.

## Uninstalling

Uninstall PairDock from **Settings → Apps → Installed apps** in Windows. Your devices stay paired
to the receiver.

PairDock keeps its files in two folders, which you can delete to remove every trace:

- `%APPDATA%\dev.pairdock.desktop` — button settings (`config.json`)
- `%LOCALAPPDATA%\dev.pairdock.desktop` — window data (language and appearance)

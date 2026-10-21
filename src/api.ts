// Typed access to the Rust backend. Outside of Tauri (plain `npm run dev` in a
// browser) a simulated backend is used so the UI can be developed and
// previewed without hardware.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type DeviceKind =
  | "Keyboard"
  | "Mouse"
  | "Numpad"
  | "Presenter"
  | "Trackball"
  | "Touchpad"
  | string;

export interface PairedDevice {
  slot: number;
  name: string;
  kind: DeviceKind;
  wpid: number;
  /** Serial number; with `wpid` it identifies one physical device. */
  unitId: string;
  online: boolean;
}

export interface ReceiverSummary {
  id: string;
  name: string;
  uniqueId: string;
  supported: boolean;
  devices: PairedDevice[];
}

export interface Battery {
  percent: number | null;
  level: "critical" | "low" | "good" | "full";
  charging: boolean;
}

export interface Control {
  cid: number;
  name: string | null;
  divertable: boolean;
  remappable: boolean;
  remapTargets: number[];
}

export interface DeviceDetails {
  battery: Battery | null;
  controls: Control[];
}

export type ButtonAction =
  | { type: "default" }
  | { type: "remap"; to: number }
  | { type: "shortcut"; keys: string }
  | { type: "disabled" };

export interface Config {
  devices: Record<string, { buttons: Record<string, ButtonAction> }>;
}

export type Address = number[];

export interface Discovered {
  address: Address;
  kind: DeviceKind;
  wpid: number;
  authentication: number;
  name: string;
}

export type PasskeyPrompt =
  | { kind: "type"; value: string }
  | { kind: "click"; value: ("left" | "right")[] };

export type PairingEvent =
  | { type: "discoveryStarted" }
  | ({ type: "discovered" } & Discovered)
  | { type: "discoveryStopped"; error: number }
  | { type: "passkey"; address: Address; prompt: PasskeyPrompt }
  | { type: "passkeyProgress"; address: Address; press: "started" | "key" | "submitted" }
  | { type: "paired"; slot: number }
  | { type: "failed"; reason: string };

interface Backend {
  listReceivers(): Promise<ReceiverSummary[]>;
  deviceDetails(receiverId: string, slot: number): Promise<DeviceDetails>;
  deviceBattery(receiverId: string, slot: number): Promise<Battery | null>;
  unpair(receiverId: string, slot: number): Promise<void>;
  startPairing(receiverId: string): Promise<void>;
  pairDevice(address: Address): Promise<void>;
  stopPairing(): Promise<void>;
  rediscover(): Promise<void>;
  getConfig(): Promise<Config>;
  setButton(wpid: number, cid: number, action: ButtonAction): Promise<Config>;
  onPairing(handler: (event: PairingEvent) => void): Promise<() => void>;
}

export const isTauri = "__TAURI_INTERNALS__" in window;

const tauriBackend: Backend = {
  listReceivers: () => invoke("list_receivers"),
  deviceDetails: (receiverId, slot) => invoke("device_details", { receiverId, slot }),
  deviceBattery: (receiverId, slot) => invoke("device_battery", { receiverId, slot }),
  unpair: (receiverId, slot) => invoke("unpair", { receiverId, slot }),
  startPairing: (receiverId) => invoke("start_pairing", { receiverId }),
  pairDevice: (address) => invoke("pair_device", { address }),
  stopPairing: () => invoke("stop_pairing"),
  rediscover: () => invoke("rediscover"),
  getConfig: () => invoke("get_config"),
  setButton: (wpid, cid, action) => invoke("set_button", { wpid, cid, action }),
  onPairing: (handler) => listen<PairingEvent>("pairing", (e) => handler(e.payload)),
};

// ---------------------------------------------------------------------------
// Simulated backend for browser previews.

const demoReceivers: ReceiverSummary[] = [
  {
    id: "demo-bolt",
    name: "Logi Bolt Receiver",
    uniqueId: "BFBE733454918F8B",
    supported: true,
    devices: [
      { slot: 1, name: "Alto Keys K98M", kind: "Keyboard", wpid: 0xb38e, unitId: "98596211", online: false },
      { slot: 2, name: "Logi M650", kind: "Mouse", wpid: 0xb02a, unitId: "CA438174", online: true },
    ],
  },
];

const demoControls: Control[] = [
  { cid: 0x52, name: "Middle Button", divertable: true, remappable: true, remapTargets: [0x50, 0x51, 0x52, 0x53, 0x56] },
  { cid: 0x53, name: "Back Button", divertable: true, remappable: true, remapTargets: [0x50, 0x51, 0x53, 0x56] },
  { cid: 0x56, name: "Forward Button", divertable: true, remappable: true, remapTargets: [0x50, 0x51, 0x53, 0x56] },
  { cid: 0xd7, name: "Virtual Gesture Button", divertable: true, remappable: false, remapTargets: [0xd7] },
];

let demoConfig: Config = { devices: {} };
const demoListeners = new Set<(event: PairingEvent) => void>();
let demoTimers: number[] = [];

function demoEmit(event: PairingEvent, delay: number) {
  demoTimers.push(window.setTimeout(() => demoListeners.forEach((l) => l(event)), delay));
}

function demoClear() {
  demoTimers.forEach(clearTimeout);
  demoTimers = [];
}

const demoAddress = [0x02, 0x6c, 0x12, 0x9a, 0xe1, 0xd4];
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

const demoBackend: Backend = {
  async listReceivers() {
    await wait(400);
    return structuredClone(demoReceivers);
  },
  async deviceDetails(_receiverId, slot) {
    await wait(500);
    const device = demoReceivers[0].devices.find((d) => d.slot === slot);
    if (device?.kind !== "Mouse") return { battery: null, controls: [] };
    return { battery: { percent: 80, level: "good", charging: false }, controls: demoControls };
  },
  async deviceBattery(_receiverId, slot) {
    await wait(700);
    return slot === 2 || slot === 3 ? { percent: 80, level: "good", charging: false } : null;
  },
  async unpair(_receiverId, slot) {
    await wait(300);
    demoReceivers[0].devices = demoReceivers[0].devices.filter((d) => d.slot !== slot);
  },
  async startPairing() {
    demoClear();
    demoEmit({ type: "discoveryStarted" }, 200);
    demoEmit(
      { type: "discovered", address: demoAddress, kind: "Mouse", wpid: 0xb02a, authentication: 2, name: "Logi M650" },
      3200,
    );
  },
  async pairDevice() {
    demoClear();
    const clicks: ("left" | "right")[] = ["left", "right", "right", "left", "right", "left", "left", "right", "right", "left"];
    demoEmit({ type: "discoveryStopped", error: 0 }, 100);
    demoEmit({ type: "passkey", address: demoAddress, prompt: { kind: "click", value: clicks } }, 900);
    demoEmit({ type: "passkeyProgress", address: demoAddress, press: "started" }, 950);
    clicks.forEach((_, i) =>
      demoEmit({ type: "passkeyProgress", address: demoAddress, press: "key" }, 2400 + i * 1100),
    );
    const end = 2400 + clicks.length * 1100 + 1200;
    demoEmit({ type: "passkeyProgress", address: demoAddress, press: "submitted" }, end);
    // Re-pairing the M650 leaves its old slot behind, like the real receiver.
    demoTimers.push(
      window.setTimeout(() => {
        const devices = demoReceivers[0].devices;
        devices.forEach((d) => (d.online = false));
        devices.push({ slot: 3, name: "Logi M650", kind: "Mouse", wpid: 0xb02a, unitId: "CA438174", online: true });
      }, end + 850),
    );
    demoEmit({ type: "paired", slot: 3 }, end + 900);
  },
  async stopPairing() {
    demoClear();
  },
  async rediscover() {},
  async getConfig() {
    return structuredClone(demoConfig);
  },
  async setButton(wpid, cid, action) {
    const device = (demoConfig.devices[wpid] ??= { buttons: {} });
    if (action.type === "default") delete device.buttons[cid];
    else device.buttons[cid] = action;
    if (Object.keys(device.buttons).length === 0) delete demoConfig.devices[wpid];
    return structuredClone(demoConfig);
  },
  async onPairing(handler) {
    demoListeners.add(handler);
    return () => demoListeners.delete(handler);
  },
};

export const api: Backend = isTauri ? tauriBackend : demoBackend;

export function actionFor(config: Config, wpid: number, cid: number): ButtonAction {
  return config.devices[wpid]?.buttons[cid] ?? { type: "default" };
}

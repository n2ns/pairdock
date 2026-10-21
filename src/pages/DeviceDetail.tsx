import { AnimatePresence, motion } from "motion/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  actionFor,
  api,
  type ButtonAction,
  type Config,
  type Control,
  type DeviceDetails,
  type PairedDevice,
} from "../api";
import { BatteryPill } from "../components/BatteryPill";
import { DeviceGlyph } from "../components/DeviceGlyph";
import { MouseArt, type MousePart, partForControl } from "../components/MouseArt";
import { MouseSideArt } from "../components/MouseSideArt";
import { Keys, ShortcutRecorder } from "../components/ShortcutRecorder";
import { type Strings, useStrings } from "../i18n";

interface Props {
  receiverId: string;
  device: PairedDevice;
  onBack: () => void;
}

export function DeviceDetail({ receiverId, device, onBack }: Props) {
  const t = useStrings();
  const [details, setDetails] = useState<DeviceDetails | null>(null);
  const [config, setConfig] = useState<Config | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [savedAt, setSavedAt] = useState(0);
  const editorRef = useRef<HTMLDivElement>(null);

  const load = useCallback(() => {
    setError(null);
    setDetails(null);
    Promise.all([api.deviceDetails(receiverId, device.slot), api.getConfig()])
      .then(([d, c]) => {
        setDetails(d);
        setConfig(c);
        setSelected((s) => s ?? d.controls[0]?.cid ?? null);
      })
      .catch((e) => setError(String(e)));
  }, [receiverId, device.slot]);

  useEffect(load, [load]);

  const controls = details?.controls ?? [];
  const control = controls.find((c) => c.cid === selected) ?? null;

  const partToCid = useMemo(() => {
    const map = new Map<MousePart, number>();
    for (const c of controls) {
      const part = partForControl[c.cid];
      if (part) map.set(part, c.cid);
    }
    return map;
  }, [controls]);

  const selectable = [...partToCid.keys()];
  const selectedPart = selected !== null ? (partForControl[selected] ?? null) : null;
  const marked = config
    ? selectable.filter((p) => actionFor(config, device.wpid, partToCid.get(p)!).type !== "default")
    : [];

  // Picking a button on the picture jumps straight to its settings.
  const selectPart = (part: MousePart) => {
    const cid = partToCid.get(part);
    if (cid === undefined) return;
    setSelected(cid);
    requestAnimationFrame(() => {
      if (editorRef.current) scrollToElement(editorRef.current);
    });
  };

  const apply = async (action: ButtonAction) => {
    if (!control) return;
    try {
      setConfig(await api.setButton(device.wpid, control.cid, action));
      setSavedAt(Date.now());
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="page device">
      <header className="page-header">
        <button className="btn btn-ghost" onClick={onBack}>
          ← {t.back}
        </button>
      </header>

      <div className="device-hero">
        <DeviceGlyph kind={device.kind} size={30} />
        <div>
          <h1>{device.name}</h1>
          <p className="muted">
            <span className={`status-dot ${device.online ? "is-online" : ""}`} />
            {device.online ? t.online : t.offline}
          </p>
        </div>
        {details?.battery && <BatteryPill battery={details.battery} />}
      </div>

      {error && (
        <div className="card notice-card">
          <p>{device.online ? error : t.deviceAsleep}</p>
          <button className="btn" onClick={load}>
            {t.retry}
          </button>
        </div>
      )}

      {!error && !details && (
        <div className="card stage-center">
          <div className="spinner" />
        </div>
      )}

      {details && controls.length === 0 && (
        <div className="card stage-center">
          <p className="muted">{t.noControls}</p>
        </div>
      )}

      {details && controls.length > 0 && config && (
        <div className="buttons-layout">
          <section className="card buttons-stage">
            <MouseArt
              size={210}
              sideLabels={{ back: t.sideBack, forward: t.sideForward }}
              selectable={selectable}
              selected={selectedPart}
              highlight={selectedPart ? [selectedPart] : []}
              marked={marked}
              onSelect={selectPart}
            />
            {(partToCid.has("back") || partToCid.has("forward")) && (
              <div className="side-view">
                <MouseSideArt
                  size={320}
                  labels={{
                    back: t.sideBack,
                    forward: t.sideForward,
                    front: t.sideFront,
                    rear: t.sideRear,
                  }}
                  selectable={selectable}
                  selected={selectedPart}
                  highlight={
                    selectedPart === "back" || selectedPart === "forward" ? [selectedPart] : []
                  }
                  marked={marked}
                  onSelect={selectPart}
                />
              </div>
            )}
            <p className="muted small">{t.buttonsHint}</p>
          </section>

          <section className="card buttons-panel">
            <h2>{t.buttonsTitle}</h2>
            <ul className="control-list">
              {controls.map((c) => (
                <li key={c.cid}>
                  <button
                    className={`control-item ${c.cid === selected ? "is-selected" : ""}`}
                    onClick={() => setSelected(c.cid)}
                  >
                    <span className="control-name">{controlLabel(t, c)}</span>
                    <ActionSummary
                      t={t}
                      action={actionFor(config, device.wpid, c.cid)}
                      controls={controls}
                    />
                  </button>
                </li>
              ))}
            </ul>

            <div ref={editorRef} className="editor-anchor" />
            <AnimatePresence mode="wait">
              {control && (
                <motion.div
                  key={control.cid}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -8 }}
                  transition={{ duration: 0.2 }}
                >
                  <ActionEditor
                    control={control}
                    controls={controls}
                    action={actionFor(config, device.wpid, control.cid)}
                    onApply={apply}
                  />
                </motion.div>
              )}
            </AnimatePresence>
            <AnimatePresence>
              {savedAt > 0 && (
                <motion.p
                  key={savedAt}
                  className="saved-toast"
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.2 }}
                  onAnimationComplete={() => setTimeout(() => setSavedAt(0), 1200)}
                >
                  ✓ {t.saved}
                </motion.p>
              )}
            </AnimatePresence>
          </section>
        </div>
      )}
    </div>
  );
}

/** Scrolls the nearest scrolling ancestor so `el` sits near its top, with an
 * eased animation that does not depend on native smooth scrolling. */
function scrollToElement(el: HTMLElement, offset = 16, duration = 380) {
  let container = el.parentElement;
  while (container && !/(auto|scroll)/.test(getComputedStyle(container).overflowY)) {
    container = container.parentElement;
  }
  if (!container) return;
  const scroller = container;

  const from = scroller.scrollTop;
  const max = scroller.scrollHeight - scroller.clientHeight;
  const target =
    el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + from - offset;
  const to = Math.max(0, Math.min(max, target));
  if (Math.abs(to - from) < 2) return;

  const start = performance.now();
  const step = (now: number) => {
    const p = Math.min(1, (now - start) / duration);
    const eased = 1 - Math.pow(1 - p, 3);
    scroller.scrollTop = from + (to - from) * eased;
    if (p < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}

function controlLabel(t: Strings, control: Control | number) {
  const cid = typeof control === "number" ? control : control.cid;
  const fallback = typeof control === "number" ? null : control.name;
  return t.controlNames[cid] ?? fallback ?? `0x${cid.toString(16)}`;
}

function ActionSummary({
  t,
  action,
  controls,
}: {
  t: Strings;
  action: ButtonAction;
  controls: Control[];
}) {
  switch (action.type) {
    case "default":
      return <span className="muted">{t.actionDefault}</span>;
    case "remap":
      return <span className="pill">→ {controlLabel(t, controls.find((c) => c.cid === action.to) ?? action.to)}</span>;
    case "shortcut":
      return <Keys shortcut={action.keys} />;
    case "disabled":
      return <span className="pill pill-muted">{t.actionDisabled}</span>;
  }
}

const PRESETS: { label: keyof Strings; keys: string }[] = [
  { label: "presetCopy", keys: "Ctrl+C" },
  { label: "presetPaste", keys: "Ctrl+V" },
  { label: "presetUndo", keys: "Ctrl+Z" },
  { label: "presetBrowserBack", keys: "Alt+Left" },
  { label: "presetBrowserForward", keys: "Alt+Right" },
  { label: "presetDesktop", keys: "Win+D" },
  { label: "presetTaskView", keys: "Win+Tab" },
  { label: "presetScreenshot", keys: "Win+Shift+S" },
  { label: "presetVolumeUp", keys: "VolumeUp" },
  { label: "presetVolumeDown", keys: "VolumeDown" },
  { label: "presetMute", keys: "Mute" },
  { label: "presetPlayPause", keys: "PlayPause" },
  { label: "presetNextTrack", keys: "NextTrack" },
];

function ActionEditor({
  control,
  controls,
  action,
  onApply,
}: {
  control: Control;
  controls: Control[];
  action: ButtonAction;
  onApply: (action: ButtonAction) => void;
}) {
  const t = useStrings();
  const [mode, setMode] = useState<ButtonAction["type"]>(action.type);
  useEffect(() => setMode(action.type), [action.type, control.cid]);

  const options: { type: ButtonAction["type"]; label: string; enabled: boolean; tray: boolean }[] = [
    { type: "default", label: t.actionDefault, enabled: true, tray: false },
    { type: "remap", label: t.actionRemap, enabled: control.remappable, tray: true },
    { type: "shortcut", label: t.actionShortcut, enabled: control.divertable, tray: true },
    { type: "disabled", label: t.actionDisabled, enabled: control.divertable, tray: true },
  ];
  const hints = {
    default: t.actionDefaultHint,
    remap: t.actionRemapHint,
    shortcut: t.actionShortcutHint,
    disabled: t.actionDisabledHint,
  };

  const choose = (type: ButtonAction["type"]) => {
    setMode(type);
    if (type === "default" || type === "disabled") onApply({ type });
  };

  return (
    <div className="editor">
      <div className="segmented">
        {options
          .filter((o) => o.enabled)
          .map((o) => (
            <button
              key={o.type}
              className={mode === o.type ? "is-active" : ""}
              onClick={() => choose(o.type)}
            >
              {mode === o.type && (
                <motion.span layoutId="segment" className="segment-bg" transition={{ type: "spring", stiffness: 400, damping: 32 }} />
              )}
              <span className="segment-label">{o.label}</span>
            </button>
          ))}
      </div>
      <p className="muted small">
        {hints[mode]}
        {options.find((o) => o.type === mode)?.tray && <span className="pill pill-muted">{t.needsTray}</span>}
      </p>

      {mode === "remap" && (
        <div className="choice-grid">
          {control.remapTargets
            .filter((cid) => cid !== control.cid)
            .map((cid) => (
              <button
                key={cid}
                className={`choice ${action.type === "remap" && action.to === cid ? "is-active" : ""}`}
                onClick={() => onApply({ type: "remap", to: cid })}
              >
                {controlLabel(t, controls.find((c) => c.cid === cid) ?? cid)}
              </button>
            ))}
        </div>
      )}

      {mode === "shortcut" && (
        <>
          <ShortcutRecorder
            value={action.type === "shortcut" ? action.keys : null}
            onChange={(keys) => onApply({ type: "shortcut", keys })}
          />
          <p className="eyebrow">{t.presets}</p>
          <div className="choice-grid">
            {PRESETS.map((p) => (
              <button
                key={p.keys}
                className={`choice ${action.type === "shortcut" && action.keys === p.keys ? "is-active" : ""}`}
                onClick={() => onApply({ type: "shortcut", keys: p.keys })}
                title={p.keys}
              >
                {t[p.label] as string}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef, useState } from "react";

import {
  api,
  type Discovered,
  type PairedDevice,
  type PairingEvent,
  type PasskeyPrompt,
  type ReceiverSummary,
} from "../api";
import { DeviceGlyph } from "../components/DeviceGlyph";
import { MouseUnderside, PasskeyKeys, ReceiverRadar } from "../components/Illustrations";
import { MouseArt, type MousePart } from "../components/MouseArt";
import { useStrings } from "../i18n";

type Phase = "prepare" | "search" | "confirm" | "success" | "failed";

interface Props {
  /** The receiver to preselect, when started from its card. */
  receiverId?: string;
  onDone: () => void;
  onCustomize: (receiverId: string, device: PairedDevice) => void;
}

export function PairWizard({ receiverId: preselected, onDone, onCustomize }: Props) {
  const t = useStrings();
  const [receivers, setReceivers] = useState<ReceiverSummary[] | null>(null);
  const [receiverId, setReceiverId] = useState<string | null>(null);
  const [phase, setPhase] = useState<Phase>("prepare");
  const [discovered, setDiscovered] = useState<Discovered[]>([]);
  const [device, setDevice] = useState<Discovered | null>(null);
  const [prompt, setPrompt] = useState<PasskeyPrompt | null>(null);
  const [presses, setPresses] = useState(0);
  const [submitted, setSubmitted] = useState(false);
  const [slot, setSlot] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const phaseRef = useRef(phase);
  phaseRef.current = phase;

  useEffect(() => {
    api
      .listReceivers()
      .then((list) => {
        const supported = list.filter((r) => r.supported);
        setReceivers(supported);
        const initial = supported.find((r) => r.id === preselected) ?? supported[0];
        setReceiverId(initial?.id ?? null);
      })
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    const handle = (event: PairingEvent) => {
      switch (event.type) {
        case "discovered":
          setDiscovered((list) =>
            list.some((d) => sameAddress(d.address, event.address)) ? list : [...list, event],
          );
          break;
        case "discoveryStopped":
          // Discovery times out after 30s; keep searching while the user waits.
          if (phaseRef.current === "search") api.rediscover().catch(() => {});
          break;
        case "passkey":
          setPrompt(event.prompt);
          setPresses(0);
          setSubmitted(false);
          break;
        case "passkeyProgress":
          if (event.press === "key") setPresses((n) => n + 1);
          if (event.press === "submitted") setSubmitted(true);
          break;
        case "paired":
          setSlot(event.slot);
          setPhase("success");
          break;
        case "failed":
          setError(event.reason);
          setPhase("failed");
          break;
      }
    };
    const unlisten = api.onPairing(handle);
    return () => {
      unlisten.then((stop) => stop());
      api.stopPairing().catch(() => {});
    };
  }, []);

  const start = async () => {
    if (!receiverId) return;
    setDiscovered([]);
    setDevice(null);
    setPrompt(null);
    setError(null);
    setPhase("search");
    try {
      await api.startPairing(receiverId);
    } catch (e) {
      setError(String(e));
      setPhase("failed");
    }
  };

  const pair = async (target: Discovered) => {
    setDevice(target);
    setPrompt(null);
    setPhase("confirm");
    try {
      await api.pairDevice(target.address);
    } catch (e) {
      setError(String(e));
      setPhase("failed");
    }
  };

  const steps = [t.pairStepPrepare, t.pairStepSearch, t.pairStepConfirm, t.pairStepDone];
  const stepIndex = { prepare: 0, search: 1, confirm: 2, success: 3, failed: 2 }[phase];

  return (
    <div className="page pair">
      <header className="page-header">
        <div>
          <h1>{t.pairTitle}</h1>
          <p className="muted">{t.pairSubtitle}</p>
        </div>
      </header>

      <ol className="stepper">
        {steps.map((label, i) => (
          <li
            key={label}
            className={`${i < stepIndex ? "is-done" : ""} ${i === stepIndex ? "is-current" : ""}`}
          >
            <span className="stepper-dot">{i < stepIndex ? "✓" : i + 1}</span>
            <span>{label}</span>
          </li>
        ))}
      </ol>

      <AnimatePresence mode="wait">
        <motion.section
          key={phase}
          className="pair-stage card"
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -12 }}
          transition={{ duration: 0.28, ease: "easeOut" }}
        >
          {phase === "prepare" && (
            <Prepare
              receivers={receivers}
              receiverId={receiverId}
              error={error}
              onReceiver={setReceiverId}
              onStart={start}
            />
          )}
          {phase === "search" && <Search discovered={discovered} onPair={pair} />}
          {phase === "confirm" && (
            <Confirm prompt={prompt} presses={presses} submitted={submitted} device={device} />
          )}
          {phase === "success" && device && receiverId && slot !== null && (
            <Success
              receiverId={receiverId}
              slot={slot}
              name={device.name}
              onDone={onDone}
              onCustomize={(paired) => onCustomize(receiverId, paired)}
            />
          )}
          {phase === "failed" && <Failed reason={error} onRetry={start} />}
        </motion.section>
      </AnimatePresence>
    </div>
  );
}

function Prepare({
  receivers,
  receiverId,
  error,
  onReceiver,
  onStart,
}: {
  receivers: ReceiverSummary[] | null;
  receiverId: string | null;
  error: string | null;
  onReceiver: (id: string) => void;
  onStart: () => void;
}) {
  const t = useStrings();
  return (
    <div className="stage-split">
      <div className="stage-art">
        <MouseUnderside />
      </div>
      <div className="stage-copy">
        <h2>{t.prepareTitle}</h2>
        <ul className="hint-list">
          <li>
            <DeviceGlyph kind="Mouse" />
            <span>{t.prepareMouse}</span>
          </li>
          <li>
            <DeviceGlyph kind="Keyboard" />
            <span>{t.prepareKeyboard}</span>
          </li>
        </ul>
        {receivers && receivers.length > 1 && (
          <label className="field">
            <span>{t.prepareReceiver}</span>
            <select value={receiverId ?? ""} onChange={(e) => onReceiver(e.target.value)}>
              {receivers.map((r) => (
                <option key={r.id} value={r.id}>
                  {r.name} · {r.uniqueId.slice(-6)}
                </option>
              ))}
            </select>
          </label>
        )}
        {receivers && receivers.length === 0 && <p className="notice">{t.noReceiverForPairing}</p>}
        {!receivers && error && <p className="notice">{error}</p>}
        <button className="btn btn-primary btn-lg" disabled={!receiverId} onClick={onStart}>
          {t.startSearch}
        </button>
      </div>
    </div>
  );
}

function Search({
  discovered,
  onPair,
}: {
  discovered: Discovered[];
  onPair: (device: Discovered) => void;
}) {
  const t = useStrings();
  return (
    <div className="stage-split">
      <div className="stage-art">
        <ReceiverRadar found={discovered.length > 0} />
      </div>
      <div className="stage-copy">
        <h2>{discovered.length ? t.foundTitle : t.searchingTitle}</h2>
        <p className="muted">{t.searchingHint}</p>
        <ul className="found-list">
          <AnimatePresence>
            {discovered.map((device) => (
              <motion.li
                key={device.address.join(":")}
                layout
                initial={{ opacity: 0, scale: 0.92, y: 10 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                transition={{ type: "spring", stiffness: 320, damping: 24 }}
              >
                <DeviceGlyph kind={device.kind} />
                <div className="found-name">
                  <strong>{device.name}</strong>
                  <span className="muted">
                    {device.kind === "Mouse"
                      ? t.kindMouse
                      : device.kind === "Keyboard"
                        ? t.kindKeyboard
                        : t.kindOther}
                  </span>
                </div>
                <button className="btn btn-primary" onClick={() => onPair(device)}>
                  {t.pairThis}
                </button>
              </motion.li>
            ))}
          </AnimatePresence>
        </ul>
      </div>
    </div>
  );
}

function Confirm({
  prompt,
  presses,
  submitted,
  device,
}: {
  prompt: PasskeyPrompt | null;
  presses: number;
  submitted: boolean;
  device: Discovered | null;
}) {
  const t = useStrings();

  if (!prompt) {
    return (
      <div className="stage-center">
        <div className="spinner" />
        <p className="muted">
          {device?.name} · {t.waitingDevice}
        </p>
      </div>
    );
  }

  if (prompt.kind === "type") {
    const total = prompt.value.length;
    return (
      <div className="stage-center">
        <h2>{submitted ? t.waitingDevice : presses >= total ? t.pressEnter : t.confirmTypeTitle}</h2>
        <p className="muted">{t.confirmTypeSubtitle}</p>
        <PasskeyKeys digits={prompt.value} done={Math.min(presses, total)} />
        <p className="progress-text">{t.progress(Math.min(presses, total), total)}</p>
      </div>
    );
  }

  const clicks = prompt.value;
  const total = clicks.length;
  const done = Math.min(presses, total);
  const allClicked = done >= total;
  const highlight: MousePart[] = submitted ? [] : allClicked ? ["left", "right"] : [clicks[done]];
  const instruction = allClicked
    ? t.clickBoth
    : clicks[done] === "left"
      ? t.clickLeft
      : t.clickRight;

  return (
    <div className="stage-split confirm">
      <div className="stage-art">
        <MouseArt highlight={highlight} size={250} />
      </div>
      <div className="stage-copy">
        <p className="eyebrow">{t.confirmClickTitle}</p>
        <AnimatePresence mode="wait">
          <motion.h2
            key={`${done}-${submitted}`}
            className="instruction"
            initial={{ opacity: 0, x: 18 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -18 }}
            transition={{ duration: 0.2 }}
          >
            {submitted ? t.waitingDevice : instruction}
          </motion.h2>
        </AnimatePresence>
        <p className="muted">{allClicked ? t.clickBothHint : t.confirmClickSubtitle}</p>

        <ol className="click-seq">
          {clicks.map((click, i) => (
            <li
              key={i}
              className={`seq-${click} ${i < done ? "is-done" : ""} ${i === done ? "is-next" : ""}`}
            >
              <ClickIcon side={click} />
            </li>
          ))}
          <li className={`seq-both ${allClicked && !submitted ? "is-next" : ""} ${submitted ? "is-done" : ""}`}>
            <ClickIcon side="both" />
          </li>
        </ol>
        <div className="progress-bar">
          <motion.span
            animate={{ width: `${((done + (submitted ? 1 : 0)) / (total + 1)) * 100}%` }}
            transition={{ type: "spring", stiffness: 140, damping: 22 }}
          />
        </div>
        <p className="progress-text">{t.progress(done, total)}</p>
      </div>
    </div>
  );
}

/** A tiny mouse with the left, right or both buttons filled. */
function ClickIcon({ side }: { side: "left" | "right" | "both" }) {
  return (
    <svg viewBox="0 0 20 28" width="18" height="25" aria-hidden>
      <rect x="1.5" y="1.5" width="17" height="25" rx="8.5" className="ci-body" />
      <path d="M10 1.5 A8.5 8.5 0 0 0 1.5 10 V12 H10 Z" className={side !== "right" ? "ci-on" : "ci-off"} />
      <path d="M10 1.5 A8.5 8.5 0 0 1 18.5 10 V12 H10 Z" className={side !== "left" ? "ci-on" : "ci-off"} />
    </svg>
  );
}

function Success({
  receiverId,
  slot,
  name,
  onDone,
  onCustomize,
}: {
  receiverId: string;
  slot: number;
  name: string;
  onDone: () => void;
  onCustomize: (device: PairedDevice) => void;
}) {
  const t = useStrings();
  const [paired, setPaired] = useState<PairedDevice | null>(null);
  // Slots this device left behind in earlier pairings: same model and serial.
  const [stale, setStale] = useState<PairedDevice[]>([]);
  const [cleanup, setCleanup] = useState<"idle" | "busy" | "done">("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .listReceivers()
      .then((receivers) => {
        const devices = receivers.find((r) => r.id === receiverId)?.devices ?? [];
        const self = devices.find((d) => d.slot === slot) ?? null;
        setPaired(self);
        if (self?.unitId) {
          setStale(
            devices.filter(
              (d) => d.slot !== slot && d.wpid === self.wpid && d.unitId === self.unitId,
            ),
          );
        }
      })
      .catch(() => {});
  }, [receiverId, slot]);

  const removeStale = async () => {
    setCleanup("busy");
    setError(null);
    try {
      for (const d of stale) await api.unpair(receiverId, d.slot);
      setCleanup("done");
    } catch (e) {
      setError(String(e));
      setCleanup("idle");
    }
  };

  return (
    <div className="stage-center success">
      <div className="burst">
        {Array.from({ length: 12 }, (_, i) => (
          <span key={i} style={{ "--i": i } as React.CSSProperties} />
        ))}
        <svg viewBox="0 0 88 88" width="96" height="96" className="check">
          <circle cx="44" cy="44" r="40" />
          <path d="M26 45 L39 58 L63 32" />
        </svg>
      </div>
      <h2>{t.successTitle}</h2>
      <p className="muted">{t.successText(name)}</p>

      {stale.length > 0 && (
        <motion.div
          className="stale-card"
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
        >
          {cleanup === "done" ? (
            <p className="stale-done">✓ {t.staleRemoved}</p>
          ) : (
            <>
              <div>
                <strong>{t.staleTitle(stale.length)}</strong>
                <p className="muted small">
                  {t.staleSlots(stale.map((d) => `#${d.slot}`).join(", "))} · {t.staleText}
                </p>
                {error && <p className="error small">{error}</p>}
              </div>
              <button className="btn" disabled={cleanup === "busy"} onClick={removeStale}>
                {t.staleRemove}
              </button>
            </>
          )}
        </motion.div>
      )}

      <div className="actions">
        {paired?.kind === "Mouse" && (
          <button className="btn btn-primary btn-lg" onClick={() => onCustomize(paired)}>
            {t.successButtons}
          </button>
        )}
        <button className="btn btn-lg" onClick={onDone}>
          {t.done}
        </button>
      </div>
    </div>
  );
}

function Failed({ reason, onRetry }: { reason: string | null; onRetry: () => void }) {
  const t = useStrings();
  return (
    <div className="stage-center failed">
      <div className="failed-icon">!</div>
      <h2>{t.failedTitle}</h2>
      <p className="muted">{t.failedHint}</p>
      {reason && <code className="reason">{reason}</code>}
      <button className="btn btn-primary btn-lg" onClick={onRetry}>
        {t.retry}
      </button>
    </div>
  );
}

function sameAddress(a: number[], b: number[]) {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

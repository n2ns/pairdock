import { AnimatePresence, motion } from "motion/react";
import { useCallback, useEffect, useState } from "react";

import { api, type PairedDevice, type ReceiverSummary } from "../api";
import { DeviceGlyph } from "../components/DeviceGlyph";
import { ReceiverRadar } from "../components/Illustrations";
import { useStrings } from "../i18n";

interface Props {
  onCustomize: (receiverId: string, device: PairedDevice) => void;
  onPair: () => void;
}

export function Devices({ onCustomize, onPair }: Props) {
  const t = useStrings();
  const [receivers, setReceivers] = useState<ReceiverSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<{ receiverId: string; device: PairedDevice } | null>(null);

  const load = useCallback(() => {
    setError(null);
    setReceivers(null);
    api
      .listReceivers()
      .then(setReceivers)
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(load, [load]);

  const unpair = async () => {
    if (!confirm) return;
    try {
      await api.unpair(confirm.receiverId, confirm.device.slot);
    } catch (e) {
      setError(String(e));
    }
    setConfirm(null);
    load();
  };

  return (
    <div className="page devices">
      <header className="page-header">
        <div>
          <h1>{t.devicesTitle}</h1>
          <p className="muted">{t.devicesSubtitle}</p>
        </div>
        <button className="btn" onClick={load}>
          ↻ {t.refresh}
        </button>
      </header>

      {error && <div className="card notice-card">{error}</div>}

      {!receivers && !error && (
        <div className="card stage-center">
          <div className="spinner" />
          <p className="muted">{t.loading}</p>
        </div>
      )}

      {receivers?.length === 0 && (
        <div className="card stage-center empty">
          <ReceiverRadar found={false} />
          <h2>{t.noReceiver}</h2>
          <p className="muted">{t.noReceiverHint}</p>
        </div>
      )}

      {receivers?.map((receiver) => (
        <section key={receiver.id} className="receiver">
          <div className="receiver-head">
            <span className="receiver-badge">
              <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden>
                <rect x="7" y="2" width="10" height="7" rx="1.5" />
                <rect x="4.5" y="8" width="15" height="14" rx="5" />
              </svg>
            </span>
            <div>
              <h2>{receiver.name}</h2>
              <p className="muted small">
                {receiver.supported ? t.slotsUsed(receiver.devices.length) : t.unsupportedReceiver}
                {receiver.uniqueId && <span className="mono"> · {receiver.uniqueId}</span>}
              </p>
            </div>
          </div>

          <div className="device-grid">
            <AnimatePresence>
              {receiver.devices.map((device, i) => (
                <motion.article
                  key={`${device.slot}-${device.wpid}`}
                  className="card device-card"
                  layout
                  initial={{ opacity: 0, y: 14 }}
                  animate={{ opacity: 1, y: 0, transition: { delay: i * 0.05 } }}
                  exit={{ opacity: 0, scale: 0.95 }}
                >
                  <div className="device-card-top">
                    <DeviceGlyph kind={device.kind} />
                    <span className="slot">#{device.slot}</span>
                  </div>
                  <h3>{device.name || "—"}</h3>
                  <p className="muted small">
                    <span className={`status-dot ${device.online ? "is-online" : ""}`} />
                    {device.kind === "Mouse"
                      ? t.kindMouse
                      : device.kind === "Keyboard"
                        ? t.kindKeyboard
                        : t.kindOther}{" "}
                    · {device.online ? t.online : t.offline}
                  </p>
                  <div className="device-card-actions">
                    {device.kind === "Mouse" && (
                      <button className="btn btn-primary" onClick={() => onCustomize(receiver.id, device)}>
                        {t.customize}
                      </button>
                    )}
                    <button
                      className="btn btn-ghost danger"
                      onClick={() => setConfirm({ receiverId: receiver.id, device })}
                    >
                      {t.unpair}
                    </button>
                  </div>
                </motion.article>
              ))}
            </AnimatePresence>

            {receiver.supported && receiver.devices.length < 6 && (
              <button className="card device-card add-card" onClick={onPair}>
                <span className="add-plus">+</span>
                <span>{t.pairNew}</span>
              </button>
            )}
          </div>
        </section>
      ))}

      <AnimatePresence>
        {confirm && (
          <motion.div
            className="modal-backdrop"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setConfirm(null)}
          >
            <motion.div
              className="card modal"
              initial={{ scale: 0.94, y: 12 }}
              animate={{ scale: 1, y: 0 }}
              exit={{ scale: 0.96, opacity: 0 }}
              onClick={(e) => e.stopPropagation()}
            >
              <h2>{t.unpair}</h2>
              <p className="muted">{t.unpairConfirm(confirm.device.name)}</p>
              <div className="actions">
                <button className="btn" onClick={() => setConfirm(null)}>
                  {t.cancel}
                </button>
                <button className="btn btn-danger" onClick={unpair}>
                  {t.unpair}
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

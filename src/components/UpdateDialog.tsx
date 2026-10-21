import { motion } from "motion/react";
import { useState } from "react";

import { version } from "../../package.json";
import { api, type UpdateInfo } from "../api";
import { useStrings } from "../i18n";

/** Offers a newer release; nothing is downloaded until the user confirms. */
export function UpdateDialog({ update, onClose }: { update: UpdateInfo; onClose: () => void }) {
  const t = useStrings();
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const install = async () => {
    setInstalling(true);
    setError(null);
    try {
      // Does not return when the installer starts: PairDock exits.
      await api.installUpdate();
    } catch (e) {
      setError(String(e));
      setInstalling(false);
    }
  };

  return (
    <motion.div
      className="modal-backdrop"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={() => !installing && onClose()}
    >
      <motion.div
        className="card modal"
        initial={{ scale: 0.94, y: 12 }}
        animate={{ scale: 1, y: 0 }}
        exit={{ scale: 0.96, opacity: 0 }}
        onClick={(e) => e.stopPropagation()}
      >
        <h2>{t.updateAvailable(update.version)}</h2>
        <p className="muted">{t.updateText(version)}</p>
        {error && (
          <p className="notice">
            {t.updateInstallFailed} {error}
          </p>
        )}
        <div className="actions">
          <button className="btn" disabled={installing} onClick={onClose}>
            {t.updateLater}
          </button>
          <button className="btn btn-primary" disabled={installing} onClick={install}>
            {installing ? t.updateDownloading : t.updateInstall}
          </button>
        </div>
      </motion.div>
    </motion.div>
  );
}

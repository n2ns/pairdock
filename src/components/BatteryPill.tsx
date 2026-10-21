import { motion } from "motion/react";

import type { Battery } from "../api";
import { useStrings } from "../i18n";

/** A battery gauge with the charge in percent (or the level when the device
 * reports no percentage). `compact` drops the outline for use inside cards. */
export function BatteryPill({ battery, compact = false }: { battery: Battery; compact?: boolean }) {
  const t = useStrings();
  const percent = battery.percent ?? { critical: 5, low: 20, good: 60, full: 100 }[battery.level];
  return (
    <div
      className={`battery battery-${battery.level} ${compact ? "battery-compact" : ""}`}
      title={t.battery}
    >
      <span className="battery-shell">
        <motion.span
          className="battery-fill"
          initial={{ width: 0 }}
          animate={{ width: `${percent}%` }}
          transition={{ duration: 0.8, ease: "easeOut" }}
        />
      </span>
      <span>{battery.percent !== null ? `${battery.percent}%` : t.battery}</span>
      {battery.charging && <span className="muted">· {t.charging}</span>}
    </div>
  );
}

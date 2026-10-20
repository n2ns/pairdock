// Small line icons for device kinds.

import type { DeviceKind } from "../api";

export function DeviceGlyph({ kind, size = 28 }: { kind: DeviceKind; size?: number }) {
  return (
    <span className="glyph" style={{ width: size + 16, height: size + 16 }}>
      {kind === "Keyboard" ? (
        <svg viewBox="0 0 32 32" width={size} height={size} aria-hidden>
          <rect x="3" y="8" width="26" height="16" rx="3.5" />
          <path d="M8 13h2M13 13h2M18 13h2M23 13h1M8 17h2M13 17h6M22 17h2M10 20.5h12" />
        </svg>
      ) : (
        <svg viewBox="0 0 32 32" width={size} height={size} aria-hidden>
          <rect x="9" y="3.5" width="14" height="25" rx="7" />
          <path d="M16 3.5V13M9 13h14" />
          <rect x="15" y="7" width="2" height="4" rx="1" className="glyph-fill" />
        </svg>
      )}
    </span>
  );
}

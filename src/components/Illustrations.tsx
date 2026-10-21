// Animated illustrations for the pairing wizard.

import { useId } from "react";

/** The underside of a mouse: the connect button is held for 3 seconds, then
 * the light blinks fast. Loops. */
export function MouseUnderside({ size = 220 }: { size?: number }) {
  const uid = useId().replace(/:/g, "");
  return (
    <svg className="underside" viewBox="0 0 220 340" width={size} height={(size * 340) / 220}>
      <defs>
        <linearGradient id={`u-body-${uid}`} x1="0" y1="0" x2="0.3" y2="1">
          <stop offset="0" stopColor="var(--mouse-body-2)" />
          <stop offset="1" stopColor="var(--mouse-body-3)" />
        </linearGradient>
      </defs>
      <ellipse cx="110" cy="326" rx="78" ry="9" className="mouse-shadow" />
      <path
        d="M110 14 C 62 14 30 56 26 126 L 24 214 C 22 282 60 320 110 320 C 160 320 198 282 196 214 L 194 126 C 190 56 158 14 110 14 Z"
        fill={`url(#u-body-${uid})`}
        className="mouse-edge"
      />
      {/* Glide pads */}
      <rect x="70" y="34" width="80" height="10" rx="5" className="underside-pad" />
      <rect x="70" y="286" width="80" height="10" rx="5" className="underside-pad" />
      {/* Sensor */}
      <rect x="96" y="170" width="28" height="40" rx="12" className="underside-sensor" />
      <circle cx="110" cy="190" r="6" className="underside-lens" />
      {/* Power switch */}
      <rect x="92" y="236" width="36" height="16" rx="8" className="underside-switch" />
      <circle cx="120" cy="244" r="6" className="underside-knob" />

      {/* Connect button with hold-progress ring and blinking light */}
      <circle cx="110" cy="112" r="30" className="underside-ring-track" />
      <circle
        cx="110"
        cy="112"
        r="30"
        className="underside-ring"
        stroke="var(--accent)"
        transform="rotate(-90 110 112)"
      />
      <circle cx="110" cy="112" r="18" className="underside-button" />
      <circle cx="110" cy="112" r="5" className="underside-led" />
      <g className="underside-finger">
        <ellipse cx="110" cy="112" rx="16" ry="19" />
      </g>
      <text x="110" y="160" className="underside-timer" textAnchor="middle">
        3s
      </text>
    </svg>
  );
}

/** A row of keycaps for typing a passkey; `done` keys are dimmed and the
 * next one is highlighted. After all digits, Enter is highlighted. */
export function PasskeyKeys({ digits, done }: { digits: string; done: number }) {
  const keys = [...digits];
  return (
    <div className="keycaps">
      {keys.map((digit, i) => (
        <span
          key={i}
          className={`keycap ${i < done ? "is-done" : ""} ${i === done ? "is-next" : ""}`}
        >
          {digit}
        </span>
      ))}
      <span className={`keycap keycap-enter ${done >= keys.length ? "is-next" : ""}`}>⏎</span>
    </div>
  );
}

/** A USB receiver sending out signal waves. */
export function ReceiverRadar({ found }: { found: boolean }) {
  return (
    <div className={`radar ${found ? "is-found" : ""}`}>
      <span className="radar-wave" />
      <span className="radar-wave radar-wave-2" />
      <span className="radar-wave radar-wave-3" />
      <svg className="radar-dongle" viewBox="0 0 64 96" width="56" height="84">
        <rect x="18" y="4" width="28" height="30" rx="4" className="dongle-plug" />
        <rect x="24" y="12" width="6" height="6" rx="1" className="dongle-hole" />
        <rect x="34" y="12" width="6" height="6" rx="1" className="dongle-hole" />
        <rect x="10" y="30" width="44" height="60" rx="14" className="dongle-body" />
        <circle cx="32" cy="62" r="7" className="dongle-dot" />
      </svg>
    </div>
  );
}

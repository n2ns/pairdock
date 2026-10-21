// A top view of a mouse whose buttons can be highlighted ("click here") and
// selected. Used by the pairing wizard and the button settings.

import { useId } from "react";

export type MousePart = "left" | "right" | "middle" | "back" | "forward";

/** Maps HID++ control IDs to the drawn parts. */
export const partForControl: Record<number, MousePart> = {
  0x50: "left",
  0x51: "right",
  0x52: "middle",
  0x53: "back",
  0x56: "forward",
};

interface Props {
  /** Parts to highlight with the animated "press here" effect. */
  highlight?: MousePart[];
  /** Parts that can be clicked; they get a hover state. */
  selectable?: MousePart[];
  selected?: MousePart | null;
  onSelect?: (part: MousePart) => void;
  /** Parts drawn with a small badge, e.g. customized buttons. */
  marked?: MousePart[];
  /** Names written next to the side buttons; widens the picture to the left. */
  sideLabels?: { back: string; forward: string };
  size?: number;
}

/** Room on the left for the side button names. */
const LABEL_SPACE = 84;

const BODY =
  "M130 22 C 74 22 38 70 33 150 L 30 252 C 28 332 72 382 130 382 C 188 382 232 332 230 252 L 227 150 C 222 70 186 22 130 22 Z";

const PARTS: Record<MousePart, { d: string; cx: number; cy: number }> = {
  left: { d: "M0 0 H128 V168 H0 Z", cx: 82, cy: 100 },
  right: { d: "M132 0 H260 V168 H132 Z", cx: 178, cy: 100 },
  middle: { d: "M116 62 a14 14 0 0 1 28 0 V114 a14 14 0 0 1 -28 0 Z", cx: 130, cy: 88 },
  forward: { d: "M16 186 a9 9 0 0 1 18 0 V218 a9 9 0 0 1 -18 0 Z", cx: 25, cy: 202 },
  back: { d: "M16 232 a9 9 0 0 1 18 0 V264 a9 9 0 0 1 -18 0 Z", cx: 25, cy: 248 },
};

// Buttons on the top surface are clipped to the body outline.
const CLIPPED: MousePart[] = ["left", "right"];

export function MouseArt({
  highlight = [],
  selectable = [],
  selected = null,
  onSelect,
  marked = [],
  sideLabels,
  size = 260,
}: Props) {
  const uid = useId().replace(/:/g, "");
  const id = (name: string) => `${name}-${uid}`;
  const order: MousePart[] = ["left", "right", "forward", "back", "middle"];
  const left = sideLabels ? -LABEL_SPACE : 0;
  const width = 260 - left;

  return (
    <svg
      className="mouse-art"
      viewBox={`${left} 0 ${width} 410`}
      width={(size * width) / 260}
      height={(size * 410) / 260}
      role="img"
      aria-label="mouse"
    >
      <defs>
        <linearGradient id={id("body")} x1="0" y1="0" x2="0.35" y2="1">
          <stop offset="0" stopColor="var(--mouse-body-1)" />
          <stop offset="1" stopColor="var(--mouse-body-2)" />
        </linearGradient>
        <radialGradient id={id("sheen")} cx="0.32" cy="0.12" r="0.7">
          <stop offset="0" stopColor="#fff" stopOpacity="0.55" />
          <stop offset="1" stopColor="#fff" stopOpacity="0" />
        </radialGradient>
        <clipPath id={id("clip")}>
          <path d={BODY} />
        </clipPath>
        <filter id={id("glow")} x="-60%" y="-60%" width="220%" height="220%">
          <feGaussianBlur stdDeviation="7" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <ellipse cx="130" cy="394" rx="92" ry="11" className="mouse-shadow" />

      {/* Side buttons sit behind the body edge. */}
      {(["forward", "back"] as MousePart[]).map((part) => (
        <Part
          key={part}
          part={part}
          id={id}
          highlight={highlight}
          selectable={selectable}
          selected={selected}
          onSelect={onSelect}
        />
      ))}

      <path d={BODY} fill={`url(#${id("body")})`} className="mouse-body" />

      <g clipPath={`url(#${id("clip")})`}>
        {CLIPPED.map((part) => (
          <Part
            key={part}
            part={part}
            id={id}
            highlight={highlight}
            selectable={selectable}
            selected={selected}
            onSelect={onSelect}
          />
        ))}
        <path d={BODY} fill={`url(#${id("sheen")})`} pointerEvents="none" />
        <line x1="130" y1="0" x2="130" y2="168" className="mouse-groove" />
        <path d="M0 168 Q130 182 260 168" className="mouse-groove" fill="none" />
      </g>

      <path d={BODY} className="mouse-edge" fill="none" />

      <Part
        part="middle"
        id={id}
        highlight={highlight}
        selectable={selectable}
        selected={selected}
        onSelect={onSelect}
      />
      {/* Wheel ribs */}
      <g className="mouse-ribs" pointerEvents="none">
        {[72, 80, 88, 96, 104].map((y) => (
          <line key={y} x1="122" x2="138" y1={y} y2={y} />
        ))}
      </g>

      {/* Ripples and badges go on top of everything. */}
      {order
        .filter((part) => highlight.includes(part))
        .map((part) => (
          <g key={`ripple-${part}`} pointerEvents="none">
            <circle cx={PARTS[part].cx} cy={PARTS[part].cy} r="18" className="mouse-ripple" />
            <circle
              cx={PARTS[part].cx}
              cy={PARTS[part].cy}
              r="18"
              className="mouse-ripple mouse-ripple-late"
            />
          </g>
        ))}
      {sideLabels &&
        (["forward", "back"] as const).map((part) => (
          <g key={`label-${part}`} pointerEvents="none">
            <line
              x1="-6"
              x2="13"
              y1={PARTS[part].cy}
              y2={PARTS[part].cy}
              className="side-direction-line"
            />
            <text
              x="-10"
              y={PARTS[part].cy + 4}
              textAnchor="end"
              className={`side-label ${selected === part || highlight.includes(part) ? "is-active" : ""}`}
            >
              {sideLabels[part]}
            </text>
          </g>
        ))}
      {order
        .filter((part) => marked.includes(part))
        .map((part) => (
          <circle
            key={`mark-${part}`}
            cx={PARTS[part].cx + (part === "back" || part === "forward" ? -14 : 22)}
            cy={PARTS[part].cy - 22}
            r="6"
            fill="var(--accent)"
            className="mouse-mark"
          />
        ))}
    </svg>
  );
}

function Part({
  part,
  id,
  highlight,
  selectable,
  selected,
  onSelect,
}: {
  part: MousePart;
  id: (name: string) => string;
  highlight: MousePart[];
  selectable: MousePart[];
  selected: MousePart | null;
  onSelect?: (part: MousePart) => void;
}) {
  const lit = highlight.includes(part);
  const canSelect = selectable.includes(part);
  const classes = [
    "mouse-part",
    `mouse-part-${part}`,
    lit && "is-lit",
    canSelect && "is-selectable",
    selected === part && "is-selected",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <path
      d={PARTS[part].d}
      className={classes}
      // Inline style, so the accent wins over the part colors set in CSS.
      style={lit || selected === part ? { fill: "var(--accent)" } : undefined}
      filter={lit ? `url(#${id("glow")})` : undefined}
      onClick={canSelect ? () => onSelect?.(part) : undefined}
    >
      {canSelect && <title>{part}</title>}
    </path>
  );
}

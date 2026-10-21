// The left side of a mouse, where the thumb rests, facing right: the low,
// tapered front carries the wheel and the button lip, the high rounded back
// is the palm rest. Shows the back and forward buttons large enough to pick
// and highlight, each with its name, and an arrow marking the front.

import { useId } from "react";

import type { MousePart } from "./MouseArt";

type SidePart = Extract<MousePart, "back" | "forward">;

export interface SideLabels {
  back: string;
  forward: string;
  front: string;
  rear: string;
}

interface Props {
  highlight?: MousePart[];
  selectable?: MousePart[];
  selected?: MousePart | null;
  onSelect?: (part: MousePart) => void;
  marked?: MousePart[];
  labels: SideLabels;
  size?: number;
}

const SHELL =
  "M26 150 C 18 108 44 58 106 42 C 152 30 208 38 256 60 C 294 78 324 108 337 138 C 340 145 339 150 333 150 Z";

const GRIP =
  "M38 134 C 62 116 112 108 172 110 C 232 112 292 120 326 140 L 331 150 L 30 150 Z";

const PARTS: Record<SidePart, { d: string; cx: number; cy: number }> = {
  back: { d: "M130 80 h46 a12 12 0 0 1 0 24 h-46 a12 12 0 0 1 0 -24 Z", cx: 153, cy: 92 },
  forward: { d: "M196 84 h46 a12 12 0 0 1 0 24 h-46 a12 12 0 0 1 0 -24 Z", cx: 219, cy: 96 },
};

export function MouseSideArt({
  highlight = [],
  selectable = [],
  selected = null,
  onSelect,
  marked = [],
  labels,
  size = 320,
}: Props) {
  const uid = useId().replace(/:/g, "");
  const id = (name: string) => `${name}-${uid}`;
  const parts: SidePart[] = ["back", "forward"];

  return (
    <svg
      className="mouse-art mouse-side"
      viewBox="0 0 360 200"
      width={size}
      height={(size * 200) / 360}
      role="img"
      aria-label="mouse side"
    >
      <defs>
        <linearGradient id={id("body")} x1="0" y1="0" x2="0.15" y2="1">
          <stop offset="0" stopColor="var(--mouse-body-1)" />
          <stop offset="1" stopColor="var(--mouse-body-2)" />
        </linearGradient>
        <linearGradient id={id("grip")} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="var(--mouse-grip-1)" />
          <stop offset="1" stopColor="var(--mouse-grip-2)" />
        </linearGradient>
        <filter id={id("glow")} x="-60%" y="-120%" width="220%" height="340%">
          <feGaussianBlur stdDeviation="5" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <marker
          id={id("arrow")}
          viewBox="0 0 10 10"
          refX="8"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto"
        >
          <path d="M0 0 L10 5 L0 10 Z" className="side-arrowhead" />
        </marker>
      </defs>

      <ellipse cx="180" cy="157" rx="150" ry="7" className="mouse-shadow" />
      <path d={SHELL} fill={`url(#${id("body")})`} className="mouse-body" />
      <path d={GRIP} fill={`url(#${id("grip")})`} />

      {/* Seam between the top buttons and the body, ending in the front lip */}
      <path
        d="M58 70 C 120 46 214 48 292 92 C 310 104 324 120 332 134"
        className="mouse-groove"
        fill="none"
      />
      <path d={SHELL} className="mouse-edge" fill="none" />
      {/* Wheel at the front */}
      <rect
        x="249"
        y="57"
        width="30"
        height="13"
        rx="6.5"
        transform="rotate(26 264 63)"
        className="side-wheel"
      />

      {parts.map((part) => {
        const lit = highlight.includes(part);
        const canSelect = selectable.includes(part);
        return (
          <path
            key={part}
            d={PARTS[part].d}
            transform={`rotate(6 ${PARTS[part].cx} ${PARTS[part].cy})`}
            className={[
              "mouse-part",
              lit && "is-lit",
              canSelect && "is-selectable",
              selected === part && "is-selected",
            ]
              .filter(Boolean)
              .join(" ")}
            style={lit || selected === part ? { fill: "var(--accent)" } : undefined}
            filter={lit ? `url(#${id("glow")})` : undefined}
            onClick={canSelect ? () => onSelect?.(part) : undefined}
          />
        );
      })}

      {parts
        .filter((part) => highlight.includes(part))
        .map((part) => (
          <g key={`ripple-${part}`} pointerEvents="none">
            <circle cx={PARTS[part].cx} cy={PARTS[part].cy} r="16" className="mouse-ripple" />
            <circle
              cx={PARTS[part].cx}
              cy={PARTS[part].cy}
              r="16"
              className="mouse-ripple mouse-ripple-late"
            />
          </g>
        ))}
      {parts
        .filter((part) => marked.includes(part))
        .map((part) => (
          <circle
            key={`mark-${part}`}
            cx={PARTS[part].cx + 26}
            cy={PARTS[part].cy - 16}
            r="5.5"
            fill="var(--accent)"
            className="mouse-mark"
          />
        ))}

      {/* Button names under the buttons */}
      {parts.map((part) => (
        <text
          key={`label-${part}`}
          x={PARTS[part].cx}
          y={PARTS[part].cy + 32}
          textAnchor="middle"
          className={`side-label ${selected === part || highlight.includes(part) ? "is-active" : ""}`}
        >
          {labels[part]}
        </text>
      ))}

      {/* Which way is the front */}
      <text x="40" y="190" textAnchor="middle" className="side-direction">
        {labels.rear}
      </text>
      <line
        x1="60"
        y1="185"
        x2="296"
        y2="185"
        className="side-direction-line"
        markerEnd={`url(#${id("arrow")})`}
      />
      <text x="320" y="190" textAnchor="middle" className="side-direction">
        {labels.front}
      </text>
    </svg>
  );
}

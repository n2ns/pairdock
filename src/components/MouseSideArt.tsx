// The left side of a mouse, where the thumb rests: shows the back and forward
// buttons large enough to pick and highlight. Same look as MouseArt.

import { useId } from "react";

import type { MousePart } from "./MouseArt";

type SidePart = Extract<MousePart, "back" | "forward">;

interface Props {
  highlight?: MousePart[];
  selectable?: MousePart[];
  selected?: MousePart | null;
  onSelect?: (part: MousePart) => void;
  marked?: MousePart[];
  size?: number;
}

const SHELL =
  "M18 118 C 14 74 52 34 120 26 C 196 18 262 34 296 70 C 318 94 322 122 314 140 L 22 140 C 19 134 18 126 18 118 Z";

const PARTS: Record<SidePart, { d: string; cx: number; cy: number }> = {
  // Forward sits towards the front (right in this view), back behind it.
  forward: { d: "M176 68 h44 a13 13 0 0 1 0 26 h-44 a13 13 0 0 1 0 -26 Z", cx: 198, cy: 81 },
  back: { d: "M98 74 h44 a13 13 0 0 1 0 26 h-44 a13 13 0 0 1 0 -26 Z", cx: 120, cy: 87 },
};

export function MouseSideArt({
  highlight = [],
  selectable = [],
  selected = null,
  onSelect,
  marked = [],
  size = 300,
}: Props) {
  const uid = useId().replace(/:/g, "");
  const id = (name: string) => `${name}-${uid}`;
  const parts: SidePart[] = ["back", "forward"];

  return (
    <svg
      className="mouse-art mouse-side"
      viewBox="0 0 340 170"
      width={size}
      height={(size * 170) / 340}
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
        <linearGradient id={id("accent")} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#FF8A5B" />
          <stop offset="0.55" stopColor="#E0518F" />
          <stop offset="1" stopColor="#6D4AFF" />
        </linearGradient>
        <filter id={id("glow")} x="-60%" y="-120%" width="220%" height="340%">
          <feGaussianBlur stdDeviation="6" result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>

      <ellipse cx="168" cy="154" rx="150" ry="9" className="mouse-shadow" />
      <path d={SHELL} fill={`url(#${id("body")})`} className="mouse-body" />
      {/* Rubber thumb grip */}
      <path
        d="M40 128 C 52 108 94 100 150 104 C 214 108 268 112 300 132 L 300 140 L 26 140 Z"
        fill={`url(#${id("grip")})`}
      />
      {/* Top shell seam and the front button lip */}
      <path d="M40 72 C 90 46 200 40 282 64" className="mouse-groove" fill="none" />
      <path d={SHELL} className="mouse-edge" fill="none" />

      {parts.map((part) => {
        const lit = highlight.includes(part);
        const canSelect = selectable.includes(part);
        return (
          <path
            key={part}
            d={PARTS[part].d}
            className={[
              "mouse-part",
              lit && "is-lit",
              canSelect && "is-selectable",
              selected === part && "is-selected",
            ]
              .filter(Boolean)
              .join(" ")}
            style={lit || selected === part ? { fill: `url(#${id("accent")})` } : undefined}
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
            cy={PARTS[part].cy - 18}
            r="6"
            fill={`url(#${id("accent")})`}
            className="mouse-mark"
          />
        ))}
    </svg>
  );
}

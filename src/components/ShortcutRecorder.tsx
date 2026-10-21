// Records a key combination into the shortcut syntax understood by the
// backend (e.g. "Ctrl+Shift+T").

import { useState } from "react";

import { isMac } from "../api";
import { useStrings } from "../i18n";

const NAMED: Record<string, string> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Enter: "Enter",
  NumpadEnter: "Enter",
  Escape: "Esc",
  Space: "Space",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  AudioVolumeUp: "VolumeUp",
  AudioVolumeDown: "VolumeDown",
  AudioVolumeMute: "Mute",
  MediaPlayPause: "PlayPause",
  MediaTrackNext: "NextTrack",
  MediaTrackPrevious: "PrevTrack",
};

/** Turns a keydown into a shortcut string, or null for a lone modifier. */
export function shortcutFromEvent(e: KeyboardEvent | React.KeyboardEvent): string | null {
  let key: string | undefined;
  if (/^Key[A-Z]$/.test(e.code)) key = e.code.slice(3);
  else if (/^Digit[0-9]$/.test(e.code)) key = e.code.slice(5);
  else if (/^F([1-9]|1[0-2])$/.test(e.code)) key = e.code;
  else key = NAMED[e.code];
  if (!key) return null;

  const parts = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push(isMac ? "Option" : "Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push(isMac ? "Cmd" : "Win");
  parts.push(key);
  return parts.join("+");
}

export function Keys({ shortcut }: { shortcut: string }) {
  return (
    <span className="keys">
      {shortcut.split("+").map((key, i) => (
        <kbd key={i}>{key}</kbd>
      ))}
    </span>
  );
}

export function ShortcutRecorder({
  value,
  onChange,
}: {
  value: string | null;
  onChange: (shortcut: string) => void;
}) {
  const t = useStrings();
  const [recording, setRecording] = useState(false);

  return (
    <button
      type="button"
      className={`recorder ${recording ? "is-recording" : ""}`}
      onClick={() => setRecording(true)}
      onBlur={() => setRecording(false)}
      onKeyDown={(e) => {
        if (!recording) return;
        e.preventDefault();
        const shortcut = shortcutFromEvent(e);
        if (shortcut) {
          setRecording(false);
          onChange(shortcut);
        }
      }}
    >
      {recording ? (
        <span className="recorder-hint">{t.pressKeys}</span>
      ) : value ? (
        <Keys shortcut={value} />
      ) : (
        <span className="recorder-hint">{t.recordShortcut}</span>
      )}
      <span className="recorder-dot" />
    </button>
  );
}

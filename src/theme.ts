// Light/dark appearance: follows the system unless chosen in Settings. The
// resolved theme is written to <html data-theme>, which the styles key on,
// and handed to the window so the title bar matches.

import { getCurrentWindow } from "@tauri-apps/api/window";

import { isTauri } from "./api";

export type Appearance = "system" | "light" | "dark";

const KEY = "boltkit.appearance";
const systemDark = window.matchMedia("(prefers-color-scheme: dark)");

export function storedAppearance(): Appearance {
  try {
    const value = localStorage.getItem(KEY);
    if (value === "light" || value === "dark") return value;
  } catch {
    // Storage can be unavailable; fall back to the system theme.
  }
  return "system";
}

export function storeAppearance(appearance: Appearance) {
  try {
    localStorage.setItem(KEY, appearance);
  } catch {
    // Not persisted; the choice still applies for this session.
  }
}

/** Applies `appearance` and keeps following the system while it is
 * "system". Returns a function that stops following. */
export function applyAppearance(appearance: Appearance): () => void {
  const update = () => {
    const dark = appearance === "dark" || (appearance === "system" && systemDark.matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  };
  update();

  if (isTauri) {
    getCurrentWindow()
      .setTheme(appearance === "system" ? null : appearance)
      .catch(() => {});
  }

  if (appearance !== "system") return () => {};

  // Two signals for the system theme: the media query (browsers) and the
  // window's theme event, which comes straight from Windows in the app.
  systemDark.addEventListener("change", update);
  const setFromWindow = (theme: "light" | "dark") => {
    document.documentElement.dataset.theme = theme;
  };
  const unlisten = isTauri
    ? getCurrentWindow().onThemeChanged(({ payload }) => setFromWindow(payload))
    : null;

  return () => {
    systemDark.removeEventListener("change", update);
    unlisten?.then((stop) => stop()).catch(() => {});
  };
}

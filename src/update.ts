// Whether PairDock asks GitHub for a newer release when it starts. On by
// default; the choice is kept in this browser profile like the other settings.

const KEY = "pairdock.autoUpdate";

export function storedAutoUpdate(): boolean {
  try {
    return localStorage.getItem(KEY) !== "off";
  } catch {
    // Storage can be unavailable; keep the default.
    return true;
  }
}

export function storeAutoUpdate(enabled: boolean) {
  try {
    localStorage.setItem(KEY, enabled ? "on" : "off");
  } catch {
    // Not persisted; the choice still applies for this session.
  }
}

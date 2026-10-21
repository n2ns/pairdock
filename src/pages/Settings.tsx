import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { useEffect, useState } from "react";

import { version } from "../../package.json";
import { api, isTauri, REPOSITORY_URL, type UpdateInfo } from "../api";
import { type Language, useStrings } from "../i18n";
import type { Appearance } from "../theme";
import { storeAutoUpdate, storedAutoUpdate } from "../update";

export function Settings({
  language,
  onLanguage,
  appearance,
  onAppearance,
  onUpdate,
}: {
  language: Language;
  onLanguage: (language: Language) => void;
  appearance: Appearance;
  onAppearance: (appearance: Appearance) => void;
  onUpdate: (update: UpdateInfo) => void;
}) {
  const t = useStrings();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [autoUpdate, setAutoUpdate] = useState(storedAutoUpdate);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "checking" | "current" | "failed">(
    "idle",
  );

  const toggleAutoUpdate = () => {
    setAutoUpdate(!autoUpdate);
    storeAutoUpdate(!autoUpdate);
  };

  const checkUpdate = async () => {
    setUpdateStatus("checking");
    try {
      const update = await api.checkUpdate();
      setUpdateStatus(update ? "idle" : "current");
      if (update) onUpdate(update);
    } catch {
      setUpdateStatus("failed");
    }
  };

  useEffect(() => {
    if (isTauri) isEnabled().then(setAutostart).catch(() => setAutostart(false));
    else setAutostart(false);
  }, []);

  const toggleAutostart = async () => {
    const next = !autostart;
    setAutostart(next);
    if (!isTauri) return;
    try {
      await (next ? enable() : disable());
    } catch {
      setAutostart(!next);
    }
  };

  return (
    <div className="page settings">
      <header className="page-header">
        <h1>{t.settingsTitle}</h1>
      </header>

      <section className="card setting-row">
        <div>
          <h3>{t.autostart}</h3>
          <p className="muted small">{t.autostartHint}</p>
        </div>
        <button
          role="switch"
          aria-checked={!!autostart}
          className={`switch ${autostart ? "is-on" : ""}`}
          disabled={autostart === null}
          onClick={toggleAutostart}
        >
          <span />
        </button>
      </section>

      <section className="card setting-row">
        <div>
          <h3>{t.appearance}</h3>
        </div>
        <div className="segmented compact">
          {(["system", "light", "dark"] as Appearance[]).map((value) => (
            <button
              key={value}
              className={appearance === value ? "is-active" : ""}
              onClick={() => onAppearance(value)}
            >
              {value === appearance && <span className="segment-bg" />}
              <span className="segment-label">
                {value === "system"
                  ? t.appearanceSystem
                  : value === "light"
                    ? t.appearanceLight
                    : t.appearanceDark}
              </span>
            </button>
          ))}
        </div>
      </section>

      <section className="card setting-row">
        <div>
          <h3>{t.language}</h3>
        </div>
        <div className="segmented compact">
          {(["auto", "zh", "en"] as Language[]).map((lang) => (
            <button
              key={lang}
              className={language === lang ? "is-active" : ""}
              onClick={() => onLanguage(lang)}
            >
              {lang === language && <span className="segment-bg" />}
              <span className="segment-label">
                {lang === "auto" ? t.languageAuto : lang === "zh" ? "中文" : "English"}
              </span>
            </button>
          ))}
        </div>
      </section>

      <section className="card setting-row">
        <div>
          <h3>{t.autoUpdate}</h3>
          <p className="muted small">{t.autoUpdateHint}</p>
        </div>
        <button
          role="switch"
          aria-checked={autoUpdate}
          className={`switch ${autoUpdate ? "is-on" : ""}`}
          onClick={toggleAutoUpdate}
        >
          <span />
        </button>
      </section>

      <section className="card setting-row">
        <div>
          <h3>{t.updates}</h3>
          {updateStatus !== "idle" && (
            <p className="muted small">
              {updateStatus === "checking"
                ? t.checkingUpdate
                : updateStatus === "current"
                  ? t.upToDate
                  : t.updateCheckFailed}
            </p>
          )}
        </div>
        <button className="btn" disabled={updateStatus === "checking"} onClick={checkUpdate}>
          {t.checkUpdate}
        </button>
      </section>

      <section className="card about">
        <img src="/icon.svg" alt="" width="64" height="64" />
        <div>
          <h3>PairDock {version}</h3>
          <p className="muted small">{t.aboutText}</p>
          <p className="small">
            <a
              href={REPOSITORY_URL}
              onClick={(e) => {
                e.preventDefault();
                api.openRepository().catch(() => {});
              }}
            >
              {t.sourceCode}
            </a>
          </p>
        </div>
        <button className="btn about-star" onClick={() => api.openRepository().catch(() => {})}>
          <span aria-hidden="true">★</span> {t.starOnGithub}
        </button>
      </section>
    </div>
  );
}

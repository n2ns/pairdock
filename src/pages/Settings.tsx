import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { useEffect, useState } from "react";

import { isTauri } from "../api";
import { type Language, useStrings } from "../i18n";

export function Settings({
  language,
  onLanguage,
}: {
  language: Language;
  onLanguage: (language: Language) => void;
}) {
  const t = useStrings();
  const [autostart, setAutostart] = useState<boolean | null>(null);

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

      <section className="card about">
        <img src="/icon.svg" alt="" width="64" height="64" />
        <div>
          <h3>boltkit 0.1.0</h3>
          <p className="muted small">{t.aboutText}</p>
        </div>
      </section>
    </div>
  );
}

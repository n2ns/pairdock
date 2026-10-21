import { AnimatePresence, motion } from "motion/react";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import { isTauri, type PairedDevice } from "./api";
import { type Language, resolveStrings, StringsContext } from "./i18n";
import { DeviceDetail } from "./pages/DeviceDetail";
import { Devices } from "./pages/Devices";
import { PairWizard } from "./pages/PairWizard";
import { Settings } from "./pages/Settings";
import { type Appearance, applyAppearance, storeAppearance, storedAppearance } from "./theme";

type Route =
  | { name: "devices" }
  | { name: "device"; receiverId: string; device: PairedDevice }
  | { name: "pair" }
  | { name: "settings" };

const LANGUAGE_KEY = "boltkit.language";

function storedLanguage(): Language {
  try {
    const value = localStorage.getItem(LANGUAGE_KEY);
    if (value === "zh" || value === "en") return value;
  } catch {
    // Storage can be unavailable; fall back to the system language.
  }
  return "auto";
}

export default function App() {
  const [route, setRoute] = useState<Route>({ name: "devices" });
  const [language, setLanguage] = useState<Language>(storedLanguage);
  const t = useMemo(() => resolveStrings(language), [language]);
  const [appearance, setAppearance] = useState<Appearance>(storedAppearance);

  // Before the first paint, so the page never shows the wrong theme.
  useLayoutEffect(() => applyAppearance(appearance), [appearance]);

  const changeAppearance = (next: Appearance) => {
    setAppearance(next);
    storeAppearance(next);
  };
  const contentRef = useRef<HTMLElement>(null);

  // Every page starts at the top; the content pane keeps its scroll otherwise.
  useEffect(() => {
    contentRef.current?.scrollTo({ top: 0 });
  }, [route]);

  const changeLanguage = (lang: Language) => {
    setLanguage(lang);
    try {
      localStorage.setItem(LANGUAGE_KEY, lang);
    } catch {
      // Not persisted; the choice still applies for this session.
    }
  };

  const customize = (receiverId: string, device: PairedDevice) =>
    setRoute({ name: "device", receiverId, device });

  const nav: { route: Route["name"]; label: string; icon: React.ReactNode }[] = [
    {
      route: "devices",
      label: t.navDevices,
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden>
          <rect x="7" y="3" width="10" height="18" rx="5" />
          <path d="M12 3v6M7 9h10" />
        </svg>
      ),
    },
    {
      route: "pair",
      label: t.navPair,
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden>
          <circle cx="12" cy="12" r="8.5" />
          <path d="M12 8v8M8 12h8" />
        </svg>
      ),
    },
    {
      route: "settings",
      label: t.navSettings,
      icon: (
        <svg viewBox="0 0 24 24" aria-hidden>
          <path d="M4 7h10M18 7h2M4 17h4M12 17h8" />
          <circle cx="16" cy="7" r="2.2" />
          <circle cx="10" cy="17" r="2.2" />
        </svg>
      ),
    },
  ];
  const active = route.name === "device" ? "devices" : route.name;

  return (
    <StringsContext.Provider value={t}>
      <div className="shell">
        <aside className="sidebar">
          <div className="brand">
            <img src="/icon.svg" alt="" width="36" height="36" />
            <strong>BoltKit</strong>
          </div>
          <nav>
            {nav.map((item) => (
              <button
                key={item.route}
                className={`nav-item ${active === item.route ? "is-active" : ""}`}
                onClick={() => setRoute({ name: item.route } as Route)}
              >
                {active === item.route && (
                  <motion.span layoutId="nav-active" className="nav-active" transition={{ type: "spring", stiffness: 420, damping: 34 }} />
                )}
                <span className="nav-icon">{item.icon}</span>
                <span className="nav-label">{item.label}</span>
              </button>
            ))}
          </nav>
          {!isTauri && <p className="demo-note">{t.demoMode}</p>}
        </aside>

        <main className="content" ref={contentRef}>
          <AnimatePresence mode="wait">
            <motion.div
              key={route.name === "device" ? `device-${route.device.slot}` : route.name}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.22, ease: "easeOut" }}
            >
              {route.name === "devices" && (
                <Devices onCustomize={customize} onPair={() => setRoute({ name: "pair" })} />
              )}
              {route.name === "device" && (
                <DeviceDetail
                  receiverId={route.receiverId}
                  device={route.device}
                  onBack={() => setRoute({ name: "devices" })}
                />
              )}
              {route.name === "pair" && (
                <PairWizard onDone={() => setRoute({ name: "devices" })} onCustomize={customize} />
              )}
              {route.name === "settings" && (
                <Settings
                  language={language}
                  onLanguage={changeLanguage}
                  appearance={appearance}
                  onAppearance={changeAppearance}
                />
              )}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>
    </StringsContext.Provider>
  );
}

//! State shared by the commands: the configuration, the remapping engine and
//! a running pairing session.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use pairdock_core::{
    config::Config,
    hidpp::channel::HidppChannel,
    pairing::{BoltPairing, Discovered},
    remap::RemapEngine,
};
use tauri::{AppHandle, Manager};

pub struct AppState {
    config_path: PathBuf,
    pub config: Mutex<Config>,
    engine: Mutex<Option<RemapEngine>>,
    pub pairing: tokio::sync::Mutex<Option<PairingSession>>,
}

/// Devices found by the running discovery, by address.
pub type DiscoveredDevices = Arc<Mutex<HashMap<[u8; 6], Discovered>>>;

/// A running pairing session. The channel has to outlive the session.
pub struct PairingSession {
    pub session: BoltPairing,
    pub _chan: Arc<HidppChannel>,
    pub discovered: DiscoveredDevices,
}

/// The app identifier before the project was renamed to PairDock. Its config
/// directory may still hold the user's button settings.
const LEGACY_IDENTIFIER: &str = "dev.boltkit.app";

/// Copies the configuration from the pre-rename config directory, once: only
/// when the current one has none yet. The old file is left in place.
fn migrate_legacy_config(config_path: &Path) {
    if config_path.exists() {
        return;
    }
    let Some(legacy) = config_path
        .parent()
        .and_then(Path::parent)
        .map(|dir| dir.join(LEGACY_IDENTIFIER).join("config.json"))
    else {
        return;
    };
    if !legacy.exists() {
        return;
    }
    let copied = config_path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::copy(&legacy, config_path));
    match copied {
        Ok(_) => tracing::info!("migrated settings from {}", legacy.display()),
        Err(err) => tracing::warn!("could not migrate {}: {err}", legacy.display()),
    }
}

impl AppState {
    /// Reads the configuration and starts the remapping engine.
    pub fn load(app: &AppHandle) -> tauri::Result<Self> {
        let config_path = app.path().app_config_dir()?.join("config.json");
        migrate_legacy_config(&config_path);
        let config = match std::fs::read_to_string(&config_path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|err| {
                tracing::warn!("ignoring invalid {}: {err}", config_path.display());
                Config::default()
            }),
            Err(_) => Config::default(),
        };

        // The engine spawns its tasks on the async runtime, so start it there.
        let engine = tauri::async_runtime::block_on({
            let config = config.clone();
            async move { RemapEngine::start(config) }
        });

        Ok(Self {
            config_path,
            config: Mutex::new(config),
            engine: Mutex::new(Some(engine)),
            pairing: tokio::sync::Mutex::new(None),
        })
    }

    /// Saves the configuration and hands it to the engine.
    pub fn store_config(&self, config: Config) -> Result<(), String> {
        if let Some(dir) = self.config_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
        std::fs::write(&self.config_path, text).map_err(|e| e.to_string())?;

        if let Some(engine) = self.engine.lock().unwrap().as_ref() {
            engine.set_config(config.clone());
        }
        *self.config.lock().unwrap() = config;
        Ok(())
    }

    /// Stops pairing and the engine, restoring buttons within `limit`.
    /// Both run side by side, each bounded by `limit`.
    pub async fn shutdown(&self, limit: Duration) {
        let pairing = tokio::time::timeout(limit, async {
            if let Some(pairing) = self.pairing.lock().await.take() {
                let _ = pairing.session.finish().await;
            }
        });
        let engine = self.engine.lock().unwrap().take();
        let engine = async {
            if let Some(engine) = engine {
                engine.stop(limit).await;
            }
        };
        let _ = tokio::join!(pairing, engine);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        error::Error,
        fs,
        path::PathBuf,
        sync::{
            Arc, Condvar, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use pairdock_core::{
        config::Config,
        hidpp::{
            async_trait,
            channel::{HidppChannel, RawHidChannel},
            receiver::detect,
        },
        pairing::BoltPairing,
    };

    use super::{AppState, LEGACY_IDENTIFIER, PairingSession, migrate_legacy_config};

    #[test]
    fn copies_the_legacy_config_once() {
        let root = std::env::temp_dir().join(format!("pairdock-migrate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let legacy = root.join(LEGACY_IDENTIFIER);
        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("config.json"), r#"{"devices":{}}"#).unwrap();
        let current = root.join("dev.pairdock.desktop").join("config.json");

        migrate_legacy_config(&current);
        assert_eq!(fs::read_to_string(&current).unwrap(), r#"{"devices":{}}"#);

        // An existing config is never overwritten.
        fs::write(&current, "mine").unwrap();
        migrate_legacy_config(&current);
        assert_eq!(fs::read_to_string(&current).unwrap(), "mine");

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn does_nothing_without_a_legacy_config() {
        let root = std::env::temp_dir().join(format!("pairdock-fresh-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let current = root.join("dev.pairdock.desktop").join("config.json");

        migrate_legacy_config(&current);
        assert!(!current.exists());
    }

    /// A Bolt receiver that echoes every request until it is told to stop
    /// answering.
    struct Receiver {
        silent: Arc<AtomicBool>,
        inbox: (Mutex<VecDeque<Vec<u8>>>, Condvar),
    }

    #[async_trait]
    impl RawHidChannel for Receiver {
        fn vendor_id(&self) -> u16 {
            0x046d
        }

        fn product_id(&self) -> u16 {
            0xc548
        }

        async fn write_report(&self, src: &[u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            if !self.silent.load(Ordering::SeqCst) {
                let (queue, ready) = &self.inbox;
                queue.lock().unwrap().push_back(src.to_vec());
                ready.notify_all();
            }
            Ok(src.len())
        }

        async fn read_report(&self, buf: &mut [u8]) -> Result<usize, Box<dyn Error + Sync + Send>> {
            let (queue, ready) = &self.inbox;
            let mut queue = queue.lock().unwrap();
            if queue.is_empty() {
                queue = ready.wait_timeout(queue, Duration::from_millis(50)).unwrap().0;
            }
            match queue.pop_front() {
                Some(report) => {
                    buf[..report.len()].copy_from_slice(&report);
                    Ok(report.len())
                },
                None => Ok(0),
            }
        }

        fn supports_short_long_hidpp(&self) -> Option<(bool, bool)> {
            Some((true, true))
        }

        async fn get_report_descriptor(
            &self,
            _buf: &mut [u8],
        ) -> Result<usize, Box<dyn Error + Sync + Send>> {
            unreachable!()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn quitting_while_pairing_is_bounded() {
        let silent = Arc::new(AtomicBool::new(false));
        let receiver = Receiver {
            silent: Arc::clone(&silent),
            inbox: Default::default(),
        };
        let chan = Arc::new(HidppChannel::from_raw_channel(receiver).await.unwrap());
        let Some(hidpp::receiver::Receiver::Bolt(bolt)) = detect(Arc::clone(&chan)) else {
            panic!("not detected as a Bolt receiver");
        };
        let (session, _events) = BoltPairing::start(bolt).await.unwrap();
        let state = AppState {
            config_path: PathBuf::new(),
            config: Mutex::new(Config::default()),
            engine: Mutex::new(None),
            pairing: tokio::sync::Mutex::new(Some(PairingSession {
                session,
                _chan: chan,
                discovered: Default::default(),
            })),
        };

        // The receiver hangs while the user quits.
        silent.store(true, Ordering::SeqCst);
        tokio::time::timeout(Duration::from_secs(2), state.shutdown(Duration::from_millis(500)))
            .await
            .expect("quitting waited for the receiver beyond the limit");
    }
}

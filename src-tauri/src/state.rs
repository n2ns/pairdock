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
    pub async fn shutdown(&self, limit: Duration) {
        if let Some(pairing) = self.pairing.lock().await.take() {
            let _ = pairing.session.finish().await;
        }
        let engine = self.engine.lock().unwrap().take();
        if let Some(engine) = engine {
            engine.stop(limit).await;
        }
    }
}

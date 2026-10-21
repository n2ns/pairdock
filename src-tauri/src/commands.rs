//! Commands invoked by the frontend.

use std::sync::Arc;

use pairdock_core::{
    config::{ButtonAction, Config},
    inventory::{self, Battery, DeviceDetails, ReceiverSummary},
    keys::{self, parse_shortcut},
    pairing::{BoltPairing, PairingEvent},
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

use crate::{
    RESTORE_LIMIT,
    update_marker,
    state::{AppState, DiscoveredDevices, PairingSession},
};

/// The event carrying [`PairingEvent`]s to the frontend.
const PAIRING_EVENT: &str = "pairing";

#[tauri::command]
pub async fn list_receivers() -> Result<Vec<ReceiverSummary>, String> {
    inventory::receivers().await
}

#[tauri::command]
pub async fn device_details(receiver_id: String, slot: u8) -> Result<DeviceDetails, String> {
    inventory::device_details(&receiver_id, slot).await
}

#[tauri::command]
pub async fn device_battery(receiver_id: String, slot: u8) -> Result<Option<Battery>, String> {
    inventory::device_battery(&receiver_id, slot).await
}

#[tauri::command]
pub async fn unpair(receiver_id: String, slot: u8) -> Result<(), String> {
    let (_chan, bolt) = inventory::open_bolt(&receiver_id).await?;
    bolt.unpair_device(slot).await.map_err(|e| e.to_string())
}

/// Starts discovery on a receiver. Progress arrives as `pairing` events.
#[tauri::command]
pub async fn start_pairing(
    app: AppHandle,
    state: State<'_, AppState>,
    receiver_id: String,
) -> Result<(), String> {
    let mut pairing = state.pairing.lock().await;
    if let Some(old) = pairing.take() {
        let _ = old.session.finish().await;
    }

    let (chan, bolt) = inventory::open_bolt(&receiver_id).await?;
    let (session, events) = BoltPairing::start(bolt).await.map_err(|e| e.to_string())?;

    let discovered = DiscoveredDevices::default();
    tauri::async_runtime::spawn({
        let discovered = Arc::clone(&discovered);
        async move {
            while let Ok(event) = events.recv().await {
                if let PairingEvent::Discovered(device) = &event {
                    discovered
                        .lock()
                        .unwrap()
                        .insert(device.address, device.clone());
                }
                let _ = app.emit(PAIRING_EVENT, &event);
            }
        }
    });

    *pairing = Some(PairingSession {
        session,
        _chan: chan,
        discovered,
    });
    Ok(())
}

/// Pairs a device reported by a `discovered` event.
#[tauri::command]
pub async fn pair_device(state: State<'_, AppState>, address: [u8; 6]) -> Result<(), String> {
    let pairing = state.pairing.lock().await;
    let pairing = pairing.as_ref().ok_or("pairing is not running")?;
    let device = pairing
        .discovered
        .lock()
        .unwrap()
        .get(&address)
        .cloned()
        .ok_or("unknown device")?;
    pairing.session.pair(&device).await.map_err(|e| e.to_string())
}

/// Ends the pairing session; also keeps discovery searching otherwise.
#[tauri::command]
pub async fn stop_pairing(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(pairing) = state.pairing.lock().await.take() {
        pairing.session.finish().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Restarts discovery after it timed out, used by the wizard while it waits.
#[tauri::command]
pub async fn rediscover(state: State<'_, AppState>) -> Result<(), String> {
    let pairing = state.pairing.lock().await;
    let pairing = pairing.as_ref().ok_or("pairing is not running")?;
    pairing.session.rediscover().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

/// Assigns an action to a button of a device model and applies it.
#[tauri::command]
pub fn set_button(
    state: State<'_, AppState>,
    wpid: u16,
    cid: u16,
    action: ButtonAction,
) -> Result<Config, String> {
    if let ButtonAction::Shortcut { keys } = &action {
        parse_shortcut(keys)?;
    }

    let mut config = state.config.lock().unwrap().clone();
    let buttons = &mut config.devices.entry(wpid).or_default().buttons;
    if action == ButtonAction::Default {
        buttons.remove(&cid);
    } else {
        buttons.insert(cid, action);
    }
    config.devices.retain(|_, device| !device.buttons.is_empty());

    state.store_config(config.clone())?;
    Ok(config)
}

/// Whether shortcuts can be sent: on macOS PairDock needs the Accessibility
/// permission for that.
#[tauri::command]
pub fn can_send_keys() -> bool {
    keys::can_send_keys()
}

/// Asks the system for the permission to send shortcuts.
#[tauri::command]
pub fn request_send_keys() {
    keys::request_send_keys();
}

/// A newer release, as shown to the user.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    version: String,
}

/// Asks GitHub for a newer release.
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<UpdateInfo>, String> {
    let updater = app
        .updater_builder()
        .on_before_exit({
            let app = app.clone();
            move || remove_tray(&app)
        })
        .build()
        .map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    let info = update.as_ref().map(|update| UpdateInfo {
        version: update.version.clone(),
    });
    *state.update.lock().await = update;
    Ok(info)
}

/// Downloads the update found by [`check_update`], restores the buttons and
/// runs the installer, which restarts PairDock with the user's settings.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let update = state.update.lock().await.clone().ok_or("no update available")?;
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;

    // Leave the buttons working while PairDock is not running. The settings
    // file is untouched; the new version applies it again when it starts.
    state.shutdown(RESTORE_LIMIT).await;
    let marker = update_marker(&app);
    if let Some(marker) = &marker {
        let _ = marker.parent().map(std::fs::create_dir_all);
        let _ = std::fs::write(marker, "");
    }
    // On Windows this exits the app once the installer has started.
    if let Err(err) = update.install(bytes) {
        if let Some(marker) = &marker {
            let _ = std::fs::remove_file(marker);
        }
        state.restart_engine();
        return Err(err.to_string());
    }
    app.restart()
}

/// Removes the tray icon before the updater ends the process, which would
/// otherwise leave a dead icon in the notification area.
fn remove_tray(app: &AppHandle) {
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let handle = app.clone();
    let queued = app.run_on_main_thread(move || {
        drop(handle.remove_tray_by_id("main"));
        let _ = done_tx.send(());
    });
    if queued.is_ok() {
        let _ = done_rx.recv_timeout(std::time::Duration::from_secs(1));
    }
}

/// The project's home, opened from Settings. Fixed here so that the page can
/// only open this one address.
const REPOSITORY_URL: &str = "https://github.com/n2ns/pairdock";

/// Opens the GitHub repository in the default browser.
#[tauri::command]
pub fn open_repository(app: AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(REPOSITORY_URL, None::<&str>)
        .map_err(|e| e.to_string())
}

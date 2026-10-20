//! Commands invoked by the frontend.

use std::sync::Arc;

use boltkit_core::{
    config::{ButtonAction, Config},
    inventory::{self, DeviceDetails, ReceiverSummary},
    keys::parse_shortcut,
    pairing::{BoltPairing, PairingEvent},
};
use tauri::{AppHandle, Emitter, State};

use crate::state::{AppState, DiscoveredDevices, PairingSession};

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

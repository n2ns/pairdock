//! Snapshots of receivers, paired devices and their controls for display.

use std::{sync::Arc, time::Duration};

use hidapi::HidApi;
use hidpp::{
    channel::HidppChannel,
    device::Device,
    feature::{
        CreatableFeature,
        reprog_controls::{ControlFlags, ReprogControlsFeature},
        unified_battery::{BatteryLevel, BatteryStatus, UnifiedBatteryFeature},
    },
    receiver::{Receiver, bolt},
};
use serde::Serialize;
use tokio::time::timeout;

use crate::{
    controls::control_name,
    enumerate::{ChannelRole, list_receivers, open_receiver},
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiverSummary {
    pub id: String,
    pub name: String,
    pub unique_id: String,
    /// Whether boltkit can pair, unpair and list devices on this receiver.
    pub supported: bool,
    pub devices: Vec<PairedDevice>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedDevice {
    pub slot: u8,
    pub name: String,
    pub kind: bolt::DeviceKind,
    pub wpid: u16,
    /// The device's serial number. Together with `wpid` it identifies one
    /// physical device, e.g. to find slots it left behind when re-paired.
    pub unit_id: String,
    pub online: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDetails {
    pub battery: Option<Battery>,
    pub controls: Vec<Control>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    /// Charge in percent, if the device reports it.
    pub percent: Option<u8>,
    /// `critical`, `low`, `good` or `full`.
    pub level: &'static str,
    pub charging: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub cid: u16,
    pub name: Option<&'static str>,
    /// Can be turned into a shortcut (needs boltkit running).
    pub divertable: bool,
    /// Can perform another control's action on the device itself.
    pub remappable: bool,
    /// The controls this one can be remapped to, itself included.
    pub remap_targets: Vec<u16>,
}

/// Lists all receivers and the devices paired to them.
pub async fn receivers() -> Result<Vec<ReceiverSummary>, String> {
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let mut out = Vec::new();

    for iface in list_receivers(&api) {
        let id = iface.id();
        let (_chan, receiver) = match open_receiver(&api, &iface, ChannelRole::Shared).await {
            Ok(opened) => opened,
            Err(err) => {
                tracing::warn!("could not open receiver {id}: {err}");
                continue;
            },
        };
        let name = receiver.name();
        let unique_id = with_timeout(receiver.get_unique_id()).await.unwrap_or_default();

        let (supported, devices) = match &receiver {
            Receiver::Bolt(bolt) => (true, bolt_devices(bolt).await),
            _ => (false, Vec::new()),
        };

        out.push(ReceiverSummary {
            id,
            name,
            unique_id,
            supported,
            devices,
        });
    }

    Ok(out)
}

async fn bolt_devices(bolt: &bolt::Receiver) -> Vec<PairedDevice> {
    let mut devices = Vec::new();
    for slot in 1..=6u8 {
        let Ok(info) = with_timeout(bolt.get_device_pairing_information(slot)).await else {
            continue;
        };
        let name = with_timeout(bolt.get_device_codename(slot))
            .await
            .unwrap_or_default();
        devices.push(PairedDevice {
            slot,
            name,
            kind: info.kind,
            wpid: info.wpid,
            unit_id: info.unit_id.iter().map(|b| format!("{b:02X}")).collect(),
            online: info.online,
        });
    }
    devices
}

/// Opens the Bolt receiver with the given id.
pub async fn open_bolt(receiver_id: &str) -> Result<(Arc<HidppChannel>, bolt::Receiver), String> {
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let iface = list_receivers(&api)
        .into_iter()
        .find(|iface| iface.id() == receiver_id)
        .ok_or("the receiver is no longer connected")?;
    match open_receiver(&api, &iface, ChannelRole::Shared).await.map_err(|e| e.to_string())? {
        (chan, Receiver::Bolt(bolt)) => Ok((chan, bolt)),
        _ => Err("only Bolt receivers are supported".into()),
    }
}

/// Reads the battery and the reprogrammable controls of an online device.
pub async fn device_details(receiver_id: &str, slot: u8) -> Result<DeviceDetails, String> {
    let (chan, _bolt) = open_bolt(receiver_id).await?;
    let mut device = with_timeout(Device::new(chan, slot)).await?;
    with_timeout(device.enumerate_features()).await?;

    let battery = match device.get_feature::<UnifiedBatteryFeature>() {
        Some(feature) => read_battery(&feature).await,
        None => None,
    };

    let mut controls = Vec::new();
    if let Some(reprog) = device.get_feature::<ReprogControlsFeature>() {
        let count = with_timeout(reprog.get_count()).await?;
        let mut infos = Vec::with_capacity(count as usize);
        for i in 0..count {
            infos.push(with_timeout(reprog.get_control_info(i)).await?);
        }

        for info in &infos {
            let remappable = info.flags.has(ControlFlags::REPROGRAMMABLE);
            let divertable = info.flags.has(ControlFlags::DIVERTABLE);
            if !remappable && !divertable {
                continue;
            }
            // Bit n of the group mask allows remapping to controls of group n+1.
            let remap_targets = infos
                .iter()
                .filter(|t| {
                    t.cid == info.cid
                        || (remappable && t.group > 0 && info.group_mask & (1 << (t.group - 1)) != 0)
                })
                .map(|t| t.cid)
                .collect();
            controls.push(Control {
                cid: info.cid,
                name: control_name(info.cid),
                divertable,
                remappable,
                remap_targets,
            });
        }
    }

    Ok(DeviceDetails { battery, controls })
}

/// Reads only the battery of an online device. Looks the battery feature up
/// directly instead of enumerating every feature, so it stays quick enough
/// for the device list.
pub async fn device_battery(receiver_id: &str, slot: u8) -> Result<Option<Battery>, String> {
    let (chan, _bolt) = open_bolt(receiver_id).await?;
    let mut device = with_timeout(Device::new(chan, slot)).await?;
    let Some(info) = with_timeout(device.root().get_feature(UnifiedBatteryFeature::ID)).await?
    else {
        return Ok(None);
    };
    let feature = device.add_feature::<UnifiedBatteryFeature>(info.index);
    Ok(read_battery(&feature).await)
}

async fn read_battery(feature: &UnifiedBatteryFeature) -> Option<Battery> {
    let info = with_timeout(feature.get_battery_info()).await.ok()?;
    Some(Battery {
        percent: (info.charging_percentage > 0).then_some(info.charging_percentage),
        level: match info.level {
            BatteryLevel::Critical => "critical",
            BatteryLevel::Low => "low",
            BatteryLevel::Good => "good",
            _ => "full",
        },
        charging: matches!(
            info.status,
            BatteryStatus::Charging | BatteryStatus::ChargingSlow
        ),
    })
}

async fn with_timeout<T, E: std::fmt::Display>(
    fut: impl Future<Output = Result<T, E>>,
) -> Result<T, String> {
    match timeout(REQUEST_TIMEOUT, fut).await {
        Ok(result) => result.map_err(|e| e.to_string()),
        Err(_) => Err("the device did not answer (is it awake?)".into()),
    }
}

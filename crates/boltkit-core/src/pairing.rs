//! Pairing new devices to a Bolt receiver.
//!
//! The flow follows Solaar's implementation:
//!
//! 1. Enable wireless notifications and start device discovery. The receiver
//!    reports every nearby device in pairing mode, split into a "details" and a
//!    "name" notification that share a counter.
//! 2. Pair one of them by its address. The receiver then asks for a passkey:
//!    keyboards type a number, mice click a left/right sequence.
//! 3. A pairing status notification reports the new slot or an error.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use hidpp::receiver::{
    ReceiverError,
    bolt::{self, DeviceKind, Event, PairingPasskeyPressType, PairingState},
};
use serde::Serialize;

/// Devices handed to [`BoltPairing::pair`], by address. The event translator
/// needs them to tell whether a passkey has to be typed or clicked.
type PairingDevices = Arc<Mutex<HashMap<[u8; 6], Discovered>>>;

/// How long discovery runs before the receiver stops it, in seconds.
pub const DISCOVERY_TIMEOUT_SECS: u8 = 30;

/// A device in pairing mode found during discovery.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct Discovered {
    pub address: [u8; 6],
    pub kind: DeviceKind,
    pub wpid: u16,
    pub authentication: u8,
    pub name: String,
}

impl Discovered {
    /// Keyboards authenticate by typing the passkey, everything else by
    /// clicking it.
    fn types_passkey(&self) -> bool {
        self.authentication & 0x01 != 0
    }

    /// How many bits of passkey the receiver should ask for. Solaar uses 20
    /// for keyboards and 10 for everything else.
    fn entropy(&self) -> u8 {
        if self.kind == DeviceKind::Keyboard { 20 } else { 10 }
    }
}

/// A mouse button to click while entering a passkey.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum Click {
    Left,
    Right,
}

/// What the user has to do on the device to confirm pairing.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub enum PasskeyPrompt {
    /// Type these digits on the keyboard, then press Enter.
    Type(String),

    /// Click these buttons in order, then press left and right together.
    Click(Vec<Click>),
}

/// Progress of a pairing session.
#[derive(Clone, PartialEq, Debug, Serialize)]
pub enum PairingEvent {
    DiscoveryStarted,

    /// A device in pairing mode was found. Every address is reported once.
    Discovered(Discovered),

    /// Discovery ended, either because it timed out, was cancelled or because
    /// pairing started. A non-zero `error` means the receiver aborted it.
    DiscoveryStopped { error: u8 },

    /// The receiver waits for the passkey to be entered on the device.
    Passkey { address: [u8; 6], prompt: PasskeyPrompt },

    /// The user pressed a key while entering the passkey.
    PasskeyProgress { address: [u8; 6], press: PasskeyPress },

    /// The device was paired to `slot`.
    Paired { slot: u8 },

    /// Pairing ended without a new device.
    Failed { reason: String },
}

/// A press reported while the passkey is entered.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
pub enum PasskeyPress {
    Started,
    Key,
    Submitted,
}

/// A running pairing session on a Bolt receiver.
pub struct BoltPairing {
    receiver: bolt::Receiver,
    notifications_were_on: bool,
    devices: PairingDevices,
}

impl BoltPairing {
    /// Enables notifications, starts discovery and returns the session along
    /// with its event stream.
    pub async fn start(
        receiver: bolt::Receiver,
    ) -> Result<(Self, async_channel::Receiver<PairingEvent>), ReceiverError> {
        let state = receiver.get_notification_state().await?;
        if !state.wireless_notifications {
            receiver
                .set_notification_state(notification_state(true))
                .await?;
        }

        let (tx, rx) = async_channel::unbounded();
        let devices = PairingDevices::default();
        tokio::spawn(translate_events(receiver.listen(), tx, Arc::clone(&devices)));

        receiver.discover_devices(Some(DISCOVERY_TIMEOUT_SECS)).await?;

        Ok((
            Self {
                receiver,
                notifications_were_on: state.wireless_notifications,
                devices,
            },
            rx,
        ))
    }

    /// Starts another discovery round, e.g. after the previous one timed out.
    pub async fn rediscover(&self) -> Result<(), ReceiverError> {
        self.receiver
            .discover_devices(Some(DISCOVERY_TIMEOUT_SECS))
            .await?;
        Ok(())
    }

    /// Stops discovery and starts pairing the given device. Progress is
    /// reported through the event stream.
    pub async fn pair(&self, device: &Discovered) -> Result<(), ReceiverError> {
        // Discovery has to be stopped before the receiver accepts a pairing
        // request; an error here only means it already stopped.
        let _ = self.receiver.cancel_device_discovery().await;

        self.devices
            .lock()
            .unwrap()
            .insert(device.address, device.clone());

        // Slot 0 lets the receiver pick a free slot.
        self.receiver
            .pair_device(0, device.address, device.authentication, device.entropy())
            .await?;
        Ok(())
    }

    /// Cancels discovery and restores the receiver's notification setting.
    pub async fn finish(self) -> Result<(), ReceiverError> {
        let _ = self.receiver.cancel_device_discovery().await;
        if !self.notifications_were_on {
            self.receiver
                .set_notification_state(notification_state(false))
                .await?;
        }
        Ok(())
    }
}

fn notification_state(wireless: bool) -> bolt::NotificationState {
    bolt::NotificationStateBuilder::default()
        .wireless_notifications(wireless)
        .build()
        .expect("all fields are set")
}

/// Turns raw receiver events into [`PairingEvent`]s until either side closes.
async fn translate_events(
    rx: async_channel::Receiver<Event>,
    tx: async_channel::Sender<PairingEvent>,
    devices: PairingDevices,
) {
    // Discovery details and names arrive separately but share a counter.
    let mut details: HashMap<u16, (DeviceKind, u16, [u8; 6], u8)> = HashMap::new();
    let mut names: HashMap<u16, String> = HashMap::new();
    let mut reported: Vec<[u8; 6]> = Vec::new();

    while let Ok(event) = rx.recv().await {
        let out = match event {
            Event::DeviceDiscoveryStatus {
                discovery_enabled,
                error,
            } => Some(if discovery_enabled {
                details.clear();
                names.clear();
                reported.clear();
                PairingEvent::DiscoveryStarted
            } else {
                PairingEvent::DiscoveryStopped { error }
            }),
            Event::DeviceDiscoveryDeviceDetails {
                counter,
                kind,
                wpid,
                address,
                authentication,
            } => {
                details.insert(counter, (kind, wpid, address, authentication));
                complete_discovery(counter, &details, &names, &mut reported)
            },
            Event::DeviceDiscoveryDeviceName { counter, name } => {
                names.insert(counter, name);
                complete_discovery(counter, &details, &names, &mut reported)
            },
            Event::PairingPasskeyRequest {
                device_address,
                passkey,
            } => {
                let device = devices.lock().unwrap().get(&device_address).cloned();
                let types = device.as_ref().is_none_or(Discovered::types_passkey);
                passkey_prompt(&passkey, types).map(|prompt| PairingEvent::Passkey {
                    address: device_address,
                    prompt,
                })
            },
            Event::PairingPasskeyPressed {
                device_address,
                press_type,
            } => Some(PairingEvent::PasskeyProgress {
                address: device_address,
                press: match press_type {
                    PairingPasskeyPressType::Initialization => PasskeyPress::Started,
                    PairingPasskeyPressType::Submit => PasskeyPress::Submitted,
                    _ => PasskeyPress::Key,
                },
            }),
            Event::PairingStatus {
                state,
                pairing_error,
                slot,
                ..
            } => match (state, pairing_error, slot) {
                (_, Some(err), _) => Some(PairingEvent::Failed {
                    reason: format!("{err:?}"),
                }),
                (PairingState::Paired, None, Some(slot)) => Some(PairingEvent::Paired { slot }),
                (PairingState::LockOpen, ..) => None,
                // Observed with an M650 when the final "both buttons" press was
                // reported as an 11th click instead of the submit press.
                _ => Some(PairingEvent::Failed {
                    reason: "the receiver aborted pairing (an extra or missing click?)".into(),
                }),
            },
            _ => None,
        };

        if let Some(out) = out
            && tx.send(out).await.is_err()
        {
            break;
        }
    }
}

fn complete_discovery(
    counter: u16,
    details: &HashMap<u16, (DeviceKind, u16, [u8; 6], u8)>,
    names: &HashMap<u16, String>,
    reported: &mut Vec<[u8; 6]>,
) -> Option<PairingEvent> {
    let (kind, wpid, address, authentication) = *details.get(&counter)?;
    let name = names.get(&counter)?.clone();
    if reported.contains(&address) {
        return None;
    }
    reported.push(address);

    Some(PairingEvent::Discovered(Discovered {
        address,
        kind,
        wpid,
        authentication,
        name,
    }))
}

/// Builds the prompt for a passkey. Mice receive the passkey as a decimal
/// number whose 10 lowest bits, most significant first, encode the clicks:
/// `1` is right, `0` is left.
fn passkey_prompt(passkey: &str, types: bool) -> Option<PasskeyPrompt> {
    if types {
        return Some(PasskeyPrompt::Type(passkey.to_string()));
    }

    let value: u32 = passkey.trim().parse().ok()?;
    let clicks = (0..10)
        .rev()
        .map(|bit| {
            if value >> bit & 1 == 1 {
                Click::Right
            } else {
                Click::Left
            }
        })
        .collect();
    Some(PasskeyPrompt::Click(clicks))
}

#[cfg(test)]
mod tests {
    use super::{Click::*, PasskeyPrompt, passkey_prompt};

    #[test]
    fn keyboard_types_the_passkey() {
        assert_eq!(
            passkey_prompt("123456", true),
            Some(PasskeyPrompt::Type("123456".into()))
        );
    }

    #[test]
    fn mouse_clicks_ten_bits_msb_first() {
        // 645 = 0b1010000101
        assert_eq!(
            passkey_prompt("000645", false),
            Some(PasskeyPrompt::Click(vec![
                Right, Left, Right, Left, Left, Left, Left, Right, Left, Right
            ]))
        );
    }

    #[test]
    fn mouse_ignores_bits_above_ten() {
        // 1024 + 1 only keeps the lowest bit set.
        let Some(PasskeyPrompt::Click(clicks)) = passkey_prompt("001025", false) else {
            panic!("expected clicks");
        };
        assert_eq!(clicks.iter().filter(|c| **c == Right).count(), 1);
        assert_eq!(clicks.last(), Some(&Right));
    }

    #[test]
    fn rejects_garbage_mouse_passkey() {
        assert_eq!(passkey_prompt("abc", false), None);
    }
}

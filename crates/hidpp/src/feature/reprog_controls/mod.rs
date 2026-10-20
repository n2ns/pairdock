//! Implements the `ReprogControlsV4` feature (ID `0x1b04`) that lists a
//! device's reprogrammable controls (buttons and keys) and allows diverting
//! them to software or remapping them to other controls.
//!
//! The byte layout follows Solaar (`KeysArrayV4`, `ReprogrammableKeyV4`).

use std::sync::Arc;

use crate::{
    channel::HidppChannel,
    event::EventEmitter,
    feature::{CreatableFeature, EmittingFeature, Feature},
    nibble::U4,
    protocol::v20::{self, Hidpp20Error},
};

/// Implements the `ReprogControlsV4` / `0x1b04` feature.
pub struct ReprogControlsFeature {
    /// The underlying HID++ channel.
    chan: Arc<HidppChannel>,

    /// The index of the device to implement the feature for.
    device_index: u8,

    /// The index of the feature in the feature table.
    feature_index: u8,

    /// The emitter used to emit events.
    emitter: Arc<EventEmitter<ReprogControlsEvent>>,

    /// The handle assigned to the message listener registered via
    /// [`HidppChannel::add_msg_listener`].
    /// This is used to remove the listener when the feature is dropped.
    msg_listener_hdl: u32,
}

impl CreatableFeature for ReprogControlsFeature {
    const ID: u16 = 0x1b04;
    const STARTING_VERSION: u8 = 0;

    fn new(chan: Arc<HidppChannel>, device_index: u8, feature_index: u8) -> Self {
        let emitter = Arc::new(EventEmitter::new());

        let hdl = chan.add_msg_listener({
            let emitter = Arc::clone(&emitter);

            move |raw, matched| {
                if matched {
                    return;
                }

                let msg = v20::Message::from(raw);
                let header = msg.header();
                if header.device_index != device_index
                    || header.feature_index != feature_index
                    || header.software_id.to_lo() != 0
                {
                    return;
                }

                let payload = msg.extend_payload();
                match header.function_id.to_lo() {
                    0 => {
                        let pressed = payload[..8]
                            .chunks_exact(2)
                            .map(|c| u16::from_be_bytes([c[0], c[1]]))
                            .filter(|&cid| cid != 0)
                            .collect();
                        emitter.emit(ReprogControlsEvent::DivertedButtons(pressed));
                    },
                    1 => {
                        emitter.emit(ReprogControlsEvent::DivertedRawXy {
                            dx: i16::from_be_bytes([payload[0], payload[1]]),
                            dy: i16::from_be_bytes([payload[2], payload[3]]),
                        });
                    },
                    _ => (),
                }
            }
        });

        Self {
            chan,
            device_index,
            feature_index,
            emitter,
            msg_listener_hdl: hdl,
        }
    }
}

impl Feature for ReprogControlsFeature {
}

impl EmittingFeature<ReprogControlsEvent> for ReprogControlsFeature {
    fn listen(&self) -> async_channel::Receiver<ReprogControlsEvent> {
        self.emitter.create_receiver()
    }
}

impl Drop for ReprogControlsFeature {
    fn drop(&mut self) {
        self.chan.remove_msg_listener(self.msg_listener_hdl);
    }
}

impl ReprogControlsFeature {
    fn header(&self, function: u8) -> v20::MessageHeader {
        v20::MessageHeader {
            device_index: self.device_index,
            feature_index: self.feature_index,
            function_id: U4::from_lo(function),
            software_id: self.chan.get_sw_id(),
        }
    }

    /// Retrieves the number of reprogrammable controls.
    pub async fn get_count(&self) -> Result<u8, Hidpp20Error> {
        let response = self
            .chan
            .send_v20(v20::Message::Short(self.header(0), [0x00, 0x00, 0x00]))
            .await?;

        Ok(response.extend_payload()[0])
    }

    /// Retrieves information about the control at `index` (`0..count`).
    pub async fn get_control_info(&self, index: u8) -> Result<ControlInfo, Hidpp20Error> {
        let response = self
            .chan
            .send_v20(v20::Message::Short(self.header(1), [index, 0x00, 0x00]))
            .await?;

        let p = response.extend_payload();
        Ok(ControlInfo {
            cid: u16::from_be_bytes([p[0], p[1]]),
            task_id: u16::from_be_bytes([p[2], p[3]]),
            flags: ControlFlags(u16::from(p[4]) | (u16::from(p[8]) << 8)),
            position: p[5],
            group: p[6],
            group_mask: p[7],
        })
    }

    /// Retrieves how the control `cid` currently reports presses.
    pub async fn get_reporting(&self, cid: u16) -> Result<ControlReporting, Hidpp20Error> {
        let [hi, lo] = cid.to_be_bytes();
        let response = self
            .chan
            .send_v20(v20::Message::Short(self.header(2), [hi, lo, 0x00]))
            .await?;

        let p = response.extend_payload();
        let mapped_to = u16::from_be_bytes([p[3], p[4]]);
        Ok(ControlReporting {
            flags: ReportingFlags(u16::from(p[2]) | (u16::from(p[5]) << 8)),
            mapped_to: if mapped_to == 0 { cid } else { mapped_to },
        })
    }

    /// Changes how the control `cid` reports presses.
    ///
    /// Every flag left as [`None`] keeps its current value. `remap` sends the
    /// control's presses as if `remap` had been pressed; it must be one of the
    /// controls allowed by the control's group mask.
    pub async fn set_reporting(
        &self,
        cid: u16,
        diverted: Option<bool>,
        persistently_diverted: Option<bool>,
        raw_xy_diverted: Option<bool>,
        remap: Option<u16>,
    ) -> Result<(), Hidpp20Error> {
        let mut flags = 0u8;
        for (flag, value) in [
            (ReportingFlags::DIVERTED, diverted),
            (ReportingFlags::PERSISTENTLY_DIVERTED, persistently_diverted),
            (ReportingFlags::RAW_XY_DIVERTED, raw_xy_diverted),
        ] {
            let Some(value) = value else { continue };
            let bit = flag as u8;
            // Every flag is followed by a "valid" bit telling the device to
            // apply it.
            flags |= bit << 1;
            if value {
                flags |= bit;
            }
        }

        let [cid_hi, cid_lo] = cid.to_be_bytes();
        let [remap_hi, remap_lo] = remap.unwrap_or(0).to_be_bytes();
        let mut payload = [0u8; 16];
        payload[..5].copy_from_slice(&[cid_hi, cid_lo, flags, remap_hi, remap_lo]);

        self.chan
            .send_v20(v20::Message::Long(self.header(3), payload))
            .await?;
        Ok(())
    }
}

/// Information about a reprogrammable control.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ControlInfo {
    /// The control ID. See Solaar's `special_keys.CONTROL` for known values.
    pub cid: u16,

    /// The task ID, i.e. the default action of the control.
    pub task_id: u16,

    pub flags: ControlFlags,

    /// The physical position on the device, or 0 if unknown.
    pub position: u8,

    /// The remapping group this control belongs to, or 0 for none.
    pub group: u8,

    /// The groups whose controls this control can be remapped to.
    pub group_mask: u8,
}

/// Capability flags of a control (bytes 4 and 8 of `getCidInfo`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ControlFlags(pub u16);

impl ControlFlags {
    pub const MOUSE: u16 = 0x01;
    pub const FKEY: u16 = 0x02;
    pub const HOTKEY: u16 = 0x04;
    pub const FN_TOGGLE: u16 = 0x08;
    pub const REPROGRAMMABLE: u16 = 0x10;
    pub const DIVERTABLE: u16 = 0x20;
    pub const PERSISTENTLY_DIVERTABLE: u16 = 0x40;
    pub const VIRTUAL: u16 = 0x80;
    pub const RAW_XY: u16 = 0x100;
    pub const FORCE_RAW_XY: u16 = 0x200;
    pub const ANALYTICS_KEY_EVENTS: u16 = 0x400;
    pub const RAW_WHEEL: u16 = 0x800;

    pub fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
}

/// The current reporting of a control.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ControlReporting {
    pub flags: ReportingFlags,

    /// The control whose action this control currently performs. Equals the
    /// control itself when it isn't remapped.
    pub mapped_to: u16,
}

/// Reporting flags of a control (bytes 2 and 5 of `getCidReporting`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ReportingFlags(pub u16);

impl ReportingFlags {
    pub const DIVERTED: u16 = 0x01;
    pub const PERSISTENTLY_DIVERTED: u16 = 0x04;
    pub const RAW_XY_DIVERTED: u16 = 0x10;

    pub fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
}

/// An event emitted by the [`ReprogControlsFeature`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[non_exhaustive]
pub enum ReprogControlsEvent {
    /// The diverted controls currently held down (up to 4). An empty list
    /// means all diverted controls were released.
    DivertedButtons(Vec<u16>),

    /// Mouse movement while a control with raw XY diversion is held.
    DivertedRawXy { dx: i16, dy: i16 },
}

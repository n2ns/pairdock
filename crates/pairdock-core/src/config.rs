//! Button assignments, stored per device model.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// All button assignments, keyed by the device's wireless product ID (WPID).
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub devices: BTreeMap<u16, DeviceConfig>,
}

/// Button assignments of one device model, keyed by control ID.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DeviceConfig {
    #[serde(default)]
    pub buttons: BTreeMap<u16, ButtonAction>,
}

/// What a button does.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ButtonAction {
    /// The button's factory behavior.
    #[default]
    Default,

    /// Perform another control's action. Stored on the device, which forgets
    /// it when it reconnects, so pairdock sets it again.
    Remap { to: u16 },

    /// Send a keyboard shortcut such as `Ctrl+C` while the button is held.
    /// Needs pairdock running.
    Shortcut { keys: String },

    /// Do nothing. Needs pairdock running.
    Disabled,
}

impl ButtonAction {
    /// Whether the button has to be diverted to pairdock for this action.
    pub fn needs_divert(&self) -> bool {
        matches!(self, Self::Shortcut { .. } | Self::Disabled)
    }
}

impl Config {
    /// The action configured for a control, or [`ButtonAction::Default`].
    pub fn action(&self, wpid: u16, cid: u16) -> ButtonAction {
        self.devices
            .get(&wpid)
            .and_then(|d| d.buttons.get(&cid))
            .cloned()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{ButtonAction, Config};

    #[test]
    fn round_trips_through_json() {
        let json = r#"{"devices":{"45098":{"buttons":{
            "83":{"type":"shortcut","keys":"Ctrl+C"},
            "86":{"type":"remap","to":82},
            "82":{"type":"disabled"}
        }}}}"#;
        let config: Config = serde_json::from_str(json).unwrap();

        assert_eq!(
            config.action(0xb02a, 0x53),
            ButtonAction::Shortcut {
                keys: "Ctrl+C".into()
            }
        );
        assert_eq!(config.action(0xb02a, 0x56), ButtonAction::Remap { to: 0x52 });
        assert_eq!(config.action(0xb02a, 0x52), ButtonAction::Disabled);
        assert_eq!(config.action(0xb02a, 0x50), ButtonAction::Default);
        assert_eq!(config.action(0x1234, 0x53), ButtonAction::Default);

        let again: Config = serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
        assert_eq!(again, config);
    }
}

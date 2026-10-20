//! Names of reprogrammable controls (HID++ `0x1b04` control IDs).
//!
//! Values come from Solaar's `special_keys.CONTROL` table; only the controls
//! found on mice are listed so far.

/// Returns a human-readable name for a control ID.
pub fn control_name(cid: u16) -> Option<&'static str> {
    Some(match cid {
        0x50 => "Left Button",
        0x51 => "Right Button",
        0x52 => "Middle Button",
        0x53 => "Back Button",
        0x54 => "Back",
        0x56 => "Forward Button",
        0x57 => "Forward",
        0x59 => "Button 6",
        0x5a => "Left Scroll (Button 7)",
        0x5b => "Left Tilt",
        0x5c => "Right Scroll (Button 8)",
        0x5d => "Right Tilt",
        0x5e => "Button 9",
        0x5f => "Button 10",
        0xc3 => "Gesture Button",
        0xc4 => "SmartShift",
        0xd7 => "Virtual Gesture Button",
        0xfd => "DPI Switch",
        _ => return None,
    })
}

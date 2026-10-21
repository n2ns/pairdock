//! Parsing keyboard shortcuts such as `Ctrl+Shift+T` and sending them with
//! `enigo`.

use enigo::Key;

/// A parsed shortcut: the keys to hold down, in order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shortcut(pub Vec<Key>);

/// Parses a shortcut like `Ctrl+Alt+Delete`, `Win+D` or `VolumeUp`.
///
/// Key names are case-insensitive. Letters and digits are written as is, and
/// on macOS also the brackets `[` `]` (for Safari's `Cmd+[` and `Cmd+]`).
pub fn parse_shortcut(text: &str) -> Result<Shortcut, String> {
    let keys = text
        .split('+')
        .map(|part| parse_key(part.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    if keys.is_empty() {
        return Err("empty shortcut".into());
    }
    Ok(Shortcut(keys))
}

fn parse_key(name: &str) -> Result<Key, String> {
    let lower = name.to_ascii_lowercase();

    let mut chars = lower.chars();
    if let (Some(c), None) = (chars.next(), chars.next())
        && (c.is_ascii_alphanumeric() || (cfg!(target_os = "macos") && matches!(c, '[' | ']')))
    {
        return Ok(char_key(c));
    }

    if let Some(n) = lower.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
        return function_key(n).ok_or_else(|| format!("unknown key: {name}"));
    }

    Ok(match lower.as_str() {
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "shift" => Key::Shift,
        "win" | "meta" | "cmd" | "super" => Key::Meta,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "tab" => Key::Tab,
        "enter" | "return" => Key::Return,
        "esc" | "escape" => Key::Escape,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "volumeup" => Key::VolumeUp,
        "volumedown" => Key::VolumeDown,
        "mute" | "volumemute" => Key::VolumeMute,
        "playpause" => Key::MediaPlayPause,
        "nexttrack" => Key::MediaNextTrack,
        "prevtrack" | "previoustrack" => Key::MediaPrevTrack,
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        "browserback" => Key::BrowserBack,
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        "browserforward" => Key::BrowserForward,
        _ => return Err(format!("unknown key: {name}")),
    })
}

/// Whether the system lets PairDock send key events. On macOS this needs the
/// Accessibility permission; elsewhere nothing has to be granted.
pub fn can_send_keys() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        unsafe extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }
        // SAFETY: takes no arguments and only reads the process's trust state.
        unsafe { AXIsProcessTrusted() }
    }
    #[cfg(not(target_os = "macos"))]
    true
}

/// Asks the system to let PairDock send key events. On macOS this adds the app
/// to the Accessibility list and shows the system's dialog that leads there.
pub fn request_send_keys() {
    #[cfg(target_os = "macos")]
    if !can_send_keys() {
        // Creating an `Enigo` with its default settings opens the prompt.
        let _ = enigo::Enigo::new(&enigo::Settings::default());
    }
}

/// The key for a lowercase ASCII letter or digit.
///
/// Windows looks the character up with `VkKeyScanExW`, whose result also
/// carries the shift state, which enigo does not strip; that is only safe for
/// characters typed without modifiers, so brackets are not offered here.
#[cfg(not(target_os = "macos"))]
fn char_key(c: char) -> Key {
    Key::Unicode(c)
}

/// The key for a lowercase ASCII letter, digit or bracket, as the virtual key
/// code of its position on a US keyboard (`kVK_ANSI_*` in HIToolbox's
/// Events.h).
///
/// `Key::Unicode` would look the key up in the current keyboard layout, and
/// the Text Input Sources API that does that must only be called on the main
/// thread (TextInputSources.h); macOS ends the process when keys are sent from
/// the injector thread.
#[cfg(target_os = "macos")]
fn char_key(c: char) -> Key {
    let code = match c {
        'a' => 0x00,
        's' => 0x01,
        'd' => 0x02,
        'f' => 0x03,
        'h' => 0x04,
        'g' => 0x05,
        'z' => 0x06,
        'x' => 0x07,
        'c' => 0x08,
        'v' => 0x09,
        'b' => 0x0b,
        'q' => 0x0c,
        'w' => 0x0d,
        'e' => 0x0e,
        'r' => 0x0f,
        'y' => 0x10,
        't' => 0x11,
        '1' => 0x12,
        '2' => 0x13,
        '3' => 0x14,
        '4' => 0x15,
        '6' => 0x16,
        '5' => 0x17,
        '9' => 0x19,
        '7' => 0x1a,
        '8' => 0x1c,
        '0' => 0x1d,
        'o' => 0x1f,
        'u' => 0x20,
        'i' => 0x22,
        'p' => 0x23,
        'l' => 0x25,
        'j' => 0x26,
        'k' => 0x28,
        'n' => 0x2d,
        'm' => 0x2e,
        ']' => 0x1e,
        '[' => 0x21,
        _ => unreachable!("only called with ASCII letters, digits and brackets"),
    };
    Key::Other(code)
}

fn function_key(n: u8) -> Option<Key> {
    Some(match n {
        1 => Key::F1,
        2 => Key::F2,
        3 => Key::F3,
        4 => Key::F4,
        5 => Key::F5,
        6 => Key::F6,
        7 => Key::F7,
        8 => Key::F8,
        9 => Key::F9,
        10 => Key::F10,
        11 => Key::F11,
        12 => Key::F12,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use enigo::Key;

    use super::{Shortcut, char_key, parse_shortcut};

    #[test]
    fn parses_modifier_combos() {
        assert_eq!(
            parse_shortcut("Ctrl+Shift+T"),
            Ok(Shortcut(vec![Key::Control, Key::Shift, char_key('t')]))
        );
    }

    #[test]
    fn is_case_insensitive_and_trims() {
        assert_eq!(
            parse_shortcut(" win + d "),
            Ok(Shortcut(vec![Key::Meta, char_key('d')]))
        );
    }

    #[test]
    fn parses_named_and_function_keys() {
        assert_eq!(parse_shortcut("VolumeUp"), Ok(Shortcut(vec![Key::VolumeUp])));
        assert_eq!(
            parse_shortcut("Alt+F4"),
            Ok(Shortcut(vec![Key::Alt, Key::F4]))
        );
    }

    /// Sending a letter through `Key::Unicode` looks the key up in the
    /// keyboard layout, which macOS only allows on the main thread; the
    /// injector thread would crash.
    #[cfg(target_os = "macos")]
    #[test]
    fn letters_and_digits_use_fixed_key_codes_on_macos() {
        assert_eq!(
            parse_shortcut("Cmd+V"),
            Ok(Shortcut(vec![Key::Meta, Key::Other(0x09)]))
        );
        assert_eq!(
            parse_shortcut("Cmd+Shift+4"),
            Ok(Shortcut(vec![Key::Meta, Key::Shift, Key::Other(0x15)]))
        );
        assert_eq!(
            parse_shortcut("Cmd+["),
            Ok(Shortcut(vec![Key::Meta, Key::Other(0x21)]))
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn rejects_brackets_outside_macos() {
        assert!(parse_shortcut("Ctrl+[").is_err());
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(parse_shortcut("Ctrl+Banana").is_err());
        assert!(parse_shortcut("F99").is_err());
        assert!(parse_shortcut("Ctrl+").is_err());
    }
}

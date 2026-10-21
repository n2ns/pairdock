//! Parsing keyboard shortcuts such as `Ctrl+Shift+T` and sending them with
//! `enigo`.

use enigo::Key;

/// A parsed shortcut: the keys to hold down, in order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shortcut(pub Vec<Key>);

/// Parses a shortcut like `Ctrl+Alt+Delete`, `Win+D` or `VolumeUp`.
///
/// Key names are case-insensitive. Letters and digits are written as is.
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
        && c.is_ascii_alphanumeric()
    {
        return Ok(Key::Unicode(c));
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

    use super::{Shortcut, parse_shortcut};

    #[test]
    fn parses_modifier_combos() {
        assert_eq!(
            parse_shortcut("Ctrl+Shift+T"),
            Ok(Shortcut(vec![Key::Control, Key::Shift, Key::Unicode('t')]))
        );
    }

    #[test]
    fn is_case_insensitive_and_trims() {
        assert_eq!(
            parse_shortcut(" win + d "),
            Ok(Shortcut(vec![Key::Meta, Key::Unicode('d')]))
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

    #[test]
    fn rejects_unknown_keys() {
        assert!(parse_shortcut("Ctrl+Banana").is_err());
        assert!(parse_shortcut("F99").is_err());
        assert!(parse_shortcut("Ctrl+").is_err());
    }
}

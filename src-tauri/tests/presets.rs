//! Checks the common shortcuts offered in the UI (src/presets.json): every
//! one of the current platform must parse and have a label in src/i18n.ts.

use pairdock_core::keys::parse_shortcut;
use serde_json::Value;

const PRESETS: &str = include_str!("../../src/presets.json");
const STRINGS: &str = include_str!("../../src/i18n.ts");

fn presets(platform: &str) -> Vec<(String, String)> {
    let all: Value = serde_json::from_str(PRESETS).unwrap();
    [platform, "all"]
        .iter()
        .flat_map(|list| all[list].as_array().unwrap().clone())
        .map(|p| {
            (
                p["label"].as_str().unwrap().to_owned(),
                p["keys"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn check(platform: &str) {
    let presets = presets(platform);
    assert!(!presets.is_empty());
    for (label, keys) in presets {
        assert!(
            parse_shortcut(&keys).is_ok(),
            "{platform} preset {label} ({keys}) does not parse"
        );
        assert!(
            STRINGS.contains(&format!("\n  {label}:")),
            "{platform} preset {label} has no text in src/i18n.ts"
        );
    }
}

#[cfg(target_os = "windows")]
#[test]
fn windows_presets_parse_and_have_labels() {
    check("windows");
}

#[cfg(target_os = "macos")]
#[test]
fn macos_presets_parse_and_have_labels() {
    check("macos");
}

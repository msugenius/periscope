use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use crosshair_core::{CrosshairSettings, Preset, PresetLibrary, VisualSettings};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(flatten)]
    pub crosshair: CrosshairSettings,
    #[serde(default)]
    pub hotkeys: HotkeySettings,
    #[serde(default)]
    pub hide_when_ads: bool,
    #[serde(flatten)]
    pub library: PresetLibrary,
}

impl AppSettings {
    pub fn validated(mut self) -> Self {
        self.crosshair = self.crosshair.validated();
        self.library = self.library.validated();
        self.crosshair = self
            .library
            .active()
            .settings
            .crosshair(self.crosshair.enabled);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "camelCase")]
enum LegacyPresetId {
    #[default]
    Classic,
    Compact,
    Dot,
    Open,
    Precision,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyAppSettings {
    #[serde(flatten)]
    crosshair: CrosshairSettings,
    #[serde(default)]
    hotkeys: HotkeySettings,
    #[serde(default)]
    hide_when_ads: bool,
    #[serde(default)]
    active_preset: LegacyPresetId,
    #[serde(default)]
    presets: BTreeMap<LegacyPresetId, CrosshairSettings>,
}

impl LegacyAppSettings {
    pub fn migrate(self) -> AppSettings {
        let mut library = PresetLibrary {
            active_preset: match self.active_preset {
                LegacyPresetId::Dot => "dot",
                LegacyPresetId::Precision => "precision",
                _ => "classic",
            }
            .into(),
            ..PresetLibrary::default()
        };
        for preset in &mut library.presets {
            let legacy_id = match preset.id.as_str() {
                "dot" => LegacyPresetId::Dot,
                "precision" => LegacyPresetId::Precision,
                _ => LegacyPresetId::Classic,
            };
            if let Some(legacy) = self.presets.get(&legacy_id) {
                preset.settings = legacy.clone().validated().visual;
            }
            preset
                .settings
                .color
                .clone_from(&self.crosshair.visual.color);
            preset.settings.opacity = self.crosshair.visual.opacity;
            preset.settings.outline = self.crosshair.visual.outline;
            preset.settings.outline_thickness = self.crosshair.visual.outline_thickness;
            preset
                .settings
                .outline_color
                .clone_from(&self.crosshair.visual.outline_color);
            preset.settings.x_offset = self.crosshair.visual.x_offset;
            preset.settings.y_offset = self.crosshair.visual.y_offset;
        }
        library.save_active(self.crosshair.clone().validated().visual);
        if let Some(dot) = library.presets.iter_mut().find(|preset| preset.id == "dot") {
            if dot.settings.length == 1 && dot.settings.gap == 0 {
                dot.settings.length = 10;
                dot.settings.gap = 3;
            }
            dot.settings.dot_only = true;
            dot.settings.center_dot = true;
            dot.settings.t_style = false;
        }
        AppSettings {
            crosshair: self.crosshair,
            hotkeys: self.hotkeys,
            hide_when_ads: self.hide_when_ads,
            library,
        }
        .validated()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    #[serde(default = "default_toggle_crosshair_hotkey")]
    pub toggle_crosshair: String,
    #[serde(default = "default_toggle_ads_hotkey")]
    pub toggle_ads: String,
}

fn default_toggle_crosshair_hotkey() -> String {
    "F2".into()
}

fn default_toggle_ads_hotkey() -> String {
    "F5".into()
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            toggle_crosshair: default_toggle_crosshair_hotkey(),
            toggle_ads: default_toggle_ads_hotkey(),
        }
    }
}

impl HotkeySettings {
    pub fn clear_reserved_bindings(&mut self) -> Vec<(&'static str, String)> {
        let mut cleared = Vec::new();
        for (field, binding) in [
            ("toggleCrosshair", &mut self.toggle_crosshair),
            ("toggleAds", &mut self.toggle_ads),
        ] {
            if reserved_shortcut_reason(binding).is_some() {
                cleared.push((field, std::mem::take(binding)));
            }
        }
        cleared
    }

    pub fn validated(self) -> Result<Self, String> {
        let toggle_crosshair = canonical_optional_shortcut(&self.toggle_crosshair)
            .map_err(|error| format!("Toggle crosshair shortcut {error}"))?;
        let toggle_ads = canonical_optional_shortcut(&self.toggle_ads)
            .map_err(|error| format!("Hide when ADS shortcut {error}"))?;

        if !toggle_crosshair.is_empty() && toggle_crosshair.eq_ignore_ascii_case(&toggle_ads) {
            return Err("Hotkey actions cannot use the same shortcut.".into());
        }

        Ok(Self {
            toggle_crosshair,
            toggle_ads,
        })
    }
}

fn canonical_optional_shortcut(value: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Ok(String::new())
    } else {
        canonical_shortcut(value)
    }
}

fn canonical_shortcut(value: &str) -> Result<String, String> {
    if let Some(reason) = reserved_shortcut_reason(value) {
        return Err(reason.into());
    }
    let tokens = value.split('+').map(str::trim).collect::<Vec<_>>();
    if tokens.is_empty() || tokens.iter().any(|token| token.is_empty()) {
        return Err("is empty or incomplete.".into());
    }

    let (key, modifiers) = tokens
        .split_last()
        .ok_or_else(|| "is empty or incomplete.".to_string())?;
    if modifier_name(key).is_some() {
        return Err("must include one non-modifier key.".into());
    }

    let mut control = false;
    let mut alt = false;
    let mut shift = false;
    let mut super_key = false;
    for token in modifiers {
        let Some(modifier) = modifier_name(token) else {
            return Err(format!("contains unsupported modifier '{token}'."));
        };
        let slot = match modifier {
            "Control" => &mut control,
            "Alt" => &mut alt,
            "Shift" => &mut shift,
            "Super" => &mut super_key,
            _ => unreachable!(),
        };
        if *slot {
            return Err(format!("contains duplicate modifier '{modifier}'."));
        }
        *slot = true;
    }

    let key = canonical_key(key)?;
    let mut canonical = Vec::with_capacity(modifiers.len() + 1);
    if control {
        canonical.push("Control".to_string());
    }
    if alt {
        canonical.push("Alt".to_string());
    }
    if shift {
        canonical.push("Shift".to_string());
    }
    if super_key {
        canonical.push("Super".to_string());
    }
    canonical.push(key);
    Ok(canonical.join("+"))
}

fn reserved_shortcut_reason(value: &str) -> Option<&'static str> {
    let tokens = value.split('+').map(str::trim).collect::<Vec<_>>();
    let (key, modifiers) = tokens.split_last()?;
    let key = key.to_ascii_uppercase();
    let has = |name| {
        modifiers
            .iter()
            .any(|token| modifier_name(token) == Some(name))
    };
    if has("Control") && has("Alt") && key == "DELETE" {
        return Some("cannot observe the secure attention shortcut Ctrl+Alt+Delete.");
    }
    None
}

fn modifier_name(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_uppercase().as_str() {
        "CTRL" | "CONTROL" | "CMDORCTRL" | "COMMANDORCONTROL" => Some("Control"),
        "ALT" | "OPTION" => Some("Alt"),
        "SHIFT" => Some("Shift"),
        "SUPER" | "META" | "WIN" | "WINDOWS" | "COMMAND" | "CMD" => Some("Super"),
        _ => None,
    }
}

fn canonical_key(value: &str) -> Result<String, String> {
    let value = value.trim();
    let upper = value.to_ascii_uppercase();

    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_alphabetic() {
            return Ok(format!("Key{}", byte as char));
        }
        if byte.is_ascii_digit() {
            return Ok(format!("Digit{}", byte as char));
        }
    }

    if let Some(number) = upper
        .strip_prefix('F')
        .and_then(|number| number.parse::<u8>().ok())
        && (1..=24).contains(&number)
    {
        return Ok(format!("F{number}"));
    }

    if upper.starts_with("KEY") && upper.len() == 4 && upper.as_bytes()[3].is_ascii_alphabetic() {
        return Ok(format!("Key{}", upper.as_bytes()[3] as char));
    }
    if upper.starts_with("DIGIT") && upper.len() == 6 && upper.as_bytes()[5].is_ascii_digit() {
        return Ok(format!("Digit{}", upper.as_bytes()[5] as char));
    }
    if upper.starts_with("NUMPAD") && upper.len() == 7 && upper.as_bytes()[6].is_ascii_digit() {
        return Ok(format!("Numpad{}", upper.as_bytes()[6] as char));
    }

    let named = match upper.as_str() {
        "ARROWUP" => "ArrowUp",
        "ARROWDOWN" => "ArrowDown",
        "ARROWLEFT" => "ArrowLeft",
        "ARROWRIGHT" => "ArrowRight",
        "BACKSPACE" => "Backspace",
        "DELETE" => "Delete",
        "END" => "End",
        "ENTER" => "Enter",
        "HOME" => "Home",
        "INSERT" => "Insert",
        "PAGEDOWN" => "PageDown",
        "PAGEUP" => "PageUp",
        "SPACE" => "Space",
        "TAB" => "Tab",
        "CAPSLOCK" => "CapsLock",
        "NUMLOCK" => "NumLock",
        "SCROLLLOCK" => "ScrollLock",
        _ => return Err(format!("contains unsupported key '{value}'.")),
    };
    Ok(named.into())
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, HotkeySettings, LegacyAppSettings};

    #[test]
    fn legacy_presets_migrate_without_losing_the_active_shape() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        value["length"] = 23.into();
        value["activePreset"] = "compact".into();
        value["presets"] = serde_json::json!({});
        let old: LegacyAppSettings = serde_json::from_value(value).unwrap();
        let migrated = old.migrate();
        assert_eq!(migrated.library.active_preset, "classic");
        assert_eq!(migrated.crosshair.visual.length, 23);
        assert_eq!(migrated.library.presets.len(), 3);
        assert_eq!(migrated.hotkeys, HotkeySettings::default());
    }

    #[test]
    fn migration_keeps_each_shape_and_converts_shared_visual_fields() {
        let mut value = serde_json::to_value(AppSettings::default()).unwrap();
        let mut classic = serde_json::to_value(AppSettings::default().crosshair).unwrap();
        classic["length"] = 22.into();
        let mut precision = serde_json::to_value(AppSettings::default().crosshair).unwrap();
        precision["length"] = 18.into();
        let mut dot = serde_json::to_value(AppSettings::default().crosshair).unwrap();
        dot["length"] = 1.into();
        value["activePreset"] = "dot".into();
        value["length"] = 1.into();
        value["dotSize"] = 9.into();
        value["color"] = "#AA44CC".into();
        value["opacity"] = 63.into();
        value["presets"] = serde_json::json!({
            "classic": classic,
            "dot": dot,
            "precision": precision,
        });
        let legacy: LegacyAppSettings = serde_json::from_value(value).unwrap();
        let migrated = legacy.migrate();
        assert_eq!(migrated.library.active().settings.dot_size, 9);
        assert_eq!(
            migrated.library.find("classic").unwrap().settings.length,
            22
        );
        assert_eq!(
            migrated.library.find("precision").unwrap().settings.length,
            18
        );
        for preset in &migrated.library.presets {
            assert_eq!(preset.settings.color, "#AA44CC");
            assert_eq!(preset.settings.opacity, 63);
        }
    }

    #[test]
    fn hotkey_validation_canonicalizes_and_rejects_duplicates() {
        let settings = HotkeySettings {
            toggle_crosshair: "ctrl + shift + f2".into(),
            toggle_ads: "f5".into(),
        }
        .validated()
        .unwrap();
        assert_eq!(settings.toggle_crosshair, "Control+Shift+F2");
        assert_eq!(settings.toggle_ads, "F5");

        let duplicate = HotkeySettings {
            toggle_crosshair: "CTRL+F4".into(),
            toggle_ads: "Control+F4".into(),
        }
        .validated();
        assert!(duplicate.is_err());

        let ads_duplicate = HotkeySettings {
            toggle_ads: "F2".into(),
            ..HotkeySettings::default()
        }
        .validated();
        assert!(ads_duplicate.is_err());
    }

    #[test]
    fn hotkey_validation_handles_supported_keys_and_incomplete_shortcuts() {
        for (input, expected) in [
            ("alt+1", "Alt+Digit1"),
            ("alt+numpad7", "Alt+Numpad7"),
            ("ArrowUp", "ArrowUp"),
            ("pageDown", "PageDown"),
        ] {
            let settings = HotkeySettings {
                toggle_ads: input.into(),
                ..HotkeySettings::default()
            }
            .validated()
            .unwrap();
            assert_eq!(settings.toggle_ads, expected);
        }

        for invalid in ["Control+", "Control", "Hyper+F3", "F25"] {
            assert!(
                HotkeySettings {
                    toggle_ads: invalid.into(),
                    ..HotkeySettings::default()
                }
                .validated()
                .is_err()
            );
        }

        let unset = HotkeySettings {
            toggle_ads: "  ".into(),
            ..HotkeySettings::default()
        }
        .validated()
        .unwrap();
        assert!(unset.toggle_ads.is_empty());
    }

    #[test]
    fn passive_shortcuts_allow_system_and_lock_keys_except_secure_attention() {
        for binding in [
            "CapsLock",
            "Control+CapsLock",
            "NumLock",
            "ScrollLock",
            "Super+KeyR",
            "Alt+Tab",
            "Alt+F4",
        ] {
            assert!(
                HotkeySettings {
                    toggle_ads: binding.into(),
                    ..HotkeySettings::default()
                }
                .validated()
                .is_ok(),
                "{binding}"
            );
        }

        assert!(
            HotkeySettings {
                toggle_ads: "Control+Alt+Delete".into(),
                ..HotkeySettings::default()
            }
            .validated()
            .is_err()
        );

        let mut saved = HotkeySettings {
            toggle_ads: "Control+Alt+Delete".into(),
            toggle_crosshair: "Control+F2".into(),
        };
        assert_eq!(
            saved.clear_reserved_bindings(),
            vec![("toggleAds", "Control+Alt+Delete".into())]
        );
        assert!(saved.toggle_ads.is_empty());
        assert_eq!(saved.toggle_crosshair, "Control+F2");
        assert!(saved.validated().is_ok());
    }

    #[test]
    fn legacy_hotkey_settings_ignore_removed_actions() {
        let settings: HotkeySettings =
            serde_json::from_str(r#"{"closeApp":"F3","showSettings":"F4"}"#).unwrap();

        assert_eq!(settings.toggle_crosshair, "F2");
        assert_eq!(settings.toggle_ads, "F5");
        let serialized = serde_json::to_value(settings).unwrap();
        assert!(serialized.get("closeApp").is_none());
        assert!(serialized.get("showSettings").is_none());
    }

    #[test]
    fn hide_when_ads_is_independent_of_presets_and_hotkeys() {
        let mut settings = AppSettings {
            hide_when_ads: true,
            ..AppSettings::default()
        };
        settings.library.active_preset = "dot".into();
        settings.hotkeys = HotkeySettings::default();
        let restored: AppSettings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(restored.validated().hide_when_ads);
    }
}

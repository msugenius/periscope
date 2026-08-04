use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(flatten)]
    pub crosshair: CrosshairSettings,
    #[serde(default)]
    pub hotkeys: HotkeySettings,
    #[serde(default)]
    pub active_preset: PresetId,
    #[serde(default)]
    pub presets: BTreeMap<PresetId, CrosshairSettings>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            crosshair: CrosshairSettings::default(),
            hotkeys: HotkeySettings::default(),
            active_preset: PresetId::default(),
            presets: default_presets(),
        }
    }
}

impl AppSettings {
    pub fn validated(mut self) -> Self {
        self.crosshair = self.crosshair.validated();
        self.presets = self
            .presets
            .into_iter()
            .map(|(preset, settings)| (preset, settings.validated()))
            .collect();
        if !self.active_preset.is_available() {
            self.active_preset = PresetId::Classic;
        }
        self.presets.retain(|preset, _| preset.is_available());
        for preset in PresetId::AVAILABLE {
            self.presets
                .entry(preset)
                .or_insert_with(|| preset.default_settings());
        }
        self.presets
            .insert(self.active_preset, self.crosshair.clone());
        for settings in self.presets.values_mut() {
            settings.inherit_shared_settings(&self.crosshair);
        }
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresetId {
    #[default]
    Classic,
    Compact,
    Dot,
    Open,
    Precision,
}

impl PresetId {
    pub const AVAILABLE: [Self; 3] = [Self::Classic, Self::Dot, Self::Precision];

    pub fn is_available(self) -> bool {
        Self::AVAILABLE.contains(&self)
    }

    pub fn default_settings(self) -> CrosshairSettings {
        let mut settings = CrosshairSettings::default();
        match self {
            Self::Classic => {}
            Self::Compact => {
                settings.length = 5;
                settings.thickness = 2;
                settings.gap = 2;
                settings.center_dot = false;
            }
            Self::Dot => {
                settings.length = 1;
                settings.thickness = 1;
                settings.gap = 0;
                settings.center_dot = true;
                settings.dot_size = 3;
            }
            Self::Open => {
                settings.length = 8;
                settings.thickness = 1;
                settings.gap = 6;
                settings.center_dot = false;
            }
            Self::Precision => {
                settings.length = 14;
                settings.thickness = 1;
                settings.gap = 2;
                settings.center_dot = true;
                settings.dot_size = 1;
                settings.t_style = true;
            }
        }
        settings
    }
}

fn default_presets() -> BTreeMap<PresetId, CrosshairSettings> {
    PresetId::AVAILABLE
        .into_iter()
        .map(|preset| (preset, preset.default_settings()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    pub close_app: String,
    pub show_settings: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            close_app: "F3".into(),
            show_settings: "F4".into(),
        }
    }
}

impl HotkeySettings {
    pub fn validated(self) -> Result<Self, String> {
        let close_app = canonical_shortcut(&self.close_app)
            .map_err(|error| format!("Close app shortcut {error}"))?;
        let show_settings = canonical_shortcut(&self.show_settings)
            .map_err(|error| format!("Show settings shortcut {error}"))?;

        if close_app.eq_ignore_ascii_case(&show_settings) {
            return Err("Close app and Show settings cannot use the same shortcut.".into());
        }

        Ok(Self {
            close_app,
            show_settings,
        })
    }
}

fn canonical_shortcut(value: &str) -> Result<String, String> {
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
        "CAPSLOCK" => "CapsLock",
        "DELETE" => "Delete",
        "END" => "End",
        "ENTER" => "Enter",
        "HOME" => "Home",
        "INSERT" => "Insert",
        "PAGEDOWN" => "PageDown",
        "PAGEUP" => "PageUp",
        "SPACE" => "Space",
        "TAB" => "Tab",
        _ => return Err(format!("contains unsupported key '{value}'.")),
    };
    Ok(named.into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrosshairSettings {
    pub enabled: bool,
    pub color: String,
    pub opacity: u8,
    pub length: i32,
    pub thickness: i32,
    pub gap: i32,
    pub center_dot: bool,
    pub dot_size: i32,
    pub t_style: bool,
    pub outline: bool,
    pub outline_thickness: i32,
    pub outline_color: String,
    pub x_offset: i32,
    pub y_offset: i32,
}

impl Default for CrosshairSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "#35E8FF".into(),
            opacity: 100,
            length: 10,
            thickness: 1,
            gap: 3,
            center_dot: true,
            dot_size: 2,
            t_style: false,
            outline: true,
            outline_thickness: 1,
            outline_color: "#000000".into(),
            x_offset: 0,
            y_offset: 0,
        }
    }
}

impl CrosshairSettings {
    pub fn inherit_shared_settings(&mut self, current: &Self) {
        self.enabled = current.enabled;
        self.color.clone_from(&current.color);
        self.opacity = current.opacity;
        self.outline = current.outline;
        self.outline_thickness = current.outline_thickness;
        self.outline_color.clone_from(&current.outline_color);
        self.x_offset = current.x_offset;
        self.y_offset = current.y_offset;
    }

    pub fn validated(mut self) -> Self {
        self.opacity = self.opacity.clamp(5, 100);
        self.length = self.length.clamp(1, 64);
        self.thickness = self.thickness.clamp(1, 16);
        self.gap = self.gap.clamp(0, 32);
        self.dot_size = self.dot_size.clamp(1, 16);
        self.outline_thickness = self.outline_thickness.clamp(1, 8);
        self.x_offset = self.x_offset.clamp(-200, 200);
        self.y_offset = self.y_offset.clamp(-200, 200);
        if !is_hex_color(&self.color) {
            self.color = "#35E8FF".into();
        }
        if !is_hex_color(&self.outline_color) {
            self.outline_color = "#000000".into();
        }
        self.color.make_ascii_uppercase();
        self.outline_color.make_ascii_uppercase();
        self
    }
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, CrosshairSettings, HotkeySettings, PresetId};

    #[test]
    fn validation_clamps_geometry_and_normalizes_colors() {
        let settings = CrosshairSettings {
            opacity: 0,
            length: 500,
            thickness: -2,
            gap: -7,
            x_offset: 999,
            y_offset: -999,
            color: "#aa11ff".into(),
            outline_color: "invalid".into(),
            ..CrosshairSettings::default()
        }
        .validated();

        assert_eq!(settings.opacity, 5);
        assert_eq!(settings.length, 64);
        assert_eq!(settings.thickness, 1);
        assert_eq!(settings.gap, 0);
        assert_eq!(settings.x_offset, 200);
        assert_eq!(settings.y_offset, -200);
        assert_eq!(settings.color, "#AA11FF");
        assert_eq!(settings.outline_color, "#000000");
    }

    #[test]
    fn missing_hotkeys_use_defaults_for_existing_settings_files() {
        let json = r##"{
            "enabled": true,
            "color": "#35E8FF",
            "opacity": 100,
            "length": 20,
            "thickness": 2,
            "gap": 6,
            "centerDot": true,
            "dotSize": 3,
            "tStyle": false,
            "outline": true,
            "outlineThickness": 1,
            "outlineColor": "#000000",
            "xOffset": 0,
            "yOffset": 0
        }"##;

        let settings: AppSettings = serde_json::from_str(json).unwrap();
        let settings = settings.validated();
        assert_eq!(settings.hotkeys, HotkeySettings::default());
        assert_eq!(settings.active_preset, PresetId::Classic);
        assert_eq!(settings.presets[&PresetId::Classic].length, 20);
        assert_eq!(settings.presets[&PresetId::Dot].dot_size, 3);
    }

    #[test]
    fn preset_snapshots_round_trip_independently() {
        let mut settings = AppSettings {
            active_preset: PresetId::Dot,
            crosshair: PresetId::Dot.default_settings(),
            ..AppSettings::default()
        };
        settings.crosshair.dot_size = 9;
        settings.crosshair.color = "#aa44cc".into();
        settings.crosshair.opacity = 63;
        settings.crosshair.outline = false;
        settings.crosshair.outline_thickness = 4;
        settings.crosshair.outline_color = "#112233".into();
        let settings = settings.validated();

        assert_eq!(settings.presets[&PresetId::Dot].dot_size, 9);
        assert_eq!(settings.presets[&PresetId::Precision].length, 14);
        for preset in PresetId::AVAILABLE {
            let snapshot = &settings.presets[&preset];
            assert_eq!(snapshot.color, "#AA44CC");
            assert_eq!(snapshot.opacity, 63);
            assert!(!snapshot.outline);
            assert_eq!(snapshot.outline_thickness, 4);
            assert_eq!(snapshot.outline_color, "#112233");
        }

        let json = serde_json::to_string(&settings).unwrap();
        let restored: AppSettings = serde_json::from_str(&json).unwrap();
        let restored = restored.validated();
        assert_eq!(restored.active_preset, PresetId::Dot);
        assert_eq!(restored.presets[&PresetId::Dot].dot_size, 9);
        assert_eq!(restored.presets[&PresetId::Classic].dot_size, 2);
        assert_eq!(restored.presets[&PresetId::Precision].color, "#AA44CC");
    }

    #[test]
    fn legacy_presets_migrate_to_classic_without_losing_the_active_shape() {
        let mut legacy_shape = PresetId::Compact.default_settings();
        legacy_shape.length = 23;
        let settings = AppSettings {
            active_preset: PresetId::Compact,
            crosshair: legacy_shape,
            ..AppSettings::default()
        }
        .validated();

        assert_eq!(settings.active_preset, PresetId::Classic);
        assert_eq!(settings.crosshair.length, 23);
        assert_eq!(settings.presets[&PresetId::Classic].length, 23);
        assert_eq!(settings.presets.len(), 3);
        assert!(!settings.presets.contains_key(&PresetId::Compact));
        assert!(!settings.presets.contains_key(&PresetId::Open));
    }

    #[test]
    fn hotkey_validation_canonicalizes_and_rejects_duplicates() {
        let settings = HotkeySettings {
            close_app: "ctrl + shift + f3".into(),
            show_settings: "f4".into(),
        }
        .validated()
        .unwrap();
        assert_eq!(settings.close_app, "Control+Shift+F3");
        assert_eq!(settings.show_settings, "F4");

        let duplicate = HotkeySettings {
            close_app: "CTRL+F4".into(),
            show_settings: "Control+F4".into(),
        }
        .validated();
        assert!(duplicate.is_err());
    }

    #[test]
    fn hotkey_validation_handles_supported_keys_and_incomplete_shortcuts() {
        for (input, expected) in [
            ("alt+1", "Alt+Digit1"),
            ("super+numpad7", "Super+Numpad7"),
            ("ArrowUp", "ArrowUp"),
            ("pageDown", "PageDown"),
        ] {
            let settings = HotkeySettings {
                close_app: input.into(),
                show_settings: "F4".into(),
            }
            .validated()
            .unwrap();
            assert_eq!(settings.close_app, expected);
        }

        for invalid in ["", "Control+", "Control", "Hyper+F3", "F25"] {
            assert!(
                HotkeySettings {
                    close_app: invalid.into(),
                    show_settings: "F4".into(),
                }
                .validated()
                .is_err()
            );
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(flatten)]
    pub crosshair: CrosshairSettings,
    #[serde(default)]
    pub hotkeys: HotkeySettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            crosshair: CrosshairSettings::default(),
            hotkeys: HotkeySettings::default(),
        }
    }
}

impl AppSettings {
    pub fn validated(mut self) -> Self {
        self.crosshair = self.crosshair.validated();
        self
    }
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
    {
        if (1..=24).contains(&number) {
            return Ok(format!("F{number}"));
        }
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
            length: 20,
            thickness: 2,
            gap: 6,
            center_dot: true,
            dot_size: 3,
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
    use super::{AppSettings, CrosshairSettings, HotkeySettings};

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
        assert_eq!(settings.hotkeys, HotkeySettings::default());
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
}

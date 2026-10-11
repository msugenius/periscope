mod rasterizer;

pub use rasterizer::{OVERLAY_SIZE, rasterize};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, hash_map::RandomState},
    hash::BuildHasher,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrosshairSettings {
    pub enabled: bool,
    #[serde(flatten)]
    pub visual: VisualSettings,
}

impl Default for CrosshairSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            visual: VisualSettings::default(),
        }
    }
}

impl CrosshairSettings {
    pub fn validated(mut self) -> Self {
        self.visual = self.visual.validated();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSettings {
    pub color: String,
    pub opacity: u8,
    pub length: i32,
    pub thickness: i32,
    pub gap: i32,
    pub center_dot: bool,
    pub dot_size: i32,
    pub t_style: bool,
    #[serde(default)]
    pub dot_only: bool,
    pub outline: bool,
    pub outline_thickness: i32,
    pub outline_color: String,
    pub x_offset: i32,
    pub y_offset: i32,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self {
            color: "#35E8FF".into(),
            opacity: 100,
            length: 10,
            thickness: 1,
            gap: 3,
            center_dot: true,
            dot_size: 2,
            t_style: false,
            dot_only: false,
            outline: true,
            outline_thickness: 1,
            outline_color: "#000000".into(),
            x_offset: 0,
            y_offset: 0,
        }
    }
}

impl VisualSettings {
    pub fn validated(mut self) -> Self {
        self.opacity = self.opacity.clamp(5, 100);
        self.length = self.length.clamp(1, 64);
        self.thickness = self.thickness.clamp(1, 16);
        self.gap = self.gap.clamp(0, 32);
        self.dot_size = self.dot_size.clamp(1, 16);
        self.outline_thickness = self.outline_thickness.clamp(1, 8);
        self.x_offset = self.x_offset.clamp(-200, 200);
        self.y_offset = self.y_offset.clamp(-200, 200);
        if self.dot_only {
            self.center_dot = true;
            self.t_style = false;
        }
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

    pub fn crosshair(&self, enabled: bool) -> CrosshairSettings {
        CrosshairSettings {
            enabled,
            visual: self.clone(),
        }
    }
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub settings: VisualSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetLibrary {
    pub active_preset: String,
    pub presets: Vec<Preset>,
}

impl Default for PresetLibrary {
    fn default() -> Self {
        let dot = VisualSettings {
            dot_size: 3,
            dot_only: true,
            ..VisualSettings::default()
        };
        let t_shape = VisualSettings {
            length: 14,
            gap: 2,
            dot_size: 1,
            t_style: true,
            ..VisualSettings::default()
        };
        Self {
            active_preset: "classic".into(),
            presets: vec![
                Preset {
                    id: "dot".into(),
                    name: "Dot".into(),
                    settings: dot,
                },
                Preset {
                    id: "classic".into(),
                    name: "Classic".into(),
                    settings: VisualSettings::default(),
                },
                Preset {
                    id: "precision".into(),
                    name: "T-Shape".into(),
                    settings: t_shape,
                },
            ],
        }
    }
}

impl PresetLibrary {
    pub fn validated(mut self) -> Self {
        let mut ids = HashSet::new();
        self.presets
            .retain(|preset| !preset.id.is_empty() && ids.insert(preset.id.clone()));
        self.presets.truncate(256);
        if self.presets.is_empty() {
            return Self::default();
        }
        for preset in &mut self.presets {
            preset.settings = preset.settings.clone().validated();
            preset.name = normalized_name(&preset.name).unwrap_or_else(|_| "Unnamed preset".into());
        }
        if !self
            .presets
            .iter()
            .any(|preset| preset.id == self.active_preset)
        {
            self.active_preset = self.presets[0].id.clone();
        }
        self
    }

    pub fn active(&self) -> &Preset {
        self.find(&self.active_preset)
            .expect("validated library has an active preset")
    }

    pub fn find(&self, id: &str) -> Option<&Preset> {
        self.presets.iter().find(|preset| preset.id == id)
    }

    pub fn select(&mut self, id: &str) -> Result<&Preset, String> {
        if self.find(id).is_none() {
            return Err("Preset does not exist.".into());
        }
        self.active_preset = id.into();
        Ok(self.active())
    }

    pub fn save_active(&mut self, settings: VisualSettings) {
        let id = self.active_preset.clone();
        self.presets
            .iter_mut()
            .find(|preset| preset.id == id)
            .expect("active preset exists")
            .settings = settings.validated();
    }

    pub fn create(&mut self, name: Option<&str>) -> Result<String, String> {
        self.add(name, VisualSettings::default())
    }

    pub fn clone_preset(&mut self, id: &str) -> Result<String, String> {
        let source = self.find(id).ok_or("Preset does not exist.")?.clone();
        let prefix = source.name.chars().take(35).collect::<String>();
        self.add(Some(&format!("{prefix} copy")), source.settings)
    }

    fn add(&mut self, name: Option<&str>, settings: VisualSettings) -> Result<String, String> {
        if self.presets.len() >= 256 {
            return Err("Preset limit reached.".into());
        }
        let name = match name.filter(|name| !name.trim().is_empty()) {
            Some(name) => normalized_name(name)?,
            None => self.random_name(),
        };
        let id = loop {
            let candidate = format!("preset-{:016x}", random_u64());
            if self.find(&candidate).is_none() {
                break candidate;
            }
        };
        self.presets.push(Preset {
            id: id.clone(),
            name,
            settings: settings.validated(),
        });
        Ok(id)
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        let name = normalized_name(name)?;
        let preset = self
            .presets
            .iter_mut()
            .find(|preset| preset.id == id)
            .ok_or("Preset does not exist.")?;
        preset.name = name;
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        if self.presets.len() == 1 {
            return Err("The last preset cannot be deleted.".into());
        }
        let index = self
            .presets
            .iter()
            .position(|preset| preset.id == id)
            .ok_or("Preset does not exist.")?;
        self.presets.remove(index);
        if self.active_preset == id {
            self.active_preset = self.presets[index.min(self.presets.len() - 1)].id.clone();
        }
        Ok(())
    }

    pub fn export(&self, id: &str) -> Result<String, String> {
        let preset = self.find(id).ok_or("Preset does not exist.")?;
        let payload = SharePayload {
            version: 1,
            name: preset.name.clone(),
            settings: preset.settings.clone(),
        };
        let json = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        Ok(format!("PS1:{}", base64url_encode(&json)))
    }

    pub fn import(&mut self, code: &str) -> Result<String, String> {
        let code = code.trim();
        if code.len() > 4096 {
            return Err("Preset code is too long.".into());
        }
        let encoded = code
            .strip_prefix("PS1:")
            .ok_or("Invalid preset code prefix.")?;
        let json = base64url_decode(encoded)?;
        let payload: SharePayload =
            serde_json::from_slice(&json).map_err(|_| "Invalid preset code data.")?;
        if payload.version != 1 {
            return Err("Unsupported preset code version.".into());
        }
        let settings = payload.settings.clone().validated();
        if settings != payload.settings {
            return Err("Preset code has invalid settings.".into());
        }
        self.add(Some(&payload.name), settings)
    }

    fn random_name(&self) -> String {
        const ADJECTIVES: [&str; 12] = [
            "Amber", "Azure", "Bright", "Calm", "Clear", "Cobalt", "Coral", "Golden", "Lunar",
            "Neon", "Silver", "Swift",
        ];
        const NOUNS: [&str; 12] = [
            "Comet", "Focus", "Orbit", "Pulse", "Ray", "Signal", "Spark", "Star", "Vector",
            "Vertex", "Wave", "Zenith",
        ];
        let value = random_u64() as usize;
        let base = format!(
            "{} {}",
            ADJECTIVES[value % ADJECTIVES.len()],
            NOUNS[(value / ADJECTIVES.len()) % NOUNS.len()]
        );
        if self.presets.iter().all(|preset| preset.name != base) {
            return base;
        }
        for suffix in 2..=256 {
            let candidate = format!("{base} {suffix}");
            if self.presets.iter().all(|preset| preset.name != candidate) {
                return candidate;
            }
        }
        format!("Preset {}", self.presets.len() + 1)
    }
}

fn normalized_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 || name.chars().any(char::is_control) {
        return Err("Preset name must contain 1–40 printable characters.".into());
    }
    Ok(name.into())
}

fn random_u64() -> u64 {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    RandomState::new().hash_one((stamp, std::process::id()))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SharePayload {
    version: u8,
    name: String,
    settings: VisualSettings,
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        out.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((value >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(value & 63) as usize] as char);
        }
    }
    out
}

fn base64url_decode(text: &str) -> Result<Vec<u8>, String> {
    if text.len() % 4 == 1 {
        return Err("Invalid preset code encoding.".into());
    }
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for chunk in text.as_bytes().chunks(4) {
        let mut value = 0_u32;
        for &byte in chunk {
            let digit = ALPHABET
                .iter()
                .position(|&candidate| candidate == byte)
                .ok_or("Invalid preset code encoding.")?;
            value = (value << 6) | digit as u32;
        }
        for _ in chunk.len()..4 {
            value <<= 6;
        }
        out.push((value >> 16) as u8);
        if chunk.len() > 2 {
            out.push((value >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(value as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_can_be_created_named_renamed_cloned_and_deleted() {
        let mut library = PresetLibrary::default();
        let generated = library.create(None).unwrap();
        assert!(!library.find(&generated).unwrap().name.is_empty());
        library.rename(&generated, "  My sight  ").unwrap();
        assert_eq!(library.find(&generated).unwrap().name, "My sight");
        let clone = library.clone_preset(&generated).unwrap();
        assert_eq!(library.find(&clone).unwrap().name, "My sight copy");
        library.rename(&generated, &"X".repeat(40)).unwrap();
        let long_name_clone = library.clone_preset(&generated).unwrap();
        assert_eq!(
            library.find(&long_name_clone).unwrap().name.chars().count(),
            40
        );
        library.delete("classic").unwrap();
        library.delete("dot").unwrap();
        library.delete("precision").unwrap();
        assert!(library.delete(&clone).is_ok());
        assert!(library.delete(&long_name_clone).is_ok());
        assert!(library.delete(&generated).is_err());
    }

    #[test]
    fn code_round_trips_every_visual_field_without_global_enabled_state() {
        let mut library = PresetLibrary::default();
        library.presets[0].settings.color = "#AB12CD".into();
        library.presets[0].settings.x_offset = 27;
        let code = library.export("dot").unwrap();
        let json = String::from_utf8(base64url_decode(code.strip_prefix("PS1:").unwrap()).unwrap())
            .unwrap();
        assert!(!json.contains("enabled"));
        let mut destination = PresetLibrary::default();
        let id = destination.import(&code).unwrap();
        assert_eq!(
            destination.find(&id).unwrap().settings,
            library.find("dot").unwrap().settings
        );
        assert!(destination.import("PS1:invalid!").is_err());
        assert!(destination.import("PS2:AAAA").is_err());
        assert!(destination.import(&"x".repeat(4097)).is_err());
    }

    #[test]
    fn deleted_defaults_are_not_restored_by_validation() {
        let mut library = PresetLibrary::default();
        library.delete("dot").unwrap();
        let json = serde_json::to_string(&library).unwrap();
        let restored: PresetLibrary = serde_json::from_str(&json).unwrap();
        assert!(restored.validated().find("dot").is_none());
    }
}

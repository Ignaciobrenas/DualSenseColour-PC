//! Parseo y validación estricta de índices y perfiles comunitarios `.sensehue`.

use serde::{Deserialize, Serialize};

use crate::hid::triggers::TriggerPreset;

pub const REGISTRY_INDEX_URL: &str =
    "https://raw.githubusercontent.com/Ignaciobrenas/sensehue-presets/main/index.json";
pub const MAX_PROFILE_BYTES: usize = 16 * 1024;
pub const MAX_INDEX_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    /// URL `https://` del perfil `.sensehue`.
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryIndex {
    pub version: u32,
    pub presets: Vec<RegistryEntry>,
}

/// Perfil comunitario: SOLO datos (nunca scripts ni comandos).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommunityProfile {
    pub name: String,
    pub author: String,
    /// Color `#RRGGBB`.
    pub color: String,
    pub brightness: u8,
    pub player_leds: u8,
    #[serde(default)]
    pub left_trigger: TriggerPreset,
    #[serde(default)]
    pub right_trigger: TriggerPreset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    TooLarge,
    InvalidJson(String),
    Field(&'static str),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "El perfil es demasiado grande."),
            Self::InvalidJson(e) => write!(f, "Perfil inválido: {e}"),
            Self::Field(name) => write!(f, "Campo fuera de rango o inválido: {name}"),
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn clean_text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}

/// Valida un perfil descargado.
pub fn parse_profile(raw: &str) -> Result<CommunityProfile, ValidationError> {
    if raw.len() > MAX_PROFILE_BYTES {
        return Err(ValidationError::TooLarge);
    }
    let p: CommunityProfile =
        serde_json::from_str(raw).map_err(|e| ValidationError::InvalidJson(e.to_string()))?;
    if !clean_text(&p.name, 48) {
        return Err(ValidationError::Field("name"));
    }
    if !clean_text(&p.author, 48) {
        return Err(ValidationError::Field("author"));
    }
    if !is_hex_color(&p.color) {
        return Err(ValidationError::Field("color"));
    }
    if p.brightness > 100 {
        return Err(ValidationError::Field("brightness"));
    }
    if p.player_leds > 5 {
        return Err(ValidationError::Field("player_leds"));
    }
    Ok(p)
}

/// Valida el índice: URLs solo `https`, ids únicos y sin duplicados.
pub fn parse_index(raw: &str) -> Result<RegistryIndex, ValidationError> {
    if raw.len() > MAX_INDEX_BYTES {
        return Err(ValidationError::TooLarge);
    }
    let idx: RegistryIndex =
        serde_json::from_str(raw).map_err(|e| ValidationError::InvalidJson(e.to_string()))?;
    let mut seen = std::collections::HashSet::new();
    for e in &idx.presets {
        if !clean_text(&e.id, 64) || !seen.insert(e.id.clone()) {
            return Err(ValidationError::Field("id"));
        }
        if !clean_text(&e.name, 64) {
            return Err(ValidationError::Field("name"));
        }
        if !e.url.starts_with("https://") {
            return Err(ValidationError::Field("url"));
        }
    }
    Ok(idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r##"{"name":"Cyberpunk","author":"neo","color":"#00E5FF","brightness":80,"player_leds":3,"left_trigger":"hair_trigger"}"##;

    #[test]
    fn accepts_valid_profile() {
        let p = parse_profile(GOOD).unwrap();
        assert_eq!(p.left_trigger, TriggerPreset::HairTrigger);
        assert_eq!(p.right_trigger, TriggerPreset::Off);
    }

    #[test]
    fn rejects_out_of_range_and_unknown_fields() {
        for (raw, field) in [
            (GOOD.replace("80", "180"), "brightness"),
            (GOOD.replace("#00E5FF", "red"), "color"),
            (GOOD.replace(":3", ":9"), "player_leds"),
        ] {
            assert_eq!(parse_profile(&raw), Err(ValidationError::Field(field)));
        }
        // Un campo ejecutable/desconocido (p. ej. "script") rompe el esquema estricto.
        let evil = GOOD.replace("{", r#"{"script":"calc.exe","#);
        assert!(matches!(
            parse_profile(&evil),
            Err(ValidationError::InvalidJson(_))
        ));
    }

    #[test]
    fn rejects_oversized_and_control_chars() {
        assert_eq!(
            parse_profile(&" ".repeat(MAX_PROFILE_BYTES + 1)),
            Err(ValidationError::TooLarge)
        );
        let bad = GOOD.replace("Cyberpunk", "A\\u0007B");
        assert_eq!(parse_profile(&bad), Err(ValidationError::Field("name")));
    }

    #[test]
    fn index_requires_https_and_unique_ids() {
        let ok = r#"{"version":1,"presets":[{"id":"a","name":"A","author":"x","description":"d","url":"https://x/a.sensehue"}]}"#;
        assert!(parse_index(ok).is_ok());
        assert_eq!(
            parse_index(&ok.replace("https", "http")),
            Err(ValidationError::Field("url"))
        );
        let dup = r#"{"version":1,"presets":[{"id":"a","name":"A","author":"x","description":"d","url":"https://x"},{"id":"a","name":"B","author":"x","description":"d","url":"https://y"}]}"#;
        assert_eq!(parse_index(dup), Err(ValidationError::Field("id")));
    }

    #[test]
    fn rejects_empty_author_and_whitespace_name() {
        assert_eq!(
            parse_profile(&GOOD.replace("\"neo\"", "\"   \"")),
            Err(ValidationError::Field("author"))
        );
        assert_eq!(
            parse_profile(&GOOD.replace("\"Cyberpunk\"", "\"   \"")),
            Err(ValidationError::Field("name"))
        );
    }
}

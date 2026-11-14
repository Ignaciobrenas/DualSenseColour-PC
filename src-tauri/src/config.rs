use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::automation::game_watcher::{GameMatch, GameProfileRule};
use crate::effects::LightingEffectMode;
use crate::error::{AppError, AppResult};
use crate::hid::color::{MicLedState, RgbColor};
use crate::hid::protocol::DualSenseOutputState;
use crate::hid::triggers::{battery_to_player_led_count, TriggerPreset};

const CONFIG_FILE_NAME: &str = "config.json";

/// Persistent user configuration stored atomically as JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub color: RgbColor,
    pub brightness: u8,
    pub preset_id: Option<String>,
    pub player_leds: u8,
    pub mic_led: MicLedState,
    pub apply_on_connect: bool,
    pub keep_alive_steam: bool,
    pub close_to_tray: bool,
    pub effect_mode: LightingEffectMode,
    pub battery_bar_leds: bool,
    pub sync_all_controllers: bool,
    pub left_trigger: TriggerPreset,
    pub right_trigger: TriggerPreset,
    pub local_api_enabled: bool,
    pub game_profiles: Vec<GameProfileRule>,
}

fn default_game_profiles() -> Vec<GameProfileRule> {
    vec![
        GameProfileRule {
            id: "cyberpunk-2077".to_string(),
            name: "Cyberpunk 2077".to_string(),
            matcher: GameMatch::ProcessName("Cyberpunk2077.exe".to_string()),
            preset_id: "cyber-yellow".to_string(),
            effect_mode: LightingEffectMode::AudioBeat,
        },
        GameProfileRule {
            id: "elden-ring".to_string(),
            name: "Elden Ring".to_string(),
            matcher: GameMatch::ProcessName("eldenring.exe".to_string()),
            preset_id: "amber-gold".to_string(),
            effect_mode: LightingEffectMode::Breathing,
        },
        GameProfileRule {
            id: "alan-wake-2".to_string(),
            name: "Alan Wake 2".to_string(),
            matcher: GameMatch::ProcessName("AlanWake2.exe".to_string()),
            preset_id: "crimson-pulse".to_string(),
            effect_mode: LightingEffectMode::InputReactive,
        },
    ]
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            color: RgbColor::default(),
            brightness: 100,
            preset_id: Some("playstation-blue".to_string()),
            player_leds: 1,
            mic_led: MicLedState::Off,
            apply_on_connect: true,
            keep_alive_steam: false,
            close_to_tray: true,
            effect_mode: LightingEffectMode::Static,
            battery_bar_leds: false,
            sync_all_controllers: true,
            left_trigger: TriggerPreset::Off,
            right_trigger: TriggerPreset::Off,
            local_api_enabled: false,
            game_profiles: default_game_profiles(),
        }
    }
}

impl AppConfig {
    /// Returns a copy with numeric bounds clamped (`brightness <= 100`, `player_leds <= 5`).
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.brightness = self.brightness.min(100);
        self.player_leds = self.player_leds.min(5);
        self
    }

    /// Converts persisted settings into the hardware report state expected by HID reports.
    #[must_use]
    pub fn to_output_state(&self) -> DualSenseOutputState {
        self.to_output_state_with_battery(None)
    }

    /// Converts persisted settings into HID output state, optionally mapping battery % onto the 5 player LEDs.
    #[must_use]
    pub fn to_output_state_with_battery(
        &self,
        battery_percent: Option<u8>,
    ) -> DualSenseOutputState {
        let player_leds = match (self.battery_bar_leds, battery_percent) {
            (true, Some(pct)) => battery_to_player_led_count(pct),
            _ => self.player_leds.min(5),
        };

        DualSenseOutputState {
            color: self.color,
            brightness: self.brightness.min(100),
            player_leds,
            mic_led: self.mic_led,
            left_trigger: self.left_trigger,
            right_trigger: self.right_trigger,
        }
    }

    /// Resolves the OS-appropriate config file path (`%APPDATA%` on Windows, `~/.config` on Linux).
    pub fn default_config_path() -> AppResult<PathBuf> {
        let project_dirs = ProjectDirs::from("com", "sensehue", "SenseHue").ok_or_else(|| {
            AppError::Config(
                "No se pudo resolver el directorio de configuración del usuario".to_string(),
            )
        })?;

        Ok(project_dirs.config_dir().join(CONFIG_FILE_NAME))
    }

    /// Loads configuration from the standard OS user config path.
    pub fn load_default() -> AppResult<Self> {
        let path = Self::default_config_path()?;
        Self::load_from_path(&path)
    }

    /// Saves configuration to the standard OS user config path.
    pub fn save_default(&self) -> AppResult<()> {
        let path = Self::default_config_path()?;
        self.save_to_path(&path)
    }

    /// Reads configuration from `path`, falling back to [`AppConfig::default`] when missing or corrupted.
    pub fn load_from_path(path: &Path) -> AppResult<Self> {
        if !path.exists() {
            debug!(path = %path.display(), "Archivo de configuración inexistente; usando valores por defecto");
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path).map_err(|e| {
            AppError::Config(format!(
                "No se pudo leer la configuración en {}: {e}",
                path.display()
            ))
        })?;

        match serde_json::from_str::<Self>(&raw) {
            Ok(parsed) => Ok(parsed.sanitized()),
            Err(err) => {
                warn!(%err, path = %path.display(), "Configuración corrupta; restaurando valores por defecto");
                Ok(Self::default())
            }
        }
    }

    /// Atomically writes configuration to `path` via a sibling `.tmp` file and `fs::rename`.
    pub fn save_to_path(&self, path: &Path) -> AppResult<()> {
        let parent = path.parent().ok_or_else(|| {
            AppError::Config(format!(
                "Ruta de configuración inválida (sin directorio padre): {}",
                path.display()
            ))
        })?;

        fs::create_dir_all(parent).map_err(|e| {
            AppError::Config(format!(
                "No se pudo crear el directorio {}: {e}",
                parent.display()
            ))
        })?;

        let clean = self.clone().sanitized();
        let json = serde_json::to_string_pretty(&clean)
            .map_err(|e| AppError::Config(format!("Error serializando configuración: {e}")))?;

        let tmp_path = parent.join(format!("{CONFIG_FILE_NAME}.tmp"));
        {
            let mut file = fs::File::create(&tmp_path).map_err(|e| {
                AppError::Config(format!(
                    "No se pudo crear el archivo temporal {}: {e}",
                    tmp_path.display()
                ))
            })?;
            file.write_all(json.as_bytes()).map_err(|e| {
                AppError::Config(format!(
                    "Error escribiendo configuración temporal {}: {e}",
                    tmp_path.display()
                ))
            })?;
            file.sync_all().map_err(|e| {
                AppError::Config(format!(
                    "Error sincronizando configuración en disco {}: {e}",
                    tmp_path.display()
                ))
            })?;
        }

        // On Windows `fs::rename` replaces existing destination files cleanly on NTFS in Rust std,
        // but removing an existing target avoids edge cases on older filesystems.
        if path.exists() {
            let _ = fs::remove_file(path);
        }

        fs::rename(&tmp_path, path).map_err(|e| {
            AppError::Config(format!(
                "No se pudo mover {} a {}: {e}",
                tmp_path.display(),
                path.display()
            ))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_defaults() {
        let dir = tempfile::tempdir().expect("tempdir created");
        let path = dir.path().join("missing/config.json");
        let cfg = AppConfig::load_from_path(&path).expect("missing file falls back");
        assert_eq!(cfg, AppConfig::default());
        assert_eq!(cfg.preset_id.as_deref(), Some("playstation-blue"));
        assert_eq!(cfg.brightness, 100);
        assert_eq!(cfg.player_leds, 1);
        assert!(cfg.apply_on_connect);
        assert!(cfg.close_to_tray);
    }

    #[test]
    fn atomic_save_and_reload_roundtrip_clamps_bounds() {
        let dir = tempfile::tempdir().expect("tempdir created");
        let path = dir.path().join("nested").join("config.json");

        let cfg = AppConfig {
            color: RgbColor::new(0, 240, 255),
            brightness: 250,
            preset_id: Some("cyber-cyan".to_string()),
            player_leds: 9,
            mic_led: MicLedState::Pulse,
            apply_on_connect: true,
            keep_alive_steam: true,
            close_to_tray: false,
            ..Default::default()
        };

        cfg.save_to_path(&path).expect("atomic save succeeds");
        assert!(path.exists());
        assert!(!path.with_file_name("config.json.tmp").exists());

        let reloaded = AppConfig::load_from_path(&path).expect("reload succeeds");
        assert_eq!(reloaded.color, RgbColor::new(0, 240, 255));
        assert_eq!(reloaded.brightness, 100);
        assert_eq!(reloaded.player_leds, 5);
        assert_eq!(reloaded.mic_led, MicLedState::Pulse);
        assert!(reloaded.keep_alive_steam);
        assert!(!reloaded.close_to_tray);

        let output = reloaded.to_output_state();
        assert_eq!(output.color, RgbColor::new(0, 240, 255));
        assert_eq!(output.brightness, 100);
        assert_eq!(output.player_leds, 5);
        assert_eq!(output.mic_led, MicLedState::Pulse);
    }

    #[test]
    fn corrupted_json_falls_back_gracefully_to_default() {
        let dir = tempfile::tempdir().expect("tempdir created");
        let path = dir.path().join("config.json");
        fs::write(&path, "{ invalid json :::").expect("corrupted seed written");

        let loaded = AppConfig::load_from_path(&path).expect("fallback succeeds");
        assert_eq!(loaded, AppConfig::default());
    }
}

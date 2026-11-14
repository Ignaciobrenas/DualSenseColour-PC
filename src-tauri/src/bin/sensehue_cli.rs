use std::env;
use std::process::ExitCode;

use sensehue_lib::commands::ControllerService;
use sensehue_lib::config::AppConfig;
use sensehue_lib::effects::{compute_battery_smart_color, LightingEffectMode};
use sensehue_lib::hid::color::{find_preset, RgbColor, PRESETS};
use sensehue_lib::hid::transport::RealHidTransport;

/// Parsed CLI command for `sensehue-cli`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Help,
    Status,
    TurnOff,
    ApplyPreset(String),
    SetColor {
        hex: String,
        brightness: Option<u8>,
    },
    SetMode {
        mode: LightingEffectMode,
        brightness: Option<u8>,
    },
}

const HELP_TEXT: &str = "\
SenseHue CLI v2.0.0 — DualSense Lighting & Telemetry
Uso:
  sensehue-cli --preset <id>                 Aplica un preset (playstation-blue, cyber-cyan, crimson-red, emerald, gold, violet)
  sensehue-cli --color <#RRGGBB> [--brightness <0..100>]  Aplica un color RGB personalizado
  sensehue-cli --mode <static|breathing|spectrum|battery> Selecciona el modo de iluminación
  sensehue-cli --off                         Apaga la barra de luz LED
  sensehue-cli --status                      Muestra el estado actual del mando y configuración
  sensehue-cli --help                        Muestra esta ayuda";

/// Pure CLI argument parser separated for deterministic CI testing.
pub fn parse_cli_args<I, S>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let tokens: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    if tokens.is_empty() {
        return Ok(CliAction::Help);
    }

    let mut idx = 0;
    let mut preset: Option<String> = None;
    let mut color: Option<String> = None;
    let mut mode: Option<LightingEffectMode> = None;
    let mut brightness: Option<u8> = None;

    while idx < tokens.len() {
        match tokens[idx].as_str() {
            "-h" | "--help" => return Ok(CliAction::Help),
            "--status" => return Ok(CliAction::Status),
            "--off" => return Ok(CliAction::TurnOff),
            "--preset" => {
                idx += 1;
                let value = tokens
                    .get(idx)
                    .ok_or_else(|| "Falta el identificador tras --preset".to_string())?;
                preset = Some(value.clone());
            }
            "--color" => {
                idx += 1;
                let value = tokens
                    .get(idx)
                    .ok_or_else(|| "Falta el código HEX tras --color".to_string())?;
                RgbColor::from_hex(value).map_err(|e| e.to_string())?;
                color = Some(value.clone());
            }
            "--brightness" => {
                idx += 1;
                let value = tokens
                    .get(idx)
                    .ok_or_else(|| "Falta el porcentaje tras --brightness".to_string())?;
                let parsed: u8 = value
                    .parse()
                    .map_err(|_| format!("Brillo inválido '{value}' (usa 0..100)"))?;
                if parsed > 100 {
                    return Err(format!("Brillo fuera de rango '{parsed}' (máximo 100)"));
                }
                brightness = Some(parsed);
            }
            "--mode" => {
                idx += 1;
                let value = tokens
                    .get(idx)
                    .ok_or_else(|| "Falta el nombre de modo tras --mode".to_string())?;
                let parsed = LightingEffectMode::parse(value).ok_or_else(|| {
                    format!(
                        "Modo inválido '{value}' (usa static, breathing, spectrum, battery, input, audio o ambilight)"
                    )
                })?;
                mode = Some(parsed);
            }
            unknown => {
                return Err(format!(
                    "Opción desconocida: '{unknown}'. Ejecuta --help para ver las opciones."
                ));
            }
        }
        idx += 1;
    }

    if let Some(preset_id) = preset {
        let resolved = find_preset(&preset_id).ok_or_else(|| {
            let valid: Vec<&str> = PRESETS.iter().map(|p| p.id).collect();
            format!(
                "Preset desconocido '{preset_id}'. Disponibles: {}",
                valid.join(", ")
            )
        })?;
        return Ok(CliAction::ApplyPreset(resolved.id.to_string()));
    }

    if let Some(hex) = color {
        return Ok(CliAction::SetColor { hex, brightness });
    }

    if let Some(effect_mode) = mode {
        return Ok(CliAction::SetMode {
            mode: effect_mode,
            brightness,
        });
    }

    Err("Debes indicar una acción (--preset, --color, --mode, --off o --status).".to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let action = match parse_cli_args(args) {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("Error: {err}");
            return ExitCode::FAILURE;
        }
    };

    if matches!(action, CliAction::Help) {
        println!("{HELP_TEXT}");
        return ExitCode::SUCCESS;
    }

    let config_path = AppConfig::default_config_path()
        .unwrap_or_else(|_| env::temp_dir().join("sensehue-config.json"));
    let initial_cfg = AppConfig::load_from_path(&config_path).unwrap_or_default();
    let mut service = ControllerService::new(RealHidTransport::new(), initial_cfg, config_path);

    let outcome = match action {
        CliAction::Help => Ok(String::new()),
        CliAction::Status => {
            let status = service.status();
            let cfg = service.config();
            let battery = status
                .battery_percent
                .map_or_else(|| "N/D".to_string(), |pct| format!("{pct}%"));
            Ok(format!(
                "SenseHue Status -> Conectado: {} | Modelo: {:?} | Transporte: {:?} | Batería: {} | Carga: {:?} | Color: {} | Brillo: {}%",
                status.connected,
                status.model,
                status.transport,
                battery,
                status.charging_state,
                cfg.color.to_hex(),
                cfg.brightness
            ))
        }
        CliAction::TurnOff => service
            .turn_off_lightbar()
            .map(|_| "Barra LED DualSense apagada correctamente.".to_string()),
        CliAction::ApplyPreset(id) => service
            .apply_preset(&id)
            .map(|cfg| format!("Preset '{id}' aplicado ({})", cfg.color.to_hex())),
        CliAction::SetColor { hex, brightness } => match RgbColor::from_hex(&hex) {
            Ok(color) => {
                let target_brightness = brightness.unwrap_or(service.config().brightness.max(10));
                service
                    .set_lightbar_color(color.r, color.g, color.b, target_brightness, true)
                    .map(|cfg| {
                        format!(
                            "Color {} aplicado al {}% de brillo.",
                            cfg.color.to_hex(),
                            cfg.brightness
                        )
                    })
            }
            Err(err) => Err(err),
        },
        CliAction::SetMode { mode, brightness } => {
            let target_brightness = brightness.unwrap_or(service.config().brightness.max(10));
            let color = match mode {
                LightingEffectMode::BatterySmart => compute_battery_smart_color(100),
                _ => RgbColor::new(0, 229, 255),
            };
            service
                .set_lightbar_color(color.r, color.g, color.b, target_brightness, true)
                .map(|cfg| {
                    format!(
                        "Modo {mode:?} aplicado ({} · {}%).",
                        cfg.color.to_hex(),
                        cfg.brightness
                    )
                })
        }
    };

    match outcome {
        Ok(msg) => {
            if !msg.is_empty() {
                println!("{msg}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Error ejecutando comando HID: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_documented_cli_flags() {
        assert_eq!(parse_cli_args(["--help"]), Ok(CliAction::Help));
        assert_eq!(parse_cli_args(["--off"]), Ok(CliAction::TurnOff));
        assert_eq!(parse_cli_args(["--status"]), Ok(CliAction::Status));
        assert_eq!(
            parse_cli_args(["--preset", "cyber-cyan"]),
            Ok(CliAction::ApplyPreset("cyber-cyan".to_string()))
        );
        assert_eq!(
            parse_cli_args(["--color", "#00E5FF", "--brightness", "65"]),
            Ok(CliAction::SetColor {
                hex: "#00E5FF".to_string(),
                brightness: Some(65),
            })
        );
        assert_eq!(
            parse_cli_args(["--mode", "battery"]),
            Ok(CliAction::SetMode {
                mode: LightingEffectMode::BatterySmart,
                brightness: None,
            })
        );
    }

    #[test]
    fn rejects_invalid_hex_brightness_or_unknown_preset() {
        assert!(parse_cli_args(["--color", "INVALID"]).is_err());
        assert!(parse_cli_args(["--color", "#00E5FF", "--brightness", "150"]).is_err());
        assert!(parse_cli_args(["--preset", "nonexistent"]).is_err());
        assert!(parse_cli_args(["--unknown-flag"]).is_err());
    }
}

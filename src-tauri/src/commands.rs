use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tracing::warn;

use crate::api::local_server::{
    preset_color_for_id, spawn_local_api_server, LocalApiCommand, LocalApiResponse,
    DEFAULT_LOCAL_API_ADDR,
};
use crate::config::AppConfig;
use crate::effects::{evaluate_effect_frame, EffectFrameInput, LightingEffectMode};
use crate::error::{AppError, AppResult};
use crate::hid::color::{find_preset, ColorPreset, MicLedState, RgbColor, PRESETS};
use crate::hid::monitor::{ConnectionStatus, ControllerEvent, ControllerMonitor};
use crate::hid::transport::{HidTransport, RealHidTransport};
use crate::hid::triggers::TriggerPreset;
use crate::tray::{toggle_main_window, TrayAction, TrayHandles};

pub const EVENT_CONTROLLER_STATUS: &str = "controller-status-changed";
pub const EVENT_LIGHTBAR_COLOR: &str = "lightbar-color-changed";

/// Core controller service coordinating HID output reports, hotplug monitoring, and disk persistence.
pub struct ControllerService<T: HidTransport> {
    monitor: ControllerMonitor<T>,
    config: AppConfig,
    config_path: PathBuf,
}

impl<T: HidTransport> ControllerService<T> {
    /// Creates a new controller service initialized from `config` and persisted at `config_path`.
    #[must_use]
    pub fn new(transport: T, config: AppConfig, config_path: PathBuf) -> Self {
        let clean = config.sanitized();
        let mut monitor = ControllerMonitor::new(
            transport,
            clean.to_output_state(),
            clean.apply_on_connect,
            clean.keep_alive_steam,
        );
        monitor.set_sync_all_controllers(clean.sync_all_controllers);
        // Perform immediate startup scan so already-connected controllers are active at 0ms.
        let _ = monitor.poll_once();
        Self {
            monitor,
            config: clean,
            config_path,
        }
    }

    /// Returns the latest controller connection snapshot.
    #[must_use]
    pub fn status(&self) -> ConnectionStatus {
        self.monitor.status()
    }

    /// Returns a clone of the current application configuration.
    #[must_use]
    pub fn config(&self) -> AppConfig {
        self.config.clone()
    }

    /// Persists config (if requested) and dispatches HID output state fail-safely on transient I/O errors.
    fn persist_and_sync_state(&mut self, persist: bool) -> AppResult<AppConfig> {
        if persist {
            self.config.save_to_path(&self.config_path)?;
        }

        self.monitor
            .set_sync_all_controllers(self.config.sync_all_controllers);
        let battery = self.monitor.status().battery_percent;
        let output_state = self.config.to_output_state_with_battery(battery);
        if let Err(err) = self.monitor.apply_output_state(output_state) {
            warn!(%err, "Fallo temporal enviando reporte HID; el estado se reaplicará automáticamente");
            let _ = self.monitor.poll_once();
        }

        Ok(self.config.clone())
    }

    /// Updates the RGB color & brightness, sends the HID report, and optionally persists to disk.
    pub fn set_lightbar_color(
        &mut self,
        r: u8,
        g: u8,
        b: u8,
        brightness: u8,
        persist: bool,
    ) -> AppResult<AppConfig> {
        let clamped_brightness = brightness.min(100);
        let color = RgbColor::new(r, g, b);

        self.config.color = color;
        self.config.brightness = clamped_brightness;
        self.config.preset_id = None;

        self.persist_and_sync_state(persist)
    }

    /// Updates player LED segment count (`0..=5`) and mic LED indicator state, persisting immediately.
    pub fn set_player_leds(
        &mut self,
        player_leds: u8,
        mic_led: MicLedState,
    ) -> AppResult<AppConfig> {
        self.config.player_leds = player_leds.min(5);
        self.config.mic_led = mic_led;

        self.persist_and_sync_state(true)
    }

    /// Applies one of the built-in named color presets (`playstation-blue`, `off`, etc.).
    pub fn apply_preset(&mut self, preset_id: &str) -> AppResult<AppConfig> {
        let preset = find_preset(preset_id)
            .ok_or_else(|| AppError::InvalidColor(format!("Preset desconocido: '{preset_id}'")))?;

        self.config.color = preset.color;
        self.config.preset_id = Some(preset.id.to_string());
        if preset.id == "off" {
            self.config.brightness = 0;
        } else if self.config.brightness == 0 {
            self.config.brightness = 100;
        }

        self.persist_and_sync_state(true)
    }

    /// Turns off the DualSense touchpad lightbar cleanly (`brightness = 0`).
    pub fn turn_off_lightbar(&mut self) -> AppResult<AppConfig> {
        self.apply_preset("off")
    }

    /// Replaces user preferences & controller settings, applies them to hardware, and saves to disk.
    pub fn save_config(&mut self, next: AppConfig) -> AppResult<AppConfig> {
        let clean = next.sanitized();
        self.config = clean.clone();
        self.monitor
            .set_preferences(clean.apply_on_connect, clean.keep_alive_steam);

        self.persist_and_sync_state(true)
    }

    /// Updates brightness percentage (`0..=100`) while keeping active color and persisting.
    pub fn set_brightness(&mut self, brightness: u8) -> AppResult<AppConfig> {
        self.config.brightness = brightness.min(100);
        if self.config.brightness == 0 {
            self.config.preset_id = Some("off".to_string());
        } else if self.config.preset_id.as_deref() == Some("off") {
            self.config.preset_id = None;
        }
        self.persist_and_sync_state(true)
    }

    /// Refreshes USB/Bluetooth HID detection and re-applies the active LED state immediately.
    pub fn resync(&mut self) -> AppResult<(ConnectionStatus, AppConfig)> {
        let _ = self.monitor.poll_once();
        let cfg = self.persist_and_sync_state(false)?;
        Ok((self.status(), cfg))
    }

    /// Updates active dynamic lighting effect mode and immediately applies an initial frame.
    pub fn set_effect_mode(&mut self, mode: LightingEffectMode) -> AppResult<AppConfig> {
        self.config.effect_mode = mode;
        if mode != LightingEffectMode::Static {
            let status = self.monitor.status();
            let frame_color = evaluate_effect_frame(
                mode,
                &EffectFrameInput {
                    base_color: self.config.color,
                    elapsed_secs: 0.25,
                    battery_percent: status.battery_percent.unwrap_or(100),
                    trigger_depth: status.telemetry.as_ref().map_or(0, |t| t.r2),
                    audio_bass_energy: 0.4,
                    ..EffectFrameInput::default()
                },
            );
            let mut state = self
                .config
                .to_output_state_with_battery(status.battery_percent);
            state.color = frame_color;
            let _ = self.monitor.apply_output_state(state);
        }
        self.persist_and_sync_state(true)
    }

    /// Programs left (L2) and right (R2) adaptive trigger resistance presets immediately.
    pub fn set_adaptive_triggers(
        &mut self,
        left: TriggerPreset,
        right: TriggerPreset,
    ) -> AppResult<AppConfig> {
        self.config.left_trigger = left;
        self.config.right_trigger = right;
        self.persist_and_sync_state(true)
    }

    /// Exports active `.sensehue` shareable JSON configuration.
    pub fn export_profile_json(&self) -> AppResult<String> {
        serde_json::to_string_pretty(&self.config)
            .map_err(|e| AppError::Config(format!("Error serializando perfil .sensehue: {e}")))
    }

    /// Validates, sanitizes, and imports a `.sensehue` JSON configuration onto hardware & disk.
    pub fn import_profile_json(&mut self, raw_json: &str) -> AppResult<AppConfig> {
        let parsed: AppConfig = serde_json::from_str(raw_json)
            .map_err(|e| AppError::Config(format!("JSON .sensehue inválido: {e}")))?;
        self.save_config(parsed)
    }

    /// Advances the underlying HID hotplug monitor by one polling cycle.
    pub fn poll_once(&mut self) -> Vec<ControllerEvent> {
        self.monitor.poll_once()
    }
}

/// Thread-safe shared Tauri application state.
#[derive(Clone)]
pub struct AppState {
    service: Arc<Mutex<ControllerService<RealHidTransport>>>,
}

impl AppState {
    /// Initializes production HID transport, loads saved config, and constructs shared [`AppState`].
    pub fn new(config: AppConfig, config_path: PathBuf) -> Self {
        let service = ControllerService::new(RealHidTransport::new(), config, config_path);
        Self {
            service: Arc::new(Mutex::new(service)),
        }
    }

    /// Returns a non-blocking snapshot of `(ConnectionStatus, AppConfig)` for tray initialization.
    #[must_use]
    pub fn snapshot(&self) -> (ConnectionStatus, AppConfig) {
        self.service
            .lock()
            .map(|guard| (guard.status(), guard.config()))
            .unwrap_or_else(|_| (ConnectionStatus::default(), AppConfig::default()))
    }

    /// Reads whether closing the main window should hide to tray.
    #[must_use]
    pub fn should_close_to_tray(&self) -> bool {
        self.service
            .lock()
            .map(|guard| guard.config().close_to_tray)
            .unwrap_or(true)
    }

    /// Executes a closure against the locked [`ControllerService`].
    pub fn with_service<R, F>(&self, f: F) -> AppResult<R>
    where
        F: FnOnce(&mut ControllerService<RealHidTransport>) -> AppResult<R>,
    {
        let mut guard = self
            .service
            .lock()
            .map_err(|_| AppError::HidIo("Error interno de bloqueo del servicio".to_string()))?;
        f(&mut guard)
    }
}

fn sync_tray_and_ui(
    app: &AppHandle,
    status: &ConnectionStatus,
    cfg: &AppConfig,
    emit_color: bool,
    emit_status: bool,
) {
    if let Some(tray) = app.try_state::<TrayHandles>() {
        tray.update_state(status, cfg);
    }
    if emit_color {
        if let Err(err) = app.emit(EVENT_LIGHTBAR_COLOR, cfg) {
            warn!(%err, "No se pudo emitir el evento de color a la UI");
        }
    }
    if emit_status {
        if let Err(err) = app.emit(EVENT_CONTROLLER_STATUS, status) {
            warn!(%err, "No se pudo emitir controller-status-changed");
        }
    }
}

/// Dispatches rich Windows system tray actions and synchronizes UI + tray state.
pub fn handle_tray_action(app: &AppHandle, state: &AppState, action: TrayAction) {
    match action {
        TrayAction::ToggleWindow => toggle_main_window(app),
        TrayAction::Quit => app.exit(0),
        TrayAction::ResyncController => match state.with_service(ControllerService::resync) {
            Ok((status, cfg)) => sync_tray_and_ui(app, &status, &cfg, true, true),
            Err(err) => warn!(%err, "Error resincronizando mando desde la bandeja"),
        },
        TrayAction::ApplyPreset(preset_id) => {
            let res = state.with_service(|svc| Ok((svc.status(), svc.apply_preset(&preset_id)?)));
            match res {
                Ok((status, cfg)) => sync_tray_and_ui(app, &status, &cfg, true, false),
                Err(err) => warn!(%err, %preset_id, "Error aplicando preset desde la bandeja"),
            }
        }
        TrayAction::SetBrightness(level) => {
            let res = state.with_service(|svc| Ok((svc.status(), svc.set_brightness(level)?)));
            match res {
                Ok((status, cfg)) => sync_tray_and_ui(app, &status, &cfg, true, false),
                Err(err) => warn!(%err, level, "Error ajustando brillo desde la bandeja"),
            }
        }
        TrayAction::SetPlayerLeds(count) => {
            let res = state.with_service(|svc| {
                let mic = svc.config().mic_led;
                Ok((svc.status(), svc.set_player_leds(count, mic)?))
            });
            match res {
                Ok((status, cfg)) => sync_tray_and_ui(app, &status, &cfg, true, false),
                Err(err) => warn!(%err, count, "Error cambiando LEDs de jugador desde la bandeja"),
            }
        }
        TrayAction::ToggleApplyOnConnect
        | TrayAction::ToggleSteamFix
        | TrayAction::ToggleCloseToTray => {
            let res = state.with_service(|svc| {
                let mut next = svc.config();
                match action {
                    TrayAction::ToggleApplyOnConnect => {
                        next.apply_on_connect = !next.apply_on_connect;
                    }
                    TrayAction::ToggleSteamFix => {
                        next.keep_alive_steam = !next.keep_alive_steam;
                    }
                    TrayAction::ToggleCloseToTray => {
                        next.close_to_tray = !next.close_to_tray;
                    }
                    _ => {}
                }
                Ok((svc.status(), svc.save_config(next)?))
            });
            match res {
                Ok((status, cfg)) => sync_tray_and_ui(app, &status, &cfg, true, false),
                Err(err) => warn!(%err, "Error cambiando preferencia desde la bandeja"),
            }
        }
    }
}

/// Spawns the 1Hz hotplug polling & telemetry loop and dynamic effect animator.
pub fn spawn_hotplug_monitor(app: AppHandle, state: AppState) {
    let api_enabled = state.snapshot().1.local_api_enabled;
    if api_enabled {
        let api_app = app.clone();
        let api_state = state.clone();
        spawn_local_api_server(DEFAULT_LOCAL_API_ADDR, move |cmd| {
            handle_local_api_command(&api_app, &api_state, cmd)
        });
    }

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        let mut last_status = state.snapshot().0;
        let mut tick_secs = 0.0f32;
        loop {
            interval.tick().await;
            tick_secs += 1.0;

            let poll_state = state.clone();
            let poll_result = tokio::task::spawn_blocking(move || {
                poll_state.with_service(|svc| Ok((svc.poll_once(), svc.status(), svc.config())))
            })
            .await;

            let Ok(Ok((events, status, cfg))) = poll_result else {
                continue;
            };

            let status_changed = events.iter().any(|event| {
                matches!(
                    event,
                    ControllerEvent::Connected(_) | ControllerEvent::Disconnected
                )
            }) || status != last_status;

            if status_changed {
                last_status = status.clone();
                sync_tray_and_ui(&app, &status, &cfg, false, true);
            }

            if status.connected && cfg.effect_mode != LightingEffectMode::Static {
                let frame_color = evaluate_effect_frame(
                    cfg.effect_mode,
                    &EffectFrameInput {
                        base_color: cfg.color,
                        elapsed_secs: tick_secs,
                        battery_percent: status.battery_percent.unwrap_or(100),
                        trigger_depth: status.telemetry.as_ref().map_or(0, |t| t.r2),
                        audio_bass_energy: 0.45,
                        ..EffectFrameInput::default()
                    },
                );
                let anim_state = state.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    anim_state.with_service(|svc| {
                        let mut out = svc
                            .config()
                            .to_output_state_with_battery(svc.status().battery_percent);
                        out.color = frame_color;
                        svc.monitor.apply_output_state(out)
                    })
                })
                .await;
            }
        }
    });
}

fn handle_local_api_command(
    app: &AppHandle,
    state: &AppState,
    cmd: LocalApiCommand,
) -> LocalApiResponse {
    match cmd {
        LocalApiCommand::GetStatus => {
            let (status, cfg) = state.snapshot();
            let json = serde_json::json!({
                "ok": true,
                "connected": status.connected,
                "connected_count": status.connected_count,
                "battery_percent": status.battery_percent,
                "color": cfg.color.to_hex(),
                "brightness": cfg.brightness,
                "effect_mode": cfg.effect_mode,
            });
            LocalApiResponse::ok(json.to_string())
        }
        LocalApiCommand::SetPreset(preset_id) => {
            if let Some(rgb) = preset_color_for_id(&preset_id) {
                let res = state.with_service(|svc| {
                    Ok((
                        svc.status(),
                        svc.set_lightbar_color(rgb.r, rgb.g, rgb.b, 100, true)?,
                    ))
                });
                if let Ok((status, cfg)) = res {
                    sync_tray_and_ui(app, &status, &cfg, true, false);
                    return LocalApiResponse::ok("{\"ok\":true}");
                }
            }
            LocalApiResponse::error(500, "Failed to apply preset")
        }
        LocalApiCommand::SetColor(rgb) => {
            let res = state.with_service(|svc| {
                let b = svc.config().brightness.max(25);
                Ok((
                    svc.status(),
                    svc.set_lightbar_color(rgb.r, rgb.g, rgb.b, b, true)?,
                ))
            });
            match res {
                Ok((status, cfg)) => {
                    sync_tray_and_ui(app, &status, &cfg, true, false);
                    LocalApiResponse::ok("{\"ok\":true}")
                }
                Err(e) => LocalApiResponse::error(500, e.to_string()),
            }
        }
        LocalApiCommand::SetMode(mode) => {
            let res = state.with_service(|svc| Ok((svc.status(), svc.set_effect_mode(mode)?)));
            match res {
                Ok((status, cfg)) => {
                    sync_tray_and_ui(app, &status, &cfg, true, false);
                    LocalApiResponse::ok("{\"ok\":true}")
                }
                Err(e) => LocalApiResponse::error(500, e.to_string()),
            }
        }
        LocalApiCommand::TurnOff => {
            let res = state.with_service(|svc| Ok((svc.status(), svc.turn_off_lightbar()?)));
            match res {
                Ok((status, cfg)) => {
                    sync_tray_and_ui(app, &status, &cfg, true, false);
                    LocalApiResponse::ok("{\"ok\":true}")
                }
                Err(e) => LocalApiResponse::error(500, e.to_string()),
            }
        }
    }
}

/// Updates the DualSense lightbar RGB color and brightness (0..=100).
#[tauri::command]
pub fn set_lightbar_color(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    r: u8,
    g: u8,
    b: u8,
    brightness: u8,
    persist: Option<bool>,
) -> AppResult<AppConfig> {
    let should_persist = persist.unwrap_or(false);
    let (status, cfg) = state.with_service(|svc| {
        let cfg = svc.set_lightbar_color(r, g, b, brightness, should_persist)?;
        Ok((svc.status(), cfg))
    })?;
    if should_persist {
        sync_tray_and_ui(&app, &status, &cfg, true, false);
    }
    Ok(cfg)
}

/// Updates player LED segment count (0..=5) and microphone mute indicator LED.
#[tauri::command]
pub fn set_player_leds(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    player_leds: u8,
    mic_led: MicLedState,
) -> AppResult<AppConfig> {
    let (status, cfg) =
        state.with_service(|svc| Ok((svc.status(), svc.set_player_leds(player_leds, mic_led)?)))?;
    sync_tray_and_ui(&app, &status, &cfg, true, false);
    Ok(cfg)
}

/// Applies one of the built-in color presets and persists the choice.
#[tauri::command]
pub fn apply_preset(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    preset_id: String,
) -> AppResult<AppConfig> {
    let (status, cfg) =
        state.with_service(|svc| Ok((svc.status(), svc.apply_preset(&preset_id)?)))?;
    sync_tray_and_ui(&app, &status, &cfg, true, false);
    Ok(cfg)
}

/// Turns off the touchpad lightbar without disconnecting the controller.
#[tauri::command]
pub fn turn_off_lightbar(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<AppConfig> {
    let (status, cfg) = state.with_service(|svc| Ok((svc.status(), svc.turn_off_lightbar()?)))?;
    sync_tray_and_ui(&app, &status, &cfg, true, false);
    Ok(cfg)
}

/// Returns the current DualSense connection snapshot.
#[tauri::command]
pub fn get_controller_status(state: tauri::State<'_, AppState>) -> AppResult<ConnectionStatus> {
    state.with_service(|svc| Ok(svc.status()))
}

/// Returns the active persisted configuration.
#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> AppResult<AppConfig> {
    state.with_service(|svc| Ok(svc.config()))
}

/// Saves full user preferences & LED settings atomically to disk.
#[tauri::command]
pub fn save_config(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    config: AppConfig,
) -> AppResult<AppConfig> {
    let (status, saved) = state.with_service(|svc| Ok((svc.status(), svc.save_config(config)?)))?;
    sync_tray_and_ui(&app, &status, &saved, true, false);
    Ok(saved)
}

/// Returns the curated catalog of named color presets.
#[tauri::command]
#[must_use]
pub fn get_presets() -> Vec<ColorPreset> {
    PRESETS.to_vec()
}

/// Switches the active dynamic lighting effect mode and persists preferences.
#[tauri::command]
pub fn set_effect_mode(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    mode: LightingEffectMode,
) -> AppResult<AppConfig> {
    let (status, cfg) = state.with_service(|svc| Ok((svc.status(), svc.set_effect_mode(mode)?)))?;
    sync_tray_and_ui(&app, &status, &cfg, true, false);
    Ok(cfg)
}

/// Applies adaptive trigger presets (`L2` and `R2`) and persists configuration.
#[tauri::command]
pub fn set_adaptive_triggers(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    left: TriggerPreset,
    right: TriggerPreset,
) -> AppResult<AppConfig> {
    let (status, cfg) =
        state.with_service(|svc| Ok((svc.status(), svc.set_adaptive_triggers(left, right)?)))?;
    sync_tray_and_ui(&app, &status, &cfg, true, false);
    Ok(cfg)
}

/// Serializes the active SenseHue configuration as a shareable `.sensehue` JSON string.
#[tauri::command]
pub fn export_profile_json(state: tauri::State<'_, AppState>) -> AppResult<String> {
    state.with_service(|svc| svc.export_profile_json())
}

/// Validates and imports a `.sensehue` JSON configuration string.
#[tauri::command]
pub fn import_profile_json(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    json: String,
) -> AppResult<AppConfig> {
    let (status, cfg) =
        state.with_service(|svc| Ok((svc.status(), svc.import_profile_json(&json)?)))?;
    sync_tray_and_ui(&app, &status, &cfg, true, false);
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::protocol::USB_REPORT_LEN;
    use crate::hid::transport::{DetectedController, DeviceModel, MockHidTransport, TransportType};

    fn attached_usb_controller() -> DetectedController {
        DetectedController {
            path: "usb\\test\\dualsense".to_string(),
            model: DeviceModel::DualSense,
            transport: TransportType::Usb,
            serial_number: Some("SN-TEST-01".to_string()),
            product_name: Some("DualSense Wireless Controller".to_string()),
            interface_number: 3,
        }
    }

    #[test]
    fn service_applies_color_presets_player_leds_and_persists_atomically() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("config.json");
        let transport = MockHidTransport::new();
        transport.set_controllers(vec![attached_usb_controller()]);

        let mut svc =
            ControllerService::new(transport.clone(), AppConfig::default(), cfg_path.clone());

        let _ = svc.poll_once();
        assert!(svc.status().connected);

        // Live color drag without persist sends HID packet without writing disk yet
        let live_cfg = svc
            .set_lightbar_color(0, 240, 255, 80, false)
            .expect("live update works");
        assert_eq!(live_cfg.color, RgbColor::new(0, 240, 255));
        assert_eq!(live_cfg.brightness, 80);
        assert!(live_cfg.preset_id.is_none());
        assert!(!cfg_path.exists());

        // Apply preset persists atomically and sends HID report
        let preset_cfg = svc.apply_preset("crimson-red").expect("preset applied");
        assert_eq!(preset_cfg.color, RgbColor::new(255, 23, 68));
        assert_eq!(preset_cfg.preset_id.as_deref(), Some("crimson-red"));
        assert!(cfg_path.exists());

        // Turn off sets brightness = 0 and preset = off
        let off_cfg = svc.turn_off_lightbar().expect("lightbar turned off");
        assert_eq!(off_cfg.brightness, 0);
        assert_eq!(off_cfg.preset_id.as_deref(), Some("off"));

        // Player LEDs + Mic LED update
        let led_cfg = svc
            .set_player_leds(4, MicLedState::Pulse)
            .expect("player leds saved");
        assert_eq!(led_cfg.player_leds, 4);
        assert_eq!(led_cfg.mic_led, MicLedState::Pulse);

        let reloaded = AppConfig::load_from_path(&cfg_path).expect("reload persisted config");
        assert_eq!(reloaded, led_cfg);

        let sent = transport.sent_reports();
        assert!(sent.len() >= 4);
        assert!(sent.iter().all(|r| r.bytes.len() == USB_REPORT_LEN));
    }

    #[test]
    fn unknown_preset_returns_typed_invalid_color_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut svc = ControllerService::new(
            MockHidTransport::new(),
            AppConfig::default(),
            dir.path().join("config.json"),
        );

        let err = svc
            .apply_preset("non-existent-preset")
            .expect_err("unknown preset must error");
        assert!(matches!(err, AppError::InvalidColor(_)));
    }

    #[test]
    fn transient_hid_write_error_still_persists_config_fail_safely() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("config.json");
        let transport = MockHidTransport::new();
        transport.set_controllers(vec![attached_usb_controller()]);

        let mut svc =
            ControllerService::new(transport.clone(), AppConfig::default(), cfg_path.clone());
        assert!(svc.status().connected);

        transport.set_write_error(Some("USB cable bumped".to_string()));
        let saved = svc
            .set_lightbar_color(16, 200, 120, 75, true)
            .expect("transient HID write failure must still save fail-safely");
        assert_eq!(saved.color, RgbColor::new(16, 200, 120));
        assert_eq!(saved.brightness, 75);

        let reloaded = AppConfig::load_from_path(&cfg_path).expect("reload fail-safe config");
        assert_eq!(reloaded.color, RgbColor::new(16, 200, 120));
    }

    #[test]
    fn service_supports_effect_modes_triggers_and_profile_import_export() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("config.json");
        let transport = MockHidTransport::new();
        transport.set_controllers(vec![attached_usb_controller()]);

        let mut svc = ControllerService::new(transport.clone(), AppConfig::default(), cfg_path);
        let cfg = svc
            .set_effect_mode(LightingEffectMode::Spectrum)
            .expect("effect mode saved");
        assert_eq!(cfg.effect_mode, LightingEffectMode::Spectrum);

        let cfg = svc
            .set_adaptive_triggers(TriggerPreset::Rigid, TriggerPreset::WeaponClick)
            .expect("triggers saved");
        assert_eq!(cfg.left_trigger, TriggerPreset::Rigid);
        assert_eq!(cfg.right_trigger, TriggerPreset::WeaponClick);

        let exported = svc.export_profile_json().expect("export succeeds");
        assert!(exported.contains("\"weapon_click\""));

        let imported = svc.import_profile_json(&exported).expect("import succeeds");
        assert_eq!(imported.effect_mode, LightingEffectMode::Spectrum);
        assert_eq!(imported.right_trigger, TriggerPreset::WeaponClick);
    }
}

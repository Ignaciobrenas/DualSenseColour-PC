use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tracing::warn;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::hid::color::{find_preset, ColorPreset, MicLedState, RgbColor, PRESETS};
use crate::hid::monitor::{ConnectionStatus, ControllerEvent, ControllerMonitor};
use crate::hid::transport::{HidTransport, RealHidTransport};
use crate::tray::TrayHandles;

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
        let monitor = ControllerMonitor::new(
            transport,
            clean.to_output_state(),
            clean.apply_on_connect,
            clean.keep_alive_steam,
        );
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

        let output_state = self.config.to_output_state();
        self.monitor.apply_output_state(output_state)?;

        if persist {
            self.config.save_to_path(&self.config_path)?;
        }

        Ok(self.config.clone())
    }

    /// Updates player LED segment count (`0..=5`) and mic LED indicator state, persisting immediately.
    pub fn set_player_leds(
        &mut self,
        player_leds: u8,
        mic_led: MicLedState,
    ) -> AppResult<AppConfig> {
        self.config.player_leds = player_leds.min(5);
        self.config.mic_led = mic_led;

        let output_state = self.config.to_output_state();
        self.monitor.apply_output_state(output_state)?;
        self.config.save_to_path(&self.config_path)?;

        Ok(self.config.clone())
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

        let output_state = self.config.to_output_state();
        self.monitor.apply_output_state(output_state)?;
        self.config.save_to_path(&self.config_path)?;

        Ok(self.config.clone())
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
        self.monitor.apply_output_state(clean.to_output_state())?;
        self.config.save_to_path(&self.config_path)?;

        Ok(self.config.clone())
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

fn emit_config_update(app: &AppHandle, cfg: &AppConfig) {
    if let Err(err) = app.emit(EVENT_LIGHTBAR_COLOR, cfg) {
        warn!(%err, "No se pudo emitir el evento de color a la UI");
    }
}

/// Applies a preset from native system tray interactions and syncs listeners.
pub fn handle_tray_preset(app: &AppHandle, state: &AppState, preset_id: &str) {
    match state.with_service(|svc| svc.apply_preset(preset_id)) {
        Ok(cfg) => emit_config_update(app, &cfg),
        Err(err) => warn!(%err, preset_id, "Error aplicando preset desde la bandeja"),
    }
}

/// Spawns the 1Hz hotplug polling loop that updates tray status and emits webview events.
pub fn spawn_hotplug_monitor(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;

            let poll_state = state.clone();
            let poll_result = tokio::task::spawn_blocking(move || {
                poll_state.with_service(|svc| Ok((svc.poll_once(), svc.status())))
            })
            .await;

            let Ok(Ok((events, status))) = poll_result else {
                continue;
            };

            let status_changed = events.iter().any(|event| {
                matches!(
                    event,
                    ControllerEvent::Connected(_) | ControllerEvent::Disconnected
                )
            });

            if status_changed {
                if let Some(tray) = app.try_state::<TrayHandles>() {
                    tray.update_status(&status);
                }
                if let Err(err) = app.emit(EVENT_CONTROLLER_STATUS, &status) {
                    warn!(%err, "No se pudo emitir controller-status-changed");
                }
            }
        }
    });
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
    let cfg =
        state.with_service(|svc| svc.set_lightbar_color(r, g, b, brightness, should_persist))?;
    if should_persist {
        emit_config_update(&app, &cfg);
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
    let cfg = state.with_service(|svc| svc.set_player_leds(player_leds, mic_led))?;
    emit_config_update(&app, &cfg);
    Ok(cfg)
}

/// Applies one of the built-in color presets and persists the choice.
#[tauri::command]
pub fn apply_preset(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    preset_id: String,
) -> AppResult<AppConfig> {
    let cfg = state.with_service(|svc| svc.apply_preset(&preset_id))?;
    emit_config_update(&app, &cfg);
    Ok(cfg)
}

/// Turns off the touchpad lightbar without disconnecting the controller.
#[tauri::command]
pub fn turn_off_lightbar(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> AppResult<AppConfig> {
    let cfg = state.with_service(ControllerService::turn_off_lightbar)?;
    emit_config_update(&app, &cfg);
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
    let saved = state.with_service(|svc| svc.save_config(config))?;
    emit_config_update(&app, &saved);
    Ok(saved)
}

/// Returns the curated catalog of named color presets.
#[tauri::command]
#[must_use]
pub fn get_presets() -> Vec<ColorPreset> {
    PRESETS.to_vec()
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
}

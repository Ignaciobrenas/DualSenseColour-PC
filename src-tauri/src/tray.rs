use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IconMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tracing::warn;

use crate::config::AppConfig;
use crate::hid::color::PRESETS;
use crate::hid::monitor::ConnectionStatus;
use crate::hid::transport::TransportType;

pub const TRAY_ID: &str = "sensehue-tray";
pub const MENU_ID_STATUS: &str = "status";
pub const MENU_ID_COLOR_SUMMARY: &str = "color-summary";
pub const MENU_ID_TOGGLE_WINDOW: &str = "toggle-window";
pub const MENU_ID_RESYNC: &str = "resync-controller";
pub const MENU_ID_QUIT: &str = "quit";

pub const MENU_PRESET_PREFIX: &str = "preset:";
pub const MENU_BRIGHTNESS_PREFIX: &str = "brightness:";
pub const MENU_PLAYER_PREFIX: &str = "player:";
pub const MENU_ID_TOGGLE_APPLY_ON_CONNECT: &str = "toggle:apply-on-connect";
pub const MENU_ID_TOGGLE_STEAM_FIX: &str = "toggle:steam-fix";
pub const MENU_ID_TOGGLE_CLOSE_TO_TRAY: &str = "toggle:close-to-tray";

const SWATCH_SIZE: usize = 16;
const TRAY_ICON_SIZE: usize = 32;

/// Semantic actions decoded from native system tray menu interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    ToggleWindow,
    ResyncController,
    Quit,
    ApplyPreset(String),
    SetBrightness(u8),
    SetPlayerLeds(u8),
    ToggleApplyOnConnect,
    ToggleSteamFix,
    ToggleCloseToTray,
}

/// Parses a Tauri menu item ID into a strongly typed [`TrayAction`].
#[must_use]
pub fn parse_tray_menu_id(id: &str) -> Option<TrayAction> {
    match id {
        MENU_ID_TOGGLE_WINDOW => return Some(TrayAction::ToggleWindow),
        MENU_ID_RESYNC => return Some(TrayAction::ResyncController),
        MENU_ID_QUIT => return Some(TrayAction::Quit),
        MENU_ID_TOGGLE_APPLY_ON_CONNECT => return Some(TrayAction::ToggleApplyOnConnect),
        MENU_ID_TOGGLE_STEAM_FIX => return Some(TrayAction::ToggleSteamFix),
        MENU_ID_TOGGLE_CLOSE_TO_TRAY => return Some(TrayAction::ToggleCloseToTray),
        _ => {}
    }

    if let Some(preset_id) = id.strip_prefix(MENU_PRESET_PREFIX) {
        if !preset_id.is_empty() {
            return Some(TrayAction::ApplyPreset(preset_id.to_string()));
        }
    }

    if let Some(level_str) = id.strip_prefix(MENU_BRIGHTNESS_PREFIX) {
        if let Ok(level) = level_str.parse::<u8>() {
            if level <= 100 {
                return Some(TrayAction::SetBrightness(level));
            }
        }
    }

    if let Some(count_str) = id.strip_prefix(MENU_PLAYER_PREFIX) {
        if let Ok(count) = count_str.parse::<u8>() {
            if count <= 5 {
                return Some(TrayAction::SetPlayerLeds(count));
            }
        }
    }

    None
}

/// Formats the human-readable controller status shown at the top of the tray menu.
#[must_use]
pub fn format_tray_status(status: &ConnectionStatus) -> String {
    if !status.connected {
        return "Estado: Sin mando conectado".to_string();
    }

    let model = status.model.map_or("DualSense", |m| m.display_name());
    let link = match status.transport {
        Some(TransportType::Usb) => "USB",
        Some(TransportType::Bluetooth) => "Bluetooth",
        None => "HID",
    };

    format!("Estado: {model} ({link})")
}

/// Formats the active color and brightness line shown inside the tray header.
#[must_use]
pub fn format_tray_color_summary(config: &AppConfig) -> String {
    let hex = config.color.to_hex();
    format!("Luz activa: {hex} · Brillo {}%", config.brightness)
}

/// Formats the native Windows multi-line tooltip for the notification area icon.
#[must_use]
pub fn format_tray_tooltip(status: &ConnectionStatus, config: &AppConfig) -> String {
    let status_text = format_tray_status(status);
    let hex = config.color.to_hex();
    format!(
        "SenseHue\n{status_text}\nLuz: {hex} ({}%)",
        config.brightness
    )
}

/// Generates a crisp 16x16 RGBA rounded swatch icon for native context menu items.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
#[must_use]
pub fn build_color_swatch_icon(r: u8, g: u8, b: u8, is_off: bool) -> Image<'static> {
    let size = SWATCH_SIZE;
    let mut rgba = vec![0u8; size * size * 4];
    let center = (size as f32 - 1.0) / 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = (y * size + x) * 4;

            if dist > 6.6 {
                continue;
            }

            if dist > 5.4 {
                // Crisp specular border
                let rim_alpha = ((6.6 - dist) / 1.2 * 210.0).clamp(0.0, 255.0) as u8;
                rgba[idx] = 226;
                rgba[idx + 1] = 232;
                rgba[idx + 2] = 240;
                rgba[idx + 3] = rim_alpha;
            } else if is_off {
                rgba[idx] = 30;
                rgba[idx + 1] = 36;
                rgba[idx + 2] = 48;
                rgba[idx + 3] = 255;
            } else {
                // Subtle top-left highlight for jewel-like Windows 11 swatch depth
                let boost = if dx + dy < -1.5 { 24 } else { 0 };
                rgba[idx] = r.saturating_add(boost);
                rgba[idx + 1] = g.saturating_add(boost);
                rgba[idx + 2] = b.saturating_add(boost);
                rgba[idx + 3] = 255;
            }
        }
    }

    Image::new_owned(rgba, size as u32, size as u32)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn blend_pixel(rgba: &mut [u8], size: usize, x: usize, y: usize, color: [u8; 4]) {
    if x >= size || y >= size {
        return;
    }
    let idx = (y * size + x) * 4;
    let src_a = f32::from(color[3]) / 255.0;
    let dst_a = f32::from(rgba[idx + 3]) / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.001 {
        return;
    }

    for channel in 0..3 {
        let s = f32::from(color[channel]) * src_a;
        let d = f32::from(rgba[idx + channel]) * dst_a * (1.0 - src_a);
        rgba[idx + channel] = ((s + d) / out_a).clamp(0.0, 255.0) as u8;
    }
    rgba[idx + 3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
}

/// Generates a 32x32 Windows 11 notification area controller icon with live RGB glow + status dot.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
#[must_use]
pub fn build_dynamic_tray_icon(
    r: u8,
    g: u8,
    b: u8,
    brightness: u8,
    connected: bool,
) -> Image<'static> {
    let size = TRAY_ICON_SIZE;
    let mut rgba = vec![0u8; size * size * 4];

    let scale = f32::from(brightness.min(100)) / 100.0;
    let is_lit = scale > 0.0 && (r > 0 || g > 0 || b > 0);
    let led_r = if is_lit {
        (f32::from(r) * (0.35 + 0.65 * scale)).clamp(0.0, 255.0) as u8
    } else {
        110
    };
    let led_g = if is_lit {
        (f32::from(g) * (0.35 + 0.65 * scale)).clamp(0.0, 255.0) as u8
    } else {
        125
    };
    let led_b = if is_lit {
        (f32::from(b) * (0.35 + 0.65 * scale)).clamp(0.0, 255.0) as u8
    } else {
        150
    };

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;

            // Stylized DualSense silhouette (central body + left/right ergonomic grips)
            let in_center = (8.0..=23.0).contains(&fx) && (7.0..=21.0).contains(&fy);
            let left_grip = ((fx - 7.5) / 4.2).powi(2) + ((fy - 18.0) / 8.5).powi(2) <= 1.0;
            let right_grip = ((fx - 23.5) / 4.2).powi(2) + ((fy - 18.0) / 8.5).powi(2) <= 1.0;
            let left_shoulder = ((fx - 8.5) / 4.8).powi(2) + ((fy - 10.5) / 4.2).powi(2) <= 1.0;
            let right_shoulder = ((fx - 22.5) / 4.8).powi(2) + ((fy - 10.5) / 4.2).powi(2) <= 1.0;

            if in_center || left_grip || right_grip || left_shoulder || right_shoulder {
                // Dark slate shell on top/inner touchpad area, subtle white DualSense outer shell
                let in_touchpad = (10.5..=20.5).contains(&fx) && (8.5..=16.5).contains(&fy);
                let in_lightbar_v = ((9.5..=11.2).contains(&fx) || (19.8..=21.5).contains(&fx))
                    && (8.0..=16.5).contains(&fy);
                let in_lightbar_bottom = (10.0..=21.0).contains(&fx) && (16.0..=17.3).contains(&fy);

                if in_lightbar_v || in_lightbar_bottom {
                    blend_pixel(&mut rgba, size, x, y, [led_r, led_g, led_b, 255]);
                } else if in_touchpad {
                    blend_pixel(&mut rgba, size, x, y, [22, 26, 36, 255]);
                } else {
                    let shade = if fy < 14.0 { 238 } else { 208 };
                    blend_pixel(&mut rgba, size, x, y, [shade, shade + 4, 248, 255]);
                }
            }

            // Soft RGB aura above touchpad when lightbar is illuminated
            if is_lit {
                let d_glow = ((fx - 15.5) / 8.0).powi(2) + ((fy - 12.0) / 7.5).powi(2);
                if d_glow > 0.35 && d_glow < 1.15 && rgba[(y * size + x) * 4 + 3] == 0 {
                    let glow_alpha = ((1.15 - d_glow) * 140.0 * scale).clamp(0.0, 180.0) as u8;
                    blend_pixel(&mut rgba, size, x, y, [r, g, b, glow_alpha]);
                }
            }

            // Bottom-right native connection indicator dot (Emerald when connected, Slate when idle)
            let d_dot = ((fx - 26.0).powi(2) + (fy - 26.0).powi(2)).sqrt();
            if d_dot <= 4.3 {
                if d_dot > 3.1 {
                    blend_pixel(&mut rgba, size, x, y, [15, 23, 42, 255]);
                } else if connected {
                    blend_pixel(&mut rgba, size, x, y, [16, 185, 129, 255]);
                } else {
                    blend_pixel(&mut rgba, size, x, y, [100, 116, 139, 255]);
                }
            }
        }
    }

    Image::new_owned(rgba, size as u32, size as u32)
}

/// Holds the active tray icon, status/color headers, and checkable settings items for live updates.
pub struct TrayHandles {
    status_item: MenuItem<Wry>,
    color_item: MenuItem<Wry>,
    apply_on_connect_item: CheckMenuItem<Wry>,
    steam_fix_item: CheckMenuItem<Wry>,
    close_to_tray_item: CheckMenuItem<Wry>,
    tray: TrayIcon<Wry>,
}

impl TrayHandles {
    /// Updates the status label text inside the system tray context menu.
    pub fn update_status(&self, status: &ConnectionStatus) {
        let text = format_tray_status(status);
        if let Err(err) = self.status_item.set_text(text) {
            warn!(%err, "No se pudo actualizar el texto de estado en el tray");
        }
    }

    /// Synchronizes icon color, multi-line tooltip, status headers, and checkmarks with current state.
    pub fn update_state(&self, status: &ConnectionStatus, config: &AppConfig) {
        self.update_status(status);

        if let Err(err) = self.color_item.set_text(format_tray_color_summary(config)) {
            warn!(%err, "No se pudo actualizar el resumen de luz en el tray");
        }
        if let Err(err) = self
            .apply_on_connect_item
            .set_checked(config.apply_on_connect)
        {
            warn!(%err, "No se pudo actualizar check Auto-aplicar en el tray");
        }
        if let Err(err) = self.steam_fix_item.set_checked(config.keep_alive_steam) {
            warn!(%err, "No se pudo actualizar check Steam Fix en el tray");
        }
        if let Err(err) = self.close_to_tray_item.set_checked(config.close_to_tray) {
            warn!(%err, "No se pudo actualizar check Minimizar a bandeja");
        }
        if let Err(err) = self
            .tray
            .set_tooltip(Some(format_tray_tooltip(status, config)))
        {
            warn!(%err, "No se pudo actualizar tooltip nativo del tray");
        }

        let icon = build_dynamic_tray_icon(
            config.color.r,
            config.color.g,
            config.color.b,
            config.brightness,
            status.connected,
        );
        if let Err(err) = self.tray.set_icon(Some(icon)) {
            warn!(%err, "No se pudo actualizar el icono RGB del tray");
        }
    }
}

/// Shows and focuses the main window, or hides it back into the system tray.
pub fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    match window.is_visible() {
        Ok(true) => {
            if let Err(err) = window.hide() {
                warn!(%err, "No se pudo ocultar la ventana al tray");
            }
        }
        Ok(false) => {
            let _ = window.unminimize();
            if let Err(err) = window.show() {
                warn!(%err, "No se pudo mostrar la ventana desde el tray");
            }
            let _ = window.set_focus();
        }
        Err(err) => warn!(%err, "Error consultando visibilidad de la ventana principal"),
    }
}

/// Intercepts window close requests and hides to tray when `close_to_tray` is active.
pub fn handle_close_requested(
    window: &tauri::Window,
    api: &tauri::CloseRequestApi,
    close_to_tray: bool,
) {
    if close_to_tray {
        api.prevent_close();
        if let Err(err) = window.hide() {
            warn!(%err, "No se pudo ocultar la ventana al cerrar hacia el tray");
        }
    }
}

/// Constructs the rich native Windows 11 tray menu, icon swatches, submenus, and live bindings.
pub fn build_tray<F>(
    app: &AppHandle,
    initial_status: &ConnectionStatus,
    initial_config: &AppConfig,
    on_action: F,
) -> tauri::Result<TrayHandles>
where
    F: Fn(&AppHandle, TrayAction) + Send + Sync + 'static,
{
    let status_item = MenuItem::with_id(
        app,
        MENU_ID_STATUS,
        format_tray_status(initial_status),
        false,
        None::<&str>,
    )?;
    let color_item = MenuItem::with_id(
        app,
        MENU_ID_COLOR_SUMMARY,
        format_tray_color_summary(initial_config),
        false,
        None::<&str>,
    )?;
    let sep_header = PredefinedMenuItem::separator(app)?;

    let toggle_item = MenuItem::with_id(
        app,
        MENU_ID_TOGGLE_WINDOW,
        "Abrir / Ocultar SenseHue",
        true,
        None::<&str>,
    )?;
    let resync_item = MenuItem::with_id(
        app,
        MENU_ID_RESYNC,
        "Sincronizar mando ahora",
        true,
        None::<&str>,
    )?;
    let sep_actions = PredefinedMenuItem::separator(app)?;

    let mut preset_items: Vec<IconMenuItem<Wry>> = Vec::with_capacity(PRESETS.len());
    for preset in PRESETS.iter().filter(|p| p.id != "off") {
        let id = format!("{MENU_PRESET_PREFIX}{}", preset.id);
        let swatch = build_color_swatch_icon(preset.color.r, preset.color.g, preset.color.b, false);
        preset_items.push(IconMenuItem::with_id(
            app,
            id,
            preset.name,
            true,
            Some(swatch),
            None::<&str>,
        )?);
    }

    let off_item = IconMenuItem::with_id(
        app,
        format!("{MENU_PRESET_PREFIX}off"),
        "Apagar barra LED",
        true,
        Some(build_color_swatch_icon(30, 36, 48, true)),
        None::<&str>,
    )?;
    let sep_controls = PredefinedMenuItem::separator(app)?;

    let b100 = MenuItem::with_id(app, "brightness:100", "100% (Máximo)", true, None::<&str>)?;
    let b75 = MenuItem::with_id(app, "brightness:75", "75% (Alto)", true, None::<&str>)?;
    let b50 = MenuItem::with_id(app, "brightness:50", "50% (Medio)", true, None::<&str>)?;
    let b25 = MenuItem::with_id(app, "brightness:25", "25% (Bajo)", true, None::<&str>)?;
    let b0 = MenuItem::with_id(app, "brightness:0", "0% (Apagado)", true, None::<&str>)?;
    let brightness_menu =
        Submenu::with_items(app, "Brillo rápido", true, &[&b100, &b75, &b50, &b25, &b0])?;

    let p0 = MenuItem::with_id(app, "player:0", "Apagados (0)", true, None::<&str>)?;
    let p1 = MenuItem::with_id(app, "player:1", "Jugador 1", true, None::<&str>)?;
    let p2 = MenuItem::with_id(app, "player:2", "Jugador 2", true, None::<&str>)?;
    let p3 = MenuItem::with_id(app, "player:3", "Jugador 3", true, None::<&str>)?;
    let p4 = MenuItem::with_id(app, "player:4", "Jugador 4", true, None::<&str>)?;
    let p5 = MenuItem::with_id(app, "player:5", "Jugador 5", true, None::<&str>)?;
    let player_menu = Submenu::with_items(
        app,
        "LEDs de jugador",
        true,
        &[&p0, &p1, &p2, &p3, &p4, &p5],
    )?;

    let apply_on_connect_item = CheckMenuItem::with_id(
        app,
        MENU_ID_TOGGLE_APPLY_ON_CONNECT,
        "Auto-aplicar al conectar",
        true,
        initial_config.apply_on_connect,
        None::<&str>,
    )?;
    let steam_fix_item = CheckMenuItem::with_id(
        app,
        MENU_ID_TOGGLE_STEAM_FIX,
        "Steam Keep-Alive Fix",
        true,
        initial_config.keep_alive_steam,
        None::<&str>,
    )?;
    let close_to_tray_item = CheckMenuItem::with_id(
        app,
        MENU_ID_TOGGLE_CLOSE_TO_TRAY,
        "Minimizar a bandeja al cerrar",
        true,
        initial_config.close_to_tray,
        None::<&str>,
    )?;
    let options_menu = Submenu::with_items(
        app,
        "Opciones en segundo plano",
        true,
        &[&apply_on_connect_item, &steam_fix_item, &close_to_tray_item],
    )?;

    let sep_exit = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, MENU_ID_QUIT, "Salir de SenseHue", true, None::<&str>)?;

    let mut menu_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = vec![
        &status_item,
        &color_item,
        &sep_header,
        &toggle_item,
        &resync_item,
        &sep_actions,
    ];
    for item in &preset_items {
        menu_refs.push(item);
    }
    menu_refs.push(&off_item);
    menu_refs.push(&sep_controls);
    menu_refs.push(&brightness_menu);
    menu_refs.push(&player_menu);
    menu_refs.push(&options_menu);
    menu_refs.push(&sep_exit);
    menu_refs.push(&quit_item);

    let menu = Menu::with_items(app, &menu_refs)?;

    let initial_icon = build_dynamic_tray_icon(
        initial_config.color.r,
        initial_config.color.g,
        initial_config.color.b,
        initial_config.brightness,
        initial_status.connected,
    );

    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(initial_icon)
        .tooltip(format_tray_tooltip(initial_status, initial_config))
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let Some(action) = parse_tray_menu_id(event.id().as_ref()) else {
                return;
            };
            on_action(app, action);
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(TrayHandles {
        status_item,
        color_item,
        apply_on_connect_item,
        steam_fix_item,
        close_to_tray_item,
        tray,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::color::RgbColor;
    use crate::hid::transport::DeviceModel;

    #[test]
    fn parses_tray_menu_ids_reliably() {
        assert_eq!(
            parse_tray_menu_id("toggle-window"),
            Some(TrayAction::ToggleWindow)
        );
        assert_eq!(
            parse_tray_menu_id("resync-controller"),
            Some(TrayAction::ResyncController)
        );
        assert_eq!(parse_tray_menu_id("quit"), Some(TrayAction::Quit));
        assert_eq!(
            parse_tray_menu_id("preset:cyber-cyan"),
            Some(TrayAction::ApplyPreset("cyber-cyan".to_string()))
        );
        assert_eq!(
            parse_tray_menu_id("brightness:75"),
            Some(TrayAction::SetBrightness(75))
        );
        assert_eq!(parse_tray_menu_id("brightness:105"), None);
        assert_eq!(
            parse_tray_menu_id("player:3"),
            Some(TrayAction::SetPlayerLeds(3))
        );
        assert_eq!(parse_tray_menu_id("player:9"), None);
        assert_eq!(
            parse_tray_menu_id("toggle:apply-on-connect"),
            Some(TrayAction::ToggleApplyOnConnect)
        );
        assert_eq!(
            parse_tray_menu_id("toggle:steam-fix"),
            Some(TrayAction::ToggleSteamFix)
        );
        assert_eq!(
            parse_tray_menu_id("toggle:close-to-tray"),
            Some(TrayAction::ToggleCloseToTray)
        );
        assert_eq!(parse_tray_menu_id("preset:"), None);
        assert_eq!(parse_tray_menu_id("unknown-item"), None);
    }

    #[test]
    fn formats_tray_status_and_summary_reliably() {
        assert_eq!(
            format_tray_status(&ConnectionStatus::default()),
            "Estado: Sin mando conectado"
        );

        let usb_status = ConnectionStatus {
            connected: true,
            transport: Some(TransportType::Usb),
            model: Some(DeviceModel::DualSense),
            serial_number: None,
            product_name: None,
        };
        assert_eq!(format_tray_status(&usb_status), "Estado: DualSense (USB)");

        let cfg = AppConfig {
            color: RgbColor::new(0, 240, 255),
            brightness: 80,
            ..Default::default()
        };
        assert_eq!(
            format_tray_color_summary(&cfg),
            "Luz activa: #00F0FF · Brillo 80%"
        );

        let tooltip = format_tray_tooltip(&usb_status, &cfg);
        assert!(tooltip.contains("DualSense (USB)"));
        assert!(tooltip.contains("#00F0FF (80%)"));
    }

    #[test]
    fn generates_valid_rgba_tray_and_swatch_images() {
        let swatch = build_color_swatch_icon(0, 67, 156, false);
        assert_eq!(swatch.width(), 16);
        assert_eq!(swatch.height(), 16);
        assert_eq!(swatch.rgba().len(), 16 * 16 * 4);

        let icon = build_dynamic_tray_icon(0, 240, 255, 100, true);
        assert_eq!(icon.width(), 32);
        assert_eq!(icon.height(), 32);
        assert_eq!(icon.rgba().len(), 32 * 32 * 4);
    }
}

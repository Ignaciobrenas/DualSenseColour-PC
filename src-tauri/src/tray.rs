use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tracing::warn;

use crate::hid::color::PRESETS;
use crate::hid::monitor::ConnectionStatus;
use crate::hid::transport::TransportType;

pub const TRAY_ID: &str = "dualsensecolour-tray";
pub const MENU_ID_STATUS: &str = "status";
pub const MENU_ID_TOGGLE_WINDOW: &str = "toggle-window";
pub const MENU_ID_QUIT: &str = "quit";
pub const MENU_PRESET_PREFIX: &str = "preset:";

/// Semantic actions decoded from native system tray menu interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    ToggleWindow,
    Quit,
    ApplyPreset(String),
}

/// Parses a Tauri menu item ID into a strongly typed [`TrayAction`].
#[must_use]
pub fn parse_tray_menu_id(id: &str) -> Option<TrayAction> {
    if id == MENU_ID_TOGGLE_WINDOW {
        return Some(TrayAction::ToggleWindow);
    }
    if id == MENU_ID_QUIT {
        return Some(TrayAction::Quit);
    }
    id.strip_prefix(MENU_PRESET_PREFIX)
        .filter(|preset_id| !preset_id.is_empty())
        .map(|preset_id| TrayAction::ApplyPreset(preset_id.to_string()))
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

/// Holds the active tray icon and dynamic status menu item for live updates.
pub struct TrayHandles {
    status_item: MenuItem<Wry>,
    _tray: TrayIcon<Wry>,
}

impl TrayHandles {
    /// Updates the status label text inside the system tray context menu.
    pub fn update_status(&self, status: &ConnectionStatus) {
        let text = format_tray_status(status);
        if let Err(err) = self.status_item.set_text(text) {
            warn!(%err, "No se pudo actualizar el texto de estado en el tray");
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

/// Constructs the native tray menu, binds quick presets + window controls, and mounts the tray icon.
pub fn build_tray<F>(app: &AppHandle, on_preset: F) -> tauri::Result<TrayHandles>
where
    F: Fn(&AppHandle, &str) + Send + Sync + 'static,
{
    let status_item = MenuItem::with_id(
        app,
        MENU_ID_STATUS,
        format_tray_status(&ConnectionStatus::default()),
        false,
        None::<&str>,
    )?;
    let sep_top = PredefinedMenuItem::separator(app)?;

    let mut preset_items: Vec<MenuItem<Wry>> = Vec::with_capacity(PRESETS.len());
    for preset in PRESETS.iter().filter(|p| p.id != "off") {
        let id = format!("{MENU_PRESET_PREFIX}{}", preset.id);
        let label = format!("● {}", preset.name);
        preset_items.push(MenuItem::with_id(app, id, label, true, None::<&str>)?);
    }

    let off_item = MenuItem::with_id(
        app,
        format!("{MENU_PRESET_PREFIX}off"),
        "Apagar LED",
        true,
        None::<&str>,
    )?;
    let sep_bottom = PredefinedMenuItem::separator(app)?;
    let toggle_item = MenuItem::with_id(
        app,
        MENU_ID_TOGGLE_WINDOW,
        "Mostrar / Ocultar ventana",
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, MENU_ID_QUIT, "Salir", true, None::<&str>)?;

    let mut menu_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> = Vec::new();
    menu_refs.push(&status_item);
    menu_refs.push(&sep_top);
    for item in &preset_items {
        menu_refs.push(item);
    }
    menu_refs.push(&off_item);
    menu_refs.push(&sep_bottom);
    menu_refs.push(&toggle_item);
    menu_refs.push(&quit_item);

    let menu = Menu::with_items(app, &menu_refs)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("DualSenseColour PC")
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let Some(action) = parse_tray_menu_id(event.id().as_ref()) else {
                return;
            };
            match action {
                TrayAction::ToggleWindow => toggle_main_window(app),
                TrayAction::Quit => app.exit(0),
                TrayAction::ApplyPreset(preset_id) => on_preset(app, &preset_id),
            }
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
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    let tray = builder.build(app)?;

    Ok(TrayHandles {
        status_item,
        _tray: tray,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::transport::DeviceModel;

    #[test]
    fn parses_tray_menu_ids_reliably() {
        assert_eq!(
            parse_tray_menu_id("toggle-window"),
            Some(TrayAction::ToggleWindow)
        );
        assert_eq!(parse_tray_menu_id("quit"), Some(TrayAction::Quit));
        assert_eq!(
            parse_tray_menu_id("preset:cyber-cyan"),
            Some(TrayAction::ApplyPreset("cyber-cyan".to_string()))
        );
        assert_eq!(
            parse_tray_menu_id("preset:off"),
            Some(TrayAction::ApplyPreset("off".to_string()))
        );
        assert_eq!(parse_tray_menu_id("preset:"), None);
        assert_eq!(parse_tray_menu_id("unknown-item"), None);
    }

    #[test]
    fn formats_tray_status_for_disconnected_and_connected_states() {
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

        let bt_edge_status = ConnectionStatus {
            connected: true,
            transport: Some(TransportType::Bluetooth),
            model: Some(DeviceModel::DualSenseEdge),
            serial_number: None,
            product_name: None,
        };
        assert_eq!(
            format_tray_status(&bt_edge_status),
            "Estado: DualSense Edge (Bluetooth)"
        );
    }
}

pub mod commands;
pub mod config;
pub mod error;
pub mod hid;
pub mod tray;

use tauri::{Manager, WindowEvent};
use tracing::warn;

#[cfg(all(target_os = "windows", target_env = "gnu"))]
#[allow(non_snake_case, non_upper_case_globals)]
mod gnu_webview2_shim {
    use std::ffi::c_void;

    extern "C" {
        fn malloc(size: usize) -> *mut c_void;
        fn free(ptr: *mut c_void);
    }

    #[no_mangle]
    pub static mut _Init_thread_epoch: i32 = i32::MIN;

    #[no_mangle]
    pub static mut _tls_index: u32 = 0;

    #[no_mangle]
    pub static mut __security_cookie: usize = 0x0000_2B99_2DDF_A232;

    #[no_mangle]
    pub unsafe extern "C" fn __security_check_cookie(_cookie: usize) {}

    #[repr(C)]
    pub struct StdNothrow {
        _unused: u8,
    }

    // SAFETY: `StdNothrow` is an immutable empty marker matching `std::nothrow` in MSVC C++.
    unsafe impl Sync for StdNothrow {}

    #[export_name = "?nothrow@std@@3Unothrow_t@1@B"]
    pub static STD_NOTHROW: StdNothrow = StdNothrow { _unused: 0 };

    #[no_mangle]
    pub unsafe extern "C" fn _Init_thread_header(p_once: *mut i32) {
        if !p_once.is_null() {
            // Simple single-check initialization guard sufficient for static WebView2Loader init.
            if p_once.read_volatile() == 0 {
                p_once.write_volatile(-1);
            }
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn _Init_thread_footer(p_once: *mut i32) {
        if !p_once.is_null() {
            p_once.write_volatile(i32::MAX);
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn _Init_thread_abort(p_once: *mut i32) {
        if !p_once.is_null() {
            p_once.write_volatile(0);
        }
    }

    #[export_name = "??2@YAPEAX_KAEBUnothrow_t@std@@@Z"]
    pub unsafe extern "C" fn cpp_new_nothrow(
        size: usize,
        _nothrow: *const StdNothrow,
    ) -> *mut c_void {
        malloc(size)
    }

    #[export_name = "??3@YAXPEAX_K@Z"]
    pub unsafe extern "C" fn cpp_delete_sized(ptr: *mut c_void, _size: usize) {
        free(ptr);
    }

    #[export_name = "??_U@YAPEAX_K@Z"]
    pub unsafe extern "C" fn cpp_new_array(size: usize) -> *mut c_void {
        malloc(size)
    }

    #[export_name = "??_V@YAXPEAX@Z"]
    pub unsafe extern "C" fn cpp_delete_array(ptr: *mut c_void) {
        free(ptr);
    }
}

/// Runs the Tauri v2 application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dualsensecolour_pc=info".into()),
        )
        .init();

    let config_path = config::AppConfig::default_config_path()
        .unwrap_or_else(|_| std::env::temp_dir().join("dualsensecolour-config.json"));
    let initial_config = config::AppConfig::load_from_path(&config_path).unwrap_or_default();
    let app_state = commands::AppState::new(initial_config, config_path);

    tauri::Builder::default()
        .manage(app_state.clone())
        .setup(move |app| {
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                if let Err(err) = window_vibrancy::apply_acrylic(&window, Some((10, 12, 18, 196))) {
                    warn!(%err, "Acrylic no disponible; intentando Mica");
                    if let Err(mica_err) = window_vibrancy::apply_mica(&window, Some(true)) {
                        warn!(%mica_err, "Mica tampoco disponible; manteniendo glassmorphism CSS");
                    }
                }
            }

            let tray_state = app_state.clone();
            let tray_handles = tray::build_tray(app.handle(), move |handle, preset_id| {
                commands::handle_tray_preset(handle, &tray_state, preset_id);
            })?;
            app.manage(tray_handles);

            commands::spawn_hotplug_monitor(app.handle().clone(), app_state.clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let close_to_tray = window
                        .app_handle()
                        .try_state::<commands::AppState>()
                        .is_none_or(|state| state.should_close_to_tray());
                    tray::handle_close_requested(window, api, close_to_tray);
                } else {
                    warn!(label = window.label(), "Cerrando ventana secundaria");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_lightbar_color,
            commands::set_player_leds,
            commands::apply_preset,
            commands::turn_off_lightbar,
            commands::get_controller_status,
            commands::get_config,
            commands::save_config,
            commands::get_presets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DualSenseColour PC");
}

#[cfg(test)]
mod tests {
    use super::error::AppError;

    #[test]
    fn error_serializes_as_message() {
        let err = AppError::InvalidColor("#ZZZ".to_string());
        let json = serde_json::to_string(&err).expect("error serializes");
        assert!(json.contains("#ZZZ"));
    }
}

use std::ffi::CString;
use std::sync::{Arc, Mutex};

use hidapi::{BusType, DeviceInfo, HidApi, HidDevice, HidError};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::{AppError, AppResult};

use super::protocol::{build_bt_reset_leds_report, is_supported_controller, DUALSENSE_EDGE_PID};

/// Physical or wireless link used by a connected DualSense controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Usb,
    Bluetooth,
}

/// Sony DualSense hardware model detected over HID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceModel {
    DualSense,
    DualSenseEdge,
}

impl DeviceModel {
    /// Derives the controller model from a Sony USB/Bluetooth Product ID.
    #[must_use]
    pub const fn from_pid(pid: u16) -> Self {
        if pid == DUALSENSE_EDGE_PID {
            Self::DualSenseEdge
        } else {
            Self::DualSense
        }
    }

    /// Human-readable hardware name shown in the UI and system tray.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::DualSense => "DualSense",
            Self::DualSenseEdge => "DualSense Edge",
        }
    }
}

/// Snapshot metadata describing a detected DualSense HID endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedController {
    pub path: String,
    pub model: DeviceModel,
    pub transport: TransportType,
    pub serial_number: Option<String>,
    pub product_name: Option<String>,
    pub interface_number: i32,
}

/// Pure classifier determining whether a controller interface runs over USB or Bluetooth.
#[must_use]
pub fn detect_transport_type(
    bus_type: Option<BusType>,
    interface_number: i32,
    path: &str,
) -> TransportType {
    match bus_type {
        Some(BusType::Bluetooth) => TransportType::Bluetooth,
        Some(BusType::Usb) => TransportType::Usb,
        Some(BusType::Unknown | BusType::I2c | BusType::Spi) | None => {
            let lower = path.to_ascii_lowercase();
            if interface_number == -1 || lower.contains("bthenum") || lower.contains("00001124") {
                TransportType::Bluetooth
            } else {
                TransportType::Usb
            }
        }
    }
}

/// Filters out secondary audio/vendor HID collections on Windows when enumerated alongside gamepad HID.
#[must_use]
pub const fn is_primary_hid_interface(usage_page: u16, usage: u16, interface_number: i32) -> bool {
    // Linux hidraw typically reports 0 for usage/usage_page; USB controller HID lives on interface 3 or -1 (BT).
    if usage_page == 0 && usage == 0 {
        return interface_number == 3 || interface_number == -1;
    }
    usage_page == 0x0001 && (usage == 0x0005 || usage == 0x0004)
}

/// Abstraction over HID hardware enumeration and I/O so monitoring and IPC can be tested offline.
pub trait HidTransport: Send + Sync {
    /// Lists all currently reachable DualSense / DualSense Edge controllers.
    fn enumerate_controllers(&self) -> AppResult<Vec<DetectedController>>;

    /// Writes a raw HID Output Report (`0x02` for USB or `0x31` for Bluetooth) to `path`.
    fn send_report(&self, path: &str, report: &[u8]) -> AppResult<usize>;

    /// Reads Feature Report `0x05` (or `0x20`) to switch a Bluetooth DualSense into full report mode.
    fn wake_bluetooth_controller(&self, path: &str) -> AppResult<()>;
}

/// Production [`HidTransport`] backed by `hidapi`.
#[derive(Debug, Default)]
pub struct RealHidTransport;

impl RealHidTransport {
    /// Creates a new hardware HID transport instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn map_hid_error(err: &HidError) -> AppError {
        let msg = err.to_string();
        let lower = msg.to_ascii_lowercase();
        if lower.contains("permission") || lower.contains("denied") || lower.contains("access") {
            AppError::PermissionDenied(msg)
        } else {
            AppError::HidIo(msg)
        }
    }

    fn open_device_by_path(api: &HidApi, path: &str) -> AppResult<HidDevice> {
        let c_path = CString::new(path)
            .map_err(|e| AppError::HidIo(format!("Ruta HID inválida '{path}': {e}")))?;
        api.open_path(c_path.as_c_str())
            .map_err(|e| Self::map_hid_error(&e))
    }

    fn device_to_detected(info: &DeviceInfo) -> Option<DetectedController> {
        if !is_supported_controller(info.vendor_id(), info.product_id()) {
            return None;
        }
        if !is_primary_hid_interface(info.usage_page(), info.usage(), info.interface_number()) {
            return None;
        }

        let path = info.path().to_string_lossy().into_owned();
        let transport =
            detect_transport_type(Some(info.bus_type()), info.interface_number(), &path);
        let model = DeviceModel::from_pid(info.product_id());

        Some(DetectedController {
            path,
            model,
            transport,
            serial_number: info.serial_number().map(str::to_owned),
            product_name: info.product_string().map(str::to_owned),
            interface_number: info.interface_number(),
        })
    }
}

impl HidTransport for RealHidTransport {
    fn enumerate_controllers(&self) -> AppResult<Vec<DetectedController>> {
        let api = HidApi::new().map_err(|e| Self::map_hid_error(&e))?;
        let mut controllers: Vec<DetectedController> = api
            .device_list()
            .filter_map(Self::device_to_detected)
            .collect();

        // Fallback if OS reports non-standard usage/interface numbers on a supported Sony VID/PID.
        if controllers.is_empty() {
            controllers = api
                .device_list()
                .filter(|info| is_supported_controller(info.vendor_id(), info.product_id()))
                .map(|info| {
                    let path = info.path().to_string_lossy().into_owned();
                    DetectedController {
                        transport: detect_transport_type(
                            Some(info.bus_type()),
                            info.interface_number(),
                            &path,
                        ),
                        path,
                        model: DeviceModel::from_pid(info.product_id()),
                        serial_number: info.serial_number().map(str::to_owned),
                        product_name: info.product_string().map(str::to_owned),
                        interface_number: info.interface_number(),
                    }
                })
                .collect();
        }

        Ok(controllers)
    }

    fn send_report(&self, path: &str, report: &[u8]) -> AppResult<usize> {
        let api = HidApi::new().map_err(|e| Self::map_hid_error(&e))?;
        let device = Self::open_device_by_path(&api, path)?;
        device.write(report).map_err(|e| Self::map_hid_error(&e))
    }

    fn wake_bluetooth_controller(&self, path: &str) -> AppResult<()> {
        let api = HidApi::new().map_err(|e| Self::map_hid_error(&e))?;
        let device = Self::open_device_by_path(&api, path)?;

        let mut feature_05 = [0u8; 64];
        feature_05[0] = 0x05;
        if let Err(err_05) = device.get_feature_report(&mut feature_05) {
            warn!(%err_05, "Feature report 0x05 no disponible; intentando 0x20");
            let mut feature_20 = [0u8; 64];
            feature_20[0] = 0x20;
            device
                .get_feature_report(&mut feature_20)
                .map_err(|e| Self::map_hid_error(&e))?;
        }

        let reset_report = build_bt_reset_leds_report(0);
        if let Err(err) = device.write(&reset_report) {
            warn!(%err, "Paquete inicial de desbloqueo de barra LED Bluetooth rechazado");
        } else {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        debug!(
            path,
            "Controlador DualSense Bluetooth despertado en modo extendido"
        );
        Ok(())
    }
}

/// Recorded HID packet produced during tests with [`MockHidTransport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedReport {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default)]
struct MockTransportState {
    controllers: Vec<DetectedController>,
    sent_reports: Vec<RecordedReport>,
    woken_paths: Vec<String>,
    fail_writes: Option<String>,
    fail_enumeration: Option<String>,
}

/// Deterministic in-memory [`HidTransport`] used by unit and integration tests.
#[derive(Debug, Clone, Default)]
pub struct MockHidTransport {
    state: Arc<Mutex<MockTransportState>>,
}

impl MockHidTransport {
    /// Creates an empty mock HID transport with no connected controllers.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the currently simulated connected controllers.
    pub fn set_controllers(&self, controllers: Vec<DetectedController>) {
        if let Ok(mut guard) = self.state.lock() {
            guard.controllers = controllers;
        }
    }

    /// Configures subsequent `send_report` calls to fail with `message` (or succeed if `None`).
    pub fn set_write_error(&self, message: Option<String>) {
        if let Ok(mut guard) = self.state.lock() {
            guard.fail_writes = message;
        }
    }

    /// Configures subsequent `enumerate_controllers` calls to fail (or succeed if `None`).
    pub fn set_enumeration_error(&self, message: Option<String>) {
        if let Ok(mut guard) = self.state.lock() {
            guard.fail_enumeration = message;
        }
    }

    /// Returns a copy of every HID packet sent through this mock transport.
    #[must_use]
    pub fn sent_reports(&self) -> Vec<RecordedReport> {
        self.state
            .lock()
            .map(|guard| guard.sent_reports.clone())
            .unwrap_or_default()
    }

    /// Returns all Bluetooth paths passed to `wake_bluetooth_controller`.
    #[must_use]
    pub fn woken_paths(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|guard| guard.woken_paths.clone())
            .unwrap_or_default()
    }

    /// Clears recorded reports and wake calls between test phases.
    pub fn clear_history(&self) {
        if let Ok(mut guard) = self.state.lock() {
            guard.sent_reports.clear();
            guard.woken_paths.clear();
        }
    }
}

impl HidTransport for MockHidTransport {
    fn enumerate_controllers(&self) -> AppResult<Vec<DetectedController>> {
        let guard = self
            .state
            .lock()
            .map_err(|_| AppError::HidIo("Mock mutex poisoned".to_string()))?;

        if let Some(err) = &guard.fail_enumeration {
            return Err(AppError::HidIo(err.clone()));
        }

        Ok(guard.controllers.clone())
    }

    fn send_report(&self, path: &str, report: &[u8]) -> AppResult<usize> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| AppError::HidIo("Mock mutex poisoned".to_string()))?;

        if let Some(err) = &guard.fail_writes {
            return Err(AppError::HidIo(err.clone()));
        }

        if !guard.controllers.iter().any(|c| c.path == path) {
            return Err(AppError::ControllerNotFound);
        }

        guard.sent_reports.push(RecordedReport {
            path: path.to_string(),
            bytes: report.to_vec(),
        });

        Ok(report.len())
    }

    fn wake_bluetooth_controller(&self, path: &str) -> AppResult<()> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| AppError::HidIo("Mock mutex poisoned".to_string()))?;

        if !guard.controllers.iter().any(|c| c.path == path) {
            return Err(AppError::ControllerNotFound);
        }

        guard.woken_paths.push(path.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_usb_controller() -> DetectedController {
        DetectedController {
            path: r"\\?\hid#vid_054c&pid_0ce6#mock".to_string(),
            model: DeviceModel::DualSense,
            transport: TransportType::Usb,
            serial_number: Some("AA:BB:CC:DD:EE:FF".to_string()),
            product_name: Some("DualSense Wireless Controller".to_string()),
            interface_number: 3,
        }
    }

    #[test]
    fn transport_detection_uses_bus_type_then_heuristics() {
        assert_eq!(
            detect_transport_type(Some(BusType::Bluetooth), 3, "usb-path"),
            TransportType::Bluetooth
        );
        assert_eq!(
            detect_transport_type(Some(BusType::Usb), -1, "bthenum"),
            TransportType::Usb
        );
        assert_eq!(
            detect_transport_type(Some(BusType::Unknown), -1, "/dev/hidraw4"),
            TransportType::Bluetooth
        );
        assert_eq!(
            detect_transport_type(
                None,
                0,
                r"\\?\hid#{00001124-0000-1000-8000-00805f9b34fb}&vid_054c"
            ),
            TransportType::Bluetooth
        );
        assert_eq!(
            detect_transport_type(None, 3, "/dev/hidraw2"),
            TransportType::Usb
        );
    }

    #[test]
    fn mock_transport_records_reports_and_simulates_errors() {
        let mock = MockHidTransport::new();
        let ctrl = sample_usb_controller();
        let path = ctrl.path.clone();

        mock.set_controllers(vec![ctrl]);
        assert_eq!(mock.enumerate_controllers().expect("enumerate").len(), 1);

        let written = mock.send_report(&path, &[0x02, 0x00, 0x04]).expect("write");
        assert_eq!(written, 3);
        assert_eq!(mock.sent_reports().len(), 1);
        assert_eq!(mock.sent_reports()[0].bytes, vec![0x02, 0x00, 0x04]);

        mock.set_write_error(Some("Simulated I/O failure".to_string()));
        assert!(matches!(
            mock.send_report(&path, &[0x02]),
            Err(AppError::HidIo(_))
        ));

        mock.set_write_error(None);
        mock.wake_bluetooth_controller(&path)
            .expect("wake recorded");
        assert_eq!(mock.woken_paths(), vec![path]);
    }
}

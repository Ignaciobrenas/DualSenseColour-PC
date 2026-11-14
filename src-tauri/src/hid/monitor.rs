use tracing::{debug, warn};

use crate::error::{AppError, AppResult};

use super::protocol::{build_bt_full_report, build_usb_full_report, DualSenseOutputState};
use super::transport::{DetectedController, DeviceModel, HidTransport, TransportType};

const KEEP_ALIVE_INTERVAL_TICKS: u32 = 5;

/// Snapshot of the active DualSense connection exposed to the UI and system tray.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub transport: Option<TransportType>,
    pub model: Option<DeviceModel>,
    pub serial_number: Option<String>,
    pub product_name: Option<String>,
}

impl ConnectionStatus {
    /// Builds a connected status snapshot from a [`DetectedController`].
    #[must_use]
    pub fn from_detected(controller: &DetectedController) -> Self {
        Self {
            connected: true,
            transport: Some(controller.transport),
            model: Some(controller.model),
            serial_number: controller.serial_number.clone(),
            product_name: controller.product_name.clone(),
        }
    }
}

/// High-level lifecycle events emitted by [`ControllerMonitor::poll_once`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerEvent {
    Connected(ConnectionStatus),
    Disconnected,
    ReappliedColor,
    TransportError(String),
}

/// Tracks controller hotplug transitions and automatically applies LED state over USB or Bluetooth.
pub struct ControllerMonitor<T: HidTransport> {
    transport: T,
    active_controller: Option<DetectedController>,
    output_state: DualSenseOutputState,
    bt_seq: u8,
    apply_on_connect: bool,
    keep_alive_steam: bool,
    ticks_since_keep_alive: u32,
}

impl<T: HidTransport> ControllerMonitor<T> {
    /// Creates a new monitor with an injected [`HidTransport`] and desired LED state.
    #[must_use]
    pub fn new(
        transport: T,
        initial_state: DualSenseOutputState,
        apply_on_connect: bool,
        keep_alive_steam: bool,
    ) -> Self {
        Self {
            transport,
            active_controller: None,
            output_state: initial_state,
            bt_seq: 0,
            apply_on_connect,
            keep_alive_steam,
            ticks_since_keep_alive: 0,
        }
    }

    /// Returns a reference to the underlying HID transport (useful in tests).
    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    /// Returns the current connection snapshot.
    #[must_use]
    pub fn status(&self) -> ConnectionStatus {
        self.active_controller
            .as_ref()
            .map_or_else(ConnectionStatus::default, ConnectionStatus::from_detected)
    }

    /// Returns the currently stored desired LED state.
    #[must_use]
    pub const fn output_state(&self) -> DualSenseOutputState {
        self.output_state
    }

    /// Updates auto-apply and Steam keep-alive behavior without resetting connection state.
    pub fn set_preferences(&mut self, apply_on_connect: bool, keep_alive_steam: bool) {
        self.apply_on_connect = apply_on_connect;
        self.keep_alive_steam = keep_alive_steam;
        if !keep_alive_steam {
            self.ticks_since_keep_alive = 0;
        }
    }

    /// Polls the HID transport once, detecting connect/disconnect/switch transitions and errors.
    pub fn poll_once(&mut self) -> Vec<ControllerEvent> {
        let controllers = match self.transport.enumerate_controllers() {
            Ok(list) => list,
            Err(err) => {
                warn!(%err, "Error enumerando controladores DualSense");
                return vec![ControllerEvent::TransportError(err.to_string())];
            }
        };

        // Prefer wired USB if both USB and Bluetooth interfaces appear simultaneously.
        let next = controllers
            .into_iter()
            .min_by_key(|controller| match controller.transport {
                TransportType::Usb => 0,
                TransportType::Bluetooth => 1,
            });

        let Some(detected) = next else {
            if self.active_controller.take().is_some() {
                debug!("Controlador DualSense desconectado");
                self.ticks_since_keep_alive = 0;
                return vec![ControllerEvent::Disconnected];
            }
            return Vec::new();
        };

        if Self::has_connection_changed(self.active_controller.as_ref(), &detected) {
            let mut events = Vec::new();
            if detected.transport == TransportType::Bluetooth {
                if let Err(err) = self.transport.wake_bluetooth_controller(&detected.path) {
                    warn!(%err, path = %detected.path, "No se pudo despertar el mando Bluetooth");
                    events.push(ControllerEvent::TransportError(err.to_string()));
                }
            }

            let status = ConnectionStatus::from_detected(&detected);
            self.active_controller = Some(detected.clone());
            self.ticks_since_keep_alive = 0;
            events.push(ControllerEvent::Connected(status));

            if self.apply_on_connect {
                let desired = self.output_state;
                match self.send_state_to_controller(&detected, &desired) {
                    Ok(_) => events.push(ControllerEvent::ReappliedColor),
                    Err(err) => {
                        warn!(%err, "Error reaplicando color al conectar");
                        events.push(ControllerEvent::TransportError(err.to_string()));
                    }
                }
            }

            return events;
        }

        if !self.keep_alive_steam {
            return Vec::new();
        }

        self.ticks_since_keep_alive += 1;
        if self.ticks_since_keep_alive < KEEP_ALIVE_INTERVAL_TICKS {
            return Vec::new();
        }

        self.ticks_since_keep_alive = 0;
        let desired = self.output_state;
        match self.send_state_to_controller(&detected, &desired) {
            Ok(_) => vec![ControllerEvent::ReappliedColor],
            Err(err) => vec![ControllerEvent::TransportError(err.to_string())],
        }
    }

    /// Stores `state` and immediately dispatches it if a controller is currently connected.
    ///
    /// Returns the number of bytes written (`0` if saved offline without an attached controller).
    pub fn apply_output_state(&mut self, state: DualSenseOutputState) -> AppResult<usize> {
        self.output_state = state;
        self.ticks_since_keep_alive = 0;

        let Some(controller) = self.active_controller.clone() else {
            return Ok(0);
        };

        self.send_state_to_controller(&controller, &state)
    }

    /// Requires an attached controller and writes the given state immediately.
    pub fn apply_output_state_strict(&mut self, state: DualSenseOutputState) -> AppResult<usize> {
        self.output_state = state;
        self.ticks_since_keep_alive = 0;

        let controller = self
            .active_controller
            .clone()
            .ok_or(AppError::ControllerNotFound)?;

        self.send_state_to_controller(&controller, &state)
    }

    fn has_connection_changed(
        prev: Option<&DetectedController>,
        next: &DetectedController,
    ) -> bool {
        match prev {
            None => true,
            Some(current) => {
                current.path != next.path
                    || current.transport != next.transport
                    || current.model != next.model
                    || current.serial_number != next.serial_number
            }
        }
    }

    fn send_state_to_controller(
        &mut self,
        controller: &DetectedController,
        state: &DualSenseOutputState,
    ) -> AppResult<usize> {
        match controller.transport {
            TransportType::Usb => {
                let report = build_usb_full_report(state);
                self.transport.send_report(&controller.path, &report)
            }
            TransportType::Bluetooth => {
                let report = build_bt_full_report(state, self.bt_seq);
                self.bt_seq = (self.bt_seq + 1) & 0x0F;
                self.transport.send_report(&controller.path, &report)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::color::{MicLedState, RgbColor};
    use crate::hid::protocol::{compute_dualsense_bt_crc32, BT_REPORT_LEN, USB_REPORT_LEN};
    use crate::hid::transport::MockHidTransport;

    fn sample_usb_controller() -> DetectedController {
        DetectedController {
            path: "usb\\dualsense\\001".to_string(),
            model: DeviceModel::DualSense,
            transport: TransportType::Usb,
            serial_number: Some("AA:BB:CC:DD:EE:01".to_string()),
            product_name: Some("DualSense Wireless Controller".to_string()),
            interface_number: 3,
        }
    }

    fn sample_bt_controller() -> DetectedController {
        DetectedController {
            path: "bt\\dualsense\\001".to_string(),
            model: DeviceModel::DualSenseEdge,
            transport: TransportType::Bluetooth,
            serial_number: Some("AA:BB:CC:DD:EE:01".to_string()),
            product_name: Some("DualSense Edge Wireless Controller".to_string()),
            interface_number: -1,
        }
    }

    #[test]
    fn hotplug_usb_disconnect_and_bt_reconnect_lifecycle() {
        let transport = MockHidTransport::new();
        let initial_state = DualSenseOutputState {
            color: RgbColor::new(0, 240, 255),
            brightness: 100,
            player_leds: 2,
            mic_led: MicLedState::Off,
        };
        let mut monitor = ControllerMonitor::new(transport.clone(), initial_state, true, false);

        // Phase 0: nothing connected yet
        assert!(monitor.poll_once().is_empty());
        assert!(!monitor.status().connected);

        // Phase 1: USB connect -> emits Connected(Usb) + sends 63B 0x02 report
        let usb = sample_usb_controller();
        transport.set_controllers(vec![usb.clone()]);
        let events = monitor.poll_once();
        assert_eq!(
            events,
            vec![
                ControllerEvent::Connected(ConnectionStatus::from_detected(&usb)),
                ControllerEvent::ReappliedColor,
            ]
        );
        assert!(monitor.status().connected);
        assert_eq!(monitor.status().transport, Some(TransportType::Usb));

        let sent = transport.sent_reports();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].path, usb.path);
        assert_eq!(sent[0].bytes.len(), USB_REPORT_LEN);
        assert_eq!(sent[0].bytes[0], 0x02);
        assert_eq!(&sent[0].bytes[45..=47], &[0, 240, 255]);

        // Phase 2: Disconnect -> emits Disconnected
        transport.clear_history();
        transport.set_controllers(Vec::new());
        let events = monitor.poll_once();
        assert_eq!(events, vec![ControllerEvent::Disconnected]);
        assert!(!monitor.status().connected);

        // Phase 3: Bluetooth reconnect -> wakes controller + emits Connected(Bt) + sends 78B 0x31 with valid CRC32
        let bt = sample_bt_controller();
        transport.set_controllers(vec![bt.clone()]);
        let events = monitor.poll_once();
        assert_eq!(
            events,
            vec![
                ControllerEvent::Connected(ConnectionStatus::from_detected(&bt)),
                ControllerEvent::ReappliedColor,
            ]
        );
        assert_eq!(transport.woken_paths(), vec![bt.path.clone()]);

        let sent = transport.sent_reports();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].bytes.len(), BT_REPORT_LEN);
        assert_eq!(sent[0].bytes[0], 0x31);
        assert_eq!(&sent[0].bytes[47..=49], &[0, 240, 255]);

        let expected_crc = compute_dualsense_bt_crc32(&sent[0].bytes[..74]);
        let actual_crc = u32::from_le_bytes([
            sent[0].bytes[74],
            sent[0].bytes[75],
            sent[0].bytes[76],
            sent[0].bytes[77],
        ]);
        assert_eq!(actual_crc, expected_crc);
    }

    #[test]
    fn simulated_write_error_surfaces_cleanly_without_panic() {
        let transport = MockHidTransport::new();
        transport.set_controllers(vec![sample_usb_controller()]);
        transport.set_write_error(Some("pipe disconnected".to_string()));

        let mut monitor = ControllerMonitor::new(
            transport.clone(),
            DualSenseOutputState::default(),
            true,
            false,
        );

        let events = monitor.poll_once();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], ControllerEvent::Connected(_)));
        assert!(matches!(events[1], ControllerEvent::TransportError(_)));

        let err = monitor
            .apply_output_state(DualSenseOutputState::default())
            .expect_err("write error must propagate cleanly");
        assert!(matches!(err, AppError::HidIo(_)));
    }

    #[test]
    fn steam_keep_alive_periodically_reapplies_report() {
        let transport = MockHidTransport::new();
        transport.set_controllers(vec![sample_usb_controller()]);

        let mut monitor = ControllerMonitor::new(
            transport.clone(),
            DualSenseOutputState::default(),
            true,
            true,
        );

        // Initial connect sends 1 report
        let _ = monitor.poll_once();
        assert_eq!(transport.sent_reports().len(), 1);

        // Next 4 idle ticks do not resend
        for _ in 0..(KEEP_ALIVE_INTERVAL_TICKS - 1) {
            assert!(monitor.poll_once().is_empty());
        }
        assert_eq!(transport.sent_reports().len(), 1);

        // 5th tick triggers periodic keep-alive resend
        let events = monitor.poll_once();
        assert_eq!(events, vec![ControllerEvent::ReappliedColor]);
        assert_eq!(transport.sent_reports().len(), 2);
    }
}

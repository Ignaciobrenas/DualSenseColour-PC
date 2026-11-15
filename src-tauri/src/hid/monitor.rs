use std::collections::{HashMap, HashSet};

use tracing::{debug, warn};

use crate::error::{AppError, AppResult};

use super::input_report::{parse_input_report, ChargingState, DualSenseTelemetry};
use super::protocol::{build_bt_full_report, build_usb_full_report, DualSenseOutputState};
use super::transport::{
    assign_controller_ids, controller_key, DetectedController, DeviceModel, HidTransport,
    TransportType,
};

const KEEP_ALIVE_INTERVAL_TICKS: u32 = 5;

/// Per-controller live hardware & battery snapshot inside [`ConnectionStatus`].
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConnectedControllerSummary {
    pub id: String,
    pub path: String,
    pub slot: usize,
    pub model: DeviceModel,
    pub transport: TransportType,
    pub serial_number: Option<String>,
    pub product_name: Option<String>,
    #[serde(default)]
    pub battery_percent: Option<u8>,
    #[serde(default)]
    pub charging_state: Option<ChargingState>,
    #[serde(default)]
    pub telemetry: Option<DualSenseTelemetry>,
}

impl ConnectedControllerSummary {
    /// Creates a summary snapshot for a detected controller slot.
    #[must_use]
    pub fn from_detected(controller: &DetectedController, slot: usize) -> Self {
        Self {
            id: controller_key(controller),
            path: controller.path.clone(),
            slot,
            model: controller.model,
            transport: controller.transport,
            serial_number: controller.serial_number.clone(),
            product_name: controller.product_name.clone(),
            battery_percent: None,
            charging_state: None,
            telemetry: None,
        }
    }
}

/// Snapshot of the active DualSense connection exposed to the UI and system tray.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub transport: Option<TransportType>,
    pub model: Option<DeviceModel>,
    pub serial_number: Option<String>,
    pub product_name: Option<String>,
    #[serde(default)]
    pub battery_percent: Option<u8>,
    #[serde(default)]
    pub charging_state: Option<ChargingState>,
    #[serde(default)]
    pub telemetry: Option<DualSenseTelemetry>,
    #[serde(default)]
    pub connected_count: usize,
    #[serde(default)]
    pub active_controller_id: Option<String>,
    #[serde(default)]
    pub controllers: Vec<ConnectedControllerSummary>,
}

impl ConnectionStatus {
    /// Builds a connected status snapshot from a [`DetectedController`].
    #[must_use]
    pub fn from_detected(controller: &DetectedController) -> Self {
        let summary = ConnectedControllerSummary::from_detected(controller, 1);
        let id = summary.id.clone();
        Self {
            connected: true,
            transport: Some(controller.transport),
            model: Some(controller.model),
            serial_number: controller.serial_number.clone(),
            product_name: controller.product_name.clone(),
            battery_percent: None,
            charging_state: None,
            telemetry: None,
            connected_count: 1,
            active_controller_id: Some(id),
            controllers: vec![summary],
        }
    }
}

/// High-level lifecycle events emitted by [`ControllerMonitor::poll_once`].
#[derive(Debug, Clone, PartialEq)]
pub enum ControllerEvent {
    Connected(ConnectionStatus),
    Disconnected,
    ReappliedColor,
    TransportError(String),
}

#[derive(Debug, Clone)]
struct TrackedController {
    id: String,
    detected: DetectedController,
    telemetry: Option<DualSenseTelemetry>,
    output_state: DualSenseOutputState,
    bt_seq: u8,
}

/// Tracks controller hotplug transitions and automatically applies LED state over USB or Bluetooth.
pub struct ControllerMonitor<T: HidTransport> {
    transport: T,
    tracked: Vec<TrackedController>,
    selected_controller_id: Option<String>,
    per_controller_states: HashMap<String, DualSenseOutputState>,
    output_state: DualSenseOutputState,
    apply_on_connect: bool,
    keep_alive_steam: bool,
    sync_all_controllers: bool,
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
            tracked: Vec::new(),
            selected_controller_id: None,
            per_controller_states: HashMap::new(),
            output_state: initial_state,
            apply_on_connect,
            keep_alive_steam,
            sync_all_controllers: true,
            ticks_since_keep_alive: 0,
        }
    }

    /// Returns a reference to the underlying HID transport (useful in tests).
    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }

    fn active_index(&self) -> Option<usize> {
        if self.tracked.is_empty() {
            return None;
        }
        if let Some(target_id) = self.selected_controller_id.as_deref() {
            if let Some(idx) = self.tracked.iter().position(|c| c.id == target_id) {
                return Some(idx);
            }
        }
        Some(0)
    }

    /// Returns the identifier of the currently focused controller slot (if any attached).
    #[must_use]
    pub fn active_controller_id(&self) -> Option<&str> {
        self.active_index()
            .and_then(|idx| self.tracked.get(idx))
            .map(|c| c.id.as_str())
    }

    /// Returns `(id, battery_percent)` pairs for every currently connected controller.
    #[must_use]
    pub fn connected_controller_ids(&self) -> Vec<(String, Option<u8>)> {
        self.tracked
            .iter()
            .map(|c| {
                (
                    c.id.clone(),
                    c.telemetry.as_ref().map(|t| t.battery_percent),
                )
            })
            .collect()
    }

    /// Selects a specific attached controller by ID (`None` falls back to primary).
    pub fn set_selected_controller(&mut self, controller_id: Option<String>) {
        self.selected_controller_id = controller_id;
    }

    /// Registers a controller-specific output state used when `sync_all_controllers` is false.
    pub fn set_per_controller_state(&mut self, controller_id: &str, state: DualSenseOutputState) {
        self.per_controller_states
            .insert(controller_id.to_string(), state);
        if let Some(entry) = self.tracked.iter_mut().find(|c| c.id == controller_id) {
            entry.output_state = state;
        }
    }

    /// Returns the current connection snapshot enriched with latest battery & stick telemetry.
    #[must_use]
    pub fn status(&self) -> ConnectionStatus {
        let Some(active_idx) = self.active_index() else {
            return ConnectionStatus::default();
        };
        let active = &self.tracked[active_idx];
        let controllers = self
            .tracked
            .iter()
            .enumerate()
            .map(|(i, item)| ConnectedControllerSummary {
                id: item.id.clone(),
                path: item.detected.path.clone(),
                slot: i + 1,
                model: item.detected.model,
                transport: item.detected.transport,
                serial_number: item.detected.serial_number.clone(),
                product_name: item.detected.product_name.clone(),
                battery_percent: item.telemetry.as_ref().map(|t| t.battery_percent),
                charging_state: item.telemetry.as_ref().map(|t| t.charging_state),
                telemetry: item.telemetry.clone(),
            })
            .collect();

        ConnectionStatus {
            connected: true,
            transport: Some(active.detected.transport),
            model: Some(active.detected.model),
            serial_number: active.detected.serial_number.clone(),
            product_name: active.detected.product_name.clone(),
            battery_percent: active.telemetry.as_ref().map(|t| t.battery_percent),
            charging_state: active.telemetry.as_ref().map(|t| t.charging_state),
            telemetry: active.telemetry.clone(),
            connected_count: self.tracked.len(),
            active_controller_id: Some(active.id.clone()),
            controllers,
        }
    }

    /// Returns the currently stored desired LED state.
    #[must_use]
    pub const fn output_state(&self) -> DualSenseOutputState {
        self.output_state
    }

    /// Enables or disables synchronized output writes across all attached DualSense controllers.
    pub fn set_sync_all_controllers(&mut self, enabled: bool) {
        self.sync_all_controllers = enabled;
    }

    /// Updates auto-apply and Steam keep-alive behavior without resetting connection state.
    pub fn set_preferences(&mut self, apply_on_connect: bool, keep_alive_steam: bool) {
        self.apply_on_connect = apply_on_connect;
        self.keep_alive_steam = keep_alive_steam;
        if !keep_alive_steam {
            self.ticks_since_keep_alive = 0;
        }
    }

    fn effective_state_for_id(&self, id: &str) -> DualSenseOutputState {
        if self.sync_all_controllers {
            self.output_state
        } else {
            self.per_controller_states
                .get(id)
                .copied()
                .unwrap_or(self.output_state)
        }
    }

    /// Polls the HID transport once, detecting connect/disconnect/switch transitions and errors.
    pub fn poll_once(&mut self) -> Vec<ControllerEvent> {
        let mut controllers = match self.transport.enumerate_controllers() {
            Ok(list) => list,
            Err(err) => {
                warn!(%err, "Error enumerando controladores DualSense");
                return vec![ControllerEvent::TransportError(err.to_string())];
            }
        };

        // Prefer wired USB before Bluetooth for deterministic slot ordering.
        controllers.sort_by_key(|controller| match controller.transport {
            TransportType::Usb => 0,
            TransportType::Bluetooth => 1,
        });

        if controllers.is_empty() {
            let had_controllers = !self.tracked.is_empty();
            self.tracked.clear();
            self.ticks_since_keep_alive = 0;
            if had_controllers {
                debug!("Controlador DualSense desconectado");
                return vec![ControllerEvent::Disconnected];
            }
            return Vec::new();
        }

        let prev_active = self
            .active_index()
            .and_then(|idx| self.tracked.get(idx))
            .map(|c| c.detected.clone());
        let prev_paths: HashSet<String> = self
            .tracked
            .iter()
            .map(|c| c.detected.path.clone())
            .collect();
        let prev_by_path: HashMap<String, TrackedController> = self
            .tracked
            .drain(..)
            .map(|item| (item.detected.path.clone(), item))
            .collect();

        let ids = assign_controller_ids(&controllers);
        let mut events = Vec::new();
        let mut newly_added_indices = Vec::new();

        for (detected, id) in controllers.into_iter().zip(ids) {
            let is_new = !prev_paths.contains(&detected.path);
            let mut item = if let Some(mut existing) = prev_by_path.get(&detected.path).cloned() {
                existing.id = id.clone();
                existing.detected = detected;
                existing
            } else {
                TrackedController {
                    id: id.clone(),
                    detected,
                    telemetry: None,
                    output_state: self.effective_state_for_id(&id),
                    bt_seq: 0,
                }
            };

            if is_new && item.detected.transport == TransportType::Bluetooth {
                if let Err(err) = self
                    .transport
                    .wake_bluetooth_controller(&item.detected.path)
                {
                    warn!(%err, path = %item.detected.path, "No se pudo despertar el mando Bluetooth");
                    events.push(ControllerEvent::TransportError(err.to_string()));
                }
            }

            let mut input_buf = [0u8; 547];
            if let Ok(read_len) = self
                .transport
                .read_input_report(&item.detected.path, &mut input_buf)
            {
                if read_len >= 11 {
                    if let Ok(parsed) = parse_input_report(&input_buf[..read_len]) {
                        let is_dummy_bt_short = item.detected.transport == TransportType::Bluetooth
                            && matches!(parsed.charging_state, ChargingState::Unknown(_))
                            && parsed.battery_percent == 0;
                        if !is_dummy_bt_short {
                            item.telemetry = Some(parsed);
                        }
                    }
                }
            }

            if is_new {
                newly_added_indices.push(self.tracked.len());
            }
            self.tracked.push(item);
        }

        let current_active = self
            .active_index()
            .and_then(|idx| self.tracked.get(idx))
            .map(|c| &c.detected);
        let active_changed = match (prev_active.as_ref(), current_active) {
            (Some(prev), Some(next)) => Self::has_connection_changed(Some(prev), next),
            (None, Some(_)) => true,
            _ => false,
        };
        let topology_changed =
            !newly_added_indices.is_empty() || self.tracked.len() != prev_paths.len();

        if active_changed || topology_changed {
            self.ticks_since_keep_alive = 0;
            events.push(ControllerEvent::Connected(self.status()));

            if self.apply_on_connect {
                let mut sent_any = false;
                let targets: Vec<usize> = if active_changed {
                    (0..self.tracked.len()).collect()
                } else {
                    newly_added_indices
                };
                for idx in targets {
                    let desired = self.effective_state_for_id(&self.tracked[idx].id);
                    self.tracked[idx].output_state = desired;
                    match Self::send_single_tracked(
                        &self.transport,
                        &mut self.tracked[idx],
                        &desired,
                    ) {
                        Ok(_) => sent_any = true,
                        Err(err) => {
                            warn!(%err, path = %self.tracked[idx].detected.path, "Error reaplicando color al conectar");
                            events.push(ControllerEvent::TransportError(err.to_string()));
                        }
                    }
                }
                if sent_any {
                    events.push(ControllerEvent::ReappliedColor);
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
        let mut reapplied = false;
        let mut keepalive_events = Vec::new();
        for idx in 0..self.tracked.len() {
            let desired = self.effective_state_for_id(&self.tracked[idx].id);
            self.tracked[idx].output_state = desired;
            match Self::send_single_tracked(&self.transport, &mut self.tracked[idx], &desired) {
                Ok(_) => reapplied = true,
                Err(err) => keepalive_events.push(ControllerEvent::TransportError(err.to_string())),
            }
        }
        if reapplied && keepalive_events.is_empty() {
            vec![ControllerEvent::ReappliedColor]
        } else {
            keepalive_events
        }
    }

    /// Stores `state` globally and immediately dispatches it across controllers (or selected controller when independent).
    pub fn apply_output_state(&mut self, state: DualSenseOutputState) -> AppResult<usize> {
        let target = if self.sync_all_controllers {
            None
        } else {
            self.active_controller_id().map(ToString::to_string)
        };
        self.apply_output_state_for(target.as_deref(), state)
    }

    /// Sends `state` specifically to `controller_id` (or broadcasts to all when `sync_all_controllers` is true).
    pub fn apply_output_state_for(
        &mut self,
        controller_id: Option<&str>,
        state: DualSenseOutputState,
    ) -> AppResult<usize> {
        self.ticks_since_keep_alive = 0;

        if self.sync_all_controllers || controller_id.is_none() {
            self.output_state = state;
            if self.tracked.is_empty() {
                return Ok(0);
            }
            return self.broadcast_state(&state);
        }

        let target_id = controller_id.unwrap_or_default();
        self.per_controller_states
            .insert(target_id.to_string(), state);
        let Some(idx) = self.tracked.iter().position(|c| c.id == target_id) else {
            return Ok(0);
        };
        self.tracked[idx].output_state = state;
        Self::send_single_tracked(&self.transport, &mut self.tracked[idx], &state)
    }

    /// Requires an attached controller and writes the given state immediately.
    pub fn apply_output_state_strict(&mut self, state: DualSenseOutputState) -> AppResult<usize> {
        if self.tracked.is_empty() {
            self.output_state = state;
            return Err(AppError::ControllerNotFound);
        }
        self.apply_output_state(state)
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

    fn send_single_tracked(
        transport: &T,
        item: &mut TrackedController,
        state: &DualSenseOutputState,
    ) -> AppResult<usize> {
        match item.detected.transport {
            TransportType::Usb => {
                let report = build_usb_full_report(state);
                transport.send_report(&item.detected.path, &report)
            }
            TransportType::Bluetooth => {
                let report = build_bt_full_report(state, item.bt_seq);
                item.bt_seq = (item.bt_seq + 1) & 0x0F;
                transport.send_report(&item.detected.path, &report)
            }
        }
    }

    fn broadcast_state(&mut self, state: &DualSenseOutputState) -> AppResult<usize> {
        let primary_idx = self.active_index().unwrap_or(0);
        let mut primary_written = 0usize;
        for idx in 0..self.tracked.len() {
            self.tracked[idx].output_state = *state;
            let res = Self::send_single_tracked(&self.transport, &mut self.tracked[idx], state);
            if idx == primary_idx {
                primary_written = res?;
            } else if let Err(err) = res {
                warn!(%err, path = %self.tracked[idx].detected.path, "Fallo sincronizando mando secundario");
            }
        }
        Ok(primary_written)
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
            ..DualSenseOutputState::default()
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

    #[test]
    fn poll_once_reads_battery_telemetry_and_syncs_multiple_controllers() {
        let transport = MockHidTransport::new();
        let primary = sample_usb_controller();
        let mut secondary = sample_bt_controller();
        secondary.path = "bt\\dualsense\\002".to_string();
        transport.set_controllers(vec![primary, secondary]);

        let mut usb_input = [0u8; 64];
        usb_input[0] = 0x01;
        usb_input[1] = 128;
        usb_input[2] = 128;
        usb_input[3] = 128;
        usb_input[4] = 128;
        usb_input[5] = 0;
        usb_input[6] = 0;
        usb_input[53] = 0x17; // charging state 1 (charging), level 7 -> 75%
        transport.set_mock_input_report(usb_input.to_vec());

        let mut monitor = ControllerMonitor::new(
            transport.clone(),
            DualSenseOutputState::default(),
            true,
            false,
        );
        let events = monitor.poll_once();
        assert!(matches!(
            events.first(),
            Some(ControllerEvent::Connected(_))
        ));

        let status = monitor.status();
        assert_eq!(status.connected_count, 2);
        assert_eq!(status.battery_percent, Some(75));
        assert_eq!(status.charging_state, Some(ChargingState::Charging));
        assert!(status.telemetry.is_some());

        // Both primary USB and secondary BT controllers received the output report
        let sent = transport.sent_reports();
        assert_eq!(sent.len(), 2);
    }
}

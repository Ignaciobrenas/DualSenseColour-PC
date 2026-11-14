use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// DualSense USB (`0x01`) and Bluetooth (`0x31`) battery status byte offsets.
pub const USB_INPUT_REPORT_ID: u8 = 0x01;
pub const BT_INPUT_REPORT_ID: u8 = 0x31;
pub const USB_BATTERY_BYTE_OFFSET: usize = 53;
pub const BT_BATTERY_BYTE_OFFSET: usize = 54;

/// High-nibble battery charging status decoded from DualSense HID input reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargingState {
    Discharging,
    Charging,
    Full,
    Unknown(u8),
}

impl ChargingState {
    #[must_use]
    pub const fn from_nibble(nibble: u8) -> Self {
        match nibble {
            0x00 => Self::Discharging,
            0x01 => Self::Charging,
            0x02 => Self::Full,
            other => Self::Unknown(other),
        }
    }
}

/// D-pad directional states decoded from the low nibble of `buttons[0]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DpadDirection {
    Neutral,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl DpadDirection {
    #[must_use]
    pub const fn from_nibble(nibble: u8) -> Self {
        match nibble & 0x0F {
            0 => Self::North,
            1 => Self::NorthEast,
            2 => Self::East,
            3 => Self::SouthEast,
            4 => Self::South,
            5 => Self::SouthWest,
            6 => Self::West,
            7 => Self::NorthWest,
            _ => Self::Neutral,
        }
    }
}

/// Snapshot of live DualSense battery telemetry, analog axes, triggers, and shortcut buttons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DualSenseTelemetry {
    pub battery_percent: u8,
    pub charging_state: ChargingState,
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    pub l2: u8,
    pub r2: u8,
    pub dpad: DpadDirection,
    pub mute_pressed: bool,
    pub ps_pressed: bool,
}

impl DualSenseTelemetry {
    /// Returns left stick displacement percentage from dead-center rest (`0.0..=100.0`).
    #[must_use]
    pub fn left_drift_percent(&self) -> f32 {
        stick_magnitude(self.lx, self.ly) * 100.0
    }

    /// Returns right stick displacement percentage from dead-center rest (`0.0..=100.0`).
    #[must_use]
    pub fn right_drift_percent(&self) -> f32 {
        stick_magnitude(self.rx, self.ry) * 100.0
    }
}

/// Normalizes an unsigned 8-bit DualSense stick axis (`0..=255`, center `128`) into `-1.0..=1.0`.
#[must_use]
pub fn normalize_stick_axis(raw: u8) -> f32 {
    if raw == 128 {
        return 0.0;
    }
    ((f32::from(raw) - 127.5) / 127.5).clamp(-1.0, 1.0)
}

/// Computes Euclidean magnitude for a normalized `(x, y)` stick coordinate pair.
#[must_use]
pub fn stick_magnitude(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

/// Computes circularity overshoot/undershoot percentage relative to the unit circle (`|r - 1.0| * 100`).
#[must_use]
pub fn circularity_error_percent(x: f32, y: f32) -> f32 {
    (stick_magnitude(x, y) - 1.0).abs() * 100.0
}

/// Parses a USB (`0x01`, >= 11B / full >= 54B) or Bluetooth (`0x31`, >= 55B) DualSense HID Input Report.
pub fn parse_input_report(data: &[u8]) -> AppResult<DualSenseTelemetry> {
    let Some(&report_id) = data.first() else {
        return Err(AppError::HidIo("Input report vacío".to_string()));
    };

    // USB 0x01 has common input fields starting at byte 1; BT 0x31 shifts them +1 to byte 2.
    let (common_offset, battery_offset) = match report_id {
        USB_INPUT_REPORT_ID => (1usize, USB_BATTERY_BYTE_OFFSET),
        BT_INPUT_REPORT_ID => (2usize, BT_BATTERY_BYTE_OFFSET),
        other => {
            return Err(AppError::HidIo(format!(
                "ID de Input Report no soportado: 0x{other:02X}"
            )));
        }
    };

    if data.len() <= common_offset + 9
        || (report_id == BT_INPUT_REPORT_ID && data.len() <= battery_offset)
    {
        return Err(AppError::HidIo(format!(
            "Input report 0x{report_id:02X} truncado ({} bytes; se requieren al menos {})",
            data.len(),
            battery_offset + 1
        )));
    }

    let lx = normalize_stick_axis(data[common_offset]);
    let ly = normalize_stick_axis(data[common_offset + 1]);
    let rx = normalize_stick_axis(data[common_offset + 2]);
    let ry = normalize_stick_axis(data[common_offset + 3]);
    let l2 = data[common_offset + 4];
    let r2 = data[common_offset + 5];

    let buttons_0 = data[common_offset + 7];
    let buttons_2 = data[common_offset + 9];
    let dpad = DpadDirection::from_nibble(buttons_0);
    let ps_pressed = (buttons_2 & 0x01) != 0;
    let mute_pressed = (buttons_2 & 0x04) != 0;

    let (battery_percent, charging_state) =
        if report_id == USB_INPUT_REPORT_ID && data.len() <= battery_offset {
            (0, ChargingState::Unknown(0))
        } else {
            let status = data[battery_offset];
            let charging_state = ChargingState::from_nibble((status & 0xF0) >> 4);
            let battery_raw = status & 0x0F;
            let battery_percent = if charging_state == ChargingState::Full {
                100
            } else {
                // Sony hid-playstation.c formula: 0 = 0-9% -> 5%, ..., 10 = 100%
                (battery_raw.min(10) * 10 + 5).min(100)
            };
            (battery_percent, charging_state)
        };

    Ok(DualSenseTelemetry {
        battery_percent,
        charging_state,
        lx,
        ly,
        rx,
        ry,
        l2,
        r2,
        dpad,
        mute_pressed,
        ps_pressed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_usb_64_byte_input_report_battery_sticks_and_buttons() {
        let mut usb = [0u8; 64];
        usb[0] = USB_INPUT_REPORT_ID;
        usb[1] = 128; // lx centered
        usb[2] = 128; // ly centered
        usb[3] = 255; // rx full right
        usb[4] = 128; // ry centered
        usb[5] = 42; // l2
        usb[6] = 220; // r2
        usb[8] = 0x02; // D-Pad East
        usb[10] = 0x04; // Mute pressed
        usb[USB_BATTERY_BYTE_OFFSET] = 0x15; // Charging (1) | 50-59% bucket -> 55%

        let telemetry = parse_input_report(&usb).expect("valid 64B USB report");
        assert_eq!(telemetry.battery_percent, 55);
        assert_eq!(telemetry.charging_state, ChargingState::Charging);
        assert_eq!(telemetry.dpad, DpadDirection::East);
        assert!(telemetry.mute_pressed);
        assert!(!telemetry.ps_pressed);
        assert_eq!(telemetry.l2, 42);
        assert_eq!(telemetry.r2, 220);
        assert!(telemetry.left_drift_percent() < 0.1);
        assert!((telemetry.rx - 1.0).abs() < 0.01);
        assert!(circularity_error_percent(telemetry.rx, telemetry.ry) < 0.1);
    }

    #[test]
    fn parses_bt_78_byte_input_report_battery_bounds() {
        let mut bt = [0u8; 78];
        bt[0] = BT_INPUT_REPORT_ID;
        bt[2] = 128;
        bt[3] = 128;
        bt[4] = 128;
        bt[5] = 128;
        bt[9] = 0x08; // D-Pad Neutral
        bt[BT_BATTERY_BYTE_OFFSET] = 0x2F; // Full (2), clamped raw 0x0F -> 100%

        let full = parse_input_report(&bt).expect("valid 78B BT report");
        assert_eq!(full.battery_percent, 100);
        assert_eq!(full.charging_state, ChargingState::Full);
        assert_eq!(full.dpad, DpadDirection::Neutral);

        bt[BT_BATTERY_BYTE_OFFSET] = 0x00; // Discharging 0-9% bucket -> 5%
        let low = parse_input_report(&bt).expect("valid 5% BT report");
        assert_eq!(low.battery_percent, 5);
        assert_eq!(low.charging_state, ChargingState::Discharging);

        bt[BT_BATTERY_BYTE_OFFSET] = 0x0A; // Discharging 100% bucket -> 100%
        let max_level = parse_input_report(&bt).expect("valid 100% BT report");
        assert_eq!(max_level.battery_percent, 100);
        assert_eq!(max_level.charging_state, ChargingState::Discharging);
    }

    #[test]
    fn rejects_invalid_or_truncated_reports() {
        assert!(parse_input_report(&[]).is_err());
        assert!(parse_input_report(&[0x99; 64]).is_err());
        assert!(parse_input_report(&[BT_INPUT_REPORT_ID; 40]).is_err());
    }
}

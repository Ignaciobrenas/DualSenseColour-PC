use serde::{Deserialize, Serialize};

/// Bitmasks inside `valid_flag0` enabling adaptive trigger resistance feedback.
pub const VALID_FLAG0_RIGHT_TRIGGER: u8 = 0x04;
pub const VALID_FLAG0_LEFT_TRIGGER: u8 = 0x08;

/// Built-in DualSense adaptive trigger profiles for quick hardware diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerPreset {
    #[default]
    Off,
    Rigid,
    WeaponClick,
    Pulse,
}

/// Encodes an 11-byte DualSense adaptive trigger effect descriptor (`mode` + parameters).
#[must_use]
pub const fn encode_trigger_effect(preset: TriggerPreset) -> [u8; 11] {
    match preset {
        TriggerPreset::Off => [0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::Rigid => [0x01, 0x00, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::WeaponClick => [0x02, 0x90, 0x0A, 0xFF, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::Pulse => [0x06, 0x90, 0xB0, 0x18, 0, 0, 0, 0, 0, 0, 0],
    }
}

/// Writes Left (`L2`, bytes `+21..+32`) and Right (`R2`, bytes `+10..+21`) trigger profiles
/// into a USB/BT DualSense common payload buffer and sets `valid_flag0`.
pub fn apply_trigger_presets(
    report: &mut [u8],
    base_offset: usize,
    left: TriggerPreset,
    right: TriggerPreset,
) {
    if report.len() < base_offset + 32 {
        return;
    }

    report[base_offset] |= VALID_FLAG0_RIGHT_TRIGGER | VALID_FLAG0_LEFT_TRIGGER;
    report[base_offset + 10..base_offset + 21].copy_from_slice(&encode_trigger_effect(right));
    report[base_offset + 21..base_offset + 32].copy_from_slice(&encode_trigger_effect(left));
}

/// Maps a battery percentage (`0..=100`) to the 5 center-out player indicator LEDs (`0..=5`)
/// so the physical controller LEDs can act as a live hardware battery bar.
#[must_use]
pub const fn battery_to_player_led_count(battery_percent: u8) -> u8 {
    match battery_percent {
        0..=9 => 0,
        10..=24 => 1,
        25..=49 => 2,
        50..=74 => 3,
        75..=89 => 4,
        _ => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_adaptive_trigger_presets_at_expected_common_offsets() {
        let mut usb = [0u8; 63];
        usb[0] = 0x02;
        apply_trigger_presets(
            &mut usb,
            1,
            TriggerPreset::WeaponClick,
            TriggerPreset::Rigid,
        );

        assert_eq!(
            usb[1] & VALID_FLAG0_RIGHT_TRIGGER,
            VALID_FLAG0_RIGHT_TRIGGER
        );
        assert_eq!(usb[1] & VALID_FLAG0_LEFT_TRIGGER, VALID_FLAG0_LEFT_TRIGGER);
        assert_eq!(usb[11], 0x01); // Right rigid mode
        assert_eq!(usb[22], 0x02); // Left weapon click mode
    }

    #[test]
    fn maps_battery_percent_to_physical_player_led_bar() {
        assert_eq!(battery_to_player_led_count(5), 0);
        assert_eq!(battery_to_player_led_count(15), 1);
        assert_eq!(battery_to_player_led_count(40), 2);
        assert_eq!(battery_to_player_led_count(60), 3);
        assert_eq!(battery_to_player_led_count(80), 4);
        assert_eq!(battery_to_player_led_count(100), 5);
    }
}

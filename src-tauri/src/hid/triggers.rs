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
    /// Tope háptico temprano: el dedo siente el punto de activación casi al inicio del recorrido.
    HairTrigger,
    /// Resistencia ligera al inicio y clic marcado al ~80 % del recorrido.
    DualStage,
    /// Dureza progresiva constante estilo pedal de freno/acelerador.
    ProgressivePedal,
}

/// Indicador visual de Gyro-Aim: color de confirmación mientras el giroscopio está activo.
#[must_use]
pub fn gyro_indicator_color(
    base: crate::hid::color::RgbColor,
    gyro_active: bool,
) -> crate::hid::color::RgbColor {
    if !gyro_active {
        return base;
    }
    // Mezcla sutil (35 %) hacia verde de confirmación.
    let mix = |a: u8, b: u8| ((u16::from(a) * 65 + u16::from(b) * 35) / 100) as u8;
    crate::hid::color::RgbColor::new(mix(base.r, 0), mix(base.g, 255), mix(base.b, 120))
}

/// Encodes an 11-byte DualSense adaptive trigger effect descriptor (`mode` + parameters).
#[must_use]
pub const fn encode_trigger_effect(preset: TriggerPreset) -> [u8; 11] {
    match preset {
        TriggerPreset::Off => [0x05, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::Rigid => [0x01, 0x00, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::WeaponClick => [0x02, 0x90, 0x0A, 0xFF, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::Pulse => [0x06, 0x90, 0xB0, 0x18, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::HairTrigger => [0x02, 0x20, 0x30, 0xFF, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::DualStage => [0x02, 0xCC, 0xE0, 0xFF, 0, 0, 0, 0, 0, 0, 0],
        TriggerPreset::ProgressivePedal => [0x01, 0x30, 0x80, 0, 0, 0, 0, 0, 0, 0, 0],
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
    fn competitive_profiles_use_expected_modes_and_zones() {
        let hair = encode_trigger_effect(TriggerPreset::HairTrigger);
        assert_eq!((hair[0], hair[1]), (0x02, 0x20));
        let dual = encode_trigger_effect(TriggerPreset::DualStage);
        // El clic empieza al ~80 % del recorrido (0xCC / 0xFF).
        assert_eq!(dual[1], 0xCC);
        assert!(f32::from(dual[1]) / 255.0 >= 0.79);
        let pedal = encode_trigger_effect(TriggerPreset::ProgressivePedal);
        assert_eq!(pedal[0], 0x01);
        // Los perfiles serializan en snake_case para .sensehue.
        assert_eq!(
            serde_json::to_string(&TriggerPreset::HairTrigger).unwrap(),
            "\"hair_trigger\""
        );
    }

    #[test]
    fn gyro_indicator_is_subtle_and_reversible() {
        use crate::hid::color::RgbColor;
        let base = RgbColor::new(200, 0, 0);
        assert_eq!(gyro_indicator_color(base, false), base);
        let on = gyro_indicator_color(base, true);
        assert!(on.g > base.g && on.r < base.r && on.r > 100);
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

    #[test]
    fn trigger_preset_off_uses_release_mode_opcode() {
        let mut expected = [0u8; 11];
        expected[0] = 0x05;
        assert_eq!(encode_trigger_effect(TriggerPreset::Off), expected);
    }
}

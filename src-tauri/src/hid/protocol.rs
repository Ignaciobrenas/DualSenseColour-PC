use serde::{Deserialize, Serialize};

use super::color::{MicLedState, RgbColor};

/// Sony Interactive Entertainment USB/Bluetooth Vendor ID.
pub const SONY_VID: u16 = 0x054C;
/// Standard PlayStation 5 DualSense Product ID.
pub const DUALSENSE_PID: u16 = 0x0CE6;
/// PlayStation 5 DualSense Edge Product ID.
pub const DUALSENSE_EDGE_PID: u16 = 0x0DF2;

/// USB HID Output Report identifier and length (including Report ID at index 0).
pub const USB_REPORT_ID: u8 = 0x02;
pub const USB_REPORT_LEN: usize = 63;

/// Bluetooth HID Output Report identifier, protocol tag, CRC seed, and total length.
pub const BT_REPORT_ID: u8 = 0x31;
pub const BT_REPORT_LEN: usize = 78;
pub const BT_OUTPUT_TAG: u8 = 0x10;
pub const BT_CRC_SEED: u8 = 0xA2;

/// Bitmasks inside `valid_flag1` enabling optional LED hardware blocks.
pub const VALID_FLAG1_MIC_MUTE_LED_ENABLE: u8 = 0x01;
pub const VALID_FLAG1_LIGHTBAR_ENABLE: u8 = 0x04;
pub const VALID_FLAG1_PLAYER_INDICATOR_ENABLE: u8 = 0x10;

/// Desired hardware state applied to connected DualSense controllers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualSenseOutputState {
    pub color: RgbColor,
    pub brightness: u8,
    pub player_leds: u8,
    pub mic_led: MicLedState,
}

impl Default for DualSenseOutputState {
    fn default() -> Self {
        Self {
            color: RgbColor::default(),
            brightness: 100,
            player_leds: 1,
            mic_led: MicLedState::Off,
        }
    }
}

/// Returns whether a USB/BT `(vendor_id, product_id)` pair identifies a supported DualSense model.
#[must_use]
pub const fn is_supported_controller(vid: u16, pid: u16) -> bool {
    vid == SONY_VID && (pid == DUALSENSE_PID || pid == DUALSENSE_EDGE_PID)
}

/// Maps player count (`0..=5`) to the official Sony 5-LED center-out bitmask.
#[must_use]
pub const fn player_count_to_mask(count: u8) -> u8 {
    match count {
        0 => 0x00,
        1 => 0x04,
        2 => 0x0A,
        3 => 0x15,
        4 => 0x1B,
        _ => 0x1F,
    }
}

fn populate_common_payload(
    buf: &mut [u8],
    base_offset: usize,
    state: &DualSenseOutputState,
    apply_player_and_mic: bool,
) {
    let mut flags = VALID_FLAG1_LIGHTBAR_ENABLE;
    if apply_player_and_mic {
        flags |= VALID_FLAG1_PLAYER_INDICATOR_ENABLE | VALID_FLAG1_MIC_MUTE_LED_ENABLE;
    }

    // valid_flag0 stays 0x00 so haptics / triggers remain untouched.
    buf[base_offset] = 0x00;
    buf[base_offset + 1] = flags;

    if apply_player_and_mic {
        buf[base_offset + 8] = state.mic_led.as_hid_byte();
        buf[base_offset + 43] = player_count_to_mask(state.player_leds);
    }

    let scaled = state.color.apply_brightness(state.brightness);
    buf[base_offset + 44] = scaled.r;
    buf[base_offset + 45] = scaled.g;
    buf[base_offset + 46] = scaled.b;
}

/// Builds a 63-byte USB HID Output Report (`0x02`) updating only the touchpad RGB lightbar.
#[must_use]
pub fn build_usb_lightbar_report(color: RgbColor, brightness: u8) -> [u8; USB_REPORT_LEN] {
    let state = DualSenseOutputState {
        color,
        brightness,
        ..DualSenseOutputState::default()
    };
    let mut report = [0u8; USB_REPORT_LEN];
    report[0] = USB_REPORT_ID;
    populate_common_payload(&mut report, 1, &state, false);
    report
}

/// Builds a 63-byte USB HID Output Report (`0x02`) updating lightbar, player LEDs, and mic LED.
#[must_use]
pub fn build_usb_full_report(state: &DualSenseOutputState) -> [u8; USB_REPORT_LEN] {
    let mut report = [0u8; USB_REPORT_LEN];
    report[0] = USB_REPORT_ID;
    populate_common_payload(&mut report, 1, state, true);
    report
}

/// Computes the DualSense Bluetooth IEEE 802.3 CRC-32 seeded with `0xA2` over the 74-byte payload.
#[must_use]
pub fn compute_dualsense_bt_crc32(payload_74: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&[BT_CRC_SEED]);
    hasher.update(payload_74);
    hasher.finalize()
}

/// Builds a 78-byte Bluetooth HID Output Report (`0x31`) updating only the touchpad RGB lightbar.
#[must_use]
pub fn build_bt_lightbar_report(color: RgbColor, brightness: u8, seq: u8) -> [u8; BT_REPORT_LEN] {
    let state = DualSenseOutputState {
        color,
        brightness,
        ..DualSenseOutputState::default()
    };
    let mut report = [0u8; BT_REPORT_LEN];
    report[0] = BT_REPORT_ID;
    report[1] = (seq & 0x0F) << 4;
    report[2] = BT_OUTPUT_TAG;
    populate_common_payload(&mut report, 3, &state, false);

    let crc = compute_dualsense_bt_crc32(&report[..74]);
    report[74..78].copy_from_slice(&crc.to_le_bytes());
    report
}

/// Builds a 78-byte Bluetooth HID Output Report (`0x31`) updating lightbar, player LEDs, and mic LED.
#[must_use]
pub fn build_bt_full_report(state: &DualSenseOutputState, seq: u8) -> [u8; BT_REPORT_LEN] {
    let mut report = [0u8; BT_REPORT_LEN];
    report[0] = BT_REPORT_ID;
    report[1] = (seq & 0x0F) << 4;
    report[2] = BT_OUTPUT_TAG;
    populate_common_payload(&mut report, 3, state, true);

    let crc = compute_dualsense_bt_crc32(&report[..74]);
    report[74..78].copy_from_slice(&crc.to_le_bytes());
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_ids_cover_dualsense_and_edge() {
        assert!(is_supported_controller(SONY_VID, DUALSENSE_PID));
        assert!(is_supported_controller(SONY_VID, DUALSENSE_EDGE_PID));
        assert!(!is_supported_controller(SONY_VID, 0x09CC)); // DualShock 4
        assert!(!is_supported_controller(0x045E, DUALSENSE_PID));
    }

    #[test]
    fn usb_lightbar_report_has_exact_offsets_and_length() {
        let color = RgbColor::new(0, 240, 255);
        let report = build_usb_lightbar_report(color, 100);

        assert_eq!(report.len(), 63);
        assert_eq!(report[0], 0x02);
        assert_eq!(report[1], 0x00);
        assert_eq!(report[2], 0x04);
        assert_eq!(&report[45..=47], &[0, 240, 255]);
        assert_eq!(report[44], 0x00); // Player LEDs not requested by lightbar-only helper
    }

    #[test]
    fn usb_full_report_sets_all_led_blocks_and_brightness() {
        let state = DualSenseOutputState {
            color: RgbColor::new(200, 100, 50),
            brightness: 50,
            player_leds: 3,
            mic_led: MicLedState::Pulse,
        };
        let report = build_usb_full_report(&state);

        assert_eq!(report[0], 0x02);
        assert_eq!(report[2], 0x15); // 0x01 | 0x04 | 0x10
        assert_eq!(report[9], 0x02); // Mic pulse
        assert_eq!(report[44], 0x15); // 3 player LEDs
        assert_eq!(&report[45..=47], &[100, 50, 25]);
    }

    #[test]
    fn bt_report_formats_seq_tag_offsets_and_seed_a2_crc32() {
        let color = RgbColor::new(255, 23, 68);
        let report = build_bt_lightbar_report(color, 100, 0x1A); // wraps to 0x0A -> 0xA0

        assert_eq!(report.len(), 78);
        assert_eq!(report[0], 0x31);
        assert_eq!(report[1], 0xA0);
        assert_eq!(report[2], 0x10);
        assert_eq!(report[3], 0x00);
        assert_eq!(report[4], 0x04);
        assert_eq!(&report[47..=49], &[255, 23, 68]);

        // Independent CRC-32 IEEE 802.3 reference verification with 0xA2 seed byte.
        let mut expected_hasher = crc32fast::Hasher::new();
        let mut seeded_input = Vec::with_capacity(75);
        seeded_input.push(0xA2);
        seeded_input.extend_from_slice(&report[0..74]);
        expected_hasher.update(&seeded_input);
        let expected_crc = expected_hasher.finalize();

        assert_eq!(
            u32::from_le_bytes([report[74], report[75], report[76], report[77]]),
            expected_crc
        );
    }

    #[test]
    fn crc32_known_vector_seed_a2_zero_payload() {
        let zero_payload = [0u8; 74];
        let crc = compute_dualsense_bt_crc32(&zero_payload);
        // CRC-32/ISO-HDLC of [0xA2, 0x00 * 74] verified deterministically
        let mut manual = crc32fast::Hasher::new();
        manual.update(&[0xA2]);
        manual.update(&zero_payload);
        assert_eq!(crc, manual.finalize());
        assert_ne!(crc, crc32fast::hash(&zero_payload));
    }

    #[test]
    fn bt_full_report_maps_player_masks_and_mic_led() {
        let cases = [
            (0, 0x00),
            (1, 0x04),
            (2, 0x0A),
            (3, 0x15),
            (4, 0x1B),
            (5, 0x1F),
            (99, 0x1F),
        ];

        for (count, expected_mask) in cases {
            let state = DualSenseOutputState {
                color: RgbColor::new(0, 67, 156),
                brightness: 100,
                player_leds: count,
                mic_led: MicLedState::On,
            };
            let report = build_bt_full_report(&state, 1);
            assert_eq!(report[1], 0x10);
            assert_eq!(report[4], 0x15);
            assert_eq!(report[11], 0x01);
            assert_eq!(report[46], expected_mask);
            assert_eq!(&report[47..=49], &[0, 67, 156]);
        }
    }
}

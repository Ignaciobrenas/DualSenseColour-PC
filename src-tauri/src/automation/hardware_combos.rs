use serde::{Deserialize, Serialize};

use crate::hid::input_report::{DpadDirection, DualSenseTelemetry};

/// Default hold duration before firing an in-game hardware combo (Zero Alt+Tab).
pub const DEFAULT_COMBO_HOLD_MS: u64 = 600;

/// Hardware controller shortcuts triggered by holding `Mute + D-Pad`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareComboEvent {
    NextPreset,
    PrevPreset,
    NightModeToggle,
    BatteryFlash,
}

/// Stateful debounced tracker preventing accidental short presses or repeated firing while held.
#[derive(Debug, Clone)]
pub struct HardwareComboTracker {
    hold_duration_ms: u64,
    active_combo: Option<HardwareComboEvent>,
    held_since_ms: Option<u64>,
    fired_current_hold: bool,
}

impl Default for HardwareComboTracker {
    fn default() -> Self {
        Self::new(DEFAULT_COMBO_HOLD_MS)
    }
}

impl HardwareComboTracker {
    /// Creates a new tracker requiring keys to remain held for `hold_duration_ms`.
    #[must_use]
    pub const fn new(hold_duration_ms: u64) -> Self {
        Self {
            hold_duration_ms,
            active_combo: None,
            held_since_ms: None,
            fired_current_hold: false,
        }
    }

    #[must_use]
    const fn decode_candidate(telemetry: &DualSenseTelemetry) -> Option<HardwareComboEvent> {
        if !telemetry.mute_pressed {
            return None;
        }
        match telemetry.dpad {
            DpadDirection::East => Some(HardwareComboEvent::NextPreset),
            DpadDirection::West => Some(HardwareComboEvent::PrevPreset),
            DpadDirection::South => Some(HardwareComboEvent::NightModeToggle),
            DpadDirection::North => Some(HardwareComboEvent::BatteryFlash),
            _ => None,
        }
    }

    /// Evaluates `telemetry` at monotonic timestamp `now_ms`, returning a combo event once per hold.
    pub fn update(
        &mut self,
        telemetry: &DualSenseTelemetry,
        now_ms: u64,
    ) -> Option<HardwareComboEvent> {
        let candidate = Self::decode_candidate(telemetry);

        if candidate != self.active_combo {
            self.active_combo = candidate;
            self.held_since_ms = candidate.map(|_| now_ms);
            self.fired_current_hold = false;
            return None;
        }

        let combo = candidate?;
        if self.fired_current_hold {
            return None;
        }

        let since = self.held_since_ms.unwrap_or(now_ms);
        if now_ms.saturating_sub(since) >= self.hold_duration_ms {
            self.fired_current_hold = true;
            return Some(combo);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hid::input_report::ChargingState;

    fn sample_telemetry(mute_pressed: bool, dpad: DpadDirection) -> DualSenseTelemetry {
        DualSenseTelemetry {
            battery_percent: 80,
            charging_state: ChargingState::Discharging,
            lx: 0.0,
            ly: 0.0,
            rx: 0.0,
            ry: 0.0,
            l2: 0,
            r2: 0,
            dpad,
            mute_pressed,
            ps_pressed: false,
        }
    }

    #[test]
    fn short_press_does_not_fire_and_600ms_hold_debounces_to_single_event() {
        let mut tracker = HardwareComboTracker::default();
        let right = sample_telemetry(true, DpadDirection::East);

        assert_eq!(tracker.update(&right, 1000), None);
        assert_eq!(tracker.update(&right, 1500), None); // 500ms < 600ms
        assert_eq!(
            tracker.update(&right, 1600),
            Some(HardwareComboEvent::NextPreset)
        );
        // Holding past 600ms must not fire again until released
        assert_eq!(tracker.update(&right, 2200), None);

        // Release resets state; holding Down afterwards fires NightModeToggle after 600ms
        let neutral = sample_telemetry(false, DpadDirection::Neutral);
        assert_eq!(tracker.update(&neutral, 2300), None);

        let down = sample_telemetry(true, DpadDirection::South);
        assert_eq!(tracker.update(&down, 2400), None);
        assert_eq!(
            tracker.update(&down, 3050),
            Some(HardwareComboEvent::NightModeToggle)
        );
    }
}

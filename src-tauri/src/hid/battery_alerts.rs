use crate::hid::input_report::ChargingState;
use serde::{Deserialize, Serialize};

/// Umbrales discretos de alerta de batería baja (20%, 10%, 5%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryAlertLevel {
    Twenty,
    Ten,
    Five,
}

impl BatteryAlertLevel {
    pub const fn threshold_percent(self) -> u8 {
        match self {
            Self::Twenty => 20,
            Self::Ten => 10,
            Self::Five => 5,
        }
    }
}

/// Notificación generada cuando el mando cruza un umbral de batería baja.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryAlertNotification {
    pub level: BatteryAlertLevel,
    pub threshold_percent: u8,
    pub battery_percent: u8,
    pub title: String,
    pub body: String,
}

/// Rastreador sin duplicados para alertas de batería baja en 20%, 10% y 5%.
///
/// Como el hardware DualSense reporta pasos discretos de 10% (`5%, 15%, 25%, ...`)
/// además de valores exactos en otros modos/pruebas, evaluamos por cruce de umbral
/// (`<= 20`, `<= 10`, `<= 5`) y reseteamos al superar el `25%`.
#[derive(Debug, Default, Clone)]
pub struct BatteryAlertTracker {
    notified_20: bool,
    notified_10: bool,
    notified_5: bool,
}

impl BatteryAlertTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.notified_20 = false;
        self.notified_10 = false;
        self.notified_5 = false;
    }

    pub fn evaluate(
        &mut self,
        connected: bool,
        battery_percent: Option<u8>,
        charging_state: Option<ChargingState>,
    ) -> Option<BatteryAlertNotification> {
        if !connected {
            return None;
        }

        let percent = battery_percent?;

        if percent > 25 {
            self.reset();
        }

        if matches!(
            charging_state,
            Some(ChargingState::Charging | ChargingState::Full)
        ) {
            return None;
        }

        if percent <= 5 {
            if self.notified_5 {
                return None;
            }
            self.notified_5 = true;
            self.notified_10 = true;
            self.notified_20 = true;
            return Some(BatteryAlertNotification {
                level: BatteryAlertLevel::Five,
                threshold_percent: 5,
                battery_percent: percent,
                title: format!("SenseHue · Batería crítica ({percent}%)"),
                body: format!(
                    "Tu mando se está quedando sin batería ({percent}%). Conéctalo por USB para evitar que se apague."
                ),
            });
        }

        if percent <= 10 {
            if self.notified_10 {
                return None;
            }
            self.notified_10 = true;
            self.notified_20 = true;
            return Some(BatteryAlertNotification {
                level: BatteryAlertLevel::Ten,
                threshold_percent: 10,
                battery_percent: percent,
                title: format!("SenseHue · Batería muy baja ({percent}%)"),
                body: format!(
                    "Tu mando se está quedando sin batería ({percent}%). Conéctalo pronto."
                ),
            });
        }

        if percent <= 20 && !self.notified_20 {
            self.notified_20 = true;
            return Some(BatteryAlertNotification {
                level: BatteryAlertLevel::Twenty,
                threshold_percent: 20,
                battery_percent: percent,
                title: format!("SenseHue · Batería baja ({percent}%)"),
                body: format!("Tu mando se está quedando sin batería ({percent}%)."),
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_thresholds_20_10_5_in_order_without_duplicates() {
        let mut tracker = BatteryAlertTracker::new();

        assert!(tracker
            .evaluate(true, Some(35), Some(ChargingState::Discharging))
            .is_none());

        let first = tracker
            .evaluate(true, Some(20), Some(ChargingState::Discharging))
            .expect("should alert at 20%");
        assert_eq!(first.level, BatteryAlertLevel::Twenty);
        assert!(first.body.contains("se está quedando sin batería"));
        assert!(tracker
            .evaluate(true, Some(20), Some(ChargingState::Discharging))
            .is_none());

        let second = tracker
            .evaluate(true, Some(10), Some(ChargingState::Discharging))
            .expect("should alert at 10%");
        assert_eq!(second.level, BatteryAlertLevel::Ten);
        assert!(tracker
            .evaluate(true, Some(10), Some(ChargingState::Discharging))
            .is_none());

        let third = tracker
            .evaluate(true, Some(5), Some(ChargingState::Discharging))
            .expect("should alert at 5%");
        assert_eq!(third.level, BatteryAlertLevel::Five);
        assert!(tracker
            .evaluate(true, Some(5), Some(ChargingState::Discharging))
            .is_none());
    }

    #[test]
    fn handles_dualsense_10_percent_buckets_25_to_15_to_5() {
        let mut tracker = BatteryAlertTracker::new();
        assert!(tracker
            .evaluate(true, Some(25), Some(ChargingState::Discharging))
            .is_none());

        let at_15 = tracker
            .evaluate(true, Some(15), Some(ChargingState::Discharging))
            .expect("15% crosses 20% bucket");
        assert_eq!(at_15.threshold_percent, 20);
        assert_eq!(at_15.battery_percent, 15);

        let at_5 = tracker
            .evaluate(true, Some(5), Some(ChargingState::Discharging))
            .expect("5% triggers critical alert");
        assert_eq!(at_5.threshold_percent, 5);
        assert_eq!(at_5.battery_percent, 5);
    }

    #[test]
    fn suppresses_while_charging_and_resets_above_25_percent() {
        let mut tracker = BatteryAlertTracker::new();
        assert!(tracker
            .evaluate(true, Some(10), Some(ChargingState::Charging))
            .is_none());

        assert!(tracker
            .evaluate(true, Some(30), Some(ChargingState::Charging))
            .is_none());

        let alert_again = tracker
            .evaluate(true, Some(20), Some(ChargingState::Discharging))
            .expect("should fire cleanly once discharged back to 20%");
        assert_eq!(alert_again.level, BatteryAlertLevel::Twenty);
    }
}

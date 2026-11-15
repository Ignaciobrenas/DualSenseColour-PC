//! Receptor UDP de telemetría pública de simuladores (F1, Forza, Assetto Corsa…).

use super::{progress_mask, LedFrame};
use crate::hid::color::RgbColor;

/// Puertos UDP estándar de telemetría abierta.
pub const F1_PORT: u16 = 20777;
pub const FORZA_PORT: u16 = 5300;
pub const ASSETTO_CORSA_PORT: u16 = 9996;

/// Estado normalizado del motor extraído de cualquier simulador.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RpmSample {
    pub rpm: f32,
    pub max_rpm: f32,
    pub pit_limiter: bool,
    pub yellow_flag: bool,
}

impl RpmSample {
    /// Porcentaje de revoluciones `0.0..=1.0`.
    #[must_use]
    pub fn ratio(&self) -> f32 {
        if self.max_rpm <= 0.0 {
            return 0.0;
        }
        (self.rpm / self.max_rpm).clamp(0.0, 1.0)
    }
}

/// Número de LEDs de jugador encendidos para un % de RPM (`0.0..=1.0`): 1 LED desde 50 %,
/// 5 LEDs desde 90 %.
#[must_use]
pub fn rpm_to_led_count(ratio: f32) -> u8 {
    let r = ratio.clamp(0.0, 1.0);
    if r < 0.50 {
        0
    } else if r < 0.60 {
        1
    } else if r < 0.70 {
        2
    } else if r < 0.80 {
        3
    } else if r < 0.90 {
        4
    } else {
        5
    }
}

/// Convierte una muestra en el frame de LEDs: barra RPM + color de bandera/pit/cambio.
#[must_use]
pub fn rpm_frame(sample: &RpmSample) -> LedFrame {
    let ratio = sample.ratio();
    let mask = progress_mask(rpm_to_led_count(ratio));
    let (color, blink) = if sample.pit_limiter {
        (RgbColor::new(0, 255, 0), false)
    } else if sample.yellow_flag {
        (RgbColor::new(255, 200, 0), false)
    } else if ratio >= 0.96 {
        // Punto óptimo de cambio: azul/rojo parpadeante.
        (RgbColor::new(0, 80, 255), true)
    } else if ratio >= 0.90 {
        (RgbColor::new(255, 0, 0), false)
    } else {
        (RgbColor::new(0, 40, 0), false)
    };
    LedFrame {
        color: Some(color),
        player_mask: Some(mask),
        blink,
    }
}

/// Errores de parseo de paquetes de telemetría.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryError {
    TooShort { expected: usize, got: usize },
    Unsupported(&'static str),
}

fn f32_at(buf: &[u8], off: usize) -> f32 {
    f32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

fn u16_at(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

/// Tamaño de cabecera de F1 2023/2024 (`PacketHeader`).
pub const F1_HEADER_LEN: usize = 29;
/// ID de paquete `CarTelemetryData` en F1.
pub const F1_PACKET_CAR_TELEMETRY: u8 = 6;
/// ID de paquete `CarStatusData` (contiene `maxRPM` y `pitLimiterStatus`).
pub const F1_PACKET_CAR_STATUS: u8 = 7;

/// Parsea un paquete `CarTelemetryData` de F1 (2023/24) para el coche del jugador y devuelve las RPM.
///
/// Por coche (60 bytes): speed u16, throttle f32, steer f32, brake f32, clutch u8, gear i8, `engineRPM` u16…
pub fn parse_f1_car_telemetry(packet: &[u8]) -> Result<u16, TelemetryError> {
    if packet.len() < F1_HEADER_LEN {
        return Err(TelemetryError::TooShort {
            expected: F1_HEADER_LEN,
            got: packet.len(),
        });
    }
    if packet[6] != F1_PACKET_CAR_TELEMETRY {
        return Err(TelemetryError::Unsupported("no es CarTelemetryData"));
    }
    let player = packet[27] as usize;
    let car = F1_HEADER_LEN + player * 60;
    let rpm_off = car + 2 + 4 + 4 + 4 + 1 + 1;
    if packet.len() < rpm_off + 2 {
        return Err(TelemetryError::TooShort {
            expected: rpm_off + 2,
            got: packet.len(),
        });
    }
    Ok(u16_at(packet, rpm_off))
}

/// Parsea `CarStatusData` de F1 (55 bytes por coche) → `(max_rpm, pit_limiter_activo)`.
pub fn parse_f1_car_status(packet: &[u8]) -> Result<(u16, bool), TelemetryError> {
    if packet.len() < F1_HEADER_LEN {
        return Err(TelemetryError::TooShort {
            expected: F1_HEADER_LEN,
            got: packet.len(),
        });
    }
    if packet[6] != F1_PACKET_CAR_STATUS {
        return Err(TelemetryError::Unsupported("no es CarStatusData"));
    }
    let car = F1_HEADER_LEN + packet[27] as usize * 55;
    let max_off = car + 17;
    if packet.len() < max_off + 2 {
        return Err(TelemetryError::TooShort {
            expected: max_off + 2,
            got: packet.len(),
        });
    }
    Ok((u16_at(packet, max_off), packet[car + 4] != 0))
}

/// Parsea el paquete UDP "Data Out" Dash de Forza Motorsport/Horizon (sled + dash).
///
/// Offsets (little endian): `EngineMaxRpm` f32 @8, `EngineIdleRpm` f32 @12, `CurrentEngineRpm` f32 @16.
pub fn parse_forza_dash(packet: &[u8]) -> Result<RpmSample, TelemetryError> {
    if packet.len() < 20 {
        return Err(TelemetryError::TooShort {
            expected: 20,
            got: packet.len(),
        });
    }
    if i32::from_le_bytes([packet[0], packet[1], packet[2], packet[3]]) == 0 {
        // IsRaceOn == 0 → en menús o pausa.
        return Ok(RpmSample {
            rpm: 0.0,
            max_rpm: f32_at(packet, 8),
            pit_limiter: false,
            yellow_flag: false,
        });
    }
    Ok(RpmSample {
        rpm: f32_at(packet, 16),
        max_rpm: f32_at(packet, 8),
        pit_limiter: false,
        yellow_flag: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f1_packet(player: u8, rpm: u16) -> Vec<u8> {
        let mut p = vec![0u8; F1_HEADER_LEN + 22 * 60];
        p[6] = F1_PACKET_CAR_TELEMETRY;
        p[27] = player;
        let off = F1_HEADER_LEN + player as usize * 60 + 2 + 12 + 2;
        p[off..off + 2].copy_from_slice(&rpm.to_le_bytes());
        p
    }

    #[test]
    fn parses_f1_player_rpm_from_recorded_layout() {
        assert_eq!(parse_f1_car_telemetry(&f1_packet(3, 11_800)), Ok(11_800));
        assert_eq!(parse_f1_car_telemetry(&f1_packet(0, 4_200)), Ok(4_200));
    }

    #[test]
    fn parses_f1_car_status_max_rpm_and_pit_limiter() {
        let mut p = vec![0u8; F1_HEADER_LEN + 22 * 55];
        p[6] = F1_PACKET_CAR_STATUS;
        p[27] = 2;
        let car = F1_HEADER_LEN + 2 * 55;
        p[car + 4] = 1;
        p[car + 17..car + 19].copy_from_slice(&13_500u16.to_le_bytes());
        assert_eq!(parse_f1_car_status(&p), Ok((13_500, true)));
    }

    #[test]
    fn rejects_short_and_wrong_packets() {
        assert!(matches!(
            parse_f1_car_telemetry(&[0u8; 10]),
            Err(TelemetryError::TooShort { .. })
        ));
        let mut p = f1_packet(0, 1);
        p[6] = 2;
        assert!(matches!(
            parse_f1_car_telemetry(&p),
            Err(TelemetryError::Unsupported(_))
        ));
    }

    #[test]
    fn rpm_percent_maps_to_five_led_mask() {
        let masks: Vec<u8> = [0.0, 0.55, 0.65, 0.75, 0.85, 0.95]
            .iter()
            .map(|r| progress_mask(rpm_to_led_count(*r)))
            .collect();
        assert_eq!(
            masks,
            vec![0b00000, 0b00001, 0b00011, 0b00111, 0b01111, 0b11111]
        );
    }

    #[test]
    fn forza_packet_drives_shift_light() {
        let mut p = vec![0u8; 64];
        p[0] = 1; // IsRaceOn
        p[8..12].copy_from_slice(&8000.0f32.to_le_bytes());
        p[16..20].copy_from_slice(&7800.0f32.to_le_bytes());
        let sample = parse_forza_dash(&p).unwrap();
        let frame = rpm_frame(&sample);
        assert_eq!(frame.player_mask, Some(0b11111));
        assert!(frame.blink);
        assert_eq!(frame.color, Some(RgbColor::new(0, 80, 255)));
    }

    #[test]
    fn flags_and_pit_limiter_override_colors() {
        let mut s = RpmSample {
            rpm: 5000.0,
            max_rpm: 10000.0,
            pit_limiter: false,
            yellow_flag: true,
        };
        assert_eq!(rpm_frame(&s).color, Some(RgbColor::new(255, 200, 0)));
        s.pit_limiter = true;
        assert_eq!(rpm_frame(&s).color, Some(RgbColor::new(0, 255, 0)));
    }

    #[test]
    fn zero_max_rpm_is_safe() {
        let s = RpmSample {
            rpm: 100.0,
            max_rpm: 0.0,
            pit_limiter: false,
            yellow_flag: false,
        };
        assert_eq!(s.ratio(), 0.0);
    }

    #[test]
    fn over_max_rpm_clamps_ratio_and_preserves_shift_light() {
        let s = RpmSample {
            rpm: 12500.0,
            max_rpm: 10000.0,
            pit_limiter: false,
            yellow_flag: false,
        };
        assert_eq!(s.ratio(), 1.0);
        let frame = rpm_frame(&s);
        assert_eq!(frame.player_mask, Some(0b11111));
        assert!(frame.blink);
    }
}

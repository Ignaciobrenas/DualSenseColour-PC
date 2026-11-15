//! Receptor de Valve Game State Integration (CS2 / Dota 2) — JSON oficial vía HTTP local.

use serde_json::Value;

use super::LedFrame;
use crate::hid::color::RgbColor;

/// Límite de seguridad fotosensible: nunca más de un destello blanco cada 2 s.
pub const FLASH_MIN_INTERVAL_MS: u64 = 2_000;

/// Estado reducido del juego extraído del payload GSI.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GsiState {
    pub health: Option<u8>,
    /// Intensidad de flashbang `0..=255`.
    pub flashed: u8,
    pub bomb_planted: bool,
    /// Segundos restantes de la bomba (si el payload los incluye).
    pub bomb_countdown: Option<f32>,
}

/// Parsea un payload GSI. Devuelve `None` si no es JSON válido de objeto.
#[must_use]
pub fn parse_gsi(body: &str) -> Option<GsiState> {
    let v: Value = serde_json::from_str(body).ok()?;
    if !v.is_object() {
        return None;
    }
    let health = v
        .pointer("/player/state/health")
        .and_then(Value::as_u64)
        .map(|h| h.min(100) as u8);
    let flashed = v
        .pointer("/player/state/flashed")
        .and_then(Value::as_u64)
        .map_or(0, |f| f.min(255) as u8);
    let bomb_planted = v.pointer("/round/bomb").and_then(Value::as_str) == Some("planted")
        || v.pointer("/bomb/state").and_then(Value::as_str) == Some("planted");
    let bomb_countdown = v
        .pointer("/bomb/countdown")
        .and_then(|c| {
            c.as_f64()
                .or_else(|| c.as_str().and_then(|s| s.parse().ok()))
        })
        .map(|c| c as f32);
    Some(GsiState {
        health,
        flashed,
        bomb_planted,
        bomb_countdown,
    })
}

/// Traduce el estado a un frame. Prioridad: flash > bomba > vida baja.
///
/// `now_ms`/`last_flash_ms` aplican el límite de seguridad fotosensible.
#[must_use]
pub fn gsi_frame(state: &GsiState, now_ms: u64, last_flash_ms: Option<u64>) -> Option<LedFrame> {
    if state.flashed > 100 {
        let allowed =
            last_flash_ms.is_none_or(|t| now_ms.saturating_sub(t) >= FLASH_MIN_INTERVAL_MS);
        if allowed {
            return Some(LedFrame::color(RgbColor::new(255, 255, 255)));
        }
    }
    if state.bomb_planted {
        // Cuanto menos tiempo queda, más intenso el rojo (40 s de cuenta atrás).
        let remaining = state.bomb_countdown.unwrap_or(40.0).clamp(0.0, 40.0);
        let intensity = 255.0 - (remaining / 40.0) * 155.0;
        return Some(LedFrame {
            color: Some(RgbColor::new(intensity as u8, 0, 0)),
            player_mask: None,
            blink: true,
        });
    }
    match state.health {
        Some(h) if h > 0 && h < 25 => Some(LedFrame {
            color: Some(RgbColor::new(90, 0, 0)),
            player_mask: None,
            blink: true,
        }),
        _ => None,
    }
}

/// Fase de parpadeo (periodo en ms) para la bomba: acelera al acercarse la explosión.
#[must_use]
pub fn bomb_blink_period_ms(countdown: Option<f32>) -> u64 {
    let c = countdown.unwrap_or(40.0).clamp(0.0, 40.0);
    (150.0 + (c / 40.0) * 850.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cs2_payload() {
        let s = parse_gsi(
            r#"{"player":{"state":{"health":18,"flashed":200}},"round":{"bomb":"planted"},"bomb":{"countdown":"10.5"}}"#,
        )
        .unwrap();
        assert_eq!(s.health, Some(18));
        assert_eq!(s.flashed, 200);
        assert!(s.bomb_planted);
        assert_eq!(s.bomb_countdown, Some(10.5));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_gsi("not json").is_none());
        assert!(parse_gsi("[1,2]").is_none());
    }

    #[test]
    fn flash_is_rate_limited_for_photosensitive_safety() {
        let s = GsiState {
            flashed: 255,
            ..Default::default()
        };
        assert_eq!(
            gsi_frame(&s, 10_000, None).unwrap().color,
            Some(RgbColor::new(255, 255, 255))
        );
        assert!(gsi_frame(&s, 10_500, Some(10_000)).is_none());
        assert!(gsi_frame(&s, 12_100, Some(10_000)).is_some());
    }

    #[test]
    fn bomb_pulses_red_faster_near_zero() {
        let near = GsiState {
            bomb_planted: true,
            bomb_countdown: Some(2.0),
            ..Default::default()
        };
        let far = GsiState {
            bomb_planted: true,
            bomb_countdown: Some(38.0),
            ..Default::default()
        };
        let n = gsi_frame(&near, 0, None).unwrap();
        let f = gsi_frame(&far, 0, None).unwrap();
        assert!(n.blink && n.color.unwrap().r > f.color.unwrap().r);
        assert!(bomb_blink_period_ms(Some(2.0)) < bomb_blink_period_ms(Some(38.0)));
    }

    #[test]
    fn low_health_heartbeat_but_dead_or_healthy_is_idle() {
        let low = GsiState {
            health: Some(20),
            ..Default::default()
        };
        assert!(gsi_frame(&low, 0, None).unwrap().blink);
        for h in [0u8, 60] {
            let s = GsiState {
                health: Some(h),
                ..Default::default()
            };
            assert!(gsi_frame(&s, 0, None).is_none());
        }
    }

    #[test]
    fn bomb_countdown_clamps_out_of_range_values() {
        assert_eq!(bomb_blink_period_ms(Some(-5.0)), 150);
        assert_eq!(bomb_blink_period_ms(Some(120.0)), 1000);
        assert_eq!(bomb_blink_period_ms(None), 1000);
    }
}

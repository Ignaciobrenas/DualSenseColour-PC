//! Cliente de la Live Client Data API de League of Legends (`https://127.0.0.1:2999`).
//!
//! Solo se parsean respuestas JSON ya obtenidas por polling local; no se toca el proceso del juego.

use serde_json::Value;

use super::LedFrame;
use crate::hid::color::RgbColor;

pub const LIVE_CLIENT_ADDR: &str = "127.0.0.1:2999";
pub const ACTIVE_PLAYER_PATH: &str = "/liveclientdata/activeplayer";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LolState {
    pub health_ratio: f32,
    /// `true` cuando la R (Ultimate) está disponible (nivel ≥ 6 y `abilities.R.abilityLevel > 0`).
    pub ultimate_ready: bool,
}

/// Parsea la respuesta de `/liveclientdata/activeplayer`.
#[must_use]
pub fn parse_active_player(body: &str) -> Option<LolState> {
    let v: Value = serde_json::from_str(body).ok()?;
    let cur = v.pointer("/championStats/currentHealth")?.as_f64()?;
    let max = v.pointer("/championStats/maxHealth")?.as_f64()?;
    if max <= 0.0 {
        return None;
    }
    let ult_level = v
        .pointer("/abilities/R/abilityLevel")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    Some(LolState {
        health_ratio: (cur / max).clamp(0.0, 1.0) as f32,
        ultimate_ready: ult_level > 0,
    })
}

/// Color de la barra: gradiente rojo→verde según vida; dorado cuando la Ultimate está lista.
#[must_use]
pub fn lol_frame(state: &LolState, ult_just_ready: bool) -> LedFrame {
    if ult_just_ready {
        return LedFrame {
            color: Some(RgbColor::new(255, 200, 0)),
            player_mask: None,
            blink: true,
        };
    }
    let r = ((1.0 - state.health_ratio) * 255.0) as u8;
    let g = (state.health_ratio * 255.0) as u8;
    LedFrame::color(RgbColor::new(r, g, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = r#"{"championStats":{"currentHealth":300.0,"maxHealth":600.0},"abilities":{"R":{"abilityLevel":1}}}"#;

    #[test]
    fn parses_live_client_player() {
        let s = parse_active_player(BODY).unwrap();
        assert!((s.health_ratio - 0.5).abs() < f32::EPSILON);
        assert!(s.ultimate_ready);
    }

    #[test]
    fn health_gradient_and_golden_flash() {
        let full = LolState {
            health_ratio: 1.0,
            ultimate_ready: false,
        };
        let low = LolState {
            health_ratio: 0.1,
            ultimate_ready: false,
        };
        assert_eq!(
            lol_frame(&full, false).color,
            Some(RgbColor::new(0, 255, 0))
        );
        assert!(lol_frame(&low, false).color.unwrap().r > 200);
        let gold = lol_frame(&full, true);
        assert_eq!(gold.color, Some(RgbColor::new(255, 200, 0)));
        assert!(gold.blink);
    }

    #[test]
    fn invalid_or_zero_max_health_is_rejected() {
        assert!(parse_active_player("{}").is_none());
        assert!(
            parse_active_player(r#"{"championStats":{"currentHealth":1,"maxHealth":0}}"#).is_none()
        );
    }
}

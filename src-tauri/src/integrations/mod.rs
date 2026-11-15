//! Integraciones v3.0 con juegos y ecosistema RGB.
//!
//! **Zero Memory Injection:** todos los módulos usan únicamente APIs oficiales locales
//! (HTTP GSI, UDP de telemetría pública, TCP de OpenRGB). Nunca se inyectan DLLs ni se lee
//! memoria de otros procesos, por lo que son compatibles con Vanguard, EAC, BattlEye y VAC.

pub mod lol_live;
pub mod openrgb_client;
pub mod runtime;
pub mod sim_telemetry;
pub mod valve_gsi;
pub mod wled_udp;

use crate::hid::color::RgbColor;

/// Resultado común de una integración: qué debe mostrar el mando en este instante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LedFrame {
    /// Color de la barra RGB (`None` = no cambiar).
    pub color: Option<RgbColor>,
    /// Máscara cruda de los 5 LEDs de jugador, bit0 = LED más a la izquierda (`None` = no cambiar).
    pub player_mask: Option<u8>,
    /// Si `true`, el color debe parpadear a ~4 Hz.
    pub blink: bool,
}

impl LedFrame {
    #[must_use]
    pub const fn color(color: RgbColor) -> Self {
        Self {
            color: Some(color),
            player_mask: None,
            blink: false,
        }
    }
}

/// Barra de progreso de `count` LEDs encendidos de izquierda a derecha (`0..=5`).
#[must_use]
pub const fn progress_mask(count: u8) -> u8 {
    match count {
        0 => 0b00000,
        1 => 0b00001,
        2 => 0b00011,
        3 => 0b00111,
        4 => 0b01111,
        _ => 0b11111,
    }
}

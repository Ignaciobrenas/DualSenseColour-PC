use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Represents an 8-bit sRGB color sent to the DualSense touchpad lightbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Represents a color in HSV space (`h` in `[0, 360)`, `s` and `v` in `[0, 100]`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HsvColor {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

/// Supported states for the DualSense microphone mute indicator LED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MicLedState {
    #[default]
    Off,
    On,
    Pulse,
}

impl MicLedState {
    /// Converts the microphone LED state into the firmware byte expected by HID reports.
    #[must_use]
    pub const fn as_hid_byte(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::On => 0x01,
            Self::Pulse => 0x02,
        }
    }
}

/// Named color preset exposed to the UI and system tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ColorPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub color: RgbColor,
    pub hex: &'static str,
}

/// Curated presets available across UI pills and the native system tray.
pub const PRESETS: &[ColorPreset] = &[
    ColorPreset {
        id: "classic-blue",
        name: "Classic Blue",
        color: RgbColor::new(0, 67, 156),
        hex: "#00439C",
    },
    ColorPreset {
        id: "crimson-red",
        name: "Crimson Red",
        color: RgbColor::new(255, 23, 68),
        hex: "#FF1744",
    },
    ColorPreset {
        id: "cyber-cyan",
        name: "Cyber Cyan",
        color: RgbColor::new(0, 240, 255),
        hex: "#00F0FF",
    },
    ColorPreset {
        id: "emerald",
        name: "Emerald",
        color: RgbColor::new(0, 230, 118),
        hex: "#00E676",
    },
    ColorPreset {
        id: "gold",
        name: "Gold",
        color: RgbColor::new(255, 214, 0),
        hex: "#FFD600",
    },
    ColorPreset {
        id: "violet",
        name: "Violet",
        color: RgbColor::new(157, 78, 221),
        hex: "#9D4EDD",
    },
    ColorPreset {
        id: "off",
        name: "Apagado",
        color: RgbColor::new(0, 0, 0),
        hex: "#000000",
    },
];

/// Looks up a built-in preset by identifier (retains legacy aliases like `playstation-blue`).
#[must_use]
pub fn find_preset(id: &str) -> Option<&'static ColorPreset> {
    if id.eq_ignore_ascii_case("playstation-blue") || id.eq_ignore_ascii_case("ps5-blue") {
        return PRESETS.first();
    }
    PRESETS.iter().find(|preset| preset.id == id)
}

impl Default for RgbColor {
    fn default() -> Self {
        Self::new(0, 67, 156)
    }
}

impl RgbColor {
    /// Creates an [`RgbColor`] from 8-bit channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#RRGGBB` or `RRGGBB` hexadecimal strings into [`RgbColor`].
    pub fn from_hex(hex: &str) -> AppResult<Self> {
        let normalized = hex.trim().strip_prefix('#').unwrap_or_else(|| hex.trim());
        if normalized.len() != 6 || !normalized.is_ascii() {
            return Err(AppError::InvalidColor(hex.to_string()));
        }

        let parse_channel = |slice: &str| {
            u8::from_str_radix(slice, 16).map_err(|_| AppError::InvalidColor(hex.to_string()))
        };

        Ok(Self {
            r: parse_channel(&normalized[0..2])?,
            g: parse_channel(&normalized[2..4])?,
            b: parse_channel(&normalized[4..6])?,
        })
    }

    /// Formats the color as an uppercase `#RRGGBB` string.
    #[must_use]
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Scales each RGB channel by `brightness` (`0..=100`, clamped) without overflowing.
    #[must_use]
    pub fn apply_brightness(self, brightness: u8) -> Self {
        let clamped = u16::from(brightness.min(100));
        if clamped == 0 {
            return Self::new(0, 0, 0);
        }
        if clamped == 100 {
            return self;
        }

        let scale = |channel: u8| -> u8 {
            let scaled = (u16::from(channel) * clamped + 50) / 100;
            scaled.min(255) as u8
        };

        Self {
            r: scale(self.r),
            g: scale(self.g),
            b: scale(self.b),
        }
    }

    /// Converts HSV values (`h`: `0..360`, `s`: `0..=100`, `v`: `0..=100`) to RGB.
    #[must_use]
    pub fn from_hsv(hsv: HsvColor) -> Self {
        let h = hsv.h.rem_euclid(360.0);
        let s = (hsv.s / 100.0).clamp(0.0, 1.0);
        let v = (hsv.v / 100.0).clamp(0.0, 1.0);

        let c = v * s;
        let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
        let m = v - c;

        let (rp, gp, bp) = match h {
            h if h < 60.0 => (c, x, 0.0),
            h if h < 120.0 => (x, c, 0.0),
            h if h < 180.0 => (0.0, c, x),
            h if h < 240.0 => (0.0, x, c),
            h if h < 300.0 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        let to_u8 = |val: f32| -> u8 { ((val + m) * 255.0).round().clamp(0.0, 255.0) as u8 };
        Self::new(to_u8(rp), to_u8(gp), to_u8(bp))
    }

    /// Converts an RGB color into HSV (`h` in `[0, 360)`, `s` & `v` in `[0, 100]`).
    #[must_use]
    pub fn to_hsv(self) -> HsvColor {
        let r = f32::from(self.r) / 255.0;
        let g = f32::from(self.g) / 255.0;
        let b = f32::from(self.b) / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let h = if delta <= f32::EPSILON {
            0.0
        } else if (max - r).abs() <= f32::EPSILON {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if (max - g).abs() <= f32::EPSILON {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let s = if max <= f32::EPSILON {
            0.0
        } else {
            (delta / max) * 100.0
        };
        let v = max * 100.0;

        HsvColor { h, s, v }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parsing_and_formatting_roundtrip() {
        let cyan = RgbColor::from_hex("#00f0ff").expect("valid cyan hex");
        assert_eq!(cyan, RgbColor::new(0, 240, 255));
        assert_eq!(cyan.to_hex(), "#00F0FF");

        let bare = RgbColor::from_hex(" ff1744 ").expect("valid bare hex");
        assert_eq!(bare, RgbColor::new(255, 23, 68));
        assert_eq!(bare.to_hex(), "#FF1744");
    }

    #[test]
    fn invalid_hex_returns_typed_error() {
        assert!(matches!(
            RgbColor::from_hex("#12345"),
            Err(AppError::InvalidColor(_))
        ));
        assert!(matches!(
            RgbColor::from_hex("GGGGGG"),
            Err(AppError::InvalidColor(_))
        ));
    }

    #[test]
    fn brightness_scaling_handles_bounds_cleanly() {
        let color = RgbColor::new(200, 100, 50);
        assert_eq!(color.apply_brightness(0), RgbColor::new(0, 0, 0));
        assert_eq!(color.apply_brightness(50), RgbColor::new(100, 50, 25));
        assert_eq!(color.apply_brightness(100), color);
        assert_eq!(color.apply_brightness(250), color);
    }

    #[test]
    fn hsv_conversion_preserves_primary_and_accent_colors() {
        let cyan = RgbColor::new(0, 255, 255);
        let hsv = cyan.to_hsv();
        assert!((hsv.h - 180.0).abs() < 0.1);
        assert!((hsv.s - 100.0).abs() < 0.1);
        assert!((hsv.v - 100.0).abs() < 0.1);
        assert_eq!(RgbColor::from_hsv(hsv), cyan);
    }

    #[test]
    fn builtin_presets_match_specification() {
        assert_eq!(PRESETS.len(), 7);
        let classic = find_preset("classic-blue").expect("preset exists");
        assert_eq!(classic.hex, "#00439C");
        assert_eq!(find_preset("playstation-blue"), Some(classic));
        let off = find_preset("off").expect("off preset exists");
        assert_eq!(off.color, RgbColor::new(0, 0, 0));
    }
}

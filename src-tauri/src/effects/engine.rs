use std::f32::consts::TAU;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::hid::color::RgbColor;

/// Dynamic lighting engine modes supported by SenseHue v2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LightingEffectMode {
    #[default]
    Static,
    Breathing,
    Spectrum,
    BatterySmart,
    InputReactive,
    AudioBeat,
    ScreenAmbilight,
}

impl LightingEffectMode {
    /// Parses a CLI/config string into [`LightingEffectMode`].
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "static" => Some(Self::Static),
            "breathing" => Some(Self::Breathing),
            "spectrum" | "prism" => Some(Self::Spectrum),
            "battery" | "battery-smart" | "battery_smart" => Some(Self::BatterySmart),
            "input" | "input-reactive" | "input_reactive" => Some(Self::InputReactive),
            "audio" | "audio-beat" | "audio_beat" => Some(Self::AudioBeat),
            "ambilight" | "screen-ambilight" | "screen_ambilight" => Some(Self::ScreenAmbilight),
            _ => None,
        }
    }
}

/// Real-time telemetry inputs fed into each frame evaluation tick of the effect engine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectFrameInput {
    pub elapsed_secs: f32,
    pub base_color: RgbColor,
    pub secondary_color: RgbColor,
    pub speed_hz: f32,
    pub battery_percent: u8,
    pub trigger_depth: u8,
    pub audio_bass_energy: f32,
    pub screen_dominant: RgbColor,
}

impl Default for EffectFrameInput {
    fn default() -> Self {
        Self {
            elapsed_secs: 0.0,
            base_color: RgbColor::new(0, 229, 255),
            secondary_color: RgbColor::new(168, 85, 247),
            speed_hz: 0.4,
            battery_percent: 100,
            trigger_depth: 0,
            audio_bass_energy: 0.0,
            screen_dominant: RgbColor::new(0, 67, 156),
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn lerp_color(a: RgbColor, b: RgbColor, t: f32) -> RgbColor {
    let clamped = t.clamp(0.0, 1.0);
    let blend = |start: u8, end: u8| -> u8 {
        let s = f32::from(start);
        let e = f32::from(end);
        (s + (e - s) * clamped).round().clamp(0.0, 255.0) as u8
    };
    RgbColor::new(blend(a.r, b.r), blend(a.g, b.g), blend(a.b, b.b))
}

/// Computes smooth sinusoidal breathing interpolation between two colors.
#[must_use]
pub fn compute_breathing_color(
    primary: RgbColor,
    secondary: RgbColor,
    elapsed_secs: f32,
    speed_hz: f32,
) -> RgbColor {
    let phase = (elapsed_secs * speed_hz.max(0.05) * TAU).sin();
    let mix = (phase + 1.0) * 0.5;
    lerp_color(primary, secondary, mix)
}

/// Computes continuous HSV spectrum rainbow cycling (`hue` rotating cleanly across `0..360`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub fn compute_spectrum_color(elapsed_secs: f32, speed_hz: f32) -> RgbColor {
    let hue = (elapsed_secs * speed_hz.max(0.05) * 360.0).rem_euclid(360.0);
    let sector = hue / 60.0;
    let x = 1.0 - ((sector % 2.0) - 1.0).abs();

    let (rn, gn, bn) = match sector as u8 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };

    RgbColor::new(
        (rn * 255.0).round() as u8,
        (gn * 255.0).round() as u8,
        (bn * 255.0).round() as u8,
    )
}

/// Maps DualSense battery level to semantic gamer colors (Emerald -> Amber -> Critical Red).
#[must_use]
pub fn compute_battery_smart_color(battery_percent: u8) -> RgbColor {
    let emerald = RgbColor::new(16, 185, 129);
    let amber = RgbColor::new(245, 158, 11);
    let critical = RgbColor::new(239, 68, 68);

    if battery_percent >= 50 {
        let t = f32::from(battery_percent.min(100) - 50) / 50.0;
        lerp_color(amber, emerald, t)
    } else if battery_percent >= 15 {
        let t = f32::from(battery_percent - 15) / 35.0;
        lerp_color(critical, amber, t)
    } else {
        critical
    }
}

/// Blends the active profile color toward a crimson/white flash proportional to L2/R2 trigger pull.
#[must_use]
pub fn compute_input_reactive_color(base: RgbColor, trigger_depth: u8) -> RgbColor {
    let flash = RgbColor::new(255, 36, 72);
    let ratio = f32::from(trigger_depth) / 255.0;
    lerp_color(base, flash, ratio)
}

/// Modulates brightness and highlights beats from loopback bass energy (`0.0..=1.0`).
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[must_use]
pub fn compute_audio_beat_color(base: RgbColor, bass_energy: f32) -> RgbColor {
    let energy = bass_energy.clamp(0.0, 1.0);
    let scale = 0.25 + 0.75 * energy;
    let boost = if energy > 0.8 { 20 } else { 0 };
    RgbColor::new(
        ((f32::from(base.r) * scale).round() as u8).saturating_add(boost),
        ((f32::from(base.g) * scale).round() as u8).saturating_add(boost),
        ((f32::from(base.b) * scale).round() as u8).saturating_add(boost),
    )
}

/// Pure dispatcher returning the computed RGB frame for a given effect mode and telemetry input.
#[must_use]
pub fn evaluate_effect_frame(mode: LightingEffectMode, input: &EffectFrameInput) -> RgbColor {
    match mode {
        LightingEffectMode::Static => input.base_color,
        LightingEffectMode::Breathing => compute_breathing_color(
            input.base_color,
            input.secondary_color,
            input.elapsed_secs,
            input.speed_hz,
        ),
        LightingEffectMode::Spectrum => compute_spectrum_color(input.elapsed_secs, input.speed_hz),
        LightingEffectMode::BatterySmart => compute_battery_smart_color(input.battery_percent),
        LightingEffectMode::InputReactive => {
            compute_input_reactive_color(input.base_color, input.trigger_depth)
        }
        LightingEffectMode::AudioBeat => {
            compute_audio_beat_color(input.base_color, input.audio_bass_energy)
        }
        LightingEffectMode::ScreenAmbilight => input.screen_dominant,
    }
}

/// Cancelable async 30–60 Hz Tokio effect loop emitting live RGB frames without blocking UI.
pub struct EffectEngine {
    stop_tx: watch::Sender<bool>,
    stopped: Arc<AtomicBool>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl EffectEngine {
    /// Spawns a cancelable background tick loop invoking `on_frame` at `tick_ms` cadence.
    #[must_use]
    pub fn spawn<F>(mode: LightingEffectMode, tick_ms: u64, mut on_frame: F) -> Self
    where
        F: FnMut(RgbColor) + Send + 'static,
    {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let stopped = Arc::new(AtomicBool::new(false));
        let stopped_flag = Arc::clone(&stopped);
        let period = Duration::from_millis(tick_ms.max(16));

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(period);
            let start = tokio::time::Instant::now();
            loop {
                tokio::select! {
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        let input = EffectFrameInput {
                            elapsed_secs: start.elapsed().as_secs_f32(),
                            ..EffectFrameInput::default()
                        };
                        on_frame(evaluate_effect_frame(mode, &input));
                    }
                }
            }
            stopped_flag.store(true, Ordering::Release);
        });

        Self {
            stop_tx,
            stopped,
            handle: Some(handle),
        }
    }

    /// Signals cancellation and awaits clean shutdown without leaving orphaned background tasks.
    pub async fn shutdown(mut self) {
        let _ = self.stop_tx.send(true);
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
        self.stopped.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_smart_curve_maps_emerald_amber_and_critical_red() {
        assert_eq!(
            compute_battery_smart_color(100),
            RgbColor::new(16, 185, 129)
        );
        assert_eq!(compute_battery_smart_color(50), RgbColor::new(245, 158, 11));
        assert_eq!(compute_battery_smart_color(10), RgbColor::new(239, 68, 68));
    }

    #[test]
    fn breathing_and_spectrum_cycle_smoothly_between_colors() {
        let cyan = RgbColor::new(0, 229, 255);
        let violet = RgbColor::new(168, 85, 247);

        let mid = compute_breathing_color(cyan, violet, 0.0, 1.0);
        assert_eq!(mid, RgbColor::new(84, 157, 251));

        let red = compute_spectrum_color(0.0, 1.0);
        let blue = compute_spectrum_color(0.6667, 1.0);
        assert_eq!(red, RgbColor::new(255, 0, 0));
        assert!(blue.b > 200);
    }

    #[test]
    fn input_and_audio_modes_respond_proportionally() {
        let base = RgbColor::new(0, 100, 200);
        assert_eq!(compute_input_reactive_color(base, 0), base);
        let full_pull = compute_input_reactive_color(base, 255);
        assert_eq!(full_pull, RgbColor::new(255, 36, 72));

        let quiet = compute_audio_beat_color(base, 0.0);
        let drop = compute_audio_beat_color(base, 1.0);
        assert!(drop.b > quiet.b);
    }

    #[tokio::test]
    async fn effect_engine_shuts_down_cleanly_without_task_leaks() {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_ref = Arc::clone(&count);
        let engine = EffectEngine::spawn(LightingEffectMode::Spectrum, 16, move |_| {
            count_ref.fetch_add(1, Ordering::Relaxed);
        });

        tokio::time::sleep(Duration::from_millis(45)).await;
        let stopped_ref = Arc::clone(&engine.stopped);
        engine.shutdown().await;

        assert!(stopped_ref.load(Ordering::Acquire));
        assert!(count.load(Ordering::Relaxed) >= 1);
    }
}

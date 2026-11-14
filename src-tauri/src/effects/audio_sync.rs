//! Audio-reactive beat and bass envelope analyzer for DualSense lightbar synchronization.
//!
//! Designed to consume normalized `[-1.0, 1.0]` PCM audio frames from a desktop
//! loopback stream (e.g., WASAPI / PipeWire) or simulated audio buffers and emit
//! beat-reactive color frames.

use crate::hid::color::RgbColor;

/// Analyzed energy frame produced by [`AudioBeatDetector`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioBeatFrame {
    /// Instantaneous normalized RMS audio energy `[0.0, 1.0]`.
    pub energy: f32,
    /// Low-pass filtered bass envelope `[0.0, 1.0]`.
    pub bass_energy: f32,
    /// True when the current buffer triggers a bass/beat transient peak.
    pub is_beat: bool,
}

/// Stateful PCM audio envelope tracker and beat detector.
#[derive(Debug, Clone)]
pub struct AudioBeatDetector {
    bass_lowpass_state: f32,
    running_average: f32,
    smoothed_energy: f32,
    beat_threshold_multiplier: f32,
    min_beat_energy: f32,
}

impl Default for AudioBeatDetector {
    fn default() -> Self {
        Self::new(1.45, 0.08)
    }
}

impl AudioBeatDetector {
    /// Creates a beat detector with tunable sensitivity multiplier and minimum floor.
    pub fn new(beat_threshold_multiplier: f32, min_beat_energy: f32) -> Self {
        Self {
            bass_lowpass_state: 0.0,
            running_average: 0.0,
            smoothed_energy: 0.0,
            beat_threshold_multiplier: beat_threshold_multiplier.max(1.05),
            min_beat_energy: min_beat_energy.clamp(0.01, 0.9),
        }
    }

    /// Processes a chunk of normalized `[-1.0, 1.0]` PCM samples and updates the beat state.
    pub fn process_pcm_samples(&mut self, samples: &[f32]) -> AudioBeatFrame {
        if samples.is_empty() {
            self.smoothed_energy *= 0.85;
            return AudioBeatFrame {
                energy: self.smoothed_energy,
                bass_energy: 0.0,
                is_beat: false,
            };
        }

        let mut total_sq = 0.0f32;
        let mut bass_sq = 0.0f32;
        // Single-pole IIR low-pass approximates sub-bass/kick envelope cleanly in O(N).
        let lowpass_alpha = 0.14f32;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            total_sq += clamped * clamped;
            self.bass_lowpass_state += lowpass_alpha * (clamped - self.bass_lowpass_state);
            bass_sq += self.bass_lowpass_state * self.bass_lowpass_state;
        }

        let len = samples.len() as f32;
        let rms = (total_sq / len).sqrt().clamp(0.0, 1.0);
        let bass_rms = ((bass_sq / len).sqrt() * 2.2).clamp(0.0, 1.0);

        self.smoothed_energy = self.smoothed_energy * 0.45 + rms * 0.55;
        let previous_avg = self.running_average;
        self.running_average = self.running_average * 0.88 + bass_rms * 0.12;

        let is_beat = bass_rms >= self.min_beat_energy
            && (previous_avg <= 0.005 || bass_rms >= previous_avg * self.beat_threshold_multiplier);

        AudioBeatFrame {
            energy: self.smoothed_energy,
            bass_energy: bass_rms,
            is_beat,
        }
    }

    /// Maps an [`AudioBeatFrame`] onto a vibrant RGB color based on the user's base accent.
    pub fn color_for_frame(&self, base: RgbColor, frame: AudioBeatFrame) -> RgbColor {
        let floor = 0.12f32;
        let boost = if frame.is_beat { 1.0 } else { 0.72 };
        let scale = (floor + frame.energy.max(frame.bass_energy) * 0.88 * boost).clamp(0.05, 1.0);

        if frame.is_beat {
            // Blend subtly toward crisp white/cyan peak during transients.
            let r = (f32::from(base.r) * scale + 36.0).round().clamp(0.0, 255.0) as u8;
            let g = (f32::from(base.g) * scale + 42.0).round().clamp(0.0, 255.0) as u8;
            let b = (f32::from(base.b) * scale + 52.0).round().clamp(0.0, 255.0) as u8;
            RgbColor::new(r, g, b)
        } else {
            RgbColor::new(
                (f32::from(base.r) * scale).round().clamp(0.0, 255.0) as u8,
                (f32::from(base.g) * scale).round().clamp(0.0, 255.0) as u8,
                (f32::from(base.b) * scale).round().clamp(0.0, 255.0) as u8,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_decays_without_triggering_beats() {
        let mut detector = AudioBeatDetector::default();
        let silence = vec![0.0f32; 256];
        let frame = detector.process_pcm_samples(&silence);
        assert!(!frame.is_beat);
        assert!(frame.energy < 0.01);
        assert!(frame.bass_energy < 0.01);
    }

    #[test]
    fn strong_low_frequency_pulse_triggers_beat_peak() {
        let mut detector = AudioBeatDetector::default();
        // Prime running average with quiet ambience
        let quiet = vec![0.02f32; 256];
        for _ in 0..4 {
            let _ = detector.process_pcm_samples(&quiet);
        }

        // Low-frequency kick impulse (sustained positive half-cycle + decay)
        let mut kick = vec![0.95f32; 96];
        kick.extend(vec![0.4f32; 64]);
        let frame = detector.process_pcm_samples(&kick);

        assert!(frame.is_beat);
        assert!(frame.bass_energy > 0.25);

        let accent = RgbColor::new(0, 120, 255);
        let lit = detector.color_for_frame(accent, frame);
        assert!(lit.b >= 200);
    }

    #[test]
    fn empty_buffer_does_not_panic() {
        let mut detector = AudioBeatDetector::default();
        let frame = detector.process_pcm_samples(&[]);
        assert!(!frame.is_beat);
        assert_eq!(frame.bass_energy, 0.0);
    }
}

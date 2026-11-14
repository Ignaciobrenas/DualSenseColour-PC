//! Screen Ambilight dominant-color sampler with temporal smoothing.
//!
//! Downsamples RGBA framebuffer slices with configurable stride, ignores near-black
//! letterboxing/HUD regions, boosts vividness gently, and applies exponential
//! smoothing so DualSense lightbar transitions stay cinematic without flicker.

use crate::hid::color::RgbColor;

/// Stateful Ambilight color extractor and temporal smoother.
#[derive(Debug, Clone)]
pub struct ScreenAmbilightSampler {
    current: RgbColor,
    initialized: bool,
}

impl Default for ScreenAmbilightSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenAmbilightSampler {
    /// Creates an uninitialized Ambilight sampler.
    pub fn new() -> Self {
        Self {
            current: RgbColor::new(0, 67, 156),
            initialized: false,
        }
    }

    /// Extracts the dominant non-black color from a tightly packed RGBA8 buffer.
    ///
    /// Returns [`RgbColor::new(0, 0, 0)`] if the buffer is empty or completely dark.
    pub fn sample_rgba_buffer(rgba: &[u8], stride_pixels: usize) -> RgbColor {
        let step = stride_pixels.max(1);
        let mut sum_r: u64 = 0;
        let mut sum_g: u64 = 0;
        let mut sum_b: u64 = 0;
        let mut weight_total: u64 = 0;

        for chunk in rgba.as_chunks::<4>().0.iter().step_by(step) {
            let r = chunk[0];
            let g = chunk[1];
            let b = chunk[2];
            let a = chunk[3];
            if a < 16 {
                continue;
            }

            let max_c = r.max(g).max(b);
            let min_c = r.min(g).min(b);
            // Skip near-black cinema bars and dark backgrounds so vivid game highlights lead.
            if max_c < 24 {
                continue;
            }

            let saturation_bonus = u64::from(max_c - min_c) / 8 + 1;
            let luma_weight = u64::from(max_c) / 16 + 1;
            let weight = saturation_bonus * luma_weight;

            sum_r += u64::from(r) * weight;
            sum_g += u64::from(g) * weight;
            sum_b += u64::from(b) * weight;
            weight_total += weight;
        }

        if weight_total == 0 {
            return RgbColor::new(0, 0, 0);
        }

        let avg_r = (sum_r / weight_total).min(255) as u8;
        let avg_g = (sum_g / weight_total).min(255) as u8;
        let avg_b = (sum_b / weight_total).min(255) as u8;

        Self::boost_vividness(RgbColor::new(avg_r, avg_g, avg_b))
    }

    /// Applies exponential moving average smoothing toward `target` with factor `alpha` `[0.05, 1.0]`.
    pub fn smooth_step(&mut self, target: RgbColor, alpha: f32) -> RgbColor {
        let factor = alpha.clamp(0.05, 1.0);
        if !self.initialized {
            self.current = target;
            self.initialized = true;
            return target;
        }

        let r = (f32::from(self.current.r) * (1.0 - factor) + f32::from(target.r) * factor)
            .round()
            .clamp(0.0, 255.0) as u8;
        let g = (f32::from(self.current.g) * (1.0 - factor) + f32::from(target.g) * factor)
            .round()
            .clamp(0.0, 255.0) as u8;
        let b = (f32::from(self.current.b) * (1.0 - factor) + f32::from(target.b) * factor)
            .round()
            .clamp(0.0, 255.0) as u8;

        self.current = RgbColor::new(r, g, b);
        self.current
    }

    fn boost_vividness(color: RgbColor) -> RgbColor {
        let mean = (f32::from(color.r) + f32::from(color.g) + f32::from(color.b)) / 3.0;
        let boost = 1.18f32;
        let r = (mean + (f32::from(color.r) - mean) * boost)
            .round()
            .clamp(0.0, 255.0) as u8;
        let g = (mean + (f32::from(color.g) - mean) * boost)
            .round()
            .clamp(0.0, 255.0) as u8;
        let b = (mean + (f32::from(color.b) - mean) * boost)
            .round()
            .clamp(0.0, 255.0) as u8;
        RgbColor::new(r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_dominant_color_and_ignores_black_letterboxing() {
        let mut frame = Vec::new();
        // Top black cinema bars
        for _ in 0..40 {
            frame.extend_from_slice(&[4, 4, 6, 255]);
        }
        // Vivid cyan gameplay area
        for _ in 0..40 {
            frame.extend_from_slice(&[0, 210, 245, 255]);
        }

        let color = ScreenAmbilightSampler::sample_rgba_buffer(&frame, 1);
        assert!(color.g > 180 && color.b > 220);
        assert!(color.r < 30);
    }

    #[test]
    fn smooth_step_interpolates_without_snapping() {
        let mut sampler = ScreenAmbilightSampler::new();
        let initial = sampler.smooth_step(RgbColor::new(0, 0, 255), 0.25);
        assert_eq!(initial, RgbColor::new(0, 0, 255));

        let next = sampler.smooth_step(RgbColor::new(255, 0, 0), 0.25);
        assert!(next.r > 50 && next.r < 90);
        assert!(next.b > 170 && next.b < 210);
    }
}

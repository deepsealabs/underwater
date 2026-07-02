//! Manual color adjustments: white balance, exposure, contrast, saturation,
//! vibrance.
//!
//! White balance and exposure operate in linear light (physically correct
//! scaling); contrast, saturation, and vibrance operate on the gamma-encoded
//! signal afterward, matching how these controls are conventionally exposed
//! in photo/video editors.

use image::{Rgb, RgbImage};

/// A single non-destructive adjustment pass. All fields default to 0.0 (no-op).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorAdjustments {
    /// Blue <-> orange shift, in linear light. -1.0 = cooler/blue, 1.0 = warmer/orange.
    pub temperature: f32,
    /// Green <-> magenta shift, in linear light. -1.0 = green, 1.0 = magenta.
    pub tint: f32,
    /// Stops of linear-light exposure change. +1.0 doubles brightness.
    pub exposure: f32,
    /// -1.0 = flat, 1.0 = high contrast, pivoted around mid-grey.
    pub contrast: f32,
    /// -1.0 = grayscale, 1.0 = doubled saturation.
    pub saturation: f32,
    /// Like `saturation` but scales less on pixels that are already
    /// saturated, protecting skin tones / already-vivid colors from clipping.
    pub vibrance: f32,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
            exposure: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            vibrance: 0.0,
        }
    }
}

impl ColorAdjustments {
    pub fn apply(&self, img: &RgbImage) -> RgbImage {
        let mut out = RgbImage::new(img.width(), img.height());
        for (x, y, pixel) in img.enumerate_pixels() {
            let mut linear = srgb_u8_to_linear(pixel.0);
            linear = self.apply_white_balance(linear);
            linear = self.apply_exposure(linear);
            let mut gamma = linear_to_srgb(linear);
            gamma = self.apply_contrast(gamma);
            gamma = self.apply_saturation_and_vibrance(gamma);
            out.put_pixel(x, y, Rgb(srgb_to_u8(gamma)));
        }
        out
    }

    fn apply_white_balance(&self, mut c: [f32; 3]) -> [f32; 3] {
        const STRENGTH: f32 = 0.4;
        c[0] *= 1.0 + self.temperature * STRENGTH; // red warms up
        c[2] *= 1.0 - self.temperature * STRENGTH; // blue cools down
        c[1] *= 1.0 - self.tint * STRENGTH * 0.5; // positive tint -> magenta (less green)
        c
    }

    fn apply_exposure(&self, c: [f32; 3]) -> [f32; 3] {
        let factor = 2f32.powf(self.exposure);
        [c[0] * factor, c[1] * factor, c[2] * factor]
    }

    fn apply_contrast(&self, c: [f32; 3]) -> [f32; 3] {
        // Slope pivoted at mid-grey (0.5); tan() keeps +/-1.0 well-behaved
        // instead of the curve going vertical or flat at the extremes.
        let factor = ((self.contrast.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4).tan();
        [
            (c[0] - 0.5) * factor + 0.5,
            (c[1] - 0.5) * factor + 0.5,
            (c[2] - 0.5) * factor + 0.5,
        ]
    }

    fn apply_saturation_and_vibrance(&self, c: [f32; 3]) -> [f32; 3] {
        let luma = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let current_sat = {
            let max = c[0].max(c[1]).max(c[2]);
            let min = c[0].min(c[1]).min(c[2]);
            max - min
        };
        let vibrance_weight = 1.0 - current_sat.clamp(0.0, 1.0);
        let factor = 1.0 + self.saturation + self.vibrance * vibrance_weight;
        [
            luma + (c[0] - luma) * factor,
            luma + (c[1] - luma) * factor,
            luma + (c[2] - luma) * factor,
        ]
    }
}

fn srgb_u8_to_linear(rgb: [u8; 3]) -> [f32; 3] {
    [
        srgb_channel_to_linear(rgb[0]),
        srgb_channel_to_linear(rgb[1]),
        srgb_channel_to_linear(rgb[2]),
    ]
}

fn srgb_channel_to_linear(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: [f32; 3]) -> [f32; 3] {
    [
        linear_channel_to_srgb(c[0]),
        linear_channel_to_srgb(c[1]),
        linear_channel_to_srgb(c[2]),
    ]
}

fn linear_channel_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_to_u8(c: [f32; 3]) -> [u8; 3] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(r: u8, g: u8, b: u8) -> RgbImage {
        let mut img = RgbImage::new(1, 1);
        img.put_pixel(0, 0, Rgb([r, g, b]));
        img
    }

    #[test]
    fn no_op_adjustment_is_identity() {
        let img = pixel(128, 64, 200);
        let out = ColorAdjustments::default().apply(&img);
        let original = img.get_pixel(0, 0).0;
        let result = out.get_pixel(0, 0).0;
        for i in 0..3 {
            assert!(
                (original[i] as i16 - result[i] as i16).abs() <= 2,
                "channel {i}: expected ~{}, got {}",
                original[i],
                result[i]
            );
        }
    }

    #[test]
    fn exposure_brightens_midtones() {
        let img = pixel(128, 128, 128);
        let adj = ColorAdjustments {
            exposure: 1.0,
            ..Default::default()
        };
        let out = adj.apply(&img).get_pixel(0, 0).0;
        assert!(out[0] > 128, "expected brighter than input, got {out:?}");
    }

    #[test]
    fn negative_saturation_desaturates_toward_grayscale() {
        let img = pixel(200, 50, 50);
        let adj = ColorAdjustments {
            saturation: -1.0,
            ..Default::default()
        };
        let out = adj.apply(&img).get_pixel(0, 0).0;
        let spread = out[0] as i16 - out[2] as i16;
        assert!(spread.abs() < 30, "expected near-grayscale, got {out:?}");
    }

    #[test]
    fn warm_temperature_increases_red_relative_to_blue() {
        let img = pixel(128, 128, 128);
        let adj = ColorAdjustments {
            temperature: 1.0,
            ..Default::default()
        };
        let out = adj.apply(&img).get_pixel(0, 0).0;
        assert!(
            out[0] > out[2],
            "expected red > blue after warming, got {out:?}"
        );
    }
}

//! One-click auto-correct: Ancuti, Ancuti, De Vleeschouwer, Bekaert,
//! "Color Balance and Fusion for Underwater Image Enhancement," IEEE TIP
//! vol. 27 no. 1, 2018. No depth map, no training data — see
//! `Docs/ROADMAP.md` Phase 1a for the full algorithm writeup and the
//! pitfalls found cross-checking two reference implementations against
//! the paper text.

mod branches;
mod buffer;
mod pyramid;
mod weights;

use buffer::Buffer;
use image::RgbImage;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FusionParams {
    /// Eq. 4's alpha. Paper's own stated default is 1.0 — one reference
    /// port uses 0.1 (10x weaker), which isn't what the paper says.
    pub red_compensation_alpha: f32,
    /// Branch A's gamma. Not numerically specified in the paper; tune
    /// against real fixtures rather than treating this as derived.
    pub gamma: f32,
    /// Branch B's unsharp-mask blur sigma, as a fraction of the image's
    /// shorter side (so it scales sensibly across resolutions).
    pub unsharp_sigma_fraction: f32,
    /// Eq. 7's regularization term. Paper's stated value: 0.1.
    pub delta: f32,
}

impl Default for FusionParams {
    fn default() -> Self {
        Self {
            red_compensation_alpha: 1.0,
            gamma: 1.2,
            unsharp_sigma_fraction: 0.01,
            delta: 0.1,
        }
    }
}

pub fn auto_correct(img: &RgbImage, params: &FusionParams) -> RgbImage {
    let input = Buffer::from_rgb_image(img);

    // Scale both correction stages by the scene's own hue diversity before
    // doing anything else -- a near-monochromatic water column (dive cave,
    // kelp forest midwater) overcorrects toward pastel/off-hue at the
    // paper's defaults, because both red compensation and gray-world lean
    // on global scene statistics that assume some color variety exists to
    // calibrate against. See `branches::diversity_trust` for why plain
    // saturation doesn't predict this.
    let trust = branches::diversity_trust(&input);
    let compensated = branches::compensate_red(&input, params.red_compensation_alpha * trust);
    let white_balanced = branches::gray_world_white_balance(&compensated, trust);

    let branch_gamma = branches::gamma_correct(&white_balanced, params.gamma);
    let sigma_px = params.unsharp_sigma_fraction * input.width.min(input.height) as f32;
    let branch_sharp = branches::unsharp_mask_normalized(&white_balanced, sigma_px);

    let w_gamma = weights::aggregate_weight(&branch_gamma);
    let w_sharp = weights::aggregate_weight(&branch_sharp);
    let (w_gamma_n, w_sharp_n) = weights::normalize_pair(&w_gamma, &w_sharp, params.delta);

    let levels = pyramid::pyramid_depth(input.width, input.height);
    let fused = pyramid::fuse(&branch_sharp, &w_sharp_n, &branch_gamma, &w_gamma_n, levels);

    fused.to_rgb_image()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgb;

    #[test]
    fn auto_correct_preserves_dimensions() {
        let mut img = RgbImage::new(16, 12);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = Rgb([20, (x * 8) as u8, (y * 10 + 40) as u8]);
        }
        let out = auto_correct(&img, &FusionParams::default());
        assert_eq!(out.dimensions(), img.dimensions());
    }

    #[test]
    fn auto_correct_reduces_blue_cast_on_synthetic_underwater_image() {
        // Strong blue/green cast, dead red channel: the canonical
        // underwater signature. Auto-correct should lift red relative to
        // blue more than it started.
        let mut img = RgbImage::new(20, 20);
        for px in img.pixels_mut() {
            *px = Rgb([15, 90, 110]);
        }
        let out = auto_correct(&img, &FusionParams::default());

        let before_r_minus_b = 15i32 - 110;
        let sample = out.get_pixel(10, 10);
        let after_r_minus_b = sample[0] as i32 - sample[2] as i32;
        assert!(
            after_r_minus_b > before_r_minus_b,
            "expected red to close the gap with blue, before={before_r_minus_b} after={after_r_minus_b}"
        );
    }
}

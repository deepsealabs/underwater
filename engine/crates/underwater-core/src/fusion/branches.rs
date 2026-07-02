//! The two Ancuti et al. input branches, built from one white-balanced image.

use super::buffer::{subtract, Buffer};
use super::pyramid;

/// Eq. 4: exploits that green survives underwater attenuation best, using
/// it as the opponent-color reference to restore red. The `(1 - r)` term
/// suppresses correction on already-red pixels (avoids reddening
/// strobe-lit foreground subjects further).
///
/// `alpha` here is expected to already be scene-adjusted by the caller
/// (see `diversity_scale`) — the paper's formula assumes a scene has
/// enough hue variety that pushing the global mean gap onto every pixel
/// mostly "restores" real color rather than just uniformly desaturating
/// the frame. On a near-monochromatic water column (a dive cave, a kelp
/// forest's blue-green water column) that assumption doesn't hold, and
/// unscaled alpha=1 washes the whole frame toward pastel.
pub(crate) fn compensate_red(buf: &Buffer, alpha: f32) -> Buffer {
    let n = buf.data.len() as f32;
    let (mut sum_r, mut sum_g) = (0f32, 0f32);
    for c in &buf.data {
        sum_r += c[0];
        sum_g += c[1];
    }
    let mean_r = sum_r / n;
    let mean_g = sum_g / n;

    let mut out = buf.clone();
    for px in out.data.iter_mut() {
        let r = px[0];
        let g = px[1];
        px[0] = (r + alpha * (mean_g - mean_r) * (1.0 - r) * g).clamp(0.0, 1.0);
    }
    out
}

/// Circular variance of hue across the scene's non-gray pixels: 0 = every
/// colored pixel shares one hue (a uniformly blue water column, even if
/// each pixel individually reads as highly "saturated"), ~1 = hues spread
/// evenly around the color wheel (varied subject matter — coral, fish,
/// substrate). Near-gray pixels (HSV saturation < 0.1) are excluded since
/// their hue is numerically unstable and not meaningful.
///
/// This is *not* the same signal as mean saturation — measured against the
/// real test fixtures, saturation alone doesn't separate the scenes that
/// overcorrect from the ones that don't (a strongly blue-tinted cave shot
/// scores *higher* mean saturation than a colorful coral reef). Hue
/// diversity is what actually predicts it: gray-world assumes the scene's
/// colors average toward neutral, which requires variety, not vividness.
pub(crate) fn hue_diversity(buf: &Buffer) -> f32 {
    let mut sin_sum = 0f32;
    let mut cos_sum = 0f32;
    let mut n = 0f32;

    for c in &buf.data {
        let max = c[0].max(c[1]).max(c[2]);
        let min = c[0].min(c[1]).min(c[2]);
        let delta = max - min;
        let sat = if max > 1e-6 { delta / max } else { 0.0 };
        if sat < 0.1 {
            continue;
        }

        let hue_deg = if max == c[0] {
            60.0 * ((c[1] - c[2]) / delta)
        } else if max == c[1] {
            60.0 * ((c[2] - c[0]) / delta + 2.0)
        } else {
            60.0 * ((c[0] - c[1]) / delta + 4.0)
        };
        let hue = hue_deg.rem_euclid(360.0).to_radians();

        sin_sum += hue.sin();
        cos_sum += hue.cos();
        n += 1.0;
    }

    if n < 1.0 {
        return 0.0; // no colored pixels at all: treat as zero diversity
    }
    let mean_resultant_length = (sin_sum * sin_sum + cos_sum * cos_sum).sqrt() / n;
    1.0 - mean_resultant_length
}

/// Maps hue diversity to a trust factor in [MIN_SCALE, 1.0], used to scale
/// down *both* red compensation and gray-world correction below. Thresholds
/// are calibrated against the real test fixtures in
/// `engine/tests/fixtures/` (see `ATTRIBUTION.md`): scenes that
/// overcorrected (dive cave, kelp forest water column, murky lake)
/// measured circular variance 0.008-0.084; scenes that looked good (coral
/// reef, strobe-lit macro, floodlit wreck) measured 0.39-0.76. The ramp
/// sits in the gap between those clusters.
///
/// Both stages need this, not just red compensation: a bounded gray-world
/// gain still overshoots on an all-water-column frame, because gray-world
/// has no concept of a *correct* target — it just forces the mean toward
/// neutral, and there's no genuinely gray content in frame to calibrate
/// against when the whole scene is one color. Low diversity means "trust
/// the global statistics less," for either correction.
pub(crate) fn diversity_trust(buf: &Buffer) -> f32 {
    const LOW: f32 = 0.05;
    const HIGH: f32 = 0.35;
    const MIN_SCALE: f32 = 0.15;
    let circular_variance = hue_diversity(buf);
    let t = ((circular_variance - LOW) / (HIGH - LOW)).clamp(0.0, 1.0);
    MIN_SCALE + t * (1.0 - MIN_SCALE)
}

/// Removes residual illuminant cast after red compensation. Plain
/// per-channel mean-matching to global luma — the paper names "Gray-World"
/// without an equation; this is the simplest faithful reading.
///
/// `trust` (from `diversity_trust`) blends each channel's gain toward 1.0
/// (no-op) as scene hue diversity drops, on top of a fixed gain clamp —
/// belt-and-suspenders against the same overcorrection failure mode as
/// `compensate_red`'s adaptive alpha.
pub(crate) fn gray_world_white_balance(buf: &Buffer, trust: f32) -> Buffer {
    const MAX_GAIN: f32 = 1.8;
    const MIN_GAIN: f32 = 1.0 / MAX_GAIN;

    let n = buf.data.len() as f32;
    let mut sums = [0f32; 3];
    for c in &buf.data {
        for i in 0..3 {
            sums[i] += c[i];
        }
    }
    let means = sums.map(|s| s / n);
    let gray = (means[0] + means[1] + means[2]) / 3.0;
    let gains = means.map(|m| {
        let raw_gain = if m > 1e-6 { (gray / m).clamp(MIN_GAIN, MAX_GAIN) } else { 1.0 };
        1.0 + (raw_gain - 1.0) * trust
    });

    let mut out = buf.clone();
    for px in out.data.iter_mut() {
        for i in 0..3 {
            px[i] = (px[i] * gains[i]).clamp(0.0, 1.0);
        }
    }
    out
}

/// Branch A. The paper doesn't give a numeric gamma — it's a tunable set
/// empirically, not a derived constant.
pub(crate) fn gamma_correct(buf: &Buffer, gamma: f32) -> Buffer {
    let mut out = buf.clone();
    for px in out.data.iter_mut() {
        for c in px.iter_mut() {
            *c = c.clamp(0.0, 1.0).powf(1.0 / gamma);
        }
    }
    out
}

/// Branch B, Eq. 6: `S = (I + N{I - G*I}) / 2`, where `N{}` is min-max
/// normalization — deliberately not the traditional `I + beta(I - G*I)`
/// unsharp mask, which needs a fragile beta tuned per image. Preserve the
/// normalized form exactly; it's the one part of the paper that removes a
/// knob rather than adding one.
pub(crate) fn unsharp_mask_normalized(buf: &Buffer, sigma_px: f32) -> Buffer {
    let blurred = pyramid::gaussian_blur_sigma(buf, sigma_px);
    let diff = subtract(buf, &blurred);
    let normalized = normalize_minmax(&diff);

    let mut out = Buffer::new(buf.width, buf.height);
    for i in 0..buf.data.len() {
        for c in 0..3 {
            out.data[i][c] = ((buf.data[i][c] + normalized.data[i][c]) / 2.0).clamp(0.0, 1.0);
        }
    }
    out
}

fn normalize_minmax(buf: &Buffer) -> Buffer {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for c in &buf.data {
        for &v in c {
            min = min.min(v);
            max = max.max(v);
        }
    }
    let range = (max - min).max(1e-6);

    let mut out = Buffer::new(buf.width, buf.height);
    for i in 0..buf.data.len() {
        for c in 0..3 {
            out.data[i][c] = (buf.data[i][c] - min) / range;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(w: u32, h: u32, v: [f32; 3]) -> Buffer {
        let mut buf = Buffer::new(w, h);
        for px in buf.data.iter_mut() {
            *px = v;
        }
        buf
    }

    #[test]
    fn red_compensation_boosts_low_red_where_green_present() {
        // Low red, strong green -> the classic underwater cast this
        // formula targets.
        let buf = flat(4, 4, [0.1, 0.6, 0.3]);
        let out = compensate_red(&buf, 1.0);
        assert!(out.data[0][0] > buf.data[0][0]);
    }

    #[test]
    fn red_compensation_leaves_already_red_pixels_alone() {
        // r close to 1.0 -> (1 - r) suppresses the correction near zero.
        let buf = flat(4, 4, [0.95, 0.6, 0.3]);
        let out = compensate_red(&buf, 1.0);
        assert!((out.data[0][0] - buf.data[0][0]).abs() < 0.06);
    }

    #[test]
    fn gray_world_equalizes_channel_means_within_moderate_imbalance_at_full_trust() {
        // Moderate imbalance (well within the gain clamp), full trust ->
        // should still fully equalize, same as unclamped gray-world would.
        let mut buf = Buffer::new(8, 8);
        for (i, px) in buf.data.iter_mut().enumerate() {
            *px = [0.3, 0.5, 0.4 + (i as f32 / 640.0)];
        }
        let out = gray_world_white_balance(&buf, 1.0);
        let n = out.data.len() as f32;
        let mut sums = [0f32; 3];
        for c in &out.data {
            for i in 0..3 {
                sums[i] += c[i];
            }
        }
        let means = sums.map(|s| s / n);
        assert!((means[0] - means[1]).abs() < 0.01);
        assert!((means[1] - means[2]).abs() < 0.01);
    }

    #[test]
    fn gray_world_bounds_gain_on_extreme_imbalance_at_full_trust() {
        // Red mean 0.1 vs green mean 0.5 would need a 5x gain to fully
        // equalize -- clamped to 1.8x instead, so it should land well
        // short of the green/blue means, not exactly match them.
        let buf = flat(4, 4, [0.1, 0.5, 0.5]);
        let out = gray_world_white_balance(&buf, 1.0);
        assert!(out.data[0][0] < 0.3, "expected bounded correction, got {}", out.data[0][0]);
    }

    #[test]
    fn gray_world_is_near_identity_at_zero_trust() {
        let buf = flat(4, 4, [0.1, 0.5, 0.5]);
        let out = gray_world_white_balance(&buf, 0.0);
        assert!((out.data[0][0] - buf.data[0][0]).abs() < 1e-5);
    }

    #[test]
    fn hue_diversity_is_near_zero_for_monochromatic_scene() {
        // Every pixel the same hue (uniformly blue-cast) -> low diversity,
        // even though each pixel is individually highly saturated.
        let buf = flat(8, 8, [0.05, 0.35, 0.45]);
        assert!(hue_diversity(&buf) < 0.05);
    }

    #[test]
    fn hue_diversity_is_high_for_varied_scene() {
        let mut buf = Buffer::new(4, 4);
        let hues = [
            [0.8, 0.1, 0.1], // red
            [0.1, 0.8, 0.1], // green
            [0.1, 0.1, 0.8], // blue
            [0.8, 0.8, 0.1], // yellow
        ];
        for (i, px) in buf.data.iter_mut().enumerate() {
            *px = hues[i % hues.len()];
        }
        assert!(hue_diversity(&buf) > 0.7);
    }

    #[test]
    fn diversity_trust_is_dampened_for_low_diversity_scene() {
        let monochrome = flat(8, 8, [0.05, 0.35, 0.45]);
        let trust = diversity_trust(&monochrome);
        assert!(trust < 0.3, "expected strong damping, got {trust}");
    }

    #[test]
    fn diversity_trust_stays_near_full_for_varied_scene() {
        let mut varied = Buffer::new(4, 4);
        let hues = [[0.8, 0.1, 0.1], [0.1, 0.8, 0.1], [0.1, 0.1, 0.8], [0.8, 0.8, 0.1]];
        for (i, px) in varied.data.iter_mut().enumerate() {
            *px = hues[i % hues.len()];
        }
        let trust = diversity_trust(&varied);
        assert!(trust > 0.7, "expected little damping, got {trust}");
    }

    #[test]
    fn gamma_correct_is_identity_at_one() {
        let buf = flat(2, 2, [0.3, 0.5, 0.7]);
        let out = gamma_correct(&buf, 1.0);
        for c in 0..3 {
            assert!((out.data[0][c] - buf.data[0][c]).abs() < 1e-5);
        }
    }
}

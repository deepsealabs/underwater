//! Weight maps and their normalization. The 2018 paper uses exactly three
//! (not four — it drops the "exposedness" map from the authors' own 2012
//! predecessor): Laplacian contrast, saliency, and saturation.

use super::buffer::Buffer;
use super::pyramid;

pub(crate) fn aggregate_weight(branch: &Buffer) -> Buffer {
    let wl = laplacian_weight(branch);
    let ws = saliency_weight(branch);
    let wsat = saturation_weight(branch);

    let mut out = Buffer::new(branch.width, branch.height);
    for i in 0..out.data.len() {
        let v = wl.data[i][0] + ws.data[i][0] + wsat.data[i][0];
        out.data[i] = [v, v, v];
    }
    out
}

/// Eq. 7's normalization, generalized... here specialized to the paper's
/// two-input case: `W~k = (Wk + delta) / (sum(Wk) + K*delta)`, K=2.
pub(crate) fn normalize_pair(w1: &Buffer, w2: &Buffer, delta: f32) -> (Buffer, Buffer) {
    let mut n1 = Buffer::new(w1.width, w1.height);
    let mut n2 = Buffer::new(w1.width, w1.height);
    for i in 0..w1.data.len() {
        let a = w1.data[i][0];
        let b = w2.data[i][0];
        let denom = (a + b + 2.0 * delta).max(1e-6);
        let na = (a + delta) / denom;
        let nb = (b + delta) / denom;
        n1.data[i] = [na, na, na];
        n2.data[i] = [nb, nb, nb];
    }
    (n1, n2)
}

/// A true Laplacian-kernel convolution on luminance — not a color-deviation
/// formula. (One reference MATLAB port silently reuses the saturation
/// weight's formula here instead; that's a bug, not an alternate reading.)
fn laplacian_weight(branch: &Buffer) -> Buffer {
    let w = branch.width as i64;
    let h = branch.height as i64;
    let luma_at = |x: i64, y: i64| -> f32 {
        let cx = x.clamp(0, w - 1) as u32;
        let cy = y.clamp(0, h - 1) as u32;
        branch.luminance_at((cy * branch.width + cx) as usize)
    };

    let mut out = Buffer::new(branch.width, branch.height);
    for y in 0..h {
        for x in 0..w {
            let center = luma_at(x, y);
            let lap = luma_at(x - 1, y) + luma_at(x + 1, y) + luma_at(x, y - 1) + luma_at(x, y + 1) - 4.0 * center;
            let v = lap.abs();
            out.data[(y * w + x) as usize] = [v, v, v];
        }
    }
    out
}

/// Achanta et al. frequency-tuned saliency: squared Lab distance from the
/// image's mean color, on a lightly blurred input. Favors salient objects
/// over flat background — but also biases toward bright/highlighted
/// regions, which is why the saturation weight below exists alongside it.
fn saliency_weight(branch: &Buffer) -> Buffer {
    let blurred = pyramid::blur(branch);
    let lab: Vec<[f32; 3]> = blurred.data.iter().map(|c| srgb_to_lab(*c)).collect();

    let n = lab.len() as f32;
    let mut mean = [0f32; 3];
    for c in &lab {
        for i in 0..3 {
            mean[i] += c[i];
        }
    }
    for m in mean.iter_mut() {
        *m /= n;
    }

    let mut out = Buffer::new(branch.width, branch.height);
    for (i, c) in lab.iter().enumerate() {
        let d = (c[0] - mean[0]).powi(2) + (c[1] - mean[1]).powi(2) + (c[2] - mean[2]).powi(2);
        out.data[i] = [d, d, d];
    }

    let max = out.data.iter().map(|c| c[0]).fold(0f32, f32::max).max(1e-6);
    for c in out.data.iter_mut() {
        *c = [c[0] / max, c[1] / max, c[2] / max];
    }
    out
}

/// Eq. 7: RMS distance of each channel from luminance. Counteracts the
/// saliency weight's bias toward bright/washed-out (low-saturation)
/// regions by rewarding chromatic pixels instead.
fn saturation_weight(branch: &Buffer) -> Buffer {
    let mut out = Buffer::new(branch.width, branch.height);
    for (i, c) in branch.data.iter().enumerate() {
        let l = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
        let v = (((c[0] - l).powi(2) + (c[1] - l).powi(2) + (c[2] - l).powi(2)) / 3.0).sqrt();
        out.data[i] = [v, v, v];
    }
    out
}

/// sRGB -> linear -> XYZ (D65) -> CIE Lab. L is native [0,100], a/b are
/// native CIE range — NOT rescaled to [0,1]. (A reference MATLAB port
/// divides L by 255 instead of 100, silently desaturating the saliency
/// term relative to the RGB channels it's compared against — don't repeat
/// that.)
fn srgb_to_lab(c: [f32; 3]) -> [f32; 3] {
    let lin = |v: f32| if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    let r = lin(c[0]);
    let g = lin(c[1]);
    let b = lin(c[2]);

    let x = r * 0.4124 + g * 0.3576 + b * 0.1805;
    let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
    let z = r * 0.0193 + g * 0.1192 + b * 0.9505;

    let (xn, yn, zn) = (0.95047, 1.0, 1.08883);
    let f = |t: f32| if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 };
    let (fx, fy, fz) = (f(x / xn), f(y / yn), f(z / zn));

    let l = 116.0 * fy - 16.0;
    let a = 500.0 * (fx - fy);
    let b_ = 200.0 * (fy - fz);
    [l, a, b_]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_pair_sums_to_one() {
        let w1 = {
            let mut b = Buffer::new(2, 2);
            for px in b.data.iter_mut() {
                *px = [0.3, 0.3, 0.3];
            }
            b
        };
        let w2 = {
            let mut b = Buffer::new(2, 2);
            for px in b.data.iter_mut() {
                *px = [0.7, 0.7, 0.7];
            }
            b
        };
        let (n1, n2) = normalize_pair(&w1, &w2, 0.1);
        for i in 0..4 {
            assert!((n1.data[i][0] + n2.data[i][0] - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn saturation_weight_is_zero_for_grayscale() {
        let mut buf = Buffer::new(2, 2);
        for px in buf.data.iter_mut() {
            *px = [0.5, 0.5, 0.5];
        }
        let w = saturation_weight(&buf);
        for c in &w.data {
            assert!(c[0] < 1e-5);
        }
    }

    #[test]
    fn laplacian_weight_is_zero_on_flat_field() {
        let mut buf = Buffer::new(4, 4);
        for px in buf.data.iter_mut() {
            *px = [0.4, 0.4, 0.4];
        }
        let w = laplacian_weight(&buf);
        for c in &w.data {
            assert!(c[0] < 1e-5);
        }
    }
}

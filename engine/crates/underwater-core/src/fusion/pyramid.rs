//! Burt-Adelson Gaussian/Laplacian pyramids and multi-scale fusion.
//!
//! Naive single-scale weighted blending introduces halos at region
//! boundaries — the paper explicitly rejects it in favor of blending each
//! spatial-frequency band separately, then reconstructing.

use super::buffer::{add, subtract, Buffer};

/// Standard Burt-Adelson binomial low-pass kernel, used for both REDUCE and
/// EXPAND so encode/decode stay consistent with each other.
const KERNEL: [f32; 5] = [1.0, 4.0, 6.0, 4.0, 1.0]; // sums to 16

/// Coarsest level's short side should land in the tens-of-pixels range
/// (paper: "e.g., 7 levels for a 600x800 image"), rather than a fixed
/// level count — a fixed count doesn't generalize across the range of
/// photo resolutions this engine needs to handle.
pub fn pyramid_depth(width: u32, height: u32) -> usize {
    let short_side = width.min(height) as f32;
    let n = (short_side / 10.0).log2().floor() as i32 + 1;
    n.clamp(1, 10) as usize
}

fn separable_blur(buf: &Buffer, pass_multiplier: f32) -> Buffer {
    let w = buf.width as i64;
    let h = buf.height as i64;

    let mut horizontal = Buffer::new(buf.width, buf.height);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 3];
            for (k, weight) in KERNEL.iter().enumerate() {
                let dx = k as i64 - 2;
                let px = buf.get(x + dx, y);
                for c in 0..3 {
                    acc[c] += px[c] * weight;
                }
            }
            for c in acc.iter_mut() {
                *c = *c * pass_multiplier / 16.0;
            }
            horizontal.set(x as u32, y as u32, acc);
        }
    }

    let mut out = Buffer::new(buf.width, buf.height);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 3];
            for (k, weight) in KERNEL.iter().enumerate() {
                let dy = k as i64 - 2;
                let px = horizontal.get(x, y + dy);
                for c in 0..3 {
                    acc[c] += px[c] * weight;
                }
            }
            for c in acc.iter_mut() {
                *c = *c * pass_multiplier / 16.0;
            }
            out.set(x as u32, y as u32, acc);
        }
    }
    out
}

/// Used by the saliency weight map (a small fixed blur, not a pyramid step).
pub fn blur(buf: &Buffer) -> Buffer {
    separable_blur(buf, 1.0)
}

/// General-sigma Gaussian blur for the unsharp-mask branch, where the blur
/// radius is a tunable parameter rather than the fixed pyramid kernel.
pub fn gaussian_blur_sigma(buf: &Buffer, sigma: f32) -> Buffer {
    let sigma = sigma.max(0.5);
    let radius = (sigma * 3.0).ceil().max(1.0) as i64;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0f32;
    for i in -radius..=radius {
        let v = (-((i * i) as f32) / (2.0 * sigma * sigma)).exp();
        kernel.push(v);
        sum += v;
    }
    for v in kernel.iter_mut() {
        *v /= sum;
    }

    let w = buf.width as i64;
    let h = buf.height as i64;
    let mut horizontal = Buffer::new(buf.width, buf.height);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 3];
            for (k, weight) in kernel.iter().enumerate() {
                let dx = k as i64 - radius;
                let px = buf.get(x + dx, y);
                for c in 0..3 {
                    acc[c] += px[c] * weight;
                }
            }
            horizontal.set(x as u32, y as u32, acc);
        }
    }

    let mut out = Buffer::new(buf.width, buf.height);
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 3];
            for (k, weight) in kernel.iter().enumerate() {
                let dy = k as i64 - radius;
                let px = horizontal.get(x, y + dy);
                for c in 0..3 {
                    acc[c] += px[c] * weight;
                }
            }
            out.set(x as u32, y as u32, acc);
        }
    }
    out
}

fn reduce(buf: &Buffer) -> Buffer {
    let blurred = separable_blur(buf, 1.0);
    let nw = (buf.width / 2).max(1);
    let nh = (buf.height / 2).max(1);
    let mut out = Buffer::new(nw, nh);
    for y in 0..nh {
        for x in 0..nw {
            out.set(x, y, blurred.get((x * 2) as i64, (y * 2) as i64));
        }
    }
    out
}

/// Expand `buf` up to `(target_w, target_h)`. Target dimensions are always
/// passed explicitly by the caller (the known finer pyramid level's exact
/// size) rather than recomputed by doubling, so odd dimensions never cause
/// a mismatch between encode and decode.
fn expand(buf: &Buffer, target_w: u32, target_h: u32) -> Buffer {
    let mut zero_inserted = Buffer::new(target_w, target_h);
    for y in 0..buf.height {
        for x in 0..buf.width {
            let tx = x * 2;
            let ty = y * 2;
            if tx < target_w && ty < target_h {
                zero_inserted.set(tx, ty, buf.get(x as i64, y as i64));
            }
        }
    }
    // pass_multiplier=2.0 per pass (4x total) compensates the energy lost
    // by inserting zero rows/columns before blurring.
    separable_blur(&zero_inserted, 2.0)
}

pub fn gaussian_pyramid(buf: &Buffer, levels: usize) -> Vec<Buffer> {
    let mut pyramid = vec![buf.clone()];
    for _ in 1..levels {
        let next = reduce(pyramid.last().unwrap());
        pyramid.push(next);
    }
    pyramid
}

pub fn laplacian_pyramid(buf: &Buffer, levels: usize) -> Vec<Buffer> {
    let gauss = gaussian_pyramid(buf, levels);
    let mut lap = Vec::with_capacity(levels);
    for i in 0..levels.saturating_sub(1) {
        let expanded = expand(&gauss[i + 1], gauss[i].width, gauss[i].height);
        lap.push(subtract(&gauss[i], &expanded));
    }
    lap.push(gauss[levels - 1].clone()); // coarsest level: residual, not a difference
    lap
}

/// Fuse two color branches, each weighted by its own (already
/// paper-normalized) weight map, via per-level Laplacian/Gaussian pyramid
/// blending and coarsest-to-finest reconstruction.
pub fn fuse(branch1: &Buffer, weight1: &Buffer, branch2: &Buffer, weight2: &Buffer, levels: usize) -> Buffer {
    let lap1 = laplacian_pyramid(branch1, levels);
    let lap2 = laplacian_pyramid(branch2, levels);
    let gw1 = gaussian_pyramid(weight1, levels);
    let gw2 = gaussian_pyramid(weight2, levels);

    let mut fused_levels = Vec::with_capacity(levels);
    for l in 0..levels {
        let mut level = Buffer::new(lap1[l].width, lap1[l].height);
        for i in 0..level.data.len() {
            for c in 0..3 {
                level.data[i][c] = gw1[l].data[i][c] * lap1[l].data[i][c] + gw2[l].data[i][c] * lap2[l].data[i][c];
            }
        }
        fused_levels.push(level);
    }

    let mut result = fused_levels[levels - 1].clone();
    for l in (0..levels - 1).rev() {
        let expanded = expand(&result, fused_levels[l].width, fused_levels[l].height);
        result = add(&expanded, &fused_levels[l]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laplacian_pyramid_reconstructs_original() {
        // A gradient, not a flat field, so the Laplacian levels aren't
        // trivially zero — exercises reduce/expand round-tripping for real.
        let (w, h) = (37, 29); // deliberately not a power of two
        let mut buf = Buffer::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let v = (x as f32 / w as f32 + y as f32 / h as f32) / 2.0;
                buf.set(x, y, [v, v, v]);
            }
        }

        let levels = pyramid_depth(w, h);
        let lap = laplacian_pyramid(&buf, levels);

        let mut result = lap[levels - 1].clone();
        for l in (0..levels - 1).rev() {
            let expanded = expand(&result, lap[l].width, lap[l].height);
            result = add(&expanded, &lap[l]);
        }

        for i in 0..buf.data.len() {
            for c in 0..3 {
                let diff = (buf.data[i][c] - result.data[i][c]).abs();
                assert!(diff < 0.01, "pixel {i} channel {c}: diff {diff}");
            }
        }
    }

    #[test]
    fn pyramid_depth_is_size_adaptive() {
        assert!(pyramid_depth(1600, 1200) > pyramid_depth(64, 64));
        assert_eq!(pyramid_depth(4, 4), 1);
    }
}

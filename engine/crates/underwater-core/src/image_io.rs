//! Thin wrappers around image decode/encode. RAW and video decode are
//! separate, larger pieces of Phase 0 scope — this module only handles the
//! standard formats needed to exercise the adjustment pipeline end to end.

use image::{ImageError, RgbImage};
use std::path::Path;

pub fn load(path: impl AsRef<Path>) -> Result<RgbImage, ImageError> {
    Ok(image::open(path)?.to_rgb8())
}

pub fn save(img: &RgbImage, path: impl AsRef<Path>) -> Result<(), ImageError> {
    img.save(path)
}

//! Float RGB working buffer for the fusion pipeline. Values stay in the
//! image's native normalized [0,1] range (no linear-light conversion) —
//! the Ancuti et al. formulas are defined directly on normalized pixel
//! values, unlike `adjustments.rs`'s physically-based linear-light math.

use image::{Rgb, RgbImage};

#[derive(Clone)]
pub(crate) struct Buffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<[f32; 3]>,
}

impl Buffer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![[0.0; 3]; (width * height) as usize],
        }
    }

    pub fn from_rgb_image(img: &RgbImage) -> Self {
        let (width, height) = img.dimensions();
        let data = img
            .pixels()
            .map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0])
            .collect();
        Self { width, height, data }
    }

    pub fn to_rgb_image(&self) -> RgbImage {
        let mut out = RgbImage::new(self.width, self.height);
        for (i, px) in self.data.iter().enumerate() {
            let x = (i as u32) % self.width;
            let y = (i as u32) / self.width;
            out.put_pixel(
                x,
                y,
                Rgb([
                    (px[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (px[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (px[2].clamp(0.0, 1.0) * 255.0).round() as u8,
                ]),
            );
        }
        out
    }

    /// Edge-clamped sample, so pyramid convolutions don't wrap or go dark
    /// at image borders.
    pub fn get(&self, x: i64, y: i64) -> [f32; 3] {
        let cx = x.clamp(0, self.width as i64 - 1) as u32;
        let cy = y.clamp(0, self.height as i64 - 1) as u32;
        self.data[(cy * self.width + cx) as usize]
    }

    pub fn set(&mut self, x: u32, y: u32, v: [f32; 3]) {
        self.data[(y * self.width + x) as usize] = v;
    }

    pub fn luminance_at(&self, i: usize) -> f32 {
        let c = self.data[i];
        0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
    }
}

pub(crate) fn add(a: &Buffer, b: &Buffer) -> Buffer {
    let mut out = Buffer::new(a.width, a.height);
    for i in 0..a.data.len() {
        for c in 0..3 {
            out.data[i][c] = a.data[i][c] + b.data[i][c];
        }
    }
    out
}

pub(crate) fn subtract(a: &Buffer, b: &Buffer) -> Buffer {
    let mut out = Buffer::new(a.width, a.height);
    for i in 0..a.data.len() {
        for c in 0..3 {
            out.data[i][c] = a.data[i][c] - b.data[i][c];
        }
    }
    out
}

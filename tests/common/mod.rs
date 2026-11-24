//! Helpers shared by the golden-output tests.
#![allow(dead_code)]

use image::{Rgba, RgbaImage};

pub type Golden = (&'static str, u64);

pub fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

pub fn hash_text(text: &str) -> u64 {
    fnv(text.as_bytes())
}

pub fn hash_image(img: &RgbaImage) -> u64 {
    let mut bytes = img.width().to_le_bytes().to_vec();
    bytes.extend(img.height().to_le_bytes());
    bytes.extend(img.as_raw());
    fnv(&bytes)
}

pub fn hash_points(paths: &[Vec<(f32, f32)>]) -> u64 {
    let mut bytes = Vec::new();
    for path in paths {
        bytes.extend((path.len() as u64).to_le_bytes());
        for (x, y) in path {
            bytes.extend(x.to_bits().to_le_bytes());
            bytes.extend(y.to_bits().to_le_bytes());
        }
    }
    fnv(&bytes)
}

pub fn pixel_noise(x: u32, y: u32, seed: u32) -> Rgba<u8> {
    let mut v = x
        .wrapping_mul(0x9e37_79b1)
        .wrapping_add(y.wrapping_mul(0x85eb_ca6b))
        .wrapping_add(seed.wrapping_mul(0xc2b2_ae35));
    v ^= v >> 15;
    v = v.wrapping_mul(0x2c1b_3c6d);
    v ^= v >> 12;
    let [a, b, c, d] = v.to_le_bytes();
    Rgba([a, b, c, d])
}

pub fn noise(w: u32, h: u32, seed: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| pixel_noise(x, y, seed))
}

/// Compares observed values with the recorded table and prints a ready-to-paste table on mismatch.
pub fn assert_golden(observed: Vec<Golden>, golden: &[Golden]) {
    let table: String = observed
        .iter()
        .map(|(name, value)| format!("    (\"{name}\", {value:#018x}),\n"))
        .collect();
    assert_eq!(observed, golden, "observed table:\n{table}");
}

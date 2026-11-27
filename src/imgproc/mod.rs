//! OpenCV-style image processing kernels on 8-bit images, written so the compiler can vectorise them.
//!
//! The functions work on whole rows of bytes (no per-pixel `Pixel` accessors, no bounds-checked
//! neighbourhood lookups) and return the same results as the equivalent `imageproc` functions, which the
//! tests check. See the `PERF_QUEUE.md` of the `rust-cv-guide` repository for the measured speed-ups.

#[macro_use]
#[path = "generated/median_networks.rs"]
mod median_networks;

pub mod bilateral;
pub mod blur;
pub mod canny;
pub mod filter2d;
pub mod gradient;
pub mod median;
pub mod morph;
pub mod pyramid;
pub mod template;

mod rows;

/// Deterministic pictures for the tests of every kernel.
#[cfg(test)]
pub(crate) mod testing {
    use image::{GrayImage, Luma};

    /// Pseudo-random grey picture, so every window position sees different values.
    pub(crate) fn noise(w: u32, h: u32, seed: u32) -> GrayImage {
        let mut s = seed;
        GrayImage::from_fn(w, h, |_, _| {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            Luma([(s >> 24) as u8])
        })
    }
}

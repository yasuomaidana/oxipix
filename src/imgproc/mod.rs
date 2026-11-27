//! OpenCV-style image processing kernels on 8-bit images, written so the compiler can vectorise them.
//!
//! The functions work on whole rows of bytes (no per-pixel `Pixel` accessors, no bounds-checked
//! neighbourhood lookups) and return the same results as the equivalent `imageproc` functions, which the
//! tests check. See the `PERF_QUEUE.md` of the `rust-cv-guide` repository for the measured speed-ups.
//!
//! Modules are grouped by topic (`filter`, `detect`, `geometry`, `color`) and re-exported here, so every
//! kernel is reachable as `oxipix::imgproc::<name>`.

#[macro_use]
#[path = "generated/median_networks.rs"]
mod median_networks;

mod rows;

#[cfg(test)]
mod parallel_tests;

pub mod color;
pub mod detect;
pub mod filter;
pub mod geometry;

pub use color::{arith, equalize, quality, threshold};
pub use detect::{canny, features, hough, template};
pub use filter::{bilateral, blur, filter2d, gradient, median, morph, pyramid};
pub use geometry::{distance, warp};

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

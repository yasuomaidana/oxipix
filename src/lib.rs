//! # `oxipix`: Sovereign Zero-Copy Accelerated Pixmaps & Multi-Tier Codecs
//!
//! `oxipix` is an independent, pure-Rust image processing workspace and accelerated codec suite
//! engineered to completely replace monolithic legacy crates:
//! - **`image`**: 100% drop-in API replacement via `oxipix::compat::image`
//! - **`imageproc`**: 2D vector/raster drawing and CV transforms via `oxipix::draw`, `oxipix::cv`, and `oxipix::compat::imageproc`
//! - **`fast_image_resize`**: Hardware SIMD convolution resampling (AVX2/NEON) via `oxipix::ops` and `oxipix::compat::fast_image_resize`
//! - **`rxing`**: 1D and 2D barcode and QR matrix codec via `oxipix::barcode` and `oxipix::compat::rxing`
//!
//! Features:
//! - **64-byte Cache-Line & SIMD Alignment**: Zero unaligned memory penalties (`AlignedBuffer`).
//! - **Zero-Allocation Strided Sub-Views**: $O(1)$ zero-copy ROI cropping (`SubPixView`).
//! - **Clean-Room Baseline JPEG Engine**: Fully compliant with ISO/IEC 10918-1 without patent encumbrance.
//! - **Universal Codec Federation**: PNG, WebP, GIF, TIFF, BMP, QOI, AVIF, PNM, Farbfeld, HDR, DDS, ICO.
//! - **Multi-Tier Hardware Acceleration**: Pure-Rust scalar (Tier 1), CPU SIMD (Tier 2), and GPU compute shaders via `wgpu` (Tier 3).
#![deny(missing_docs)]

pub use oxipix_core::*;

#[cfg(feature = "simd")]
pub use oxipix_simd as simd;

#[cfg(feature = "codecs")]
pub use oxipix_codecs as codecs;

#[cfg(feature = "ops")]
pub use oxipix_ops as ops;

#[cfg(feature = "ops")]
pub use oxipix_ops::draw;

#[cfg(feature = "ops")]
pub use oxipix_ops::cv;

#[cfg(feature = "gpu")]
pub use oxipix_gpu as gpu;

#[cfg(feature = "barcode")]
pub use oxipix_barcode as barcode;

/// Compatibility facades for 100% drop-in replacement of legacy image crates.
pub mod compat {
    #[cfg(feature = "compat-image")]
    /// 100% Drop-in replacement for the upstream `image` crate (v0.25).
    pub mod image {
        pub use oxipix_compat_image::*;
    }

    #[cfg(feature = "ops")]
    /// 100% Drop-in replacement for the `imageproc` crate (v0.27).
    pub mod imageproc {
        pub use oxipix_ops::imageproc::*;
    }

    #[cfg(feature = "ops")]
    /// 100% Drop-in replacement for the `fast_image_resize` crate (v6.1).
    pub mod fast_image_resize {
        pub use oxipix_ops::fir::*;
    }

    #[cfg(feature = "barcode")]
    /// 100% Drop-in replacement for the `rxing` crate (v0.9).
    pub mod rxing {
        pub use oxipix_barcode::*;
    }
}

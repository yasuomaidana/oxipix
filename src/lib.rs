//! # `oxipix`: Sovereign Zero-Copy Accelerated Pixmaps & Multi-Tier Codecs
//!
//! `oxipix` is an independent, 100% pure-Rust image processing workspace and codec suite
//! engineered to completely replace the monolithic `image` crate while providing:
//!
//! - **64-byte Cache-Line & SIMD Alignment**: Zero unaligned memory penalties (`AlignedBuffer`).
//! - **Zero-Allocation Strided Sub-Views**: $O(1)$ zero-copy ROI cropping (`SubPixView`).
//! - **Clean-Room Baseline JPEG Engine**: Fully compliant with ISO/IEC 10918-1 without patent encumbrance.
//! - **Universal Codec Federation**: PNG, WebP, GIF, TIFF, BMP, QOI, AVIF, PNM, Farbfeld, HDR, DDS, ICO.
//! - **Multi-Tier Hardware Acceleration**: Pure-Rust scalar (Tier 1), CPU SIMD via `oximedia-simd` (Tier 2), and GPU compute shaders via `wgpu` (Tier 3).
//! - **100% Drop-in `image` Replacement**: Replicates public APIs and passes all 569 upstream tests via `oxipix::compat::image`.
#![deny(missing_docs)]

pub use oxipix_core::*;

#[cfg(feature = "simd")]
pub use oxipix_simd as simd;

#[cfg(feature = "codecs")]
pub use oxipix_codecs as codecs;

#[cfg(feature = "ops")]
pub use oxipix_ops as ops;

#[cfg(feature = "gpu")]
pub use oxipix_gpu as gpu;

#[cfg(feature = "compat-image")]
pub mod compat {
    //! Compatibility layers for seamless migration from legacy crates.
    
    /// Drop-in compatibility layer replicating the upstream `image` crate public API.
    pub mod image {
        pub use oxipix_compat_image::*;
    }
}

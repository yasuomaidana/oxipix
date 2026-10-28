//! # `oxipix`
//!
//! Document-capture building blocks that sit on top of the [`image`] crate:
//!
//! - [`gpu`] (feature `gpu`): cached wgpu compute pipelines for 3x3 convolution and
//!   perspective warping, exposed as an extension trait on [`image::ImageBuffer`]. Every
//!   method falls back to a CPU reference when no adapter is available.
//! - [`mosaic`] (feature `mosaic`): 2D registration, strip overlap detection, perspective
//!   unwarping and feathered stitching.
//! - [`vectorize`] (feature `vectorize`): Otsu thresholding, contour tracing, RDP
//!   simplification and SVG path synthesis.
#![warn(missing_docs)]

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "mosaic")]
pub mod mosaic;

#[cfg(feature = "vectorize")]
pub mod vectorize;

#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

#[cfg(any(feature = "mosaic", feature = "vectorize"))]
mod luma;

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "mosaic")]
pub mod mosaic;

#[cfg(feature = "vectorize")]
pub mod vectorize;

#[cfg(feature = "imgproc")]
pub mod imgproc;

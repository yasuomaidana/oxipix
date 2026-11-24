//! Optional wgpu acceleration for document-sized RGBA images.
//!
//! [`GpuImageExt`] adds GPU-backed operations to [`image::ImageBuffer`]. Each one runs on
//! the shared [`GpuContext`] when an adapter exists and otherwise on the CPU reference in
//! [`cpu`], so callers never need to branch.

mod context;
pub mod cpu;
mod ext;
pub mod shaders;

pub use context::{GpuContext, GpuError};
pub use ext::GpuImageExt;

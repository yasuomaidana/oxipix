//! Blurs an image on the GPU, falling back to the CPU when no adapter exists.
//!
//! Run with `cargo run --example gpu_blur --features gpu`.

use image::{Rgba, RgbaImage};
use oxipix::gpu::{GpuContext, GpuImageExt};

fn main() {
    match GpuContext::global() {
        Some(gpu) => println!("running on the GPU: {}", gpu.adapter_name()),
        None => println!("no GPU adapter found, using the CPU reference"),
    }

    let page = RgbaImage::from_fn(1600, 1200, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255])
    });

    // Same call either way: the extension trait picks the GPU or the CPU reference.
    let blurred = page.gpu_blur3x3();
    println!("blurred a {}x{} image", blurred.width(), blurred.height());
}

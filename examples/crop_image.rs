//! Example: Cropping Images with `oxipix`
//!
//! This example demonstrates two distinct cropping techniques:
//! 1. Sovereign zero-allocation O(1) strided cropping via `oxipix::FrameBuffer::sub_view`
//! 2. Drop-in buffer cropping via `oxipix::compat::image::RgbaImage`
//!
//! It also demonstrates loading an image from disk and exporting crops to multiple formats (BMP, PPM, JPEG).

use oxipix::compat::image::{Rgba, RgbaImage};
use oxipix::{FrameBuffer, PixelHeight, PixelWidth, Rgba8, state};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Oxipix Cropping Example ===");

    // 1. Prepare input directory
    let output_dir = Path::new("target/examples_output");
    std::fs::create_dir_all(output_dir)?;

    let input_path = output_dir.join("sample_source.bmp");
    let crop_bmp_path = output_dir.join("cropped_output.bmp");
    let crop_ppm_path = output_dir.join("cropped_output.ppm");
    let crop_jpg_path = output_dir.join("cropped_output.jpg");

    // 2. Synthesize a 400x300 gradient test image and save to disk
    println!("[1] Creating source image (400x300) at: {}", input_path.display());
    let mut source_img = RgbaImage::new(400, 300);
    for (x, y, pixel) in source_img.enumerate_pixels_mut() {
        let r = ((x * 255) / 400) as u8;
        let g = ((y * 255) / 300) as u8;
        let b = 180u8;
        *pixel = Rgba([r, g, b, 255]);
    }
    source_img.save(&input_path)?;

    // 3. Consume image from path
    println!("[2] Loading image from path: {}", input_path.display());
    let loaded = oxipix::compat::image::open(&input_path)?;
    println!("    Loaded dimensions: {}x{}", loaded.width(), loaded.height());

    // 4. Technique A: Drop-In Cropping with RgbaImage
    let crop_x = 50;
    let crop_y = 50;
    let crop_w = 200;
    let crop_h = 150;
    println!("[3] Cropping ROI (x={}, y={}, w={}, h={}) via RgbaImage...", crop_x, crop_y, crop_w, crop_h);

    let rgba = loaded.to_rgba8();
    let mut cropped_buffer = RgbaImage::new(crop_w, crop_h);
    for cy in 0..crop_h {
        for cx in 0..crop_w {
            let px = rgba.get_pixel(crop_x + cx, crop_y + cy);
            cropped_buffer.put_pixel(cx, cy, *px);
        }
    }

    // Export cropped image as Windows 24-bit BMP
    cropped_buffer.save(&crop_bmp_path)?;
    println!("    [✓] Saved BMP crop: {}", crop_bmp_path.display());

    // Export cropped image as PPM P6 format
    cropped_buffer.save_with_format(&crop_ppm_path, oxipix::codecs::ImageFormat::Pnm)?;
    println!("    [✓] Saved PPM crop: {}", crop_ppm_path.display());

    // Export cropped image as Baseline JPEG
    cropped_buffer.save_with_format(&crop_jpg_path, oxipix::codecs::ImageFormat::Jpeg)?;
    println!("    [✓] Saved JPEG crop: {}", crop_jpg_path.display());

    // 5. Technique B: Sovereign Zero-Allocation O(1) SubPixView Cropping
    println!("[4] Sovereign zero-allocation O(1) strided cropping via FrameBuffer...");
    let raw_frame = FrameBuffer::<Rgba8, state::Uninitialized>::allocate(
        PixelWidth(400),
        PixelHeight(300),
    )?;
    let mut frame = raw_frame.fill_bytes(0);
    // Copy pixels into aligned memory
    frame.as_mut_bytes()?.copy_from_slice(rgba.as_raw());

    // SubPixView performs zero allocation, calculating strided pointer offsets:
    let sub_view = frame.sub_view(crop_x, crop_y, crop_w, crop_h)?;
    println!("    SubPixView width: {}, height: {}", sub_view.width().get(), sub_view.height().get());
    println!("    Row 0 bytes: {}", sub_view.row(0)?.len());

    println!("=== Cropping example completed successfully! ===");
    Ok(())
}

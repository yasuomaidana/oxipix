//! Example: Removing an RGB Color (Chroma Key & Background Substitution) with `oxipix`
//!
//! This example demonstrates:
//! 1. Loading an image from a filesystem path
//! 2. Identifying pixels matching a specific RGB color (within an adjustable color distance tolerance)
//! 3. Removing the color by setting alpha to 0 (transparency mask / green screen keying)
//! 4. Substituting the removed color with a new background color
//! 5. Exporting and converting the result into multiple image formats (BMP, PPM, JPEG)

use oxipix::compat::image::{Rgba, RgbaImage};
use std::path::Path;

/// Calculates Euclidean RGB color distance: sqrt((r1-r2)^2 + (g1-g2)^2 + (b1-b2)^2)
fn color_distance(c1: [u8; 3], c2: [u8; 3]) -> f32 {
    let dr = (c1[0] as f32) - (c2[0] as f32);
    let dg = (c1[1] as f32) - (c2[1] as f32);
    let db = (c1[2] as f32) - (c2[2] as f32);
    (dr * dr + dg * dg + db * db).sqrt()
}

/// Removes a target RGB color by setting matching pixels to transparent [0, 0, 0, 0].
fn remove_rgb_color_to_transparent(
    image: &mut RgbaImage,
    target_rgb: [u8; 3],
    tolerance: f32,
) -> usize {
    let mut modified_count = 0;
    for (_x, _y, pixel) in image.enumerate_pixels_mut() {
        let current_rgb = [pixel[0], pixel[1], pixel[2]];
        if color_distance(current_rgb, target_rgb) <= tolerance {
            *pixel = Rgba([0, 0, 0, 0]); // Transparent
            modified_count += 1;
        }
    }
    modified_count
}

/// Replaces a target RGB color with a replacement RGB color.
fn replace_rgb_color(
    image: &mut RgbaImage,
    target_rgb: [u8; 3],
    replacement_rgb: [u8; 3],
    tolerance: f32,
) -> usize {
    let mut modified_count = 0;
    for (_x, _y, pixel) in image.enumerate_pixels_mut() {
        let current_rgb = [pixel[0], pixel[1], pixel[2]];
        if color_distance(current_rgb, target_rgb) <= tolerance {
            *pixel = Rgba([replacement_rgb[0], replacement_rgb[1], replacement_rgb[2], 255]);
            modified_count += 1;
        }
    }
    modified_count
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Oxipix RGB Color Removal & Chroma-Key Example ===");

    let output_dir = Path::new("target/examples_output");
    std::fs::create_dir_all(output_dir)?;

    let source_path = output_dir.join("chromakey_source.bmp");
    let transparent_out = output_dir.join("chromakey_transparent.bmp");
    let replaced_out_bmp = output_dir.join("chromakey_replaced.bmp");
    let replaced_out_ppm = output_dir.join("chromakey_replaced.ppm");
    let replaced_out_jpg = output_dir.join("chromakey_replaced.jpg");

    // 1. Synthesize a test image: a circular object over a bright green background [0, 255, 0]
    println!("[1] Synthesizing green screen test image (300x300)...");
    let mut test_image = RgbaImage::new(300, 300);
    let center_x = 150.0f32;
    let center_y = 150.0f32;
    let radius = 80.0f32;

    for (x, y, pixel) in test_image.enumerate_pixels_mut() {
        let dx = (x as f32) - center_x;
        let dy = (y as f32) - center_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= radius {
            // Foreground subject: Royal Blue sphere [30, 90, 220]
            *pixel = Rgba([30, 90, 220, 255]);
        } else {
            // Background: Chroma Green [0, 255, 0] with slight natural variance
            let noise = ((x + y) % 5) as u8;
            *pixel = Rgba([0, 255 - noise, 0, 255]);
        }
    }
    test_image.save(&source_path)?;
    println!("    [✓] Saved source image to: {}", source_path.display());

    // 2. Consume image from path
    println!("[2] Consuming image from path: {}", source_path.display());
    let dynamic_img = oxipix::compat::image::open(&source_path)?;
    let mut working_img = dynamic_img.to_rgba8();

    // 3. Remove Chroma Green [0, 255, 0] with tolerance 30.0 -> make transparent
    println!("[3] Removing Chroma Green [0, 255, 0] to transparency (tolerance 30.0)...");
    let target_green = [0, 255, 0];
    let removed = remove_rgb_color_to_transparent(&mut working_img, target_green, 30.0);
    println!("    [✓] Keyed out {} green pixels to transparent alpha", removed);
    working_img.save(&transparent_out)?;
    println!("    [✓] Saved transparent result: {}", transparent_out.display());

    // 4. Substitute removed background with clean Warm Gray [230, 230, 235]
    println!("[4] Replacing green screen with studio Warm Gray [230, 230, 235]...");
    let mut substituted_img = dynamic_img.to_rgba8();
    let warm_gray = [230, 230, 235];
    let replaced = replace_rgb_color(&mut substituted_img, target_green, warm_gray, 30.0);
    println!("    [✓] Replaced {} background pixels with studio color", replaced);

    // 5. Produce and convert image formats
    substituted_img.save(&replaced_out_bmp)?;
    println!("    [✓] Produced BMP: {}", replaced_out_bmp.display());

    substituted_img.save_with_format(&replaced_out_ppm, oxipix::codecs::ImageFormat::Pnm)?;
    println!("    [✓] Converted to PPM: {}", replaced_out_ppm.display());

    substituted_img.save_with_format(&replaced_out_jpg, oxipix::codecs::ImageFormat::Jpeg)?;
    println!("    [✓] Converted to JPEG: {}", replaced_out_jpg.display());

    println!("=== Color removal & chroma-key example completed! ===");
    Ok(())
}

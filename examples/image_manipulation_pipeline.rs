//! Example: Comprehensive Image Manipulation Pipeline with `oxipix`
//!
//! Demonstrates:
//! 1. Consuming an image from path
//! 2. 2D Drawing primitives (bounding boxes, crosshairs)
//! 3. Edge detection and feature extraction (Sobel gradients, Canny)
//! 4. Morphological filters (dilation and erosion)
//! 5. Otsu global thresholding binarization
//! 6. QR code generation and watermark embedding onto the image
//! 7. Resampling / Scaling down to thumbnail
//! 8. Exporting produced images to multiple formats (BMP, PPM, JPEG)

use oxipix::barcode::qrcode::QRCodeWriter;
use oxipix::barcode::{BarcodeFormat, Writer};
use oxipix::compat::image::{GrayImage, Luma, Rgba, RgbaImage};
use oxipix::ops::cv::{canny, dilate, sobel_horizontal};
use oxipix::ops::draw::{draw_hollow_rect_mut, draw_line_segment_mut, Rect};
use oxipix::ops::fir::FilterType;
use oxipix::ops::resample::resize_simd_rgba;
use oxipix::ops::otsu_threshold;
use oxipix::{PixelHeight, PixelWidth};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Oxipix Advanced Image Manipulation Pipeline ===");

    let output_dir = Path::new("target/examples_output");
    std::fs::create_dir_all(output_dir)?;

    let source_path = output_dir.join("manipulation_input.bmp");
    let annotated_bmp_path = output_dir.join("manipulation_annotated.bmp");
    let edges_bmp_path = output_dir.join("manipulation_edges.bmp");
    let binarized_bmp_path = output_dir.join("manipulation_binarized.bmp");
    let thumbnail_jpg_path = output_dir.join("manipulation_thumb.jpg");
    let thumbnail_ppm_path = output_dir.join("manipulation_thumb.ppm");

    // 1. Synthesize input image and save to disk
    println!("[1] Preparing input image (400x400)...");
    let mut canvas = RgbaImage::new(400, 400);
    for (x, y, pixel) in canvas.enumerate_pixels_mut() {
        let pattern = (((x / 20) + (y / 20)) % 2) as u8 * 50;
        *pixel = Rgba([180 + pattern, 190 + pattern, 210, 255]);
    }
    canvas.save(&source_path)?;

    // 2. Consume image from path
    println!("[2] Loading image from path: {}", source_path.display());
    let loaded = oxipix::compat::image::open(&source_path)?;
    let mut working_image = loaded.to_rgba8();

    // 3. Stage A: 2D Vector Drawing (Bounding Box & Crosshair)
    println!("[3] Stage A: Drawing bounding boxes and target crosshairs...");
    let detection_box = Rect::at(60, 60).of_size(180, 120);
    draw_hollow_rect_mut(&mut working_image, detection_box, Rgba([255, 30, 30, 255]));

    draw_line_segment_mut(
        &mut working_image,
        (150.0, 40.0),
        (150.0, 200.0),
        Rgba([0, 200, 50, 255]),
    );

    // 4. Stage B: Generate QR Code and Stamp as Watermark
    println!("[4] Stage B: Generating QR code and embedding watermark...");
    let writer = QRCodeWriter;
    let qr_matrix = writer.encode("https://oxipix.rs/verified", &BarcodeFormat::QR_CODE, 80, 80)?;
    let qr_w = qr_matrix.getWidth() as u32;
    let qr_h = qr_matrix.getHeight() as u32;
    let stamp_x = 300;
    let stamp_y = 300;

    for y in 0..qr_h {
        for x in 0..qr_w {
            if qr_matrix.get(x as i32, y as i32) {
                working_image.put_pixel(stamp_x + x, stamp_y + y, Rgba([0, 0, 0, 255]));
            } else {
                working_image.put_pixel(stamp_x + x, stamp_y + y, Rgba([255, 255, 255, 255]));
            }
        }
    }
    working_image.save(&annotated_bmp_path)?;
    println!("    [✓] Saved annotated image: {}", annotated_bmp_path.display());

    // 5. Stage C: Computer Vision (Canny Edges & Sobel Filtering)
    println!("[5] Stage C: Running Canny edge detection & Sobel gradient analysis...");
    let gray_img = loaded.to_luma8();
    let edge_map = canny(&gray_img, 30.0, 100.0);
    let _sobel = sobel_horizontal(&gray_img);

    // Morphological cleanup: Dilate detected edges
    let dilated_edges = dilate(&edge_map, 1);
    let mut edge_output = RgbaImage::new(400, 400);
    for (x, y, pixel) in edge_output.enumerate_pixels_mut() {
        let v = dilated_edges.get_pixel(x, y)[0];
        *pixel = Rgba([v, v, v, 255]);
    }
    edge_output.save(&edges_bmp_path)?;
    println!("    [✓] Saved edge detection map: {}", edges_bmp_path.display());

    // 6. Stage D: Otsu Global Binarization
    println!("[6] Stage D: Applying Otsu automatic thresholding...");
    let threshold = otsu_threshold(gray_img.as_raw());
    println!("    Calculated optimal Otsu threshold: {}", threshold);
    let mut binarized = GrayImage::new(400, 400);
    for (x, y, pixel) in binarized.enumerate_pixels_mut() {
        let val = gray_img.get_pixel(x, y)[0];
        *pixel = if val >= threshold { Luma([255]) } else { Luma([0]) };
    }
    binarized.save(&binarized_bmp_path)?;
    println!("    [✓] Saved binarized mask: {}", binarized_bmp_path.display());

    // 7. Stage E: Multi-Threaded SIMD Resampling down to 100x100 Thumbnail
    println!("[7] Stage E: Resampling image to 100x100 using Lanczos3 convolution...");
    let thumb_raw = resize_simd_rgba(
        working_image.as_raw(),
        PixelWidth(working_image.width()),
        PixelHeight(working_image.height()),
        PixelWidth(100),
        PixelHeight(100),
        FilterType::Lanczos3,
    )?;
    let thumbnail = RgbaImage::from_raw(100, 100, thumb_raw).expect("valid thumbnail buffer");

    // Produce output in both JPEG and PPM formats
    thumbnail.save_with_format(&thumbnail_jpg_path, oxipix::codecs::ImageFormat::Jpeg)?;
    println!("    [✓] Produced thumbnail (JPEG): {}", thumbnail_jpg_path.display());

    thumbnail.save_with_format(&thumbnail_ppm_path, oxipix::codecs::ImageFormat::Pnm)?;
    println!("    [✓] Produced thumbnail (PPM): {}", thumbnail_ppm_path.display());

    println!("=== Advanced manipulation pipeline completed successfully! ===");
    Ok(())
}

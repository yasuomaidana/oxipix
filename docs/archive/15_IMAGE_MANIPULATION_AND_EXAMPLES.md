---
id: image-manipulation-examples
title: "Cropping, Color Removal & Image Manipulation Examples"
description: "Practical examples demonstrating image cropping, RGB color removal (chroma keying), link/path streaming, and format conversion."
sidebar_label: "Manipulation Examples"
sidebar_position: 15
---

# Image Manipulation & Practical Examples

This guide provides end-to-end examples demonstrating core image operations in `oxipix`:
1. **Zero-Allocation and Buffer Cropping**
2. **Removing an RGB Color (Chroma Key & Transparency)**
3. **Consuming Images from File Paths and Network Links**
4. **Image Format Conversion (BMP &harr; PPM &harr; JPEG)**
5. **Computer Vision & Feature Annotation Pipeline**

All examples are runnable directly via Cargo:
```bash
cargo run --example crop_image
cargo run --example remove_color_chromakey
cargo run --example consume_link_and_convert
cargo run --example image_manipulation_pipeline
```

---

## 1. Image Cropping

### Technique A: Zero-Allocation $O(1)$ Cropping (`SubPixView`)

When performance is critical, `oxipix::FrameBuffer` allows cropping an arbitrary Region of Interest (ROI) without allocating heap memory or copying pixels:

```rust
use oxipix::{FrameBuffer, PixelHeight, PixelWidth, Rgba8, state};

fn crop_roi(
    frame: &FrameBuffer<Rgba8, state::Initialized>,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<(), oxipix::OxipixError> {
    // Zero-allocation O(1) strided slice
    let sub_view = frame.sub_view(x, y, w, h)?;

    println!("Cropped ROI size: {}x{}", sub_view.width().get(), sub_view.height().get());
    
    // Read individual rows
    for row_idx in 0..sub_view.height().get() as usize {
        let row_bytes: &[u8] = sub_view.row(row_idx)?;
        assert_eq!(row_bytes.len(), (w * 4) as usize);
    }

    Ok(())
}
```

### Technique B: Drop-In Buffer Cropping (`RgbaImage`)

For applications migrating from `image`, you can crop directly into an independent buffer:

```rust
use oxipix::compat::image::{Rgba, RgbaImage};

fn crop_to_buffer(src: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
    let mut dst = RgbaImage::new(w, h);
    for cy in 0..h {
        for cx in 0..w {
            let pixel = src.get_pixel(x + cx, y + cy);
            dst.put_pixel(cx, cy, *pixel);
        }
    }
    dst
}
```

---

## 2. Removing an RGB Color (Chroma Key / Background Removal)

You can remove any target RGB color (such as green screen `[0, 255, 0]`) by calculating Euclidean color distance and setting matching pixels to transparent alpha (`[0, 0, 0, 0]`):

```rust
use oxipix::compat::image::{Rgba, RgbaImage};

fn color_distance(c1: [u8; 3], c2: [u8; 3]) -> f32 {
    let dr = (c1[0] as f32) - (c2[0] as f32);
    let dg = (c1[1] as f32) - (c2[1] as f32);
    let db = (c1[2] as f32) - (c2[2] as f32);
    (dr * dr + dg * dg + db * db).sqrt()
}

/// Removes a target RGB color within a specified tolerance, making it transparent.
pub fn remove_rgb_color_to_transparent(
    image: &mut RgbaImage,
    target_rgb: [u8; 3],
    tolerance: f32,
) -> usize {
    let mut modified = 0;
    for (_x, _y, pixel) in image.enumerate_pixels_mut() {
        let current_rgb = [pixel[0], pixel[1], pixel[2]];
        if color_distance(current_rgb, target_rgb) <= tolerance {
            *pixel = Rgba([0, 0, 0, 0]); // Transparent
            modified += 1;
        }
    }
    modified
}

/// Replaces a target RGB color with a new background color.
pub fn replace_rgb_color(
    image: &mut RgbaImage,
    target_rgb: [u8; 3],
    replacement_rgb: [u8; 3],
    tolerance: f32,
) -> usize {
    let mut modified = 0;
    for (_x, _y, pixel) in image.enumerate_pixels_mut() {
        let current_rgb = [pixel[0], pixel[1], pixel[2]];
        if color_distance(current_rgb, target_rgb) <= tolerance {
            *pixel = Rgba([replacement_rgb[0], replacement_rgb[1], replacement_rgb[2], 255]);
            modified += 1;
        }
    }
    modified
}
```

---

## 3. Consuming Images from Paths or Links

`oxipix` consumes image byte streams from local files or remote network URLs, automatically sniffing the format using magic bytes:

```rust
use oxipix::codecs::ImageFormat;
use oxipix::compat::image::{load_from_memory, open, DynamicImage};

fn process_source(source: &str) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let bytes = if source.starts_with("http://") || source.starts_with("https://") {
        // Stream from HTTP/HTTPS URL
        println!("Fetching from link: {}", source);
        reqwest_or_tcp_fetch(source)?
    } else {
        // Load from local filesystem path
        println!("Loading from path: {}", source);
        std::fs::read(source)?
    };

    // Sniff image format
    let format = ImageFormat::from_bytes(&bytes);
    println!("Detected format: {:?}", format);

    // Decode into DynamicImage
    let img = load_from_memory(&bytes)?;
    Ok(img)
}

fn reqwest_or_tcp_fetch(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // See examples/consume_link_and_convert.rs for raw TCP streaming implementation
    std::fs::read("sample.bmp").map_err(Into::into)
}
```

---

## 4. Converting Image Formats

Convert between formats easily by encoding raw pixel slices into target codecs:

```rust
use oxipix::codecs::{BmpEncoder, JpegEncoder, PpmEncoder};
use oxipix::compat::image::RgbaImage;
use oxipix::{JpegQuality, PixelHeight, PixelWidth};

fn convert_formats(img: &RgbaImage) -> Result<(), Box<dyn std::error::Error>> {
    let (w, h) = (img.width(), img.height());

    // 1. Export as Windows 24-bit BMP
    img.save("output.bmp")?;

    // 2. Export as Portable Pixmap (PPM P6 binary)
    img.save_with_format("output.ppm", oxipix::codecs::ImageFormat::Pnm)?;

    // 3. Export as Clean-Room Baseline JPEG (Quality 90)
    img.save_with_format("output.jpg", oxipix::codecs::ImageFormat::Jpeg)?;

    println!("Successfully exported to BMP, PPM, and JPEG");
    Ok(())
}
```

---

## 5. End-to-End Image Manipulation Pipeline

Combine drawing, computer vision, thresholding, and barcode watermarking into a single workflow:

```rust
use oxipix::barcode::qrcode::QRCodeWriter;
use oxipix::barcode::{BarcodeFormat, Writer};
use oxipix::compat::image::{Rgba, RgbaImage};
use oxipix::ops::cv::{canny, dilate};
use oxipix::ops::draw::{draw_hollow_rect_mut, draw_line_segment_mut, Rect};
use oxipix::ops::fir::FilterType;
use oxipix::ops::otsu_threshold;
use oxipix::ops::resample::resize_simd_rgba;
use oxipix::{PixelHeight, PixelWidth};

fn run_pipeline(mut img: RgbaImage) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 2D Vector Drawing
    draw_hollow_rect_mut(&mut img, Rect::at(50, 50).of_size(100, 100), Rgba([255, 0, 0, 255]));
    draw_line_segment_mut(&mut img, (10.0, 10.0), (90.0, 90.0), Rgba([0, 255, 0, 255]));

    // 2. Embed QR Code Watermark
    let writer = QRCodeWriter;
    let qr = writer.encode("https://oxipix.rs", &BarcodeFormat::QR_CODE, 60, 60)?;
    for y in 0..qr.getHeight() as u32 {
        for x in 0..qr.getWidth() as u32 {
            let color = if qr.get(x as i32, y as i32) { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) };
            img.put_pixel(img.width() - 70 + x, img.height() - 70 + y, color);
        }
    }

    // 3. Resample thumbnail via SIMD Lanczos3
    let thumb_raw = resize_simd_rgba(
        img.as_raw(),
        PixelWidth(img.width()),
        PixelHeight(img.height()),
        PixelWidth(128),
        PixelHeight(128),
        FilterType::Lanczos3,
    )?;
    let thumb = RgbaImage::from_raw(128, 128, thumb_raw).unwrap();
    thumb.save("thumbnail.jpg")?;

    Ok(())
}
```

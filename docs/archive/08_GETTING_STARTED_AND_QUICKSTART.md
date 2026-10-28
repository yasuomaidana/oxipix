---
id: getting-started
title: "Getting Started & Quickstart"
description: "Quickstart guide for integrating oxipix, configuring feature flags, and performing basic image processing operations."
sidebar_label: "Getting Started"
sidebar_position: 8
---

# Getting Started with `oxipix`

`oxipix` is an independent, pure-Rust image processing workspace and accelerated codec suite engineered to replace monolithic legacy crates (`image`, `imageproc`, `fast_image_resize`, `rxing`) with zero-copy, 64-byte cache-aligned memory primitives and multi-tier SIMD/GPU acceleration.

---

## 1. Installation & Cargo Configuration

Add `oxipix` to your `Cargo.toml`:

```toml
[dependencies]
oxipix = "0.1.0"
```

### Feature Flags

`oxipix` uses modular feature flags so you only compile what your system needs:

| Feature Flag   | Default | Description                                                 | Included Crates       |
|:---------------|:-------:|:------------------------------------------------------------|:----------------------|
| `simd`         | **Yes** | Dynamic runtime CPU feature detection (AVX2, AVX-512, NEON) | `oxipix-simd`         |
| `codecs`       | **Yes** | Clean-room JPEG, PNG, WebP, GIF, TIFF, BMP, QOI codecs      | `oxipix-codecs`       |
| `ops`          | **Yes** | Resampling, convolutions, blurs, morphology, and drawing    | `oxipix-ops`          |
| `compat-image` | **Yes** | 100% drop-in API replacement for `image` v0.25              | `oxipix-compat-image` |
| `barcode`      | **Yes** | QR code, Aztec, Data Matrix, PDF417, and 1D barcodes        | `oxipix-barcode`      |
| `gpu`          | **Yes** | WGPU compute shaders for parallel GPU raster operations     | `oxipix-gpu`          |

To use only minimal sovereign core memory primitives without codecs or acceleration:

```toml
[dependencies]
oxipix = { version = "0.1.0", default-features = false }
```

---

## 2. 5-Minute Quickstart

### Basic Image Loading & Manipulation

```rust
use oxipix::compat::image::{DynamicImage, RgbaImage, Rgba};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create a 400x300 RGBA image buffer with zero allocations on heap lines
    let mut img = RgbaImage::new(400, 300);

    // 2. Set individual pixels
    img.put_pixel(10, 10, Rgba([255, 0, 0, 255])); // Red pixel

    // 3. Fast mutable pixel iteration
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if x == y {
            *pixel = Rgba([0, 255, 0, 255]); // Green diagonal line
        }
    }

    println!("Image dimensions: {}x{}", img.width(), img.height());
    Ok(())
}
```

### Sovereign Typestate `FrameBuffer`

When building low-latency computer vision pipelines, use `oxipix_core::FrameBuffer` to guarantee 64-byte alignment and prevent uninitialized reads:

```rust
use oxipix::{
    FrameBuffer, PixelWidth, PixelHeight, Rgba8, state,
};

fn main() -> Result<(), oxipix::OxipixError> {
    // 1. Allocate an uninitialized 64-byte aligned buffer (SIMD-safe)
    let raw_frame: FrameBuffer<Rgba8, state::Uninitialized> =
        FrameBuffer::allocate(PixelWidth(1920), PixelHeight(1080))?;

    // 2. Typestate transition: initialize memory with solid black
    let mut frame: FrameBuffer<Rgba8, state::Initialized> = raw_frame.fill_bytes(0);

    // 3. Direct zero-copy slice access
    let bytes: &[u8] = frame.as_bytes();
    assert_eq!(bytes.len(), 1920 * 1080 * 4);

    // 4. Zero-allocation ROI crop (sub-view)
    let roi = frame.sub_view(100, 100, 640, 480)?;
    let row_0 = roi.row(0)?;
    assert_eq!(row_0.len(), 640 * 4);

    // 5. Freeze into an immutable thread-safe Arc for concurrent consumers
    let shared = frame.freeze();
    println!("Allocated, cropped, and shared 1080p frame: {} bytes", shared.as_bytes().len());

    Ok(())
}
```

---

## 3. High-Speed Image Resizing

`oxipix` provides SIMD-accelerated resampling filters that replace `fast_image_resize`:

```rust
use oxipix::ops::{resize_nearest_rgba, premultiply_alpha_inplace, demultiply_alpha_inplace};
use oxipix::{PixelWidth, PixelHeight};

fn main() -> Result<(), oxipix::OxipixError> {
    let mut src_pixels = vec![255u8; 800 * 600 * 4];
    
    // Premultiply alpha to prevent dark borders during scaling
    premultiply_alpha_inplace(&mut src_pixels)?;

    // Resize using vectorized nearest-neighbor
    let dst = resize_nearest_rgba(
        &src_pixels,
        PixelWidth(800),
        PixelHeight(600),
        PixelWidth(400),
        PixelHeight(300),
    )?;

    assert_eq!(dst.len(), 400 * 300 * 4);
    println!("Resized 800x600 -> 400x300 in zero-copy layout");
    Ok(())
}
```

---

## 4. QR Code & Barcode Generation

Generate ISO/IEC 18004 compliant QR codes directly without external C dependencies:

```rust
use oxipix::barcode::qrcode::QRCodeWriter;
use oxipix::barcode::{BarcodeFormat, Writer};

fn main() -> Result<(), oxipix::OxipixError> {
    let writer = QRCodeWriter;
    let matrix = writer.encode(
        "https://github.com/oxipix",
        &BarcodeFormat::QR_CODE,
        256,
        256,
    )?;

    println!("Generated QR Code: {}x{} modules", matrix.getWidth(), matrix.getHeight());
    assert!(matrix.getWidth() > 0);
    Ok(())
}
```

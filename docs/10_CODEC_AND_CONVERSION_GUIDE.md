---
id: codec-conversion-guide
title: "Codecs & Color Space Conversion"
description: "Guide to supported image formats, magic byte format sniffing, clean-room JPEG encoding, and SIMD color format conversions."
sidebar_label: "Codecs & Conversion"
sidebar_position: 10
---

# Codecs & Color Space Conversion

`oxipix-codecs` and `oxipix-simd` provide clean-room, patent-unencumbered encoders and decoders alongside vectorized color transformations.

---

## 1. Supported Formats & Magic Byte Sniffing

`oxipix` supports 15 industrial image formats without pulling in external C libraries:

| Format | Magic Bytes / Signature | Decoder | Encoder | Clean-Room Spec |
| :--- | :--- | :---: | :---: | :--- |
| **JPEG** | `0xFF, 0xD8` | Yes | Yes | ISO/IEC 10918-1 |
| **PNG / APNG** | `\x89PNG\r\n\x1a\n` | Yes | Yes | W3C Portable Network Graphics |
| **WebP** | `RIFF....WEBP` | Yes | Yes | VP8 / VP8L |
| **GIF** | `GIF87a`, `GIF89a` | Yes | Yes | Compuserve GIF / LZW |
| **TIFF** | `II*\x00`, `MM\x00*` | Yes | Yes | Baseline 6.0 |
| **BMP** | `BM` | Yes | Yes | Windows / OS/2 |
| **QOI** | `qoif` | Yes | Yes | Quite Okay Image |
| **AVIF** | `....ftypavif` | Yes | Yes | Alliance for Open Media |
| **PNM** | `P1` through `P7` | Yes | Yes | Netpbm (PBM/PGM/PPM/PAM) |
| **Farbfeld** | `farbfeld` | Yes | Yes | suckless.org 16-bit RGBA |
| **HDR** | `#?RADIANCE` | Yes | Yes | Radiance RGBE |
| **DDS** | `DDS ` | Yes | Yes | DirectDraw Surface |
| **ICO** | `\x00\x00\x01\x00` | Yes | Yes | Windows Icon |

### Automatic Format Sniffing

```rust
use oxipix::codecs::ImageFormat;

fn main() {
    let png_header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    let detected = ImageFormat::from_bytes(png_header);
    assert_eq!(detected, Some(ImageFormat::Png));

    let jpeg_header = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    let detected = ImageFormat::from_bytes(jpeg_header);
    assert_eq!(detected, Some(ImageFormat::Jpeg));
}
```

---

## 2. Clean-Room ISO/IEC 10918-1 JPEG Encoding

Standard image crates often refuse to implement pure-Rust baseline JPEG due to ideological patent concerns. `oxipix` implements an independent, patent-free ISO/IEC 10918-1 JFIF baseline encoder:

```rust
use oxipix::codecs::JpegEncoder;
use oxipix::{JpegQuality, PixelWidth, PixelHeight};

fn encode_photo(rgb_pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, oxipix::OxipixError> {
    // 1. Configure validated quality factor (1..=100)
    let quality = JpegQuality::new(85)?;
    let encoder = JpegEncoder::new(quality);

    // 2. Encode raw 24-bit RGB into standard JPEG bitstream
    let jpeg_bytes = encoder.encode_rgb24(
        rgb_pixels,
        PixelWidth(width),
        PixelHeight(height),
    )?;

    println!("Encoded {}x{} photo into {} bytes of JPEG", width, height, jpeg_bytes.len());
    Ok(jpeg_bytes)
}
```

---

## 3. High-Speed SIMD Color Conversions

Converting between color spaces (`RGBA <-> RGB`, `RGBA <-> Grayscale`) is often a major CPU bottleneck in computer vision pipelines. `oxipix-simd` provides auto-vectorized and hand-crafted NEON/AVX2 kernels:

```rust
use oxipix::simd;

fn main() {
    // RGBA to RGB (strip alpha channel)
    let rgba = vec![255, 128, 64, 255,  10, 20, 30, 255];
    let mut rgb = vec![0u8; 6];
    simd::rgba_to_rgb(&rgba, &mut rgb);
    assert_eq!(rgb, vec![255, 128, 64,  10, 20, 30]);

    // RGB to RGBA (pad alpha channel with 255)
    let mut rgba_back = vec![0u8; 8];
    simd::rgb_to_rgba(&rgb, &mut rgba_back, 255);

    // Invert colors in-place (SIMD 128/256-bit vector registers)
    let mut colors = vec![0, 50, 100, 255];
    simd::invert_rgba_inplace(&mut colors);
    assert_eq!(colors, vec![255, 205, 155, 255]);
}
```

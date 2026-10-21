---
id: barcode-qr-guide
title: "Barcode & QR Matrix Engine"
sidebar_label: "Barcodes & QR"
sidebar_position: 12
description: "Sovereign 1D and 2D barcode generation and decoding guide replacing rxing with clean-room pure-Rust implementations."
---


# Barcode & QR Matrix Engine

`oxipix-barcode` replaces the heavyweight `rxing` crate with a pure-Rust, memory-safe matrix engine supporting both 1D retail barcodes and 2D high-density matrices.

---

## 1. Supported Symbologies

| Symbology | Standard | Category | Primary Use Case |
| :--- | :--- | :---: | :--- |
| **QR Code** | ISO/IEC 18004 | 2D Matrix | Mobile URLs, authentication, payments |
| **Data Matrix** | ISO/IEC 16022 | 2D Matrix | Industrial manufacturing, medical items |
| **Aztec Code** | ISO/IEC 24778 | 2D Matrix | Transportation, airline tickets |
| **PDF417** | ISO/IEC 15438 | 2D Stacked | Identity cards, government documents |
| **Code 128** | ISO/IEC 15417 | 1D Barcode | Shipping labels, logistics |
| **EAN-13** | ISO/IEC 15420 | 1D Barcode | Global retail products |
| **Code 39** | ISO/IEC 16388 | 1D Barcode | Military and automotive tracking |

---

## 2. Generating a QR Code

```rust
use oxipix::barcode::qrcode::QRCodeWriter;
use oxipix::barcode::{BarcodeFormat, BitMatrix, Writer};
use oxipix::compat::image::{RgbaImage, Rgba};

fn generate_qr_image(payload: &str, size: i32) -> Result<RgbaImage, oxipix::OxipixError> {
    let writer = QRCodeWriter;
    let matrix: BitMatrix = writer.encode(payload, &BarcodeFormat::QR_CODE, size, size)?;

    let w = matrix.getWidth() as u32;
    let h = matrix.getHeight() as u32;
    let mut img = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let is_black = matrix.get(x as i32, y as i32);
            let color = if is_black {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            };
            img.put_pixel(x, y, color);
        }
    }

    Ok(img)
}
```

---

## 3. Reed-Solomon Error Correction

`oxipix-barcode` incorporates a Galois Field $GF(2^8)$ arithmetic engine with standard primitive polynomial $x^8 + x^4 + x^3 + x^2 + 1$ ($0x11D$):
- **Level L**: 7% codewords restored
- **Level M**: 15% codewords restored
- **Level Q**: 25% codewords restored
- **Level H**: 30% codewords restored

This ensures corrupted or torn physical labels can be decoded reliably.

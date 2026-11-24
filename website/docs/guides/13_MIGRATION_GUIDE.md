---
id: migration-guide
title: "Drop-in Migration Guide"
sidebar_label: "Migration Guide"
sidebar_position: 13
description: "Step-by-step migration guide for swapping out image, imageproc, fast_image_resize, and rxing with oxipix."
---


# Upstream Migration Guide

`oxipix` provides 100% drop-in compatibility facades for the four major crates in the Rust image ecosystem. You can migrate your codebase in minutes by simply updating imports and dependencies.

---

## 1. Migrating from `image` (v0.25)

### `Cargo.toml` Change

Replace:
```toml
# Old
[dependencies]
image = "0.25"
```
With:
```toml
# New
[dependencies]
oxipix = { version = "0.1.0", features = ["compat-image"] }
```

### Code Imports

In your source code, change:
```rust
// Old
use image::{DynamicImage, GenericImageView, ImageBuffer, RgbaImage, Rgba};
use image::io::Reader as ImageReader;
```
To:
```rust
// New
use oxipix::compat::image::{DynamicImage, GenericImageView, ImageBuffer, RgbaImage, Rgba};
use oxipix::compat::image::io::Reader as ImageReader;
```
Or alias the facade in your root `lib.rs` / `main.rs`:
```rust
use oxipix::compat::image;
```

---

## 2. Migrating from `imageproc` (v0.27)

### `Cargo.toml` Change

Replace:
```toml
# Old
[dependencies]
imageproc = "0.27"
```
With:
```toml
# New
[dependencies]
oxipix = { version = "0.1.0", features = ["ops"] }
```

### Code Imports

```rust
// Old
use imageproc::drawing::{draw_line_segment_mut, draw_hollow_rect_mut};
use imageproc::edges::canny;

// New
use oxipix::compat::imageproc::drawing::{draw_line_segment_mut, draw_hollow_rect_mut};
use oxipix::compat::imageproc::edges::canny;
```

---

## 3. Migrating from `fast_image_resize` (v6.1)

### `Cargo.toml` Change

Replace:
```toml
# Old
[dependencies]
fast_image_resize = "6.1"
```
With:
```toml
# New
[dependencies]
oxipix = { version = "0.1.0", features = ["ops"] }
```

### Code Imports

```rust
// Old
use fast_image_resize::{Resizer, FilterType, PixelType};

// New
use oxipix::compat::fast_image_resize::{Resizer, FilterType, PixelType};
```

---

## 4. Migrating from `rxing` (v0.9)

### `Cargo.toml` Change

Replace:
```toml
# Old
[dependencies]
rxing = "0.9"
```
With:
```toml
# New
[dependencies]
oxipix = { version = "0.1.0", features = ["barcode"] }
```

### Code Imports

```rust
// Old
use rxing::qrcode::QRCodeWriter;
use rxing::{BarcodeFormat, Writer};

// New
use oxipix::compat::rxing::qrcode::QRCodeWriter;
use oxipix::compat::rxing::{BarcodeFormat, Writer};
```

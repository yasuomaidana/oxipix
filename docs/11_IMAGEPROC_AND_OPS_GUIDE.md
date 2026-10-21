---
id: imageproc-ops-guide
title: "Image Processing, CV & Resampling"
description: "Guide to drawing 2D primitives, edge detection, morphological filters, Otsu binarization, and SIMD resampling in oxipix-ops."
sidebar_label: "Operations & CV"
sidebar_position: 11
---

# Image Processing, Computer Vision & Resampling

`oxipix-ops` serves as the drop-in replacement for `imageproc` and `fast_image_resize`. It combines 2D raster drawing, feature detection, morphological mathematics, and high-order convolution filters.

---

## 1. 2D Drawing Primitives

`oxipix::draw` renders anti-aliased geometric primitives and text directly into image buffers:

```rust
use oxipix::compat::image::{RgbaImage, Rgba};
use oxipix::ops::draw::{draw_line_segment_mut, draw_hollow_rect_mut, Rect, Point};

fn main() {
    let mut canvas = RgbaImage::new(400, 400);

    // 1. Draw a colored bounding box
    let box_rect = Rect::at(50, 50).of_size(200, 100);
    draw_hollow_rect_mut(&mut canvas, box_rect, Rgba([0, 255, 0, 255]));

    // 2. Draw a line segment
    draw_line_segment_mut(
        &mut canvas,
        Point::new(0, 0),
        Point::new(399, 399),
        Rgba([255, 0, 0, 255]),
    );
}
```

---

## 2. Edge Detection & Spatial Filtering

Detect boundaries and gradients with Sobel, Scharr, and Canny edge detectors:

```rust
use oxipix::compat::image::GrayImage;
use oxipix::ops::cv::{canny, sobel_horizontal, sobel_vertical};

fn detect_edges(gray_input: &GrayImage) -> GrayImage {
    // Run Canny edge detector with low/high hysteresis thresholds
    let edges = canny(gray_input, 50.0, 150.0);
    edges
}
```

---

## 3. Morphological Operations (Dilation & Erosion)

Mathematical morphology operations for noise removal, skeletonization, and contour smoothing:

```rust
use oxipix::compat::image::GrayImage;
use oxipix::ops::cv::{dilate, erode};

fn clean_binary_mask(mask: &GrayImage) -> GrayImage {
    // Erode noise speckles
    let eroded = erode(mask, 1);
    // Dilate to reconnect fractured edges
    let dilated = dilate(&eroded, 2);
    dilated
}
```

---

## 4. Otsu Global Binarization

Extract optimal foreground/background thresholds automatically:

```rust
use oxipix::ops::otsu_threshold;

fn binarize_receipt(grayscale_bytes: &[u8]) -> u8 {
    let threshold = otsu_threshold(grayscale_bytes);
    println!("Calculated Otsu threshold: {}", threshold);
    threshold
}
```

---

## 5. High-Order SIMD Resampling (`fir` Compatibility)

`oxipix::ops::fir` directly replaces `fast_image_resize` with multi-threaded, tiled convolution engines:

```rust
use oxipix::ops::fir::{Resizer, FilterType};
use oxipix::compat::image::RgbaImage;

fn downscale_thumbnail(src: &RgbaImage, target_w: u32, target_h: u32) -> RgbaImage {
    let mut dst = RgbaImage::new(target_w, target_h);
    let mut resizer = Resizer::new();

    // Resize using high-quality Lanczos3 filter
    resizer.resize_rgba(src, &mut dst, FilterType::Lanczos3).unwrap();
    dst
}
```

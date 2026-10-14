//! Sovereign Computer Vision algorithms: edge detection, Sobel gradients, and morphology.
#![deny(missing_docs)]

use oxipix_compat_image::{GrayImage, ImageBuffer, Luma};

/// Computes horizontal Sobel gradients: G_x.
#[must_use]
pub fn sobel_horizontal(image: &GrayImage) -> ImageBuffer<Luma<i16>, Vec<i16>> {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let raw = image.as_raw();
    let mut out = vec![0i16; w * h];

    for y in 1..h.saturating_sub(1) {
        let r0 = (y - 1) * w;
        let r1 = y * w;
        let r2 = (y + 1) * w;

        for x in 1..w.saturating_sub(1) {
            let p00 = raw[r0 + x - 1] as i32;
            let p02 = raw[r0 + x + 1] as i32;
            let p10 = raw[r1 + x - 1] as i32;
            let p12 = raw[r1 + x + 1] as i32;
            let p20 = raw[r2 + x - 1] as i32;
            let p22 = raw[r2 + x + 1] as i32;

            let gx = (-p00 + p02) + (-2 * p10 + 2 * p12) + (-p20 + p22);
            out[y * w + x] = gx.clamp(-32768, 32767) as i16;
        }
    }

    ImageBuffer::from_raw_vec(image.width(), image.height(), out).expect("valid size")
}

/// Computes vertical Sobel gradients: G_y.
#[must_use]
pub fn sobel_vertical(image: &GrayImage) -> ImageBuffer<Luma<i16>, Vec<i16>> {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let raw = image.as_raw();
    let mut out = vec![0i16; w * h];

    for y in 1..h.saturating_sub(1) {
        let r0 = (y - 1) * w;
        let r2 = (y + 1) * w;

        for x in 1..w.saturating_sub(1) {
            let p00 = raw[r0 + x - 1] as i32;
            let p01 = raw[r0 + x] as i32;
            let p02 = raw[r0 + x + 1] as i32;
            let p20 = raw[r2 + x - 1] as i32;
            let p21 = raw[r2 + x] as i32;
            let p22 = raw[r2 + x + 1] as i32;

            let gy = (-p00 - 2 * p01 - p02) + (p20 + 2 * p21 + p22);
            out[y * w + x] = gy.clamp(-32768, 32767) as i16;
        }
    }

    ImageBuffer::from_raw_vec(image.width(), image.height(), out).expect("valid size")
}

/// Runs Canny edge detector on a grayscale image with hysteresis thresholding.
#[must_use]
pub fn canny(image: &GrayImage, low_threshold: f32, high_threshold: f32) -> GrayImage {
    let w = image.width() as usize;
    let h = image.height() as usize;
    if w < 3 || h < 3 {
        return GrayImage::new(image.width(), image.height());
    }

    let gx_img = sobel_horizontal(image);
    let gy_img = sobel_vertical(image);
    let gx = gx_img.as_slice();
    let gy = gy_img.as_slice();

    let mut mag = vec![0.0f32; w * h];
    for i in 0..w * h {
        let x = gx[i] as f32;
        let y = gy[i] as f32;
        mag[i] = (x * x + y * y).sqrt();
    }

    // Non-maximum suppression
    let mut nms = vec![0.0f32; w * h];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = y * w + x;
            let m = mag[idx];
            if m < low_threshold {
                continue;
            }

            let angle = (gy[idx] as f32).atan2(gx[idx] as f32) * 180.0 / std::f32::consts::PI;
            let a = if angle < 0.0 { angle + 180.0 } else { angle };

            let (n1, n2) = if (0.0..=22.5).contains(&a) || (157.5..=180.0).contains(&a) {
                (mag[y * w + x - 1], mag[y * w + x + 1])
            } else if (22.5..=67.5).contains(&a) {
                (mag[(y - 1) * w + x + 1], mag[(y + 1) * w + x - 1])
            } else if (67.5..=112.5).contains(&a) {
                (mag[(y - 1) * w + x], mag[(y + 1) * w + x])
            } else {
                (mag[(y - 1) * w + x - 1], mag[(y + 1) * w + x + 1])
            };

            if m >= n1 && m >= n2 {
                nms[idx] = m;
            }
        }
    }

    // Double threshold and hysteresis
    let mut out = vec![0u8; w * h];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let idx = y * w + x;
            if nms[idx] >= high_threshold {
                out[idx] = 255;
            } else if nms[idx] >= low_threshold {
                // Check 8-neighborhood for strong edge
                let mut strong = false;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let ny = (y as i32 + dy) as usize;
                        let nx = (x as i32 + dx) as usize;
                        if nms[ny * w + nx] >= high_threshold {
                            strong = true;
                            break;
                        }
                    }
                }
                if strong {
                    out[idx] = 255;
                }
            }
        }
    }

    GrayImage::from_raw(image.width(), image.height(), out).expect("valid size")
}

/// Performs morphological dilation using a square structuring element of given radius.
#[must_use]
pub fn dilate(image: &GrayImage, radius: u8) -> GrayImage {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let raw = image.as_raw();
    let r = radius as usize;
    let mut out = vec![0u8; w * h];

    for y in 0..h {
        let y_min = y.saturating_sub(r);
        let y_max = (y + r).min(h - 1);

        for x in 0..w {
            let x_min = x.saturating_sub(r);
            let x_max = (x + r).min(w - 1);

            let mut max_val = 0u8;
            for ny in y_min..=y_max {
                let row = ny * w;
                for nx in x_min..=x_max {
                    max_val = max_val.max(raw[row + nx]);
                }
            }
            out[y * w + x] = max_val;
        }
    }

    GrayImage::from_raw(image.width(), image.height(), out).expect("valid size")
}

/// Performs morphological erosion using a square structuring element of given radius.
#[must_use]
pub fn erode(image: &GrayImage, radius: u8) -> GrayImage {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let raw = image.as_raw();
    let r = radius as usize;
    let mut out = vec![255u8; w * h];

    for y in 0..h {
        let y_min = y.saturating_sub(r);
        let y_max = (y + r).min(h - 1);

        for x in 0..w {
            let x_min = x.saturating_sub(r);
            let x_max = (x + r).min(w - 1);

            let mut min_val = 255u8;
            for ny in y_min..=y_max {
                let row = ny * w;
                for nx in x_min..=x_max {
                    min_val = min_val.min(raw[row + nx]);
                }
            }
            out[y * w + x] = min_val;
        }
    }

    GrayImage::from_raw(image.width(), image.height(), out).expect("valid size")
}

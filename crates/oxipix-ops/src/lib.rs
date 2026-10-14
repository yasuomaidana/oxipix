//! Image Operations: High-performance resizing filters, spatial blurs, and binarization.
#![deny(missing_docs)]

use oxipix_core::{OxipixError, PixelHeight, PixelWidth};

/// Resampling filter algorithm used during scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    /// Nearest Neighbor (fastest, sharp pixel edges).
    Nearest,
    /// Triangle (bilinear interpolation).
    Triangle,
    /// Catmull-Rom cubic spline.
    CatmullRom,
    /// Lanczos windowed sinc with radius 3 (highest fidelity downsampling).
    Lanczos3,
}

/// Resizes RGBA32 pixel buffer using Nearest Neighbor sampling.
pub fn resize_nearest_rgba(
    src: &[u8],
    src_w: PixelWidth,
    src_h: PixelHeight,
    dst_w: PixelWidth,
    dst_h: PixelHeight,
) -> Result<Vec<u8>, OxipixError> {
    let sw = src_w.get() as usize;
    let sh = src_h.get() as usize;
    let dw = dst_w.get() as usize;
    let dh = dst_h.get() as usize;

    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return Err(OxipixError::ZeroDimensions { width: dst_w.get(), height: dst_h.get() });
    }

    let src_expected = sw.checked_mul(sh).and_then(|px| px.checked_mul(4))
        .ok_or(OxipixError::AllocationOverflow { width: src_w.get(), height: src_h.get(), bytes_per_pixel: 4 })?;
    if src.len() < src_expected {
        return Err(OxipixError::BufferUnderflow);
    }

    let dst_size = dw.checked_mul(dh).and_then(|px| px.checked_mul(4))
        .ok_or(OxipixError::AllocationOverflow { width: dst_w.get(), height: dst_h.get(), bytes_per_pixel: 4 })?;

    let mut out = vec![0u8; dst_size];

    for y in 0..dh {
        let src_y = (y * sh) / dh;
        let dst_row_start = y * dw * 4;
        let src_row_start = src_y * sw * 4;

        for x in 0..dw {
            let src_x = (x * sw) / dw;
            let src_idx = src_row_start + (src_x * 4);
            let dst_idx = dst_row_start + (x * 4);

            if let (Some(src_slice), Some(dst_slice)) = (src.get(src_idx..src_idx + 4), out.get_mut(dst_idx..dst_idx + 4)) {
                dst_slice.copy_from_slice(src_slice);
            }
        }
    }

    Ok(out)
}

/// Computes the Otsu global binarization threshold on an 8-bit grayscale image.
pub fn otsu_threshold(gray: &[u8]) -> u8 {
    let mut histogram = [0u32; 256];
    for &val in gray {
        histogram[val as usize] += 1;
    }

    let total = gray.len() as f64;
    if total == 0.0 {
        return 128;
    }

    let mut sum = 0.0;
    for (i, &count) in histogram.iter().enumerate() {
        sum += (i as f64) * (count as f64);
    }

    let mut sum_b = 0.0;
    let mut w_b = 0.0;
    let mut max_variance = 0.0;
    let mut best_threshold = 0u8;

    for (t, &count) in histogram.iter().enumerate() {
        w_b += count as f64;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }

        sum_b += (t as f64) * (count as f64);
        let m_b = sum_b / w_b;
        let m_f = (sum - sum_b) / w_f;

        let between_variance = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if between_variance > max_variance {
            max_variance = between_variance;
            best_threshold = t as u8;
        }
    }

    best_threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_nearest() {
        let src = [
            255, 0, 0, 255,  0, 255, 0, 255,
            0, 0, 255, 255,  255, 255, 255, 255,
        ];
        let resized = resize_nearest_rgba(&src, PixelWidth(2), PixelHeight(2), PixelWidth(4), PixelHeight(4))
            .expect("resize should succeed");
        assert_eq!(resized.len(), 4 * 4 * 4);
        // Top-left pixel remains red
        assert_eq!(&resized[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn test_otsu_threshold_bimodal() {
        let mut gray = vec![20u8; 500];
        gray.extend_from_slice(&[200u8; 500]);
        let t = otsu_threshold(&gray);
        assert!((20..=199).contains(&t));
    }
}

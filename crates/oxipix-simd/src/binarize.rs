//! Hardware-accelerated and adaptive binarization algorithms (Otsu & Bradley-Roth).
#![deny(missing_docs)]

use oxipix_core::OxipixError;

/// Binarizes grayscale buffer against a fixed threshold value (vectorized).
pub fn threshold(gray: &[u8], thresh: u8, out: &mut [u8]) -> Result<(), OxipixError> {
    if out.len() < gray.len() {
        return Err(OxipixError::BufferUnderflow);
    }
    let len = gray.len();

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            let mut i = 0;
            let thresh_vec = unsafe { std::arch::aarch64::vdupq_n_u8(thresh) };
            while i + 16 <= len {
                unsafe {
                    use std::arch::aarch64::*;
                    let val = vld1q_u8(gray.as_ptr().add(i));
                    // Compare greater or equal
                    let mask = vcgeq_u8(val, thresh_vec);
                    vst1q_u8(out.as_mut_ptr().add(i), mask);
                }
                i += 16;
            }
            for p in i..len {
                out[p] = if gray[p] >= thresh { 255 } else { 0 };
            }
            return Ok(());
        }
    }

    for p in 0..len {
        out[p] = if gray[p] >= thresh { 255 } else { 0 };
    }

    Ok(())
}

/// Computes the optimal global binarization threshold via Otsu's method.
#[must_use]
pub fn otsu_threshold(gray: &[u8]) -> u8 {
    if gray.is_empty() {
        return 128;
    }

    let mut histogram = [0u64; 256];
    for &b in gray {
        histogram[b as usize] += 1;
    }

    let total = gray.len() as f64;
    let mut sum = 0.0;
    for (i, &count) in histogram.iter().enumerate() {
        sum += (i as f64) * (count as f64);
    }

    let mut sum_b = 0.0;
    let mut w_b = 0.0;
    let mut max_variance = 0.0;
    let mut optimal_threshold = 128u8;

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

        let variance = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if variance > max_variance {
            max_variance = variance;
            optimal_threshold = t as u8;
        }
    }

    optimal_threshold
}

/// Bradley-Roth adaptive local binarization using an integral image.
///
/// Efficiently segments non-uniformly illuminated 1D/2D barcodes.
pub fn bradley_roth_threshold(
    gray: &[u8],
    width: usize,
    height: usize,
    out: &mut [u8],
) -> Result<(), OxipixError> {
    if width == 0 || height == 0 {
        return Err(OxipixError::ZeroDimensions {
            width: width as u32,
            height: height as u32,
        });
    }
    if gray.len() < width * height || out.len() < width * height {
        return Err(OxipixError::BufferUnderflow);
    }

    // Window size S = width / 8
    let s = (width / 8).max(4);
    let s2 = s / 2;
    let t_percent = 0.15f32; // 15% threshold standard

    // Build integral image (32-bit sums)
    let stride = width + 1;
    let mut integral = vec![0u32; stride * (height + 1)];

    for y in 0..height {
        let mut row_sum = 0u32;
        let src_row = y * width;
        let int_row = (y + 1) * stride;
        let int_prev_row = y * stride;

        for x in 0..width {
            row_sum += gray[src_row + x] as u32;
            integral[int_row + x + 1] = integral[int_prev_row + x + 1] + row_sum;
        }
    }

    // Adaptive threshold check for each pixel
    for y in 0..height {
        let y1 = y.saturating_sub(s2);
        let y2 = (y + s2).min(height - 1);

        for x in 0..width {
            let x1 = x.saturating_sub(s2);
            let x2 = (x + s2).min(width - 1);

            let count = ((x2 - x1 + 1) * (y2 - y1 + 1)) as u32;

            // Area sum from integral image: D - B - C + A
            let br = integral[(y2 + 1) * stride + (x2 + 1)];
            let bl = integral[(y2 + 1) * stride + x1];
            let tr = integral[y1 * stride + (x2 + 1)];
            let tl = integral[y1 * stride + x1];
            let sum = br + tl - bl - tr;

            let pixel_val = gray[y * width + x] as u32;
            // (pixel * count) <= sum * (1 - T)
            let factor = ((1.0 - t_percent) * 256.0) as u32;
            let thresh = (sum * factor) / 256;

            if pixel_val * count <= thresh {
                out[y * width + x] = 0; // Black / Barcode module
            } else {
                out[y * width + x] = 255; // White / Background
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otsu_and_fixed_threshold() {
        let gray = [10u8, 20, 200, 220, 15, 230];
        let thresh = otsu_threshold(&gray);
        assert!((20..=200).contains(&thresh));

        let mut out = [0u8; 6];
        threshold(&gray, 100, &mut out).expect("threshold succeeds");
        assert_eq!(out, [0, 0, 255, 255, 0, 255]);
    }

    #[test]
    fn test_bradley_roth_adaptive() {
        let width = 16;
        let height = 16;
        let mut gray = vec![240u8; width * height];
        // Introduce a dark barcode pattern block in center
        for y in 6..10 {
            for x in 6..10 {
                gray[y * width + x] = 20;
            }
        }
        let mut binary = vec![255u8; width * height];
        bradley_roth_threshold(&gray, width, height, &mut binary).expect("adaptive binarization succeeds");
        // Center should be detected as 0 (black module)
        assert_eq!(binary[8 * width + 8], 0);
        // Outer region should be 255 (white)
        assert_eq!(binary[0], 255);
    }
}

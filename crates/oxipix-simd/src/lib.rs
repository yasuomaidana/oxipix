//! Accelerated SIMD Vector Engine: AAN Discrete Cosine Transform and Color Space Kernels.
#![deny(missing_docs)]

use oxipix_core::OxipixError;

/// Available SIMD vector extensions detected at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdLevel {
    /// Pure Rust portable scalar fallback.
    Scalar,
    /// ARM 128-bit NEON vector extensions.
    Neon,
    /// x86_64 256-bit Advanced Vector Extensions 2.
    Avx2,
    /// x86_64 512-bit Advanced Vector Extensions 512.
    Avx512,
}

/// Detects the highest available vector execution level on the current CPU host.
#[must_use]
pub fn detect_simd_level() -> SimdLevel {
    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            return SimdLevel::Neon;
        }
    }
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            return SimdLevel::Avx512;
        }
        if is_x86_feature_detected!("avx2") {
            return SimdLevel::Avx2;
        }
    }
    SimdLevel::Scalar
}

/// Fast 1D & 2D Arai, Agneche, and Bellas (AAN) Discrete Cosine Transform.
pub struct FastDct8x8;

impl FastDct8x8 {
    /// Fixed-point scaling factor: 2^11 = 2048
    const SHIFT: i32 = 11;
    const CONST_BITS: i32 = 11;
    const FIX_0_382683432: i32 = 784;  // sin(pi/8) * 2048
    const FIX_0_541196100: i32 = 1108; // cos(pi/8) - sin(pi/8) * 2048
    const FIX_0_707106781: i32 = 1448; // cos(pi/4) * 2048
    const FIX_1_306562965: i32 = 2676; // cos(pi/8) + sin(pi/8) * 2048

    /// Executes forward 2D AAN DCT in-place on an 8x8 block of i16 samples.
    pub fn forward(block: &mut [i16; 64]) {
        // Row-wise 1D DCT pass
        for row in 0..8 {
            let offset = row * 8;
            Self::forward_1d_row(block, offset);
        }
        // Column-wise 1D DCT pass
        for col in 0..8 {
            Self::forward_1d_col(block, col);
        }
    }

    fn forward_1d_row(block: &mut [i16; 64], offset: usize) {
        let tmp0 = i32::from(block[offset]) + i32::from(block[offset + 7]);
        let tmp7 = i32::from(block[offset]) - i32::from(block[offset + 7]);
        let tmp1 = i32::from(block[offset + 1]) + i32::from(block[offset + 6]);
        let tmp6 = i32::from(block[offset + 1]) - i32::from(block[offset + 6]);
        let tmp2 = i32::from(block[offset + 2]) + i32::from(block[offset + 5]);
        let tmp5 = i32::from(block[offset + 2]) - i32::from(block[offset + 5]);
        let tmp3 = i32::from(block[offset + 3]) + i32::from(block[offset + 4]);
        let tmp4 = i32::from(block[offset + 3]) - i32::from(block[offset + 4]);

        // Even part
        let tmp10 = tmp0 + tmp3;
        let tmp13 = tmp0 - tmp3;
        let tmp11 = tmp1 + tmp2;
        let tmp12 = tmp1 - tmp2;

        block[offset] = (tmp10 + tmp11) as i16;
        block[offset + 4] = (tmp10 - tmp11) as i16;

        let z1 = (tmp12 + tmp13) * Self::FIX_0_707106781;
        block[offset + 2] = (tmp13 + (z1 >> Self::CONST_BITS)) as i16;
        block[offset + 6] = (tmp13 - (z1 >> Self::CONST_BITS)) as i16;

        // Odd part
        let tmp10_odd = tmp4 + tmp5;
        let tmp11_odd = tmp5 + tmp6;
        let tmp12_odd = tmp6 + tmp7;

        let z5 = (tmp10_odd - tmp12_odd) * Self::FIX_0_382683432;
        let z2 = (Self::FIX_0_541196100 * tmp10_odd) + z5;
        let z4 = (Self::FIX_1_306562965 * tmp12_odd) + z5;
        let z3 = tmp11_odd * Self::FIX_0_707106781;

        let z11 = tmp7 + (z3 >> Self::SHIFT);
        let z13 = tmp7 - (z3 >> Self::SHIFT);

        block[offset + 5] = (z13 + (z2 >> Self::SHIFT)) as i16;
        block[offset + 3] = (z13 - (z2 >> Self::SHIFT)) as i16;
        block[offset + 1] = (z11 + (z4 >> Self::SHIFT)) as i16;
        block[offset + 7] = (z11 - (z4 >> Self::SHIFT)) as i16;
    }

    fn forward_1d_col(block: &mut [i16; 64], col: usize) {
        let tmp0 = i32::from(block[col]) + i32::from(block[col + 56]);
        let tmp7 = i32::from(block[col]) - i32::from(block[col + 56]);
        let tmp1 = i32::from(block[col + 8]) + i32::from(block[col + 48]);
        let tmp6 = i32::from(block[col + 8]) - i32::from(block[col + 48]);
        let tmp2 = i32::from(block[col + 16]) + i32::from(block[col + 40]);
        let tmp5 = i32::from(block[col + 16]) - i32::from(block[col + 40]);
        let tmp3 = i32::from(block[col + 24]) + i32::from(block[col + 32]);
        let tmp4 = i32::from(block[col + 24]) - i32::from(block[col + 32]);

        let tmp10 = tmp0 + tmp3;
        let tmp13 = tmp0 - tmp3;
        let tmp11 = tmp1 + tmp2;
        let tmp12 = tmp1 - tmp2;

        block[col] = ((tmp10 + tmp11) >> 3) as i16;
        block[col + 32] = ((tmp10 - tmp11) >> 3) as i16;

        let z1 = (tmp12 + tmp13) * Self::FIX_0_707106781;
        block[col + 16] = ((tmp13 + (z1 >> Self::CONST_BITS)) >> 3) as i16;
        block[col + 48] = ((tmp13 - (z1 >> Self::CONST_BITS)) >> 3) as i16;

        let tmp10_odd = tmp4 + tmp5;
        let tmp11_odd = tmp5 + tmp6;
        let tmp12_odd = tmp6 + tmp7;

        let z5 = (tmp10_odd - tmp12_odd) * Self::FIX_0_382683432;
        let z2 = (Self::FIX_0_541196100 * tmp10_odd) + z5;
        let z4 = (Self::FIX_1_306562965 * tmp12_odd) + z5;
        let z3 = tmp11_odd * Self::FIX_0_707106781;

        let z11 = tmp7 + (z3 >> Self::SHIFT);
        let z13 = tmp7 - (z3 >> Self::SHIFT);

        block[col + 40] = ((z13 + (z2 >> Self::SHIFT)) >> 3) as i16;
        block[col + 24] = ((z13 - (z2 >> Self::SHIFT)) >> 3) as i16;
        block[col + 8] = ((z11 + (z4 >> Self::SHIFT)) >> 3) as i16;
        block[col + 56] = ((z11 - (z4 >> Self::SHIFT)) >> 3) as i16;
    }
}

/// Converts RGB24 pixel triples into YCbCr according to ITU-R BT.601 standard.
pub fn rgb_to_ycbcr(rgb: &[u8], y_out: &mut [u8], cb_out: &mut [u8], cr_out: &mut [u8]) -> Result<(), OxipixError> {
    if !rgb.len().is_multiple_of(3) {
        return Err(OxipixError::BufferUnderflow);
    }
    let count = rgb.len() / 3;
    if y_out.len() < count || cb_out.len() < count || cr_out.len() < count {
        return Err(OxipixError::BufferUnderflow);
    }

    for i in 0..count {
        let r = i32::from(rgb[i * 3]);
        let g = i32::from(rgb[i * 3 + 1]);
        let b = i32::from(rgb[i * 3 + 2]);

        // Y = 0.299*R + 0.587*G + 0.114*B
        let y = ((19595 * r + 38470 * g + 7471 * b) >> 16) as u8;
        // Cb = -0.1687*R - 0.3313*G + 0.5*B + 128
        let cb = ((-11059 * r - 21709 * g + 32768 * b + 8388608) >> 16).clamp(0, 255) as u8;
        // Cr = 0.5*R - 0.4187*G - 0.0813*B + 128
        let cr = ((32768 * r - 27439 * g - 5329 * b + 8388608) >> 16).clamp(0, 255) as u8;

        y_out[i] = y;
        cb_out[i] = cb;
        cr_out[i] = cr;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_simd() {
        let level = detect_simd_level();
        assert!(matches!(level, SimdLevel::Scalar | SimdLevel::Neon | SimdLevel::Avx2 | SimdLevel::Avx512));
    }

    #[test]
    fn test_fast_dct_uniform_block() {
        let mut block = [128i16; 64];
        FastDct8x8::forward(&mut block);
        // Uniform block DC coefficient dominates, AC terms near zero
        assert!(block[0] > 100);
        assert_eq!(block[1], 0);
        assert_eq!(block[2], 0);
    }

    #[test]
    fn test_rgb_to_ycbcr() {
        let rgb = [255, 255, 255, 0, 0, 0];
        let mut y = [0; 2];
        let mut cb = [0; 2];
        let mut cr = [0; 2];

        rgb_to_ycbcr(&rgb, &mut y, &mut cb, &mut cr).expect("conversion should succeed");
        // White -> Y: 255, Cb: 128, Cr: 128
        assert_eq!(y[0], 255);
        assert_eq!(cb[0], 128);
        assert_eq!(cr[0], 128);
        // Black -> Y: 0, Cb: 128, Cr: 128
        assert_eq!(y[1], 0);
        assert_eq!(cb[1], 128);
        assert_eq!(cr[1], 128);
    }
}

//! Accelerated color space transformations and channel conversions.
#![deny(missing_docs)]

use oxipix_core::OxipixError;

/// Converts RGBA 4-byte packed pixels into RGB 3-byte packed pixels.
pub fn rgba_to_rgb(rgba: &[u8], rgb: &mut [u8]) -> Result<(), OxipixError> {
    if !rgba.len().is_multiple_of(4) {
        return Err(OxipixError::BufferUnderflow);
    }
    let px_count = rgba.len() / 4;
    let expected_rgb = px_count
        .checked_mul(3)
        .ok_or(OxipixError::BufferUnderflow)?;
    if rgb.len() < expected_rgb {
        return Err(OxipixError::BufferUnderflow);
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            let mut i = 0;
            // Process 16 RGBA pixels (64 bytes) -> 16 RGB pixels (48 bytes) per step
            while i + 16 <= px_count {
                unsafe {
                    use std::arch::aarch64::*;
                    let rgba_ptr = rgba.as_ptr().add(i * 4);
                    let quad = vld4q_u8(rgba_ptr);
                    let rgb_tuple = uint8x16x3_t(quad.0, quad.1, quad.2);
                    let rgb_ptr = rgb.as_mut_ptr().add(i * 3);
                    vst3q_u8(rgb_ptr, rgb_tuple);
                }
                i += 16;
            }
            // Tail
            for p in i..px_count {
                let src = p * 4;
                let dst = p * 3;
                rgb[dst] = rgba[src];
                rgb[dst + 1] = rgba[src + 1];
                rgb[dst + 2] = rgba[src + 2];
            }
            return Ok(());
        }
    }

    // Scalar / auto-vectorized loop
    for p in 0..px_count {
        let src = p * 4;
        let dst = p * 3;
        rgb[dst] = rgba[src];
        rgb[dst + 1] = rgba[src + 1];
        rgb[dst + 2] = rgba[src + 2];
    }

    Ok(())
}

/// Converts RGB 3-byte packed pixels into RGBA 4-byte packed pixels with given alpha.
pub fn rgb_to_rgba(rgb: &[u8], alpha: u8, rgba: &mut [u8]) -> Result<(), OxipixError> {
    if !rgb.len().is_multiple_of(3) {
        return Err(OxipixError::BufferUnderflow);
    }
    let px_count = rgb.len() / 3;
    let expected_rgba = px_count
        .checked_mul(4)
        .ok_or(OxipixError::BufferUnderflow)?;
    if rgba.len() < expected_rgba {
        return Err(OxipixError::BufferUnderflow);
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            let mut i = 0;
            let a_vec = unsafe { std::arch::aarch64::vdupq_n_u8(alpha) };
            while i + 16 <= px_count {
                unsafe {
                    use std::arch::aarch64::*;
                    let rgb_ptr = rgb.as_ptr().add(i * 3);
                    let rgb_tuple = vld3q_u8(rgb_ptr);
                    let rgba_tuple = uint8x16x4_t(rgb_tuple.0, rgb_tuple.1, rgb_tuple.2, a_vec);
                    let rgba_ptr = rgba.as_mut_ptr().add(i * 4);
                    vst4q_u8(rgba_ptr, rgba_tuple);
                }
                i += 16;
            }
            for p in i..px_count {
                let src = p * 3;
                let dst = p * 4;
                rgba[dst] = rgb[src];
                rgba[dst + 1] = rgb[src + 1];
                rgba[dst + 2] = rgb[src + 2];
                rgba[dst + 3] = alpha;
            }
            return Ok(());
        }
    }

    for p in 0..px_count {
        let src = p * 3;
        let dst = p * 4;
        rgba[dst] = rgb[src];
        rgba[dst + 1] = rgb[src + 1];
        rgba[dst + 2] = rgb[src + 2];
        rgba[dst + 3] = alpha;
    }

    Ok(())
}

/// Converts RGBA pixels to 8-bit Grayscale (Luma) using ITU-R BT.601 coefficients.
pub fn rgba_to_gray(rgba: &[u8], gray: &mut [u8]) -> Result<(), OxipixError> {
    if !rgba.len().is_multiple_of(4) {
        return Err(OxipixError::BufferUnderflow);
    }
    let px_count = rgba.len() / 4;
    if gray.len() < px_count {
        return Err(OxipixError::BufferUnderflow);
    }

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            let mut i = 0;
            while i + 8 <= px_count {
                unsafe {
                    use std::arch::aarch64::*;
                    // Load 8 RGBA pixels (32 bytes)
                    let ptr = rgba.as_ptr().add(i * 4);
                    let channels = vld4_u8(ptr);
                    // R * 77 + G * 150 + B * 29 >> 8
                    let r_w = vmovl_u8(channels.0);
                    let g_w = vmovl_u8(channels.1);
                    let b_w = vmovl_u8(channels.2);

                    let mut acc = vmull_n_u16(vget_low_u16(r_w), 77);
                    acc = vmlal_n_u16(acc, vget_low_u16(g_w), 150);
                    acc = vmlal_n_u16(acc, vget_low_u16(b_w), 29);
                    let low_8 = vshrn_n_u32(acc, 8);

                    let mut acc_hi = vmull_n_u16(vget_high_u16(r_w), 77);
                    acc_hi = vmlal_n_u16(acc_hi, vget_high_u16(g_w), 150);
                    acc_hi = vmlal_n_u16(acc_hi, vget_high_u16(b_w), 29);
                    let high_8 = vshrn_n_u32(acc_hi, 8);

                    let res16 = vcombine_u16(low_8, high_8);
                    let res8 = vmovn_u16(res16);
                    vst1_u8(gray.as_mut_ptr().add(i), res8);
                }
                i += 8;
            }
            for (p, dest) in gray.iter_mut().enumerate().take(px_count).skip(i) {
                let idx = p * 4;
                let r = rgba[idx] as u32;
                let g = rgba[idx + 1] as u32;
                let b = rgba[idx + 2] as u32;
                *dest = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
            }
            return Ok(());
        }
    }

    for (p, dest) in gray.iter_mut().enumerate().take(px_count) {
        let idx = p * 4;
        let r = rgba[idx] as u32;
        let g = rgba[idx + 1] as u32;
        let b = rgba[idx + 2] as u32;
        *dest = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
    }

    Ok(())
}

/// Converts Grayscale (Luma) into RGBA pixels with given alpha.
pub fn gray_to_rgba(gray: &[u8], alpha: u8, rgba: &mut [u8]) -> Result<(), OxipixError> {
    let px_count = gray.len();
    let expected = px_count
        .checked_mul(4)
        .ok_or(OxipixError::BufferUnderflow)?;
    if rgba.len() < expected {
        return Err(OxipixError::BufferUnderflow);
    }

    for (p, &g) in gray.iter().enumerate().take(px_count) {
        let idx = p * 4;
        rgba[idx] = g;
        rgba[idx + 1] = g;
        rgba[idx + 2] = g;
        rgba[idx + 3] = alpha;
    }

    Ok(())
}

/// Inverts color channels in place (255 - c) for RGBA (RGB inverted, Alpha untouched).
pub fn invert_rgba_inplace(rgba: &mut [u8]) {
    let px_count = rgba.len() / 4;

    #[cfg(target_arch = "aarch64")]
    {
        if std::arch::is_aarch64_feature_detected!("neon") {
            let mut i = 0;
            while i + 16 <= px_count {
                unsafe {
                    use std::arch::aarch64::*;
                    let ptr = rgba.as_mut_ptr().add(i * 4);
                    let quad = vld4q_u8(ptr);
                    let inv_r = vmvnq_u8(quad.0);
                    let inv_g = vmvnq_u8(quad.1);
                    let inv_b = vmvnq_u8(quad.2);
                    let new_quad = uint8x16x4_t(inv_r, inv_g, inv_b, quad.3);
                    vst4q_u8(ptr, new_quad);
                }
                i += 16;
            }
            for p in i..px_count {
                let idx = p * 4;
                rgba[idx] = 255 - rgba[idx];
                rgba[idx + 1] = 255 - rgba[idx + 1];
                rgba[idx + 2] = 255 - rgba[idx + 2];
            }
            return;
        }
    }

    for p in 0..px_count {
        let idx = p * 4;
        rgba[idx] = 255 - rgba[idx];
        rgba[idx + 1] = 255 - rgba[idx + 1];
        rgba[idx + 2] = 255 - rgba[idx + 2];
    }
}

/// Inverts Grayscale bytes in place (255 - c).
pub fn invert_gray_inplace(gray: &mut [u8]) {
    for b in gray.iter_mut() {
        *b = 255 - *b;
    }
}

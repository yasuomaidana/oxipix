//! CPU reference implementations. They define the semantics the WGSL kernels must match
//! (truncating quantisation, clamp-to-edge convolution, white outside the source).

/// 3x3 convolution over RGB with clamp-to-edge borders; alpha is copied through.
#[must_use]
pub fn convolve3x3(input: &[u8], width: u32, height: u32, kernel: &[f32; 9]) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 3];
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let sx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
                    let sy = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
                    let weight = kernel[((dy + 1) * 3 + (dx + 1)) as usize];
                    let src = (sy * w + sx) * 4;
                    for (c, a) in acc.iter_mut().enumerate() {
                        *a += f32::from(input[src + c]) * weight;
                    }
                }
            }
            let dst = (y * w + x) * 4;
            for (c, a) in acc.iter().enumerate() {
                out[dst + c] = a.clamp(0.0, 255.0) as u8;
            }
            out[dst + 3] = input[dst + 3];
        }
    }
    out
}

/// Bilinear perspective warp. `inv_h` maps destination pixel coordinates to source
/// coordinates (row-major 3x3). Samples outside the source are white.
#[must_use]
pub fn warp_perspective(
    input: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
    inv_h: &[f32; 9],
) -> Vec<u8> {
    let (sw, sh) = (src_w as usize, src_h as usize);
    let mut out = vec![255u8; dst_w as usize * dst_h as usize * 4];
    let m = inv_h;
    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let (x, y) = (dx as f32, dy as f32);
            let w = m[6] * x + m[7] * y + m[8];
            if w.abs() <= 0.000_01 {
                continue;
            }
            let u = (m[0] * x + m[1] * y + m[2]) / w;
            let v = (m[3] * x + m[4] * y + m[5]) / w;
            if u < 0.0 || u > (sw - 1) as f32 || v < 0.0 || v > (sh - 1) as f32 {
                continue;
            }
            let x0 = (u.floor() as usize).min(sw - 1);
            let y0 = (v.floor() as usize).min(sh - 1);
            let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
            let (fx, fy) = (u - x0 as f32, v - y0 as f32);
            let dst = (dy as usize * dst_w as usize + dx as usize) * 4;
            for c in 0..4 {
                let p = |xx: usize, yy: usize| f32::from(input[(yy * sw + xx) * 4 + c]);
                let top = p(x0, y0) + (p(x1, y0) - p(x0, y0)) * fx;
                let bot = p(x0, y1) + (p(x1, y1) - p(x0, y1)) * fx;
                out[dst + c] = (top + (bot - top) * fy).clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

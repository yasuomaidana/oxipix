//! Image quality metrics for 8-bit grey images: mean squared error, PSNR and SSIM.

use image::GrayImage;

/// Mean squared error, or `None` when the sizes differ or the pictures are empty. The squares are summed in
/// blocks small enough for 32-bit lanes (4096 x 255^2 < 2^32) and widened once per block.
#[must_use]
pub fn mse(a: &GrayImage, b: &GrayImage) -> Option<f64> {
    if a.dimensions() != b.dimensions() || a.as_raw().is_empty() {
        return None;
    }
    let total: u64 = a
        .as_raw()
        .chunks(4096)
        .zip(b.as_raw().chunks(4096))
        .map(|(x, y)| {
            let block: u32 = x
                .iter()
                .zip(y)
                .map(|(p, q)| u32::from(p.abs_diff(*q)).pow(2))
                .sum();
            u64::from(block)
        })
        .sum();
    Some(total as f64 / a.as_raw().len() as f64)
}

/// Peak signal-to-noise ratio in dB; identical pictures give 100, as OpenCV's tutorial caps it.
#[must_use]
pub fn psnr(a: &GrayImage, b: &GrayImage) -> Option<f64> {
    let error = mse(a, b)?;
    Some(if error < 1e-10 {
        100.0
    } else {
        10.0 * (255.0f64 * 255.0 / error).log10()
    })
}

/// Normalised 11-tap Gaussian (sigma 1.5) as `f32`.
fn kernel() -> [f32; 11] {
    let raw: [f32; 11] = std::array::from_fn(|i| {
        let d = i as f32 - 5.0;
        (-d * d / (2.0 * 1.5 * 1.5)).exp()
    });
    let total: f32 = raw.iter().sum();
    raw.map(|v| v / total)
}

/// Separable 11x11 Gaussian of a `w x h` plane; only positions at least 5 from every edge are meaningful.
fn blur(src: &[f32], w: usize, h: usize, k: &[f32; 11]) -> Vec<f32> {
    let inner = w - 10;
    let mut across = vec![0.0f32; w * h];
    for (row, out) in src.chunks_exact(w).zip(across.chunks_exact_mut(w)) {
        for (t, weight) in k.iter().enumerate() {
            out[5..5 + inner]
                .iter_mut()
                .zip(&row[t..])
                .for_each(|(o, s)| *o += weight * s);
        }
    }
    let mut out = vec![0.0f32; w * h];
    for y in 5..h - 5 {
        let dst = &mut out[y * w + 5..y * w + 5 + inner];
        for (t, weight) in k.iter().enumerate() {
            let row = &across[(y + t - 5) * w + 5..][..inner];
            dst.iter_mut().zip(row).for_each(|(o, s)| *o += weight * s);
        }
    }
    out
}

/// Mean structural similarity over 11x11 Gaussian windows (sigma 1.5) with the usual constants, over the
/// windows that fit inside the picture. `None` when the sizes differ; pictures smaller than 11 x 11 give 1.
#[must_use]
pub fn ssim(a: &GrayImage, b: &GrayImage) -> Option<f64> {
    if a.dimensions() != b.dimensions() {
        return None;
    }
    let (w, h) = (a.width() as usize, a.height() as usize);
    if w < 11 || h < 11 {
        return Some(1.0);
    }
    let to_f32 = |img: &GrayImage| {
        img.as_raw()
            .iter()
            .map(|p| f32::from(*p))
            .collect::<Vec<_>>()
    };
    let (x, y) = (to_f32(a), to_f32(b));
    let square = |v: &[f32]| v.iter().map(|p| p * p).collect::<Vec<_>>();
    let product: Vec<f32> = x.iter().zip(&y).map(|(p, q)| p * q).collect();
    let k = kernel();
    let planes = [
        &x[..],
        &y[..],
        &square(&x)[..],
        &square(&y)[..],
        &product[..],
    ]
    .map(|p| blur(p, w, h, &k));
    let [mu_x, mu_y, xx, yy, xy] = planes;
    let (c1, c2) = ((0.01f64 * 255.0).powi(2), (0.03f64 * 255.0).powi(2));
    let (mut total, mut count) = (0.0f64, 0usize);
    for row in 5..h - 5 {
        for col in 5..w - 5 {
            let i = row * w + col;
            let (ma, mb) = (f64::from(mu_x[i]), f64::from(mu_y[i]));
            let (va, vb, cov) = (
                f64::from(xx[i]) - ma * ma,
                f64::from(yy[i]) - mb * mb,
                f64::from(xy[i]) - ma * mb,
            );
            total += (2.0 * ma * mb + c1) * (2.0 * cov + c2)
                / ((ma * ma + mb * mb + c1) * (va + vb + c2));
            count += 1;
        }
    }
    Some(total / count as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::blur::gaussian_blur;
    use crate::imgproc::testing::noise;
    use image::Luma;

    #[test]
    fn mse_and_psnr_follow_the_definition() {
        let (a, b) = (noise(40, 30, 1), noise(40, 30, 2));
        let direct: f64 = a
            .iter()
            .zip(b.iter())
            .map(|(p, q)| f64::from(p.abs_diff(*q)).powi(2))
            .sum::<f64>()
            / 1200.0;
        assert!((mse(&a, &b).unwrap() - direct).abs() < 1e-9);
        assert!((psnr(&a, &b).unwrap() - 10.0 * (65025.0 / direct).log10()).abs() < 1e-9);
        assert_eq!(psnr(&a, &a), Some(100.0));
        assert!(
            mse(&a, &noise(41, 30, 1)).is_none()
                && mse(&GrayImage::new(0, 0), &GrayImage::new(0, 0)).is_none()
        );
        let big = GrayImage::from_pixel(300, 300, Luma([255]));
        assert_eq!(mse(&big, &GrayImage::new(300, 300)), Some(65025.0));
    }

    #[test]
    fn ssim_is_one_for_equal_pictures_and_drops_with_degradation() {
        let photo = GrayImage::from_fn(64, 48, |x, y| {
            Luma([
                ((x * 5 + y * 3) % 200 + u32::from(noise(64, 48, 7).get_pixel(x, y).0[0]) / 8)
                    as u8,
            ])
        });
        assert!((ssim(&photo, &photo).unwrap() - 1.0).abs() < 1e-6);
        let soft = ssim(&photo, &gaussian_blur(&photo, 1.0)).unwrap();
        let softer = ssim(&photo, &gaussian_blur(&photo, 3.0)).unwrap();
        assert!(
            1.0 > soft && soft > softer && softer > 0.0,
            "{soft} {softer}"
        );
    }

    #[test]
    fn ssim_matches_a_direct_window_computation() {
        let (a, b) = (noise(24, 20, 3), noise(24, 20, 4));
        let k: Vec<f64> = kernel().iter().map(|v| f64::from(*v)).collect();
        let (c1, c2) = ((0.01f64 * 255.0).powi(2), (0.03f64 * 255.0).powi(2));
        let (mut total, mut count) = (0.0, 0);
        for y in 5..15 {
            for x in 5..19 {
                let (mut ma, mut mb, mut aa, mut bb, mut ab) = (0.0, 0.0, 0.0, 0.0, 0.0);
                for j in 0..11 {
                    for i in 0..11 {
                        let wgt = k[i] * k[j];
                        let (p, q) = (
                            f64::from(a.get_pixel((x + i - 5) as u32, (y + j - 5) as u32).0[0]),
                            f64::from(b.get_pixel((x + i - 5) as u32, (y + j - 5) as u32).0[0]),
                        );
                        (ma, mb, aa, bb, ab) = (
                            ma + wgt * p,
                            mb + wgt * q,
                            aa + wgt * p * p,
                            bb + wgt * q * q,
                            ab + wgt * p * q,
                        );
                    }
                }
                let (va, vb, cov) = (aa - ma * ma, bb - mb * mb, ab - ma * mb);
                total += (2.0 * ma * mb + c1) * (2.0 * cov + c2)
                    / ((ma * ma + mb * mb + c1) * (va + vb + c2));
                count += 1;
            }
        }
        assert!((ssim(&a, &b).unwrap() - total / f64::from(count)).abs() < 1e-3);
        assert_eq!(
            ssim(&GrayImage::new(5, 5), &GrayImage::new(5, 5)),
            Some(1.0)
        );
        assert!(ssim(&a, &noise(24, 21, 4)).is_none());
    }
}

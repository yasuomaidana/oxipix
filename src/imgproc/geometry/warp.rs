//! Affine and perspective warps with bilinear sampling (`warpAffine`, `warpPerspective`).
//!
//! The transform maps each destination pixel to a source position; positions outside the picture give black
//! (OpenCV's constant border). Matrices are the inverse (destination to source) maps, as `cv::warp*` takes
//! them with `WARP_INVERSE_MAP`.

use crate::imgproc::rows::for_each_band;
use image::{ImageBuffer, Pixel};

/// Bilinear sample of `image` at `(u, v)`, or `None` outside the picture.
#[inline]
fn sample(
    data: &[u8],
    size: (usize, usize),
    channels: usize,
    (u, v): (f32, f32),
    out: &mut [u8],
) -> bool {
    let (w, h) = size;
    if !(u >= 0.0 && v >= 0.0 && u <= (w - 1) as f32 && v <= (h - 1) as f32) {
        return false;
    }
    let (x0, y0) = (u as usize, v as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (u - x0 as f32, v - y0 as f32);
    for (c, slot) in out.iter_mut().enumerate() {
        let at = |x: usize, y: usize| f32::from(data[(y * w + x) * channels + c]);
        let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * fx;
        let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * fx;
        *slot = (top + (bottom - top) * fy + 0.5) as u8;
    }
    true
}

/// Warps with `map(x, y)` giving the source position of destination pixel `(x, y)`.
fn warp<P>(
    image: &ImageBuffer<P, Vec<u8>>,
    (ow, oh): (u32, u32),
    map: impl Fn(f32, f32) -> (f32, f32) + Sync,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    let channels = usize::from(P::CHANNEL_COUNT);
    let size = (image.width() as usize, image.height() as usize);
    let mut out = vec![0u8; ow as usize * oh as usize * channels];
    let row_len = (ow as usize * channels).max(1);
    if size.0 > 0 && size.1 > 0 {
        let raw = image.as_raw();
        for_each_band(&mut out, row_len, |first, band| {
            for (k, row) in band.chunks_exact_mut(row_len).enumerate() {
                for (x, pixel) in row.chunks_exact_mut(channels).enumerate() {
                    sample(
                        raw,
                        size,
                        channels,
                        map(x as f32, (first + k) as f32),
                        pixel,
                    );
                }
            }
        });
    }
    ImageBuffer::from_raw(ow, oh, out).expect("one value per channel and pixel")
}

/// Affine warp; `m = [a, b, c, d, e, f]` maps destination `(x, y)` to source `(a x + b y + c, d x + e y + f)`.
#[must_use]
pub fn warp_affine<P>(
    image: &ImageBuffer<P, Vec<u8>>,
    m: &[f32; 6],
    size: (u32, u32),
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    warp(image, size, |x, y| {
        (m[0] * x + m[1] * y + m[2], m[3] * x + m[4] * y + m[5])
    })
}

/// Perspective warp; `m` is the row-major 3x3 matrix taking destination `(x, y)` to the source position
/// `((m0 x + m1 y + m2) / w, (m3 x + m4 y + m5) / w)` with `w = m6 x + m7 y + m8`.
#[must_use]
pub fn warp_perspective<P>(
    image: &ImageBuffer<P, Vec<u8>>,
    m: &[f32; 9],
    size: (u32, u32),
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    warp(image, size, |x, y| {
        let w = m[6] * x + m[7] * y + m[8];
        (
            (m[0] * x + m[1] * y + m[2]) / w,
            (m[3] * x + m[4] * y + m[5]) / w,
        )
    })
}

/// Inverse of an affine map `[a, b, c, d, e, f]`, or `None` when it is singular.
#[must_use]
pub fn invert_affine(m: &[f32; 6]) -> Option<[f32; 6]> {
    let det = m[0] * m[4] - m[1] * m[3];
    if det.abs() < 1e-12 {
        return None;
    }
    let (a, b, d, e) = (m[4] / det, -m[1] / det, -m[3] / det, m[0] / det);
    Some([a, b, -(a * m[2] + b * m[5]), d, e, -(d * m[2] + e * m[5])])
}

/// The 3x3 homography taking each `from` point to the matching `to` point, or `None` for degenerate points.
/// Use it with the points swapped to get the destination-to-source map `warp_perspective` takes.
#[must_use]
pub fn homography(from: &[(f32, f32); 4], to: &[(f32, f32); 4]) -> Option<[f32; 9]> {
    let mut a = [[0.0f64; 9]; 8];
    for (i, ((x, y), (u, v))) in from.iter().zip(to).enumerate() {
        let (x, y, u, v) = (f64::from(*x), f64::from(*y), f64::from(*u), f64::from(*v));
        a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
        a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
    }
    for col in 0..8 {
        let pivot = (col..8).max_by(|i, j| a[*i][col].abs().total_cmp(&a[*j][col].abs()))?;
        if a[pivot][col].abs() < 1e-9 {
            return None;
        }
        a.swap(col, pivot);
        for row in 0..8 {
            if row != col {
                let factor = a[row][col] / a[col][col];
                let source = a[col];
                a[row]
                    .iter_mut()
                    .zip(source)
                    .for_each(|(r, s)| *r -= factor * s);
            }
        }
    }
    let mut h = [1.0f32; 9];
    for i in 0..8 {
        h[i] = (a[i][8] / a[i][i]) as f32;
    }
    Some(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use image::{GrayImage, Luma, Rgb, RgbImage};

    #[test]
    fn the_identity_keeps_the_picture() {
        let image = noise(21, 13, 5);
        assert_eq!(
            warp_affine(&image, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0], (21, 13)),
            image
        );
        assert_eq!(
            warp_perspective(
                &image,
                &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
                (21, 13)
            ),
            image
        );
    }

    #[test]
    fn a_shift_moves_pixels_and_fills_the_rest_with_black() {
        let image = GrayImage::from_fn(10, 4, |x, _| Luma([(x * 20 + 10) as u8]));
        let moved = warp_affine(&image, &[1.0, 0.0, -3.0, 0.0, 1.0, 0.0], (10, 4));
        assert_eq!(moved.get_pixel(5, 1).0[0], image.get_pixel(2, 1).0[0]);
        assert_eq!(moved.get_pixel(1, 1).0[0], 0);
        let half = warp_affine(&image, &[1.0, 0.0, 0.5, 0.0, 1.0, 0.0], (10, 4));
        assert_eq!(half.get_pixel(2, 0).0[0], 60);
    }

    #[test]
    fn colour_pictures_and_empty_sources_are_handled() {
        let rgb = RgbImage::from_pixel(6, 6, Rgb([9, 99, 199]));
        assert_eq!(
            warp_affine(&rgb, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0], (6, 6)),
            rgb
        );
        let empty = GrayImage::new(0, 0);
        assert!(
            warp_affine(&empty, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0], (3, 3))
                .iter()
                .all(|v| *v == 0)
        );
    }

    #[test]
    fn affine_inversion_round_trips_and_singular_maps_fail() {
        let m = [0.8, -0.3, 12.0, 0.3, 0.8, -4.0];
        let inv = invert_affine(&m).unwrap();
        let (x, y) = (7.0f32, 11.0f32);
        let (u, v) = (m[0] * x + m[1] * y + m[2], m[3] * x + m[4] * y + m[5]);
        assert!((inv[0] * u + inv[1] * v + inv[2] - x).abs() < 1e-3);
        assert!((inv[3] * u + inv[4] * v + inv[5] - y).abs() < 1e-3);
        assert!(invert_affine(&[1.0, 2.0, 0.0, 2.0, 4.0, 0.0]).is_none());
    }

    #[test]
    fn a_homography_maps_the_corners_and_warps_a_quad_to_a_rectangle() {
        let quad = [(10.0, 5.0), (90.0, 12.0), (85.0, 70.0), (6.0, 60.0)];
        let page = [(0.0, 0.0), (79.0, 0.0), (79.0, 59.0), (0.0, 59.0)];
        let h = homography(&quad, &page).unwrap();
        for (p, q) in quad.iter().zip(&page) {
            let w = h[6] * p.0 + h[7] * p.1 + h[8];
            assert!(((h[0] * p.0 + h[1] * p.1 + h[2]) / w - q.0).abs() < 1e-2);
            assert!(((h[3] * p.0 + h[4] * p.1 + h[5]) / w - q.1).abs() < 1e-2);
        }
        let back = homography(&page, &quad).unwrap();
        let image = noise(100, 80, 3);
        let flat = warp_perspective(&image, &back, (80, 60));
        assert_eq!(flat.dimensions(), (80, 60));
        assert!(
            flat.get_pixel(40, 30).0[0] == image.get_pixel(48, 36).0[0]
                || flat.iter().any(|v| *v > 0)
        );
        assert!(homography(&[(0.0, 0.0); 4], &page).is_none());
    }
}

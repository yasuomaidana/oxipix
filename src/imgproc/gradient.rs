//! Sobel and Laplacian derivatives of 8-bit images into 16-bit signed responses.
//!
//! Every derivative is separable (or, for the Laplacian, a sum of shifted rows), so each pass is an
//! element-wise `i16` add of shifted row slices. Pixels outside the picture repeat the nearest edge pixel.

use super::rows::{clamp_row, pad_row};
use image::{GrayImage, ImageBuffer, Luma};

/// A signed derivative response.
pub type Response = ImageBuffer<Luma<i16>, Vec<i16>>;

/// `out[i] = f(a[i], b[i], c[i])` over three equally long slices.
fn combine3(a: &[i16], b: &[i16], c: &[i16], out: &mut [i16], f: impl Fn(i16, i16, i16) -> i16) {
    let triples = a.iter().zip(b).zip(c).zip(out);
    triples.for_each(|(((x, y), z), o)| *o = f(*x, *y, *z));
}

/// The picture as `i16` rows of three differently shifted views: `(left, centre, right)` per output pixel,
/// combined by `f`, for every row.
fn horizontal(image: &GrayImage, f: impl Fn(i16, i16, i16) -> i16) -> Vec<i16> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let (mut padded, mut wide, mut out) = (Vec::new(), vec![0i16; w + 2], vec![0i16; w * h]);
    for (src, dst) in image.as_raw().chunks_exact(w).zip(out.chunks_exact_mut(w)) {
        pad_row(src, 1, 1, &mut padded);
        wide.iter_mut()
            .zip(&padded)
            .for_each(|(d, s)| *d = i16::from(*s));
        combine3(&wide[..w], &wide[1..=w], &wide[2..], dst, &f);
    }
    out
}

/// Combines rows `y - 1`, `y` and `y + 1` of an `i16` plane element-wise with `f`.
fn vertical(plane: &[i16], size: (usize, usize), f: impl Fn(i16, i16, i16) -> i16) -> Vec<i16> {
    let (w, h) = size;
    let mut out = vec![0i16; w * h];
    for (y, dst) in out.chunks_exact_mut(w).enumerate() {
        let row = |dy: isize| &plane[clamp_row(y, dy, h) * w..][..w];
        combine3(row(-1), row(0), row(1), dst, &f);
    }
    out
}

fn response(image: &GrayImage, data: Vec<i16>) -> Response {
    ImageBuffer::from_raw(image.width(), image.height(), data).expect("one value per pixel")
}

/// Sobel derivative along x: `[1 2 1]^T * [-1 0 1]`.
#[must_use]
pub fn sobel_x(image: &GrayImage) -> Response {
    if image.as_raw().is_empty() {
        return response(image, Vec::new());
    }
    let size = (image.width() as usize, image.height() as usize);
    let across = horizontal(image, |l, _, r| r - l);
    response(image, vertical(&across, size, |a, b, c| a + 2 * b + c))
}

/// Sobel derivative along y: `[1 0 -1]^T * [1 2 1]`, positive towards larger y.
#[must_use]
pub fn sobel_y(image: &GrayImage) -> Response {
    if image.as_raw().is_empty() {
        return response(image, Vec::new());
    }
    let size = (image.width() as usize, image.height() as usize);
    let across = horizontal(image, |l, c, r| l + 2 * c + r);
    response(image, vertical(&across, size, |a, _, c| c - a))
}

/// Laplacian with the 4-neighbour kernel `[[0 1 0] [1 -4 1] [0 1 0]]` (`cv::Laplacian`, `ksize = 1`).
#[must_use]
pub fn laplacian(image: &GrayImage) -> Response {
    if image.as_raw().is_empty() {
        return response(image, Vec::new());
    }
    let size = (image.width() as usize, image.height() as usize);
    let across = horizontal(image, |l, c, r| l - 2 * c + r);
    let down = horizontal(image, |_, c, _| c);
    let rows = vertical(&down, size, |a, b, c| a - 2 * b + c);
    let data = across.iter().zip(&rows).map(|(x, y)| x + y).collect();
    response(image, data)
}

/// `|response|` saturated to 255 (`cv::convertScaleAbs`).
#[must_use]
pub fn scale_abs(response: &Response) -> GrayImage {
    let data = response
        .iter()
        .map(|v| v.unsigned_abs().min(255) as u8)
        .collect();
    GrayImage::from_raw(response.width(), response.height(), data).expect("one byte per value")
}

/// `|response|` scaled so the strongest value becomes 255.
#[must_use]
pub fn normalize_abs(response: &Response) -> GrayImage {
    let strongest = response
        .iter()
        .map(|v| u32::from(v.unsigned_abs()))
        .max()
        .unwrap_or(0)
        .max(1);
    let data = response
        .iter()
        .map(|v| (u32::from(v.unsigned_abs()) * 255 / strongest) as u8)
        .collect();
    GrayImage::from_raw(response.width(), response.height(), data).expect("one byte per value")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use rstest::rstest;

    /// The 3x3 kernel applied directly with replicated edges (correlation, row-major).
    fn direct(image: &GrayImage, kernel: [i16; 9]) -> Vec<i16> {
        let (w, h) = (i64::from(image.width()), i64::from(image.height()));
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                (0..9)
                    .map(|k| {
                        let (px, py) = (
                            (x + k % 3 - 1).clamp(0, w - 1),
                            (y + k / 3 - 1).clamp(0, h - 1),
                        );
                        kernel[k as usize] * i16::from(image.get_pixel(px as u32, py as u32).0[0])
                    })
                    .sum()
            })
            .collect()
    }

    #[rstest]
    #[case(17, 11)]
    #[case(1, 1)]
    #[case(2, 3)]
    fn derivatives_equal_the_direct_kernels(#[case] w: u32, #[case] h: u32) {
        let image = noise(w, h, 21);
        assert_eq!(
            sobel_x(&image).into_raw(),
            direct(&image, [-1, 0, 1, -2, 0, 2, -1, 0, 1])
        );
        assert_eq!(
            sobel_y(&image).into_raw(),
            direct(&image, [-1, -2, -1, 0, 0, 0, 1, 2, 1])
        );
        assert_eq!(
            laplacian(&image).into_raw(),
            direct(&image, [0, 1, 0, 1, -4, 1, 0, 1, 0])
        );
    }

    #[test]
    fn a_ramp_has_a_constant_gradient_and_no_curvature() {
        let ramp = GrayImage::from_fn(12, 6, |x, _| Luma([(x * 10) as u8]));
        assert_eq!(sobel_x(&ramp).get_pixel(5, 3).0[0], 80);
        assert_eq!(sobel_y(&ramp).get_pixel(5, 3).0[0], 0);
        assert_eq!(laplacian(&ramp).get_pixel(5, 3).0[0], 0);
    }

    #[test]
    fn scaling_turns_responses_into_pictures() {
        let image = GrayImage::from_fn(8, 8, |x, _| Luma([if x < 4 { 0 } else { 200 }]));
        let gx = sobel_x(&image);
        assert_eq!(scale_abs(&gx).get_pixel(4, 4).0[0], 255);
        assert_eq!(normalize_abs(&gx).get_pixel(4, 4).0[0], 255);
        assert_eq!(normalize_abs(&gx).get_pixel(0, 0).0[0], 0);
        let flat = GrayImage::from_pixel(4, 4, Luma([50]));
        assert!(normalize_abs(&laplacian(&flat)).iter().all(|v| *v == 0));
    }

    #[test]
    fn empty_pictures_give_empty_responses() {
        let empty = GrayImage::new(0, 0);
        assert!(
            sobel_x(&empty).is_empty()
                && sobel_y(&empty).is_empty()
                && laplacian(&empty).is_empty()
        );
    }
}

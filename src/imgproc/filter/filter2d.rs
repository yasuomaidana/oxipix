//! `filter2D`: correlation of an 8-bit image with a square `f32` kernel.
//!
//! The sum is organised by kernel tap: for each tap, one weighted copy of a shifted row is added to a row of
//! `f32` accumulators, so the inner loop is a plain element-wise multiply-add that vectorises. Pixels outside
//! the picture repeat the nearest edge pixel; results are rounded and saturated to 0..=255.

use crate::imgproc::rows::{clamp_row, for_each_band, pad_row};
use image::{ImageBuffer, Pixel};

/// Correlates every channel with the `side x side` row-major `kernel` (`side` odd).
///
/// # Panics
///
/// Panics when `side` is even or zero, or when `kernel` does not hold `side * side` values.
#[must_use]
pub fn filter2d<P>(
    image: &ImageBuffer<P, Vec<u8>>,
    kernel: &[f32],
    side: usize,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    assert!(
        side % 2 == 1 && kernel.len() == side * side,
        "the kernel must be square with an odd side"
    );
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return image.clone();
    }
    let c = usize::from(P::CHANNEL_COUNT);
    let (r, row_len, height) = (side / 2, w as usize * c, h as usize);
    let stride = row_len + 2 * r * c;
    let mut padded = Vec::with_capacity(stride * height);
    let mut row = Vec::new();
    for src in image.as_raw().chunks_exact(row_len) {
        pad_row(src, c, r, &mut row);
        padded.extend_from_slice(&row);
    }
    let mut out = vec![0u8; row_len * height];
    for_each_band(&mut out, row_len, |first, band| {
        let mut acc = vec![0.0f32; row_len];
        for (k, dst) in band.chunks_exact_mut(row_len).enumerate() {
            acc.fill(0.0);
            for (j, weight) in kernel.iter().enumerate() {
                let source = clamp_row(first + k, (j / side) as isize - r as isize, height);
                let line = &padded[source * stride + (j % side) * c..];
                acc.iter_mut()
                    .zip(line)
                    .for_each(|(a, s)| *a += weight * f32::from(*s));
            }
            dst.iter_mut()
                .zip(&acc)
                .for_each(|(d, a)| *d = a.round().clamp(0.0, 255.0) as u8);
        }
    });
    ImageBuffer::from_raw(w, h, out).expect("same length as the input buffer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::blur::box_blur;
    use crate::imgproc::testing::noise;
    use image::{GrayImage, Luma, Rgb, RgbImage};
    use rstest::rstest;

    #[test]
    fn the_identity_kernel_keeps_the_picture() {
        let image = noise(13, 9, 3);
        let identity = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(filter2d(&image, &identity, 3), image);
    }

    #[rstest]
    #[case(3)]
    #[case(5)]
    fn a_box_kernel_is_a_box_blur(#[case] side: usize) {
        let image = noise(31, 17, 8);
        let kernel = vec![1.0 / (side * side) as f32; side * side];
        let got = filter2d(&image, &kernel, side);
        let want = box_blur(&image, (side / 2) as u32);
        assert!(
            got.iter()
                .zip(want.iter())
                .all(|(a, b)| a.abs_diff(*b) <= 1)
        );
    }

    #[test]
    fn results_saturate_and_colour_channels_stay_apart() {
        let image = RgbImage::from_pixel(8, 8, Rgb([200, 100, 0]));
        let boost = [0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(filter2d(&image, &boost, 3).get_pixel(3, 3).0, [255, 200, 0]);
        let negate = [0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(filter2d(&image, &negate, 3).get_pixel(3, 3).0, [0, 0, 0]);
    }

    #[test]
    fn the_kernel_is_correlated_not_flipped() {
        let image = GrayImage::from_fn(6, 1, |x, _| Luma([(x * 10) as u8]));
        let right = [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        assert_eq!(filter2d(&image, &right, 3).get_pixel(2, 0).0[0], 30);
        assert_eq!(
            filter2d(&GrayImage::new(0, 2), &right, 3),
            GrayImage::new(0, 2)
        );
    }

    #[rstest]
    #[case(2, 4)]
    #[case(3, 8)]
    #[case(0, 0)]
    #[should_panic(expected = "odd side")]
    fn bad_kernels_panic(#[case] side: usize, #[case] len: usize) {
        let _ = filter2d(&noise(4, 4, 1), &vec![0.0; len], side);
    }
}

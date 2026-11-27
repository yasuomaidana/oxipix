//! `pyrDown`: smooth with the 1-4-6-4-1 binomial kernel and keep every second row and column.
//!
//! Only the kept rows are filtered. The vertical pass sums five rows into 16-bit lanes, the horizontal pass
//! sums five shifted copies of that row, and the result is divided by 256 once. Pixels outside the picture
//! repeat the nearest edge pixel.

use crate::imgproc::rows::{accumulate, clamp_row, pad_row};
use image::{ImageBuffer, Pixel};

/// The binomial kernel `[1, 4, 6, 4, 1] / 16`, as integers.
const KERNEL: [u16; 5] = [1, 4, 6, 4, 1];

/// Halves both sides (rounding up) after a binomial blur, like `cv::pyrDown`.
#[must_use]
pub fn pyr_down<P>(image: &ImageBuffer<P, Vec<u8>>) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return image.clone();
    }
    let channels = usize::from(P::CHANNEL_COUNT);
    let (row_len, height) = (w as usize * channels, h as usize);
    let (out_w, out_h) = (w.div_ceil(2) as usize, h.div_ceil(2) as usize);
    let mut out = Vec::with_capacity(out_w * out_h * channels);
    let (mut down, mut across) = (vec![0u16; row_len], vec![0u16; row_len]);
    let mut padded = Vec::new();
    for oy in 0..out_h {
        down.fill(0);
        for (t, weight) in KERNEL.iter().enumerate() {
            let row = clamp_row(2 * oy, t as isize - 2, height);
            accumulate(
                &mut down,
                &image.as_raw()[row * row_len..][..row_len],
                *weight,
            );
        }
        pad_row(&down, channels, 2, &mut padded);
        across.fill(0);
        for (t, weight) in KERNEL.iter().enumerate() {
            accumulate(&mut across, &padded[t * channels..], *weight);
        }
        for ox in 0..out_w {
            out.extend(
                across[2 * ox * channels..][..channels]
                    .iter()
                    .map(|v| ((v + 128) >> 8) as u8),
            );
        }
    }
    ImageBuffer::from_raw(out_w as u32, out_h as u32, out).expect("out_w * out_h * channels bytes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use image::{GrayImage, Luma, Rgb, RgbImage};
    use rstest::rstest;

    /// `[1 4 6 4 1] / 16` in both directions at the kept positions, replicated edges, in f64.
    fn reference(image: &GrayImage, x: i64, y: i64) -> f64 {
        let (w, h) = (i64::from(image.width()), i64::from(image.height()));
        let k = [1.0, 4.0, 6.0, 4.0, 1.0];
        let mut sum = 0.0;
        for (j, kj) in k.iter().enumerate() {
            for (i, ki) in k.iter().enumerate() {
                let px = (x + i as i64 - 2).clamp(0, w - 1);
                let py = (y + j as i64 - 2).clamp(0, h - 1);
                sum += kj * ki * f64::from(image.get_pixel(px as u32, py as u32).0[0]);
            }
        }
        sum / 256.0
    }

    #[rstest]
    #[case(32, 24)]
    #[case(33, 25)]
    #[case(1, 1)]
    #[case(2, 5)]
    fn sizes_halve_rounding_up_and_values_follow_the_kernel(#[case] w: u32, #[case] h: u32) {
        let image = noise(w, h, 17);
        let small = pyr_down(&image);
        assert_eq!(small.dimensions(), (w.div_ceil(2), h.div_ceil(2)));
        for (x, y, p) in small.enumerate_pixels() {
            let want = reference(&image, i64::from(2 * x), i64::from(2 * y));
            assert!(
                (f64::from(p.0[0]) - want).abs() <= 0.51,
                "({x},{y}): {} vs {want}",
                p.0[0]
            );
        }
    }

    #[test]
    fn colour_channels_are_independent_and_flat_stays_flat() {
        let rgb = RgbImage::from_pixel(10, 8, Rgb([10, 200, 77]));
        let small = pyr_down(&rgb);
        assert!(small.pixels().all(|p| p.0 == [10, 200, 77]));
        let grey = noise(14, 10, 4);
        let mixed = RgbImage::from_fn(14, 10, |x, y| Rgb([grey.get_pixel(x, y).0[0], 1, 2]));
        let red = pyr_down(&mixed);
        let want = pyr_down(&grey);
        assert!(
            red.enumerate_pixels()
                .all(|(x, y, p)| p.0[0] == want.get_pixel(x, y).0[0])
        );
    }

    #[test]
    fn an_empty_picture_is_returned_unchanged() {
        let empty = GrayImage::new(0, 5);
        assert_eq!(pyr_down(&empty), empty);
        assert_eq!(
            pyr_down(&GrayImage::from_pixel(2, 2, Luma([9])))
                .get_pixel(0, 0)
                .0[0],
            9
        );
    }
}

//! Box and Gaussian blur on 8-bit images with integer weights.
//!
//! Both are separable. Each pass sums `weight * pixel` over shifted copies of a row into 16-bit lanes (the
//! weights add up to at most 257, so a sum of 8-bit values cannot overflow) and divides once at the end.
//! Pixels outside the picture repeat the nearest edge pixel. Results differ from an exact floating-point
//! blur by at most 1 level, because the intermediate image is rounded to 8 bits as well.

use crate::imgproc::rows::{accumulate, clamp_row, pad_row};
use image::{ImageBuffer, Pixel};

/// Largest radius whose weights still fit a 16-bit sum of 8-bit values.
const MAX_RADIUS: u32 = 128;

/// Integer Gaussian weights for `2 * radius + 1` taps that add up to exactly 256.
fn gaussian_weights(sigma: f32, radius: u32) -> Vec<u16> {
    let raw: Vec<f32> = (0..=2 * radius)
        .map(|i| {
            let d = i as f32 - radius as f32;
            (-d * d / (2.0 * sigma * sigma)).exp()
        })
        .collect();
    let total: f32 = raw.iter().sum();
    let mut weights: Vec<u16> = raw
        .iter()
        .map(|w| (w / total * 256.0).round() as u16)
        .collect();
    let drift = 256 - weights.iter().map(|w| i32::from(*w)).sum::<i32>();
    let middle = weights.len() / 2;
    weights[middle] = (i32::from(weights[middle]) + drift) as u16;
    weights
}

/// Weighted sum of shifted row slices into `acc`, then `out = round(acc / total)`.
fn finish(acc: &[u16], inverse: f32, out: &mut [u8]) {
    out.iter_mut()
        .zip(acc)
        .for_each(|(o, a)| *o = (f32::from(*a) * inverse + 0.5) as u8);
}

/// Runs the horizontal then the vertical pass with the same integer `weights`.
fn separable<P>(image: &ImageBuffer<P, Vec<u8>>, weights: &[u16]) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return image.clone();
    }
    let channels = usize::from(P::CHANNEL_COUNT);
    let (row_len, height, radius) = (w as usize * channels, h as usize, weights.len() / 2);
    let inverse = 1.0 / weights.iter().map(|w| f32::from(*w)).sum::<f32>();
    let mut across = vec![0u8; row_len * height];
    let (mut padded, mut acc) = (Vec::new(), vec![0u16; row_len]);
    for (src, dst) in image
        .as_raw()
        .chunks_exact(row_len)
        .zip(across.chunks_exact_mut(row_len))
    {
        pad_row(src, channels, radius, &mut padded);
        acc.fill(0);
        for (t, weight) in weights.iter().enumerate() {
            accumulate(&mut acc, &padded[t * channels..], *weight);
        }
        finish(&acc, inverse, dst);
    }
    let mut out = vec![0u8; row_len * height];
    for (y, dst) in out.chunks_exact_mut(row_len).enumerate() {
        acc.fill(0);
        for (t, weight) in weights.iter().enumerate() {
            let row = clamp_row(y, t as isize - radius as isize, height);
            accumulate(&mut acc, &across[row * row_len..][..row_len], *weight);
        }
        finish(&acc, inverse, dst);
    }
    ImageBuffer::from_raw(w, h, out).expect("same length as the input buffer")
}

/// Mean over the `(2 * radius + 1)` square window (`cv::blur`), applied to every channel.
///
/// # Panics
///
/// Panics when `radius` is larger than 128.
#[must_use]
pub fn box_blur<P>(image: &ImageBuffer<P, Vec<u8>>, radius: u32) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    assert!(
        radius <= MAX_RADIUS,
        "the radius must be at most {MAX_RADIUS}"
    );
    separable(image, &vec![1u16; 2 * radius as usize + 1])
}

/// Gaussian blur with spread `sigma` and `radius` taps on each side.
///
/// # Panics
///
/// Panics when `sigma` is not positive or `radius` is larger than 128.
#[must_use]
pub fn gaussian_blur_radius<P>(
    image: &ImageBuffer<P, Vec<u8>>,
    sigma: f32,
    radius: u32,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    assert!(sigma > 0.0, "sigma must be positive");
    assert!(
        radius <= MAX_RADIUS,
        "the radius must be at most {MAX_RADIUS}"
    );
    separable(image, &gaussian_weights(sigma, radius))
}

/// Gaussian blur with spread `sigma`, using `ceil(2 * sigma)` taps on each side like `imageproc`.
///
/// # Panics
///
/// Panics when `sigma` is not positive or larger than 64.
#[must_use]
pub fn gaussian_blur<P>(image: &ImageBuffer<P, Vec<u8>>, sigma: f32) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    gaussian_blur_radius(image, sigma, (2.0 * sigma).ceil().max(1.0) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use image::{GrayImage, Luma, Rgb, RgbImage};
    use rstest::rstest;

    /// Exact blur in f64 with replicated edges and the given real-valued taps.
    fn reference(image: &GrayImage, taps: &[f64]) -> Vec<f64> {
        let (w, h) = (i64::from(image.width()), i64::from(image.height()));
        let r = taps.len() as i64 / 2;
        let at = |x: i64, y: i64| {
            f64::from(
                image
                    .get_pixel(x.clamp(0, w - 1) as u32, y.clamp(0, h - 1) as u32)
                    .0[0],
            )
        };
        let across: Vec<f64> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                taps.iter()
                    .enumerate()
                    .map(|(t, k)| k * at(x + t as i64 - r, y))
                    .sum()
            })
            .collect();
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .map(|(x, y)| {
                taps.iter()
                    .enumerate()
                    .map(|(t, k)| k * across[((y + t as i64 - r).clamp(0, h - 1) * w + x) as usize])
                    .sum()
            })
            .collect()
    }

    fn close(got: &GrayImage, want: &[f64], tolerance: f64) {
        for (g, w) in got.as_raw().iter().zip(want) {
            assert!((f64::from(*g) - w).abs() <= tolerance, "{g} vs {w}");
        }
    }

    #[rstest]
    #[case(1)]
    #[case(2)]
    #[case(5)]
    fn box_blur_is_the_window_mean(#[case] radius: u32) {
        let image = noise(29, 17, 5);
        let taps = vec![1.0 / f64::from(2 * radius + 1); 2 * radius as usize + 1];
        close(&box_blur(&image, radius), &reference(&image, &taps), 1.01);
    }

    #[rstest]
    #[case(0.8, 2)]
    #[case(1.1, 2)]
    #[case(1.4, 3)]
    #[case(3.0, 6)]
    fn gaussian_blur_follows_the_exact_kernel(#[case] sigma: f32, #[case] radius: u32) {
        let image = noise(33, 21, 9);
        let weights = gaussian_weights(sigma, radius);
        assert_eq!(weights.iter().map(|w| u32::from(*w)).sum::<u32>(), 256);
        let taps: Vec<f64> = weights.iter().map(|w| f64::from(*w) / 256.0).collect();
        close(
            &gaussian_blur_radius(&image, sigma, radius),
            &reference(&image, &taps),
            1.01,
        );
        assert_eq!(
            gaussian_blur(&image, sigma),
            gaussian_blur_radius(&image, sigma, (2.0 * sigma).ceil() as u32)
        );
    }

    #[test]
    fn colour_pictures_are_blurred_per_channel() {
        let grey = noise(19, 11, 3);
        let rgb = RgbImage::from_fn(19, 11, |x, y| {
            let v = grey.get_pixel(x, y).0[0];
            Rgb([v, 255 - v, 7])
        });
        let blurred = box_blur(&rgb, 2);
        let red = GrayImage::from_fn(19, 11, |x, y| Luma([blurred.get_pixel(x, y).0[0]]));
        assert_eq!(red, box_blur(&grey, 2));
        assert!(blurred.pixels().all(|p| p.0[2] == 7));
    }

    #[test]
    fn flat_pictures_stay_flat_and_empty_ones_are_returned() {
        let flat = GrayImage::from_pixel(9, 9, Luma([123]));
        assert_eq!(gaussian_blur(&flat, 1.5), flat);
        assert_eq!(box_blur(&flat, 128), flat);
        let empty = GrayImage::new(0, 3);
        assert_eq!(box_blur(&empty, 2), empty);
    }

    #[rstest]
    #[case(0.0, 1)]
    #[case(-1.0, 1)]
    #[case(1.0, 129)]
    #[should_panic]
    fn invalid_parameters_panic(#[case] sigma: f32, #[case] radius: u32) {
        let _ = gaussian_blur_radius(&GrayImage::new(4, 4), sigma, radius);
    }

    #[test]
    #[should_panic(expected = "at most")]
    fn a_huge_box_radius_panics() {
        let _ = box_blur(&GrayImage::new(4, 4), 129);
    }
}

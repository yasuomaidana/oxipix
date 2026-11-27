//! Bilateral filter with precomputed weights, as in OpenCV.
//!
//! Each output pixel is the average of the neighbours inside a disc, weighted by the distance in the picture
//! (a table built once from `sigma_space`) and by how different the neighbour looks: the sum of absolute
//! channel differences indexes a 256-entry-per-channel table built from `sigma_color`, so a tap costs two
//! table reads and a few multiply-adds instead of an `exp`. Pixels outside the picture repeat the nearest
//! edge pixel.

use super::rows::pad_row;
use image::{ImageBuffer, Pixel};

/// A neighbour offset inside the window with its spatial weight.
struct Tap {
    dx: usize,
    dy: usize,
    weight: f32,
}

/// The taps inside the disc of `radius` (offsets are from the window's top-left corner).
fn taps(radius: usize, sigma_space: f32) -> Vec<Tap> {
    let side = 2 * radius as isize + 1;
    let coefficient = -0.5 / (sigma_space * sigma_space);
    (0..side)
        .flat_map(|y| (0..side).map(move |x| (x, y)))
        .filter_map(|(x, y)| {
            let (dx, dy) = ((x - radius as isize) as f32, (y - radius as isize) as f32);
            let distance2 = dx * dx + dy * dy;
            (distance2 <= (radius * radius) as f32).then(|| Tap {
                dx: x as usize,
                dy: y as usize,
                weight: (distance2 * coefficient).exp(),
            })
        })
        .collect()
}

/// `weights[a] = exp(-a^2 / (2 sigma^2))` for every possible sum `a` of channel differences.
fn color_weights(channels: usize, sigma_color: f32) -> Vec<f32> {
    let coefficient = -0.5 / (sigma_color * sigma_color);
    (0..=255 * channels)
        .map(|a| ((a * a) as f32 * coefficient).exp())
        .collect()
}

/// Edge-preserving smoothing: `radius` is the window half-size, `sigma_color` says how different two pixels
/// may be and still mix, `sigma_space` how far apart.
///
/// # Panics
///
/// Panics when a sigma is not positive.
#[must_use]
pub fn bilateral_filter<P>(
    image: &ImageBuffer<P, Vec<u8>>,
    radius: u32,
    sigma_color: f32,
    sigma_space: f32,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    assert!(
        sigma_color > 0.0 && sigma_space > 0.0,
        "the sigmas must be positive"
    );
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 || radius == 0 {
        return image.clone();
    }
    let c = usize::from(P::CHANNEL_COUNT);
    let (r, width, height) = (radius as usize, w as usize, h as usize);
    let stride = (width + 2 * r) * c;
    let mut padded: Vec<u8> = Vec::with_capacity(stride * (height + 2 * r));
    let mut row = Vec::new();
    for y in 0..height + 2 * r {
        let src = &image.as_raw()[y.saturating_sub(r).min(height - 1) * width * c..][..width * c];
        pad_row(src, c, r, &mut row);
        padded.extend_from_slice(&row);
    }
    let (spatial, colour) = (taps(r, sigma_space), color_weights(c, sigma_color));
    let mut out = Vec::with_capacity(width * height * c);
    for y in 0..height {
        for x in 0..width {
            let centre = &padded[(y + r) * stride + (x + r) * c..][..c];
            let (mut sum, mut total) = ([0.0f32; 4], 0.0f32);
            for tap in &spatial {
                let at = &padded[(y + tap.dy) * stride + (x + tap.dx) * c..][..c];
                let difference: usize = at
                    .iter()
                    .zip(centre)
                    .map(|(a, b)| usize::from(a.abs_diff(*b)))
                    .sum();
                let weight = tap.weight * colour[difference];
                total += weight;
                at.iter()
                    .zip(&mut sum)
                    .for_each(|(a, s)| *s += weight * f32::from(*a));
            }
            out.extend(sum[..c].iter().map(|s| (s / total + 0.5) as u8));
        }
    }
    ImageBuffer::from_raw(w, h, out).expect("same length as the input buffer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use image::{GrayImage, Luma, Rgb, RgbImage};
    use rstest::rstest;

    #[test]
    fn flat_regions_stay_flat_and_steps_stay_sharp() {
        let step = GrayImage::from_fn(30, 12, |x, _| Luma([if x < 15 { 40 } else { 220 }]));
        let filtered = bilateral_filter(&step, 3, 20.0, 3.0);
        assert_eq!(filtered, step);
    }

    #[test]
    fn noise_is_smoothed_inside_a_region() {
        let noisy = GrayImage::from_fn(40, 40, |x, y| {
            Luma([(120 + noise(40, 40, 6).get_pixel(x, y).0[0] / 16) as u8])
        });
        let filtered = bilateral_filter(&noisy, 2, 30.0, 2.0);
        let spread = |image: &GrayImage| {
            let (lo, hi) = image
                .iter()
                .fold((255u8, 0u8), |(l, h), v| (l.min(*v), h.max(*v)));
            hi - lo
        };
        assert!(spread(&filtered) < spread(&noisy));
    }

    #[test]
    fn colour_pictures_mix_only_similar_pixels() {
        let image = RgbImage::from_fn(24, 10, |x, _| {
            if x < 12 {
                Rgb([200, 10, 10])
            } else {
                Rgb([10, 10, 200])
            }
        });
        assert_eq!(bilateral_filter(&image, 2, 15.0, 2.0), image);
    }

    #[test]
    fn matches_the_definition_for_one_pixel() {
        let image = noise(9, 9, 12);
        let (radius, sc, ss) = (2usize, 25.0f32, 1.5f32);
        let filtered = bilateral_filter(&image, radius as u32, sc, ss);
        let (cx, cy) = (4i64, 4i64);
        let centre = f32::from(image.get_pixel(4, 4).0[0]);
        let (mut num, mut den) = (0.0f32, 0.0f32);
        for dy in -(radius as i64)..=radius as i64 {
            for dx in -(radius as i64)..=radius as i64 {
                let d2 = (dx * dx + dy * dy) as f32;
                if d2 > (radius * radius) as f32 {
                    continue;
                }
                let v = f32::from(image.get_pixel((cx + dx) as u32, (cy + dy) as u32).0[0]);
                let weight =
                    (-0.5 * d2 / (ss * ss)).exp() * (-0.5 * (v - centre).powi(2) / (sc * sc)).exp();
                num += weight * v;
                den += weight;
            }
        }
        assert_eq!(filtered.get_pixel(4, 4).0[0], (num / den + 0.5) as u8);
    }

    #[rstest]
    #[case(0, 10.0, 1.0)]
    fn a_zero_radius_or_empty_picture_is_returned_as_it_is(
        #[case] radius: u32,
        #[case] sc: f32,
        #[case] ss: f32,
    ) {
        let image = noise(5, 5, 1);
        assert_eq!(bilateral_filter(&image, radius, sc, ss), image);
        let empty = GrayImage::new(0, 4);
        assert_eq!(bilateral_filter(&empty, 2, sc, ss), empty);
    }

    #[rstest]
    #[case(0.0, 1.0)]
    #[case(1.0, 0.0)]
    #[should_panic(expected = "positive")]
    fn non_positive_sigmas_panic(#[case] sc: f32, #[case] ss: f32) {
        let _ = bilateral_filter(&noise(4, 4, 1), 1, sc, ss);
    }
}

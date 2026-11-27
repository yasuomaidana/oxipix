//! Median filter on 8-bit images with a square window.
//!
//! Windows of 3x3 and 5x5 run a selection network: the nine (or 25) neighbours of 16 neighbouring pixels
//! are held in byte lanes, and `n / 2 + 1` bubble passes of element-wise `min` / `max` move the median into
//! place, which compiles to byte-wise vector instructions. Larger windows fall back to selecting the middle
//! value of each window. Pixels outside the picture repeat the nearest edge pixel.

use crate::imgproc::rows::for_each_band;
use image::{ImageBuffer, Pixel};

/// Pixels a network step handles at once.
const LANES: usize = 16;

type Lane = [u8; LANES];

/// `values[a] = min(values[a], values[b])` and `values[b] = max(..)` for every lane, with literal indices so
/// the lanes stay in vector registers.
#[inline(always)]
fn exchange<const N: usize>(values: &mut [Lane; N], a: usize, b: usize) {
    let (x, y) = (values[a], values[b]);
    values[a] = std::array::from_fn(|i| x[i].min(y[i]));
    values[b] = std::array::from_fn(|i| x[i].max(y[i]));
}

/// Moves the median of the nine values of every lane to index 4.
///
/// Pass `p` of the network bubbles the largest of the first `n - p` values to the end, so after `n / 2 + 1`
/// passes the `(n / 2 + 1)`-th largest value, the median, sits at index `n / 2`.
fn select_median9(values: &mut [Lane; 9]) {
    network9!(values, exchange);
}

/// Moves the median of the 25 values of every lane to index 12 (same rule as [`select_median9`]).
fn select_median25(values: &mut [Lane; 25]) {
    network25!(values, exchange);
}

/// The image plane copied with `radius` rows of repeated edge pixels above and below, and `radius` columns
/// on the left and `radius + extra` on the right.
fn pad(plane: &[u8], size: (usize, usize), radius: usize, extra: usize) -> Vec<u8> {
    let (width, height) = size;
    let mut out = Vec::with_capacity((width + 2 * radius + extra) * (height + 2 * radius));
    for y in 0..height + 2 * radius {
        let row = &plane[y.saturating_sub(radius).min(height - 1) * width..][..width];
        out.extend(std::iter::repeat_n(row[0], radius));
        out.extend_from_slice(row);
        out.extend(std::iter::repeat_n(row[width - 1], radius + extra));
    }
    out
}

/// Median of one plane with the selection network, `N = (2 * radius + 1)^2` values per pixel. The padded
/// rows are a whole number of lane groups wide, so every step is one fixed-size load per neighbour.
fn network_plane<const N: usize>(
    plane: &[u8],
    size: (usize, usize),
    radius: usize,
    select: fn(&mut [Lane; N]),
) -> Vec<u8> {
    let (width, height) = size;
    let wide = width.next_multiple_of(LANES);
    let padded = pad(plane, size, radius, wide - width);
    let stride = wide + 2 * radius;
    let side = 2 * radius + 1;
    let mut out = vec![0u8; wide * height];
    for_each_band(&mut out, wide, |first, band| {
        for (k, row) in band.chunks_exact_mut(wide).enumerate() {
            let y = first + k;
            for x0 in (0..wide).step_by(LANES) {
                let mut values = [[0u8; LANES]; N];
                for (j, lane) in values.iter_mut().enumerate() {
                    let start = (y + j / side) * stride + x0 + j % side;
                    lane.copy_from_slice(&padded[start..start + LANES]);
                }
                select(&mut values);
                row[x0..][..LANES].copy_from_slice(&values[N / 2]);
            }
        }
    });
    out.chunks_exact(wide)
        .flat_map(|row| &row[..width])
        .copied()
        .collect()
}

/// Median of one plane by selecting the middle value of each window (any radius).
fn select_plane(padded: &[u8], size: (usize, usize), radius: usize) -> Vec<u8> {
    let (width, height) = size;
    let stride = width + 2 * radius;
    let side = 2 * radius + 1;
    let mut window = vec![0u8; side * side];
    let mut out = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            for (dy, row) in window.chunks_exact_mut(side).enumerate() {
                row.copy_from_slice(&padded[(y + dy) * stride + x..][..side]);
            }
            out.push(*window.select_nth_unstable(side * side / 2).1);
        }
    }
    out
}

/// Median filter over the `(2 * radius + 1)` square window, applied to every channel on its own.
#[must_use]
pub fn median_blur<P>(image: &ImageBuffer<P, Vec<u8>>, radius: u32) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    let (w, h) = image.dimensions();
    if radius == 0 || w == 0 || h == 0 {
        return image.clone();
    }
    let channels = usize::from(P::CHANNEL_COUNT);
    let (width, height, r) = (w as usize, h as usize, radius as usize);
    let mut out = vec![0u8; image.as_raw().len()];
    for c in 0..channels {
        let plane: Vec<u8> = image
            .as_raw()
            .iter()
            .skip(c)
            .step_by(channels)
            .copied()
            .collect();
        let size = (width, height);
        let filtered = match r {
            1 => network_plane(&plane, size, 1, select_median9),
            2 => network_plane(&plane, size, 2, select_median25),
            _ => select_plane(&pad(&plane, size, r, 0), size, r),
        };
        for (slot, value) in out.iter_mut().skip(c).step_by(channels).zip(filtered) {
            *slot = value;
        }
    }
    ImageBuffer::from_raw(w, h, out).expect("same length as the input buffer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise as seeded;
    use image::{GrayImage, Luma, Rgb, RgbImage};
    use rstest::rstest;

    fn noise(w: u32, h: u32) -> GrayImage {
        seeded(w, h, 0xC0FF_EE11)
    }

    /// Sorted window with replicated edges.
    fn naive(image: &GrayImage, radius: i64) -> GrayImage {
        let (w, h) = (i64::from(image.width()), i64::from(image.height()));
        GrayImage::from_fn(image.width(), image.height(), |x, y| {
            let mut window: Vec<u8> = (-radius..=radius)
                .flat_map(|dy| (-radius..=radius).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| {
                    let (u, v) = (
                        (i64::from(x) + dx).clamp(0, w - 1),
                        (i64::from(y) + dy).clamp(0, h - 1),
                    );
                    image.get_pixel(u as u32, v as u32).0[0]
                })
                .collect();
            window.sort_unstable();
            Luma([window[window.len() / 2]])
        })
    }

    #[rstest]
    #[case(20, 9, 1)]
    #[case(33, 17, 1)]
    #[case(16, 16, 2)]
    #[case(37, 11, 2)]
    #[case(13, 9, 3)]
    #[case(1, 1, 1)]
    #[case(2, 3, 2)]
    fn the_median_equals_the_sorted_window(#[case] w: u32, #[case] h: u32, #[case] radius: u32) {
        let image = noise(w, h);
        assert_eq!(
            median_blur(&image, radius),
            naive(&image, i64::from(radius))
        );
    }

    #[test]
    fn colour_images_are_filtered_per_channel() {
        let grey = noise(21, 14);
        let rgb = RgbImage::from_fn(21, 14, |x, y| {
            let v = grey.get_pixel(x, y).0[0];
            Rgb([v, v.wrapping_add(80), 255 - v])
        });
        let filtered = median_blur(&rgb, 2);
        let red = GrayImage::from_fn(21, 14, |x, y| Luma([filtered.get_pixel(x, y).0[0]]));
        assert_eq!(red, naive(&grey, 2));
    }

    #[rstest]
    #[case(0, 5, 5)]
    #[case(2, 0, 5)]
    #[case(2, 5, 0)]
    fn a_zero_radius_or_empty_picture_is_returned_as_it_is(
        #[case] radius: u32,
        #[case] w: u32,
        #[case] h: u32,
    ) {
        let image = noise(w, h);
        assert_eq!(median_blur(&image, radius), image);
    }
}

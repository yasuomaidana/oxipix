//! Histograms, Otsu's level, binary thresholds and a fused HSV range test.

use image::{GrayImage, RgbImage};

/// Count of every grey level, built from four interleaved sub-histograms so consecutive pixels do not wait
/// on each other's increments.
#[must_use]
pub fn histogram(data: &[u8]) -> [u32; 256] {
    let mut parts = [[0u32; 256]; 4];
    let (chunks, rest) = data.as_chunks::<4>();
    for chunk in chunks {
        for (part, value) in parts.iter_mut().zip(chunk) {
            part[usize::from(*value)] += 1;
        }
    }
    for value in rest {
        parts[0][usize::from(*value)] += 1;
    }
    std::array::from_fn(|i| parts.iter().map(|p| p[i]).sum())
}

/// Otsu's level: the one that maximises the between-class variance of the grey levels.
#[must_use]
pub fn otsu_level(image: &GrayImage) -> u8 {
    let counts = histogram(image.as_raw());
    let total = image.as_raw().len() as f64;
    let sum_all: f64 = counts
        .iter()
        .enumerate()
        .map(|(i, c)| i as f64 * f64::from(*c))
        .sum();
    let (mut below, mut sum_below, mut best, mut level) = (0.0f64, 0.0f64, -1.0f64, 0u8);
    for (i, count) in counts.iter().enumerate() {
        below += f64::from(*count);
        sum_below += i as f64 * f64::from(*count);
        let above = total - below;
        if below == 0.0 || above == 0.0 {
            continue;
        }
        let (m_below, m_above) = (sum_below / below, (sum_all - sum_below) / above);
        let variance = below * above * (m_below - m_above).powi(2);
        if variance > best {
            (best, level) = (variance, i as u8);
        }
    }
    level
}

/// 255 where the pixel is above `level`, 0 elsewhere.
#[must_use]
pub fn threshold_binary(image: &GrayImage, level: u8) -> GrayImage {
    let data = image
        .as_raw()
        .iter()
        .map(|p| if *p > level { 255 } else { 0 })
        .collect();
    GrayImage::from_raw(image.width(), image.height(), data)
        .expect("same length as the input buffer")
}

/// A range in HSV space with OpenCV's 8-bit full-range hue (0 to 255 around the circle).
///
/// A range with `hue_low > hue_high` wraps through 0, as red does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HsvRange {
    /// First hue of the range.
    pub hue_low: u8,
    /// Last hue of the range.
    pub hue_high: u8,
    /// Smallest saturation.
    pub saturation: u8,
    /// Smallest value.
    pub value: u8,
}

/// Hue (0..=255 around the circle), saturation and value of an RGB pixel, in integers.
#[must_use]
pub fn rgb_to_hsv_full(r: u8, g: u8, b: u8) -> (u8, u8, u8) {
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
    let (high, low) = (r.max(g).max(b), r.min(g).min(b));
    let spread = high - low;
    if spread == 0 {
        return (0, 0, high as u8);
    }
    let sector = if high == r {
        (g - b) * 256 / (6 * spread)
    } else if high == g {
        (b - r) * 256 / (6 * spread) + 256 / 3
    } else {
        (r - g) * 256 / (6 * spread) + 512 / 3
    };
    (
        sector.rem_euclid(256) as u8,
        (spread * 255 / high) as u8,
        high as u8,
    )
}

/// 255 where the pixel's HSV colour is inside any of `ranges` (hue within the range, at least the saturation
/// and value), computed in one pass without an intermediate HSV image (`cvtColor` + `inRange` + `bitwise_or`).
#[must_use]
pub fn in_range_hsv(image: &RgbImage, ranges: &[HsvRange]) -> GrayImage {
    let inside = |hue: u8, saturation: u8, value: u8| {
        ranges.iter().any(|r| {
            let in_hue = if r.hue_low <= r.hue_high {
                (r.hue_low..=r.hue_high).contains(&hue)
            } else {
                hue >= r.hue_low || hue <= r.hue_high
            };
            in_hue && saturation >= r.saturation && value >= r.value
        })
    };
    let data = image
        .as_raw()
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| {
            let (h, s, v) = rgb_to_hsv_full(p[0], p[1], p[2]);
            if inside(h, s, v) { 255 } else { 0 }
        })
        .collect();
    GrayImage::from_raw(image.width(), image.height(), data).expect("one byte per pixel")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Luma, Rgb};
    use rstest::rstest;

    #[rstest]
    #[case(0)]
    #[case(1)]
    #[case(3)]
    #[case(1000)]
    fn the_histogram_counts_every_value(#[case] len: usize) {
        let data: Vec<u8> = (0..len).map(|i| (i * 7 % 13) as u8).collect();
        let counts = histogram(&data);
        assert_eq!(counts.iter().sum::<u32>() as usize, len);
        for v in 0..13u8 {
            assert_eq!(
                counts[usize::from(v)] as usize,
                data.iter().filter(|d| **d == v).count()
            );
        }
    }

    #[test]
    fn otsu_splits_two_populations_between_them() {
        let image = GrayImage::from_fn(40, 10, |x, _| Luma([if x < 20 { 30 } else { 200 }]));
        let level = otsu_level(&image);
        assert!((30..200).contains(&level), "{level}");
        let mask = threshold_binary(&image, level);
        assert_eq!(
            (mask.get_pixel(0, 0).0[0], mask.get_pixel(39, 0).0[0]),
            (0, 255)
        );
        assert_eq!(otsu_level(&GrayImage::from_pixel(4, 4, Luma([9]))), 0);
    }

    #[rstest]
    #[case([255, 0, 0], (0, 255, 255))]
    #[case([0, 255, 0], (85, 255, 255))]
    #[case([0, 0, 255], (170, 255, 255))]
    #[case([128, 128, 128], (0, 0, 128))]
    #[case([0, 0, 0], (0, 0, 0))]
    fn hsv_matches_the_primary_colours(#[case] rgb: [u8; 3], #[case] want: (u8, u8, u8)) {
        assert_eq!(rgb_to_hsv_full(rgb[0], rgb[1], rgb[2]), want);
    }

    #[test]
    fn red_wraps_through_zero_and_other_hues_are_excluded() {
        let image = RgbImage::from_fn(3, 1, |x, _| match x {
            0 => Rgb([250, 5, 5]),
            1 => Rgb([250, 5, 40]),
            _ => Rgb([5, 5, 250]),
        });
        let red = [
            HsvRange {
                hue_low: 244,
                hue_high: 255,
                saturation: 128,
                value: 102,
            },
            HsvRange {
                hue_low: 0,
                hue_high: 11,
                saturation: 128,
                value: 102,
            },
        ];
        let mask = in_range_hsv(&image, &red);
        assert_eq!(mask.as_raw(), &[255, 255, 0]);
        let wrapping = HsvRange {
            hue_low: 244,
            hue_high: 11,
            saturation: 128,
            value: 102,
        };
        assert_eq!(in_range_hsv(&image, &[wrapping]).as_raw(), &[255, 255, 0]);
        let dark = HsvRange {
            hue_low: 0,
            hue_high: 11,
            saturation: 0,
            value: 255,
        };
        assert!(in_range_hsv(&image, &[dark]).iter().all(|v| *v == 0));
    }
}

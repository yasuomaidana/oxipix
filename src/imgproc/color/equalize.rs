//! Histogram equalisation of 8-bit grey images (`cv::equalizeHist`).

use super::threshold::histogram;
use image::GrayImage;

/// Spreads the grey levels so their cumulative distribution is as close to linear as possible:
/// `lut[i] = round((cdf(i) - cdf_min) / (n - cdf_min) * 255)`. A flat picture is returned as it is.
#[must_use]
pub fn equalize_hist(image: &GrayImage) -> GrayImage {
    let counts = histogram(image.as_raw());
    let total = image.as_raw().len() as u64;
    let first = counts.iter().copied().find(|c| *c > 0).map_or(0, u64::from);
    if first == total {
        return image.clone();
    }
    let mut running = 0u64;
    let lut: Vec<u8> = counts
        .iter()
        .map(|c| {
            running += u64::from(*c);
            (running.saturating_sub(first) as f64 / (total - first) as f64 * 255.0).round() as u8
        })
        .collect();
    let data = image
        .as_raw()
        .iter()
        .map(|p| lut[usize::from(*p)])
        .collect();
    GrayImage::from_raw(image.width(), image.height(), data)
        .expect("same length as the input buffer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn a_two_level_picture_is_stretched_to_the_full_range() {
        let image = GrayImage::from_fn(10, 10, |x, _| Luma([if x < 5 { 100 } else { 110 }]));
        let out = equalize_hist(&image);
        assert_eq!(
            (out.get_pixel(0, 0).0[0], out.get_pixel(9, 0).0[0]),
            (0, 255)
        );
    }

    #[test]
    fn levels_keep_their_order_and_flat_pictures_stay_flat() {
        let ramp = GrayImage::from_fn(64, 4, |x, _| Luma([(x * 2) as u8]));
        let out = equalize_hist(&ramp);
        assert!(out.as_raw().windows(2).take(63).all(|w| w[0] <= w[1]));
        assert_eq!(out.get_pixel(63, 0).0[0], 255);
        let flat = GrayImage::from_pixel(5, 5, Luma([77]));
        assert_eq!(equalize_hist(&flat), flat);
        assert_eq!(equalize_hist(&GrayImage::new(0, 0)), GrayImage::new(0, 0));
    }
}

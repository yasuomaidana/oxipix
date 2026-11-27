//! Saturating per-element arithmetic on 8-bit buffers: `addWeighted` and `convertTo` with scale and shift.
//!
//! Both are single passes over the raw buffers in `f32`, which the compiler turns into widening loads, vector
//! multiply-adds and a saturating narrow.

use image::{ImageBuffer, Pixel};

/// `round(x)` saturated to 0..=255.
#[inline]
fn narrow(x: f32) -> u8 {
    x.clamp(0.0, 255.0).round() as u8
}

/// `alpha * a + beta * b + gamma` per element, saturated (`cv::addWeighted`); `None` when the sizes differ.
#[must_use]
pub fn add_weighted<P>(
    a: &ImageBuffer<P, Vec<u8>>,
    b: &ImageBuffer<P, Vec<u8>>,
    (alpha, beta, gamma): (f32, f32, f32),
) -> Option<ImageBuffer<P, Vec<u8>>>
where
    P: Pixel<Subpixel = u8>,
{
    if a.dimensions() != b.dimensions() {
        return None;
    }
    let data = a
        .as_raw()
        .iter()
        .zip(b.as_raw())
        .map(|(x, y)| narrow(f32::from(*x) * alpha + f32::from(*y) * beta + gamma))
        .collect();
    ImageBuffer::from_raw(a.width(), a.height(), data)
}

/// `alpha * x + beta` per element, saturated (`Mat::convertTo(dst, -1, alpha, beta)`).
#[must_use]
pub fn convert_scale<P>(
    image: &ImageBuffer<P, Vec<u8>>,
    alpha: f32,
    beta: f32,
) -> ImageBuffer<P, Vec<u8>>
where
    P: Pixel<Subpixel = u8>,
{
    let data = image
        .as_raw()
        .iter()
        .map(|x| narrow(f32::from(*x) * alpha + beta))
        .collect();
    ImageBuffer::from_raw(image.width(), image.height(), data)
        .expect("same length as the input buffer")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use image::{GrayImage, Luma, Rgb, RgbImage};
    use rstest::rstest;

    #[rstest]
    #[case(0.5, 0.5, 0.0)]
    #[case(1.0, 1.0, 0.0)]
    #[case(0.3, 0.9, 20.0)]
    #[case(-1.0, 2.0, 7.5)]
    fn add_weighted_follows_the_formula(#[case] alpha: f32, #[case] beta: f32, #[case] gamma: f32) {
        let (a, b) = (noise(19, 7, 1), noise(19, 7, 2));
        let sum = add_weighted(&a, &b, (alpha, beta, gamma)).unwrap();
        for ((x, y), z) in a.iter().zip(b.iter()).zip(sum.iter()) {
            let want = (f64::from(*x) * f64::from(alpha)
                + f64::from(*y) * f64::from(beta)
                + f64::from(gamma))
            .clamp(0.0, 255.0);
            assert!((f64::from(*z) - want).abs() <= 0.51, "{z} vs {want}");
        }
    }

    #[test]
    fn different_sizes_have_no_sum() {
        assert!(add_weighted(&noise(4, 4, 1), &noise(4, 5, 1), (1.0, 1.0, 0.0)).is_none());
    }

    #[test]
    fn scaling_saturates_and_keeps_channels_together() {
        let image = RgbImage::from_pixel(3, 2, Rgb([10, 100, 200]));
        assert_eq!(
            convert_scale(&image, 1.3, 40.0).get_pixel(1, 1).0,
            [53, 170, 255]
        );
        let dark = GrayImage::from_pixel(2, 2, Luma([30]));
        assert_eq!(convert_scale(&dark, 1.0, -50.0).get_pixel(0, 0).0[0], 0);
    }
}

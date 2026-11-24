//! ITU-R BT.601 luminance shared by `mosaic` and `vectorize`.

use image::RgbaImage;

/// Calculates ITU-R BT.601 perceived luminance for an RGB triple.
#[inline]
#[must_use]
pub fn rgb_to_luma_u8(r: u8, g: u8, b: u8) -> u8 {
    ((u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000) as u8
}

/// Converts an RGBA image into an in-memory grayscale byte buffer.
#[must_use]
pub fn rgba_to_luma_vec(img: &RgbaImage) -> Vec<u8> {
    img.pixels()
        .map(|p| rgb_to_luma_u8(p[0], p[1], p[2]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use rstest::rstest;

    #[rstest]
    #[case(255, 255, 255, 255)]
    #[case(0, 0, 0, 0)]
    #[case(255, 0, 0, 76)]
    #[case(0, 255, 0, 149)]
    #[case(0, 0, 255, 29)]
    fn luma_matches_bt601_weights(#[case] r: u8, #[case] g: u8, #[case] b: u8, #[case] want: u8) {
        assert_eq!(rgb_to_luma_u8(r, g, b), want);
    }

    #[test]
    fn luma_vec_has_one_byte_per_pixel_and_ignores_alpha() {
        let img = RgbaImage::from_fn(3, 2, |x, _| Rgba([255, 0, 0, (x * 90) as u8]));
        assert_eq!(rgba_to_luma_vec(&img), vec![76; 6]);
    }
}

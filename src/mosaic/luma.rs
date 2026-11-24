//! Luminance calculation and channel mapping helpers.

use image::RgbaImage;

pub use crate::luma::{rgb_to_luma_u8, rgba_to_luma_vec};

/// A per-channel transform: `(channel, plane, width, height)` to the new plane.
type ChannelMapper<'a> = dyn FnMut(usize, &[u8], usize, usize) -> Vec<u8> + 'a;

/// Runs `f` over the first `channels` channels of `img`, one planar buffer at a time.
///
/// A result whose length is not `width * height` is ignored and leaves that channel untouched.
fn map_channels(img: &mut RgbaImage, channels: usize, f: &mut ChannelMapper) {
    let (width, height) = img.dimensions();
    let (w, h) = (width as usize, height as usize);

    for c in 0..channels {
        let channel: Vec<u8> = img.pixels().map(|p| p[c]).collect();
        let processed = f(c, &channel, w, h);
        if processed.len() == w * h {
            for (pixel, value) in img.pixels_mut().zip(processed) {
                pixel[c] = value;
            }
        }
    }
}

/// Defines a public channel mapper over the first `$n` channels.
macro_rules! channel_mapper {
    ($(#[$doc:meta])* $name:ident, $n:literal) => {
        $(#[$doc])*
        pub fn $name<F>(img: &mut RgbaImage, mut f: F)
        where
            F: FnMut(usize, &[u8], usize, usize) -> Vec<u8>,
        {
            map_channels(img, $n, &mut f);
        }
    };
}

channel_mapper!(
    /// Maps each RGB color channel (indices 0..3) independently through a transforming closure.
    map_rgb_channels,
    3
);
channel_mapper!(
    /// Maps each RGBA channel (indices 0..4) independently through a transforming closure.
    map_rgba_channels,
    4
);

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn invert(_: usize, buf: &[u8], _: usize, _: usize) -> Vec<u8> {
        buf.iter().map(|v| 255 - v).collect()
    }

    type Plane = fn(usize, &[u8], usize, usize) -> Vec<u8>;

    fn inverted(map: fn(&mut RgbaImage, Plane)) -> Rgba<u8> {
        let mut img = RgbaImage::from_pixel(3, 2, Rgba([10, 20, 30, 77]));
        map(&mut img, invert);
        *img.get_pixel(1, 1)
    }

    #[test]
    fn rgb_mapping_leaves_alpha_alone() {
        assert_eq!(inverted(map_rgb_channels), Rgba([245, 235, 225, 77]));
    }

    #[test]
    fn rgba_mapping_reaches_alpha() {
        assert_eq!(inverted(map_rgba_channels), Rgba([245, 235, 225, 178]));
    }

    #[test]
    fn a_result_of_the_wrong_length_is_ignored() {
        let mut img = RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 4]));
        map_rgba_channels(&mut img, |_, _, _, _| vec![9]);
        assert_eq!(img.get_pixel(1, 1), &Rgba([1, 2, 3, 4]));
    }
}

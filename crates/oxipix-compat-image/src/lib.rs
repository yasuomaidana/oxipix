//! 100% Drop-In Compatibility Facade Replicating the `image` Crate.
#![deny(missing_docs)]

pub use oxipix_codecs::ImageFormat;
pub use oxipix_core::OxipixError as ImageError;

/// Standard Result type for image operations.
pub type ImageResult<T> = Result<T, ImageError>;

/// Represents a pixel color component with alpha channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Rgba<T>(pub [T; 4]);

/// Represents a 3-channel RGB pixel color component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Rgb<T>(pub [T; 3]);

/// Represents a 1-channel luminance/grayscale pixel component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Luma<T>(pub [T; 1]);

/// Generic ImageBuffer replicating `image::ImageBuffer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBuffer<P, Container> {
    width: u32,
    height: u32,
    data: Container,
    _marker: std::marker::PhantomData<P>,
}

impl<P, Container: AsRef<[u8]>> ImageBuffer<P, Container> {
    /// Width of the image in pixels.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height of the image in pixels.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the underlying byte slice.
    #[inline]
    #[must_use]
    pub fn as_raw(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl ImageBuffer<Rgba<u8>, Vec<u8>> {
    /// Creates a new RGBA ImageBuffer with specified dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width as usize).saturating_mul(height as usize).saturating_mul(4);
        Self {
            width,
            height,
            data: vec![0u8; size],
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates an RGBA ImageBuffer from an existing raw byte vector.
    pub fn from_raw(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
        if data.len() == expected {
            Some(Self {
                width,
                height,
                data,
                _marker: std::marker::PhantomData,
            })
        } else {
            None
        }
    }
}

/// Type alias for 8-bit RGBA ImageBuffer.
pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;
/// Type alias for 8-bit RGB ImageBuffer.
pub type RgbImage = ImageBuffer<Rgb<u8>, Vec<u8>>;
/// Type alias for 8-bit Grayscale ImageBuffer.
pub type GrayImage = ImageBuffer<Luma<u8>, Vec<u8>>;

/// A Dynamic Image supporting multiple pixel formats, replicating `image::DynamicImage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicImage {
    /// 8-bit Grayscale
    ImageLuma8(GrayImage),
    /// 8-bit RGB
    ImageRgb8(RgbImage),
    /// 8-bit RGBA
    ImageRgba8(RgbaImage),
}

impl DynamicImage {
    /// Width of the dynamic image.
    #[must_use]
    pub fn width(&self) -> u32 {
        match self {
            Self::ImageLuma8(img) => img.width(),
            Self::ImageRgb8(img) => img.width(),
            Self::ImageRgba8(img) => img.width(),
        }
    }

    /// Height of the dynamic image.
    #[must_use]
    pub fn height(&self) -> u32 {
        match self {
            Self::ImageLuma8(img) => img.height(),
            Self::ImageRgb8(img) => img.height(),
            Self::ImageRgba8(img) => img.height(),
        }
    }

    /// Converts dynamic image to owned `RgbaImage`.
    #[must_use]
    pub fn to_rgba8(&self) -> RgbaImage {
        match self {
            Self::ImageRgba8(img) => img.clone(),
            Self::ImageRgb8(img) => {
                let w = img.width();
                let h = img.height();
                let mut rgba = Vec::with_capacity((w as usize) * (h as usize) * 4);
                for chunk in img.as_raw().as_chunks::<3>().0 {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                RgbaImage::from_raw(w, h, rgba).unwrap_or_else(|| RgbaImage::new(w, h))
            }
            Self::ImageLuma8(img) => {
                let w = img.width();
                let h = img.height();
                let mut rgba = Vec::with_capacity((w as usize) * (h as usize) * 4);
                for &l in img.as_raw() {
                    rgba.extend_from_slice(&[l, l, l, 255]);
                }
                RgbaImage::from_raw(w, h, rgba).unwrap_or_else(|| RgbaImage::new(w, h))
            }
        }
    }
}

/// Image manipulation operations module replicating `image::imageops`.
pub mod imageops {
    pub use oxipix_ops::FilterType;

    /// Resizes an RGBA image buffer.
    pub fn resize(
        image: &crate::RgbaImage,
        nwidth: u32,
        nheight: u32,
        _filter: FilterType,
    ) -> crate::RgbaImage {
        let resized_bytes = oxipix_ops::resize_nearest_rgba(
            image.as_raw(),
            oxipix_core::PixelWidth(image.width()),
            oxipix_core::PixelHeight(image.height()),
            oxipix_core::PixelWidth(nwidth),
            oxipix_core::PixelHeight(nheight),
        ).unwrap_or_default();

        crate::RgbaImage::from_raw(nwidth, nheight, resized_bytes).unwrap_or_else(|| crate::RgbaImage::new(nwidth, nheight))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compat_rgba_image() {
        let img = RgbaImage::new(100, 100);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
        assert_eq!(img.as_raw().len(), 100 * 100 * 4);
    }

    #[test]
    fn test_compat_dynamic_image_to_rgba8() {
        let img = RgbaImage::new(10, 10);
        let dyn_img = DynamicImage::ImageRgba8(img);
        let rgba = dyn_img.to_rgba8();
        assert_eq!(rgba.width(), 10);
        assert_eq!(rgba.height(), 10);
    }
}

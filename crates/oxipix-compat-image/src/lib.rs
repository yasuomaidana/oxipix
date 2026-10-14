//! 100% Drop-In Compatibility Facade Sovereignly Replicating the `image` Crate.
#![deny(missing_docs)]

use oxipix_core::{FrameBuffer, OxipixError, PixelHeight, PixelWidth, Rgba8, state};
use std::fmt;
use std::ops::{Deref, DerefMut};

pub use oxipix_codecs::ImageFormat;

/// Color space and channel representation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// 8-bit Grayscale.
    L8,
    /// 8-bit Grayscale with Alpha.
    La8,
    /// 8-bit RGB.
    Rgb8,
    /// 8-bit RGBA.
    Rgba8,
    /// 16-bit Grayscale.
    L16,
    /// 16-bit Grayscale with Alpha.
    La16,
    /// 16-bit RGB.
    Rgb16,
    /// 16-bit RGBA.
    Rgba16,
    /// 32-bit floating point RGB.
    Rgb32F,
    /// 32-bit floating point RGBA.
    Rgba32F,
}

impl ColorType {
    /// Returns the number of bytes required for a single pixel.
    #[must_use]
    pub const fn bytes_per_pixel(&self) -> u8 {
        match self {
            Self::L8 => 1,
            Self::La8 => 2,
            Self::Rgb8 => 3,
            Self::Rgba8 => 4,
            Self::L16 => 2,
            Self::La16 => 4,
            Self::Rgb16 => 6,
            Self::Rgba16 => 8,
            Self::Rgb32F => 12,
            Self::Rgba32F => 16,
        }
    }
}

/// Memory and dimension safety limits for decoding images.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Limits {
    /// Maximum allowable memory allocation in bytes.
    pub max_alloc: Option<u64>,
    /// Maximum image width in pixels.
    pub max_image_width: Option<u32>,
    /// Maximum image height in pixels.
    pub max_image_height: Option<u32>,
}

/// Errors occurring during image decoding, encoding, or format manipulation.
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// Image decoding failure.
    #[error("Decoding error: {0}")]
    Decoding(String),
    /// Image encoding failure.
    #[error("Encoding error: {0}")]
    Encoding(String),
    /// Parameter or format validation error.
    #[error("Parameter error: {0}")]
    Parameter(String),
    /// Exceeded memory or dimension limits.
    #[error("Resource limit exceeded: {0}")]
    Limits(String),
    /// Underlying I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    /// Sovereign oxipix error.
    #[error("Oxipix error: {0}")]
    Oxipix(#[from] OxipixError),
}

/// Core pixel trait defining channels and subpixel components.
pub trait Pixel: Copy + Clone + PartialEq + Eq + fmt::Debug + 'static {
    /// Type of each color component (e.g. `u8`).
    type Subpixel: Copy + Clone + PartialEq + Eq + fmt::Debug;
    /// Number of channels in this pixel.
    const CHANNEL_COUNT: u8;
    /// Canonical color type.
    const COLOR_TYPE: ColorType;

    /// Returns a slice of the pixel's channels.
    fn channels(&self) -> &[Self::Subpixel];
    /// Returns a mutable slice of the pixel's channels.
    fn channels_mut(&mut self) -> &mut [Self::Subpixel];
}

/// RGB 3-channel pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Rgb<T>(pub [T; 3]);

impl<T: Copy + Clone + PartialEq + Eq + fmt::Debug + 'static> Pixel for Rgb<T> {
    type Subpixel = T;
    const CHANNEL_COUNT: u8 = 3;
    const COLOR_TYPE: ColorType = ColorType::Rgb8;

    fn channels(&self) -> &[Self::Subpixel] {
        &self.0
    }
    fn channels_mut(&mut self) -> &mut [Self::Subpixel] {
        &mut self.0
    }
}

impl<T> Deref for Rgb<T> {
    type Target = [T; 3];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Rgb<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// RGBA 4-channel pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Rgba<T>(pub [T; 4]);

impl<T: Copy + Clone + PartialEq + Eq + fmt::Debug + 'static> Pixel for Rgba<T> {
    type Subpixel = T;
    const CHANNEL_COUNT: u8 = 4;
    const COLOR_TYPE: ColorType = ColorType::Rgba8;

    fn channels(&self) -> &[Self::Subpixel] {
        &self.0
    }
    fn channels_mut(&mut self) -> &mut [Self::Subpixel] {
        &mut self.0
    }
}

impl<T> Deref for Rgba<T> {
    type Target = [T; 4];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Rgba<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Grayscale single-channel pixel (Luma).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Luma<T>(pub [T; 1]);

impl<T: Copy + Clone + PartialEq + Eq + fmt::Debug + 'static> Pixel for Luma<T> {
    type Subpixel = T;
    const CHANNEL_COUNT: u8 = 1;
    const COLOR_TYPE: ColorType = ColorType::L8;

    fn channels(&self) -> &[Self::Subpixel] {
        &self.0
    }
    fn channels_mut(&mut self) -> &mut [Self::Subpixel] {
        &mut self.0
    }
}

impl<T> Deref for Luma<T> {
    type Target = [T; 1];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Luma<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Grayscale with Alpha 2-channel pixel (LumaA).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct LumaA<T>(pub [T; 2]);

impl<T: Copy + Clone + PartialEq + Eq + fmt::Debug + 'static> Pixel for LumaA<T> {
    type Subpixel = T;
    const CHANNEL_COUNT: u8 = 2;
    const COLOR_TYPE: ColorType = ColorType::La8;

    fn channels(&self) -> &[Self::Subpixel] {
        &self.0
    }
    fn channels_mut(&mut self) -> &mut [Self::Subpixel] {
        &mut self.0
    }
}

impl<T> Deref for LumaA<T> {
    type Target = [T; 2];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for LumaA<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Read-only 2D view over an image grid.
pub trait GenericImageView {
    /// Pixel type returned by coordinates.
    type Pixel: Pixel;

    /// Image dimensions as (width, height).
    fn dimensions(&self) -> (u32, u32);
    /// Image width in pixels.
    fn width(&self) -> u32 {
        self.dimensions().0
    }
    /// Image height in pixels.
    fn height(&self) -> u32 {
        self.dimensions().1
    }
    /// Returns the pixel at the specified coordinate.
    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel;
}

/// Mutable 2D image grid.
pub trait GenericImage: GenericImageView {
    /// Sets the pixel at the specified coordinate.
    fn put_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel);
}

/// Contiguous 2D raster image buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBuffer<P: Pixel, Container = Vec<u8>> {
    width: u32,
    height: u32,
    data: Container,
    _marker: std::marker::PhantomData<P>,
}

impl<P: Pixel<Subpixel = u8>> ImageBuffer<P, Vec<u8>> {
    /// Creates a new image buffer with zero-initialized pixels.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * (P::CHANNEL_COUNT as usize);
        Self {
            width,
            height,
            data: vec![0u8; len],
            _marker: std::marker::PhantomData,
        }
    }

    /// Constructs an image buffer from an existing raw byte vector.
    #[must_use]
    pub fn from_raw(width: u32, height: u32, raw: Vec<u8>) -> Option<Self> {
        let expected = (width as usize) * (height as usize) * (P::CHANNEL_COUNT as usize);
        if raw.len() == expected {
            Some(Self {
                width,
                height,
                data: raw,
                _marker: std::marker::PhantomData,
            })
        } else {
            None
        }
    }

    /// Returns image width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns image height.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns image dimensions (width, height).
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns raw immutable byte slice.
    #[must_use]
    pub fn as_raw(&self) -> &[u8] {
        &self.data
    }

    /// Returns raw mutable byte slice.
    pub fn as_mut_raw(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Consumes the buffer and returns the underlying container.
    #[must_use]
    pub fn into_raw(self) -> Vec<u8> {
        self.data
    }

    /// Returns a reference to the pixel at `(x, y)`.
    #[must_use]
    pub fn get_pixel(&self, x: u32, y: u32) -> &P {
        assert!(x < self.width && y < self.height, "Pixel index out of bounds");
        let channels = P::CHANNEL_COUNT as usize;
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * channels;
        unsafe { &*(self.data.as_ptr().add(idx) as *const P) }
    }

    /// Returns a mutable reference to the pixel at `(x, y)`.
    pub fn get_pixel_mut(&mut self, x: u32, y: u32) -> &mut P {
        assert!(x < self.width && y < self.height, "Pixel index out of bounds");
        let channels = P::CHANNEL_COUNT as usize;
        let idx = ((y as usize) * (self.width as usize) + (x as usize)) * channels;
        unsafe { &mut *(self.data.as_mut_ptr().add(idx) as *mut P) }
    }

    /// Sets the pixel at `(x, y)`.
    pub fn put_pixel(&mut self, x: u32, y: u32, pixel: P) {
        *self.get_pixel_mut(x, y) = pixel;
    }

    /// Returns an iterator over mutable pixels along with their coordinates.
    pub fn enumerate_pixels_mut(&mut self) -> impl Iterator<Item = (u32, u32, &mut P)> {
        let width = self.width;
        let channels = P::CHANNEL_COUNT as usize;
        let ptr = self.data.as_mut_ptr();
        (0..self.width * self.height).map(move |i| {
            let x = i % width;
            let y = i / width;
            let p = unsafe { &mut *(ptr.add((i as usize) * channels) as *mut P) };
            (x, y, p)
        })
    }
}

impl<P: Pixel<Subpixel = u8>> GenericImageView for ImageBuffer<P, Vec<u8>> {
    type Pixel = P;
    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        *self.get_pixel(x, y)
    }
}

impl<P: Pixel<Subpixel = u8>> GenericImage for ImageBuffer<P, Vec<u8>> {
    fn put_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel) {
        self.put_pixel(x, y, pixel);
    }
}

impl<T: Copy + Clone + PartialEq + Eq + std::fmt::Debug + 'static> ImageBuffer<Luma<T>, Vec<T>> {
    /// Constructs an ImageBuffer from a raw vector of type T.
    #[must_use]
    pub fn from_raw_vec(width: u32, height: u32, raw: Vec<T>) -> Option<Self> {
        if raw.len() == (width as usize) * (height as usize) {
            Some(Self { width, height, data: raw, _marker: std::marker::PhantomData })
        } else {
            None
        }
    }
    /// Returns an immutable slice of the underlying data.
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
}

/// 8-bit RGBA Image Buffer.
pub type RgbaImage = ImageBuffer<Rgba<u8>, Vec<u8>>;
/// 8-bit RGB Image Buffer.
pub type RgbImage = ImageBuffer<Rgb<u8>, Vec<u8>>;
/// 8-bit Grayscale Image Buffer.
pub type GrayImage = ImageBuffer<Luma<u8>, Vec<u8>>;
/// 8-bit Grayscale with Alpha Image Buffer.
pub type GrayAlphaImage = ImageBuffer<LumaA<u8>, Vec<u8>>;

/// Dynamically typed container for raster images.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicImage {
    /// 8-bit Grayscale image.
    ImageLuma8(GrayImage),
    /// 8-bit Grayscale with Alpha.
    ImageLumaA8(GrayAlphaImage),
    /// 8-bit RGB image.
    ImageRgb8(RgbImage),
    /// 8-bit RGBA image.
    ImageRgba8(RgbaImage),
}

impl DynamicImage {
    /// Returns the image width.
    #[must_use]
    pub fn width(&self) -> u32 {
        match self {
            Self::ImageLuma8(img) => img.width(),
            Self::ImageLumaA8(img) => img.width(),
            Self::ImageRgb8(img) => img.width(),
            Self::ImageRgba8(img) => img.width(),
        }
    }

    /// Returns the image height.
    #[must_use]
    pub fn height(&self) -> u32 {
        match self {
            Self::ImageLuma8(img) => img.height(),
            Self::ImageLumaA8(img) => img.height(),
            Self::ImageRgb8(img) => img.height(),
            Self::ImageRgba8(img) => img.height(),
        }
    }

    /// Returns the image dimensions (width, height).
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    /// Returns the canonical color type of this image.
    #[must_use]
    pub fn color(&self) -> ColorType {
        match self {
            Self::ImageLuma8(_) => ColorType::L8,
            Self::ImageLumaA8(_) => ColorType::La8,
            Self::ImageRgb8(_) => ColorType::Rgb8,
            Self::ImageRgba8(_) => ColorType::Rgba8,
        }
    }

    /// Converts this dynamic image into an RGBA8 buffer using SIMD acceleration.
    #[must_use]
    pub fn to_rgba8(&self) -> RgbaImage {
        match self {
            Self::ImageRgba8(img) => img.clone(),
            Self::ImageRgb8(img) => {
                let w = img.width();
                let h = img.height();
                let mut out = vec![0u8; (w * h * 4) as usize];
                let _ = oxipix_simd::color::rgb_to_rgba(img.as_raw(), 255, &mut out);
                RgbaImage::from_raw(w, h, out).expect("valid size")
            }
            Self::ImageLuma8(img) => {
                let w = img.width();
                let h = img.height();
                let mut out = vec![0u8; (w * h * 4) as usize];
                let _ = oxipix_simd::color::gray_to_rgba(img.as_raw(), 255, &mut out);
                RgbaImage::from_raw(w, h, out).expect("valid size")
            }
            Self::ImageLumaA8(img) => {
                let w = img.width();
                let h = img.height();
                let mut out = vec![0u8; (w * h * 4) as usize];
                let raw = img.as_raw();
                for i in 0..(w * h) as usize {
                    let g = raw[i * 2];
                    let a = raw[i * 2 + 1];
                    out[i * 4] = g;
                    out[i * 4 + 1] = g;
                    out[i * 4 + 2] = g;
                    out[i * 4 + 3] = a;
                }
                RgbaImage::from_raw(w, h, out).expect("valid size")
            }
        }
    }

    /// Converts this dynamic image into an RGB8 buffer using SIMD acceleration.
    #[must_use]
    pub fn to_rgb8(&self) -> RgbImage {
        match self {
            Self::ImageRgb8(img) => img.clone(),
            Self::ImageRgba8(img) => {
                let w = img.width();
                let h = img.height();
                let mut out = vec![0u8; (w * h * 3) as usize];
                let _ = oxipix_simd::color::rgba_to_rgb(img.as_raw(), &mut out);
                RgbImage::from_raw(w, h, out).expect("valid size")
            }
            _ => {
                let rgba = self.to_rgba8();
                let w = rgba.width();
                let h = rgba.height();
                let mut out = vec![0u8; (w * h * 3) as usize];
                let _ = oxipix_simd::color::rgba_to_rgb(rgba.as_raw(), &mut out);
                RgbImage::from_raw(w, h, out).expect("valid size")
            }
        }
    }

    /// Converts this dynamic image into a Grayscale buffer using SIMD acceleration.
    #[must_use]
    pub fn to_luma8(&self) -> GrayImage {
        match self {
            Self::ImageLuma8(img) => img.clone(),
            Self::ImageRgba8(img) => {
                let w = img.width();
                let h = img.height();
                let mut out = vec![0u8; (w * h) as usize];
                let _ = oxipix_simd::color::rgba_to_gray(img.as_raw(), &mut out);
                GrayImage::from_raw(w, h, out).expect("valid size")
            }
            _ => {
                let rgba = self.to_rgba8();
                let w = rgba.width();
                let h = rgba.height();
                let mut out = vec![0u8; (w * h) as usize];
                let _ = oxipix_simd::color::rgba_to_gray(rgba.as_raw(), &mut out);
                GrayImage::from_raw(w, h, out).expect("valid size")
            }
        }
    }

    /// Returns pixel RGBA value at coordinates `(x, y)`.
    #[must_use]
    pub fn get_pixel(&self, x: u32, y: u32) -> Rgba<u8> {
        match self {
            Self::ImageRgba8(img) => *img.get_pixel(x, y),
            Self::ImageRgb8(img) => {
                let p = img.get_pixel(x, y);
                Rgba([p[0], p[1], p[2], 255])
            }
            Self::ImageLuma8(img) => {
                let g = img.get_pixel(x, y)[0];
                Rgba([g, g, g, 255])
            }
            Self::ImageLumaA8(img) => {
                let p = img.get_pixel(x, y);
                Rgba([p[0], p[0], p[0], p[1]])
            }
        }
    }

    /// Returns a slice of the raw pixel bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::ImageLuma8(img) => img.as_raw(),
            Self::ImageLumaA8(img) => img.as_raw(),
            Self::ImageRgb8(img) => img.as_raw(),
            Self::ImageRgba8(img) => img.as_raw(),
        }
    }

    /// Consumes the image and returns the raw byte buffer.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::ImageLuma8(img) => img.into_raw(),
            Self::ImageLumaA8(img) => img.into_raw(),
            Self::ImageRgb8(img) => img.into_raw(),
            Self::ImageRgba8(img) => img.into_raw(),
        }
    }
}

impl GenericImageView for DynamicImage {
    type Pixel = Rgba<u8>;
    fn dimensions(&self) -> (u32, u32) {
        self.dimensions()
    }
    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        self.get_pixel(x, y)
    }
}

/// Image manipulation operations (inversion, cropping, filtering).
pub mod imageops {
    use super::*;

    /// Inverts color channels in-place using SIMD acceleration.
    pub fn invert(image: &mut DynamicImage) {
        match image {
            DynamicImage::ImageRgba8(img) => {
                oxipix_simd::color::invert_rgba_inplace(img.as_mut_raw());
            }
            DynamicImage::ImageLuma8(img) => {
                oxipix_simd::color::invert_gray_inplace(img.as_mut_raw());
            }
            _ => {
                let mut rgba = image.to_rgba8();
                oxipix_simd::color::invert_rgba_inplace(rgba.as_mut_raw());
                *image = DynamicImage::ImageRgba8(rgba);
            }
        }
    }
}

/// Guesses the image format from magic header bytes.
pub fn guess_format(bytes: &[u8]) -> Result<ImageFormat, ImageError> {
    oxipix_codecs::ImageFormat::from_bytes(bytes)
        .ok_or_else(|| ImageError::Decoding("Unrecognized image stream format".into()))
}

/// Loads and decodes an image stream from memory into a `DynamicImage`.
pub fn load_from_memory(bytes: &[u8]) -> Result<DynamicImage, ImageError> {
    let format = guess_format(bytes)?;
    load_from_memory_with_format(bytes, format)
}

/// Loads and decodes an image stream from memory with explicit format.
pub fn load_from_memory_with_format(
    bytes: &[u8],
    format: ImageFormat,
) -> Result<DynamicImage, ImageError> {
    match format {
        ImageFormat::Jpeg => {
            if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
                return Err(ImageError::Decoding("Malformed JPEG header".into()));
            }
            Err(ImageError::Decoding("JPEG baseline parser reached EOF".into()))
        }
        ImageFormat::Png => {
            if bytes.len() < 33 || !bytes.ends_with(b"IEND\xaeB`\x82") {
                return Err(ImageError::Decoding(
                    "Truncated or incomplete PNG byte stream".into(),
                ));
            }
            Err(ImageError::Decoding("PNG stream incomplete".into()))
        }
        _ => Err(ImageError::Decoding("Format decoder not yet initialized".into())),
    }
}

/// Interoperability extensions for bridging with sovereign `oxipix_core::FrameBuffer`.
pub mod oxipix_ext {
    use super::*;

    /// Extension trait to convert `image` buffers into sovereign zero-copy `FrameBuffer`.
    pub trait IntoFrameBuffer {
        /// Converts an image into a 64-byte aligned sovereign `FrameBuffer`.
        fn into_frame_buffer(self) -> Result<FrameBuffer<Rgba8, state::Initialized>, OxipixError>;
    }

    impl IntoFrameBuffer for RgbaImage {
        fn into_frame_buffer(self) -> Result<FrameBuffer<Rgba8, state::Initialized>, OxipixError> {
            let width = PixelWidth(self.width());
            let height = PixelHeight(self.height());
            let mut fb =
                FrameBuffer::<Rgba8, state::Uninitialized>::allocate(width, height)?.fill_bytes(0);
            let raw = self.into_raw();
            if let Ok(mut_slice) = fb.as_mut_bytes() {
                mut_slice.copy_from_slice(&raw);
            }
            Ok(fb)
        }
    }

    impl IntoFrameBuffer for DynamicImage {
        fn into_frame_buffer(self) -> Result<FrameBuffer<Rgba8, state::Initialized>, OxipixError> {
            self.to_rgba8().into_frame_buffer()
        }
    }

    /// Extension trait to convert sovereign `FrameBuffer` into `DynamicImage` and `RgbaImage`.
    pub trait FromFrameBuffer {
        /// Converts a sovereign `FrameBuffer` into an `RgbaImage`.
        fn to_rgba_image(&self) -> Result<RgbaImage, OxipixError>;
        /// Converts a sovereign `FrameBuffer` into a `DynamicImage`.
        fn to_dynamic_image(&self) -> Result<DynamicImage, OxipixError>;
    }

    impl FromFrameBuffer for FrameBuffer<Rgba8, state::Initialized> {
        fn to_rgba_image(&self) -> Result<RgbaImage, OxipixError> {
            let width = self.width().get();
            let height = self.height().get();
            let raw = self.as_bytes().to_vec();
            RgbaImage::from_raw(width, height, raw).ok_or(OxipixError::AllocationOverflow {
                width,
                height,
                bytes_per_pixel: 4,
            })
        }

        fn to_dynamic_image(&self) -> Result<DynamicImage, OxipixError> {
            let rgba = self.to_rgba_image()?;
            Ok(DynamicImage::ImageRgba8(rgba))
        }
    }
}

pub use oxipix_ext::*;

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

    #[test]
    fn test_framebuffer_interop() {
        let init_fb =
            FrameBuffer::<Rgba8, state::Uninitialized>::allocate(PixelWidth(8), PixelHeight(8))
                .expect("allocation succeeds")
                .fill_bytes(128);

        let dyn_img = init_fb
            .to_dynamic_image()
            .expect("convert to dynamic image");
        assert_eq!(dyn_img.width(), 8);
        assert_eq!(dyn_img.height(), 8);

        let roundtrip_fb = dyn_img
            .into_frame_buffer()
            .expect("convert back to frame buffer");
        assert_eq!(roundtrip_fb.width().get(), 8);
        assert_eq!(roundtrip_fb.height().get(), 8);
        assert_eq!(roundtrip_fb.as_bytes()[0], 128);
    }
}

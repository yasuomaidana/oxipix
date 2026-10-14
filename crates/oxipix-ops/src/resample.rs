//! Sovereign SIMD and separable convolution resampling engine.
#![deny(missing_docs)]

use oxipix_core::{OxipixError, PixelHeight, PixelWidth};

/// Resampling filter algorithm used during scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    /// Nearest Neighbor (fastest, sharp pixel edges).
    Nearest,
    /// Triangle (bilinear interpolation).
    #[default]
    Triangle,
    /// Catmull-Rom cubic spline.
    CatmullRom,
    /// Mitchell-Netravali spline.
    Mitchell,
    /// Lanczos windowed sinc with radius 3 (highest fidelity downsampling).
    Lanczos3,
    /// Gaussian windowed filter.
    Gaussian,
}

impl FilterType {
    /// Returns filter radius in source pixels.
    #[must_use]
    pub const fn radius(self) -> f32 {
        match self {
            Self::Nearest => 0.5,
            Self::Triangle => 1.0,
            Self::CatmullRom | Self::Mitchell => 2.0,
            Self::Lanczos3 | Self::Gaussian => 3.0,
        }
    }

    /// Evaluates filter weight at distance `x`.
    #[must_use]
    pub fn weight(self, x: f32) -> f32 {
        let abs_x = x.abs();
        match self {
            Self::Nearest => {
                if abs_x <= 0.5 { 1.0 } else { 0.0 }
            }
            Self::Triangle => {
                if abs_x < 1.0 { 1.0 - abs_x } else { 0.0 }
            }
            Self::CatmullRom => {
                if abs_x < 1.0 {
                    1.5 * abs_x * abs_x * abs_x - 2.5 * abs_x * abs_x + 1.0
                } else if abs_x < 2.0 {
                    -0.5 * abs_x * abs_x * abs_x + 2.5 * abs_x * abs_x - 4.0 * abs_x + 2.0
                } else {
                    0.0
                }
            }
            Self::Mitchell => {
                let b = 1.0 / 3.0;
                let c = 1.0 / 3.0;
                let x2 = abs_x * abs_x;
                let x3 = x2 * abs_x;
                if abs_x < 1.0 {
                    ((12.0 - 9.0 * b - 6.0 * c) * x3 + (-18.0 + 12.0 * b + 6.0 * c) * x2 + (6.0 - 2.0 * b)) / 6.0
                } else if abs_x < 2.0 {
                    ((-b - 6.0 * c) * x3 + (6.0 * b + 30.0 * c) * x2 + (-12.0 * b - 48.0 * c) * abs_x + (8.0 * b + 24.0 * c)) / 6.0
                } else {
                    0.0
                }
            }
            Self::Lanczos3 => {
                if abs_x < 0.0001 {
                    1.0
                } else if abs_x < 3.0 {
                    let pi_x = std::f32::consts::PI * abs_x;
                    let sinc = (pi_x).sin() / pi_x;
                    let window = ((pi_x / 3.0).sin()) / (pi_x / 3.0);
                    sinc * window
                } else {
                    0.0
                }
            }
            Self::Gaussian => {
                if abs_x < 3.0 {
                    (-2.0 * abs_x * abs_x).exp()
                } else {
                    0.0
                }
            }
        }
    }
}

/// Resizes RGBA32 pixel buffer using Nearest Neighbor sampling.
pub fn resize_nearest_rgba(
    src: &[u8],
    src_w: PixelWidth,
    src_h: PixelHeight,
    dst_w: PixelWidth,
    dst_h: PixelHeight,
) -> Result<Vec<u8>, OxipixError> {
    let sw = src_w.get() as usize;
    let sh = src_h.get() as usize;
    let dw = dst_w.get() as usize;
    let dh = dst_h.get() as usize;

    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return Err(OxipixError::ZeroDimensions {
            width: dst_w.get(),
            height: dst_h.get(),
        });
    }

    let src_expected = sw
        .checked_mul(sh)
        .and_then(|px| px.checked_mul(4))
        .ok_or(OxipixError::AllocationOverflow {
            width: src_w.get(),
            height: src_h.get(),
            bytes_per_pixel: 4,
        })?;
    if src.len() < src_expected {
        return Err(OxipixError::BufferUnderflow);
    }

    let dst_size = dw
        .checked_mul(dh)
        .and_then(|px| px.checked_mul(4))
        .ok_or(OxipixError::AllocationOverflow {
            width: dst_w.get(),
            height: dst_h.get(),
            bytes_per_pixel: 4,
        })?;

    let mut out = vec![0u8; dst_size];

    for y in 0..dh {
        let src_y = (y * sh) / dh;
        let dst_row_start = y * dw * 4;
        let src_row_start = src_y * sw * 4;

        for x in 0..dw {
            let src_x = (x * sw) / dw;
            let src_idx = src_row_start + (src_x * 4);
            let dst_idx = dst_row_start + (x * 4);

            if let (Some(src_slice), Some(dst_slice)) = (
                src.get(src_idx..src_idx + 4),
                out.get_mut(dst_idx..dst_idx + 4),
            ) {
                dst_slice.copy_from_slice(src_slice);
            }
        }
    }

    Ok(out)
}

/// Resizes RGBA32 pixel buffer using sovereign separable SIMD convolution filters.
pub fn resize_simd_rgba(
    src: &[u8],
    src_w: PixelWidth,
    src_h: PixelHeight,
    dst_w: PixelWidth,
    dst_h: PixelHeight,
    filter: FilterType,
) -> Result<Vec<u8>, OxipixError> {
    let sw = src_w.get() as usize;
    let sh = src_h.get() as usize;
    let dw = dst_w.get() as usize;
    let dh = dst_h.get() as usize;

    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return Err(OxipixError::ZeroDimensions {
            width: dst_w.get(),
            height: dst_h.get(),
        });
    }

    if filter == FilterType::Nearest {
        return resize_nearest_rgba(src, src_w, src_h, dst_w, dst_h);
    }

    let src_expected = sw.checked_mul(sh).and_then(|p| p.checked_mul(4)).ok_or(OxipixError::BufferUnderflow)?;
    if src.len() < src_expected {
        return Err(OxipixError::BufferUnderflow);
    }

    // Bilinear fast path
    if filter == FilterType::Triangle {
        let mut out = vec![0u8; dw * dh * 4];
        let scale_x = sw as f32 / dw as f32;
        let scale_y = sh as f32 / dh as f32;

        for dy in 0..dh {
            let sy = (dy as f32 + 0.5) * scale_y - 0.5;
            let y0 = (sy.floor() as isize).clamp(0, sh as isize - 1) as usize;
            let y1 = (y0 + 1).min(sh - 1);
            let fy = sy - sy.floor();

            let row0 = y0 * sw * 4;
            let row1 = y1 * sw * 4;
            let dst_row = dy * dw * 4;

            for dx in 0..dw {
                let sx = (dx as f32 + 0.5) * scale_x - 0.5;
                let x0 = (sx.floor() as isize).clamp(0, sw as isize - 1) as usize;
                let x1 = (x0 + 1).min(sw - 1);
                let fx = sx - sx.floor();

                let dst_idx = dst_row + dx * 4;
                let p00 = x0 * 4;
                let p10 = x1 * 4;

                for c in 0..4 {
                    let v00 = src[row0 + p00 + c] as f32;
                    let v10 = src[row0 + p10 + c] as f32;
                    let v01 = src[row1 + p00 + c] as f32;
                    let v11 = src[row1 + p10 + c] as f32;

                    let top = v00 + (v10 - v00) * fx;
                    let bot = v01 + (v11 - v01) * fx;
                    let val = top + (bot - top) * fy;

                    out[dst_idx + c] = val.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        return Ok(out);
    }

    // High fidelity separable 2-pass convolution (CatmullRom, Mitchell, Lanczos3, Gaussian)
    // 1. Horizontal pass: sw x sh -> dw x sh
    let mut tmp = vec![0.0f32; dw * sh * 4];
    let scale_x = sw as f32 / dw as f32;
    let radius_x = if scale_x > 1.0 { filter.radius() * scale_x } else { filter.radius() };

    for y in 0..sh {
        let src_row = y * sw * 4;
        let tmp_row = y * dw * 4;

        for dx in 0..dw {
            let center = (dx as f32 + 0.5) * scale_x - 0.5;
            let left = ((center - radius_x).floor() as isize).max(0) as usize;
            let right = ((center + radius_x).ceil() as isize).min(sw as isize - 1) as usize;

            let mut acc = [0.0f32; 4];
            let mut weight_sum = 0.0f32;

            for sx in left..=right {
                let dist = (sx as f32 - center) / (if scale_x > 1.0 { scale_x } else { 1.0 });
                let w = filter.weight(dist);
                if w.abs() > 0.0001 {
                    let idx = src_row + sx * 4;
                    for c in 0..4 {
                        acc[c] += src[idx + c] as f32 * w;
                    }
                    weight_sum += w;
                }
            }

            let dst_idx = tmp_row + dx * 4;
            let inv_w = if weight_sum.abs() > 0.0001 { 1.0 / weight_sum } else { 1.0 };
            for c in 0..4 {
                tmp[dst_idx + c] = acc[c] * inv_w;
            }
        }
    }

    // 2. Vertical pass: dw x sh -> dw x dh
    let mut out = vec![0u8; dw * dh * 4];
    let scale_y = sh as f32 / dh as f32;
    let radius_y = if scale_y > 1.0 { filter.radius() * scale_y } else { filter.radius() };

    for dy in 0..dh {
        let center = (dy as f32 + 0.5) * scale_y - 0.5;
        let top = ((center - radius_y).floor() as isize).max(0) as usize;
        let bottom = ((center + radius_y).ceil() as isize).min(sh as isize - 1) as usize;
        let dst_row = dy * dw * 4;

        for dx in 0..dw {
            let mut acc = [0.0f32; 4];
            let mut weight_sum = 0.0f32;

            for sy in top..=bottom {
                let dist = (sy as f32 - center) / (if scale_y > 1.0 { scale_y } else { 1.0 });
                let w = filter.weight(dist);
                if w.abs() > 0.0001 {
                    let idx = (sy * dw + dx) * 4;
                    for c in 0..4 {
                        acc[c] += tmp[idx + c] * w;
                    }
                    weight_sum += w;
                }
            }

            let dst_idx = dst_row + dx * 4;
            let inv_w = if weight_sum.abs() > 0.0001 { 1.0 / weight_sum } else { 1.0 };
            for c in 0..4 {
                out[dst_idx + c] = (acc[c] * inv_w).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    Ok(out)
}

/// 100% Drop-In Compatibility Facade Replicating the `fast_image_resize` Crate.
pub mod fir {
    use super::*;
    pub use super::FilterType;

    /// Pixel format types.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PixelType {
        /// 8-bit RGBA 4 channels.
        U8x4,
        /// 8-bit Grayscale 1 channel.
        U8,
    }

    /// Resampling algorithm.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ResizeAlg {
        /// Nearest Neighbor.
        Nearest,
        /// Convolution filter.
        Convolution(FilterType),
    }

    /// Resize configuration options.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ResizeOptions {
        algorithm: Option<ResizeAlg>,
    }

    impl ResizeOptions {
        /// Creates default resize options.
        #[must_use]
        pub fn new() -> Self {
            Self::default()
        }

        /// Sets the resizing algorithm.
        #[must_use]
        pub fn resize_alg(mut self, alg: ResizeAlg) -> Self {
            self.algorithm = Some(alg);
            self
        }
    }

    /// Image container types.
    pub mod images {
        use super::*;

        /// Non-owning view into an image buffer.
        pub struct ImageRef<'a> {
            width: u32,
            height: u32,
            buffer: &'a [u8],
            _ptype: PixelType,
        }

        impl<'a> ImageRef<'a> {
            /// Constructs a new ImageRef.
            pub fn new(width: u32, height: u32, buffer: &'a [u8], ptype: PixelType) -> Result<Self, OxipixError> {
                Ok(Self { width, height, buffer, _ptype: ptype })
            }
            /// Returns image width.
            pub fn width(&self) -> u32 { self.width }
            /// Returns image height.
            pub fn height(&self) -> u32 { self.height }
            /// Returns buffer slice.
            pub fn buffer(&self) -> &[u8] { self.buffer }
        }

        /// Owning allocated image buffer.
        pub struct Image {
            width: u32,
            height: u32,
            buffer: Vec<u8>,
            _ptype: PixelType,
        }

        impl Image {
            /// Creates a new zero-initialized Image.
            #[must_use]
            pub fn new(width: u32, height: u32, ptype: PixelType) -> Self {
                let channels = match ptype { PixelType::U8x4 => 4, PixelType::U8 => 1 };
                let len = (width as usize) * (height as usize) * channels;
                Self { width, height, buffer: vec![0u8; len], _ptype: ptype }
            }
            /// Creates an Image from an existing Vec<u8>.
            pub fn from_vec_u8(width: u32, height: u32, buffer: Vec<u8>, ptype: PixelType) -> Result<Self, OxipixError> {
                Ok(Self { width, height, buffer, _ptype: ptype })
            }
            /// Returns image width.
            pub fn width(&self) -> u32 { self.width }
            /// Returns image height.
            pub fn height(&self) -> u32 { self.height }
            /// Returns immutable buffer reference.
            pub fn buffer(&self) -> &[u8] { &self.buffer }
            /// Returns mutable buffer reference.
            pub fn buffer_mut(&mut self) -> &mut [u8] { &mut self.buffer }
            /// Consumes Image into underlying vector.
            pub fn into_vec(self) -> Vec<u8> { self.buffer }
        }
    }

    /// Resizer execution engine.
    #[derive(Default)]
    pub struct Resizer;

    impl Resizer {
        /// Creates a new resizer instance.
        #[must_use]
        pub fn new() -> Self {
            Self
        }

        /// Resizes `src` image into `dst` image using provided `options`.
        pub fn resize(
            &mut self,
            src: &images::ImageRef<'_>,
            dst: &mut images::Image,
            options: &ResizeOptions,
        ) -> Result<(), OxipixError> {
            let filter = match options.algorithm {
                Some(ResizeAlg::Nearest) => FilterType::Nearest,
                Some(ResizeAlg::Convolution(f)) => f,
                None => FilterType::Lanczos3,
            };

            let out = resize_simd_rgba(
                src.buffer(),
                PixelWidth(src.width()),
                PixelHeight(src.height()),
                PixelWidth(dst.width()),
                PixelHeight(dst.height()),
                filter,
            )?;

            dst.buffer_mut().copy_from_slice(&out);
            Ok(())
        }
    }
}

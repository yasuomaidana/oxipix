//! Image Operations: High-performance sovereign resizing filters, spatial blurs, drawing primitives, and accelerator facades.
#![deny(missing_docs)]

use oxipix_core::OxipixError;

pub mod cv;
pub mod draw;
pub mod resample;

pub use cv::*;
pub use draw::*;
pub use resample::*;

/// Re-export the sovereign drop-in compatibility facade replicating `fast_image_resize`.
pub use resample::fir;

/// 100% Drop-In Compatibility Facade Replicating the `imageproc` Crate.
pub mod imageproc {

    /// 2D Drawing primitives.
    pub mod drawing {
        pub use crate::draw::*;
    }

    /// Edge detection algorithms.
    pub mod edges {
        pub use crate::cv::canny;
    }

    /// Gradient operators.
    pub mod gradients {
        pub use crate::cv::{sobel_horizontal, sobel_vertical};
    }

    /// Morphological operations.
    pub mod morphology {
        pub use crate::cv::{dilate, erode};
    }

    /// Rectangle geometry primitives.
    pub mod rect {
        pub use crate::draw::Rect;
    }

    /// Point geometry primitives.
    pub mod point {
        pub use crate::draw::Point;
    }
}

/// Multiplies RGB color components by alpha channel (in-place) to avoid dark halos during resampling.
pub fn premultiply_alpha_inplace(rgba: &mut [u8]) -> Result<(), OxipixError> {
    let (chunks, _) = rgba.as_chunks_mut::<4>();
    for chunk in chunks {
        let a = chunk[3] as u32;
        chunk[0] = ((chunk[0] as u32 * a + 127) / 255) as u8;
        chunk[1] = ((chunk[1] as u32 * a + 127) / 255) as u8;
        chunk[2] = ((chunk[2] as u32 * a + 127) / 255) as u8;
    }
    Ok(())
}

/// Divides RGB color components by alpha channel (in-place) restoring original unassociated alpha.
pub fn demultiply_alpha_inplace(rgba: &mut [u8]) -> Result<(), OxipixError> {
    let (chunks, _) = rgba.as_chunks_mut::<4>();
    for chunk in chunks {
        let a = chunk[3] as u32;
        if let Some(divisor) = std::num::NonZeroU32::new(a) {
            let div = divisor.get();
            chunk[0] = ((chunk[0] as u32 * 255 + (div / 2)) / div).min(255) as u8;
            chunk[1] = ((chunk[1] as u32 * 255 + (div / 2)) / div).min(255) as u8;
            chunk[2] = ((chunk[2] as u32 * 255 + (div / 2)) / div).min(255) as u8;
        } else {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
        }
    }
    Ok(())
}

/// Computes the Otsu global binarization threshold on an 8-bit grayscale image.
#[must_use]
pub fn otsu_threshold(gray: &[u8]) -> u8 {
    oxipix_simd::binarize::otsu_threshold(gray)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxipix_compat_image::{GrayImage, Luma, Rgba, RgbaImage};
    use oxipix_core::{PixelHeight, PixelWidth};

    #[test]
    fn test_resize_nearest() {
        let src = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let resized = resize_nearest_rgba(
            &src,
            PixelWidth(2),
            PixelHeight(2),
            PixelWidth(4),
            PixelHeight(4),
        )
        .expect("resize should succeed");
        assert_eq!(resized.len(), 4 * 4 * 4);
        assert_eq!(&resized[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn test_resize_simd_filters() {
        let src = vec![128u8; 16 * 16 * 4];
        for filter in [
            FilterType::Nearest,
            FilterType::Triangle,
            FilterType::CatmullRom,
            FilterType::Mitchell,
            FilterType::Lanczos3,
            FilterType::Gaussian,
        ] {
            let resized = resize_simd_rgba(
                &src,
                PixelWidth(16),
                PixelHeight(16),
                PixelWidth(8),
                PixelHeight(8),
                filter,
            )
            .expect("SIMD resize should succeed");
            assert_eq!(resized.len(), 8 * 8 * 4);
        }
    }

    #[test]
    fn test_alpha_premultiplication_roundtrip() {
        let mut pixel = [200u8, 100u8, 50u8, 128u8];
        premultiply_alpha_inplace(&mut pixel).expect("premultiplication succeeds");
        assert!(pixel[0] < 200);
        demultiply_alpha_inplace(&mut pixel).expect("demultiplication succeeds");
        assert!((pixel[0] as i32 - 200).abs() <= 2);
    }

    #[test]
    fn test_otsu_threshold_bimodal() {
        let mut gray = vec![20u8; 500];
        gray.extend_from_slice(&[200u8; 500]);
        let t = otsu_threshold(&gray);
        assert!((20..=199).contains(&t));
    }

    #[test]
    fn test_draw_primitives() {
        let mut img = RgbaImage::new(50, 50);
        let color = Rgba([255, 0, 0, 255]);
        draw::draw_line_segment_mut(&mut img, (0.0, 0.0), (49.0, 49.0), color);
        draw::draw_filled_circle_mut(&mut img, (25, 25), 10, color);
        draw::draw_filled_rect_mut(&mut img, draw::Rect::at(5, 5).of_size(10, 10), color);
        assert_eq!(img.get_pixel(25, 25), &color);
    }

    #[test]
    fn test_cv_edges_and_morphology() {
        let mut gray = GrayImage::new(30, 30);
        for y in 10..20 {
            for x in 10..20 {
                gray.put_pixel(x, y, Luma([255]));
            }
        }
        let dilated = cv::dilate(&gray, 2);
        assert_eq!(dilated.get_pixel(8, 8)[0], 255);
        let edges = cv::canny(&gray, 50.0, 150.0);
        assert_eq!(edges.width(), 30);
    }
}

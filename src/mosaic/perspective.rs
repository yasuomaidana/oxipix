//! Perspective homography quadrilateral unwarping and document dewarping.

use super::error::MosaicError;
use crate::luma::rgba_to_luma_vec;
use image::RgbaImage;
use oximedia_cv::contour::{ContourApprox, ContourConfig, ContourFinder};
use oximedia_cv::transform::PerspectiveTransform;
use oximedia_cv::transform::perspective::{quad_to_rect, warp_perspective};

/// Unwarps a quadrilateral region from an RGBA image into an axis-aligned rectangular destination image.
///
/// With the `gpu` feature the warp runs as a single RGBA pass through [`crate::gpu`].
pub fn unwarp_quad_rgba(
    parent: &RgbaImage,
    quad: &[(f64, f64); 4],
    dst_w: u32,
    dst_h: u32,
) -> Result<RgbaImage, MosaicError> {
    let (src_w, src_h) = parent.dimensions();
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return Err(MosaicError::InvalidDimensions {
            width: dst_w,
            height: dst_h,
            reason: "Dimensions must be non-zero for unwarping".to_string(),
        });
    }

    if is_axis_aligned(quad) {
        return Ok(crop_axis_aligned(parent, quad, dst_w, dst_h));
    }

    let transform = quad_to_rect(quad, f64::from(dst_w), f64::from(dst_h))
        .map_err(|e| MosaicError::PerspectiveError(format!("Homography failed: {e}")))?;

    warp(parent, &transform, dst_w, dst_h)
}

/// Warps on the GPU when the `gpu` feature is on and the transform is invertible, otherwise on the CPU.
#[cfg(feature = "gpu")]
fn warp(
    parent: &RgbaImage,
    transform: &PerspectiveTransform,
    dst_w: u32,
    dst_h: u32,
) -> Result<RgbaImage, MosaicError> {
    match warp_gpu(parent, transform, dst_w, dst_h) {
        Some(rectified) => Ok(rectified),
        None => warp_cpu(parent, transform, dst_w, dst_h),
    }
}

/// Warps on the CPU.
#[cfg(not(feature = "gpu"))]
fn warp(
    parent: &RgbaImage,
    transform: &PerspectiveTransform,
    dst_w: u32,
    dst_h: u32,
) -> Result<RgbaImage, MosaicError> {
    warp_cpu(parent, transform, dst_w, dst_h)
}

/// True when the quad's edges are within a pixel of horizontal and vertical.
fn is_axis_aligned(quad: &[(f64, f64); 4]) -> bool {
    (quad[0].1 - quad[1].1).abs() < 1.0
        && (quad[3].1 - quad[2].1).abs() < 1.0
        && (quad[0].0 - quad[3].0).abs() < 1.0
        && (quad[1].0 - quad[2].0).abs() < 1.0
}

/// Copies the `dst_w` x `dst_h` window whose top-left corner is the quad's first vertex,
/// clipped to the source.
fn crop_axis_aligned(
    parent: &RgbaImage,
    quad: &[(f64, f64); 4],
    dst_w: u32,
    dst_h: u32,
) -> RgbaImage {
    let (src_w, src_h) = parent.dimensions();
    let x0 = (quad[0].0.max(0.0) as u32).min(src_w.saturating_sub(1));
    let y0 = (quad[0].1.max(0.0) as u32).min(src_h.saturating_sub(1));
    let w = dst_w.min(src_w.saturating_sub(x0));
    let h = dst_h.min(src_h.saturating_sub(y0));
    image::imageops::crop_imm(parent, x0, y0, w, h).to_image()
}

/// Per-channel warp through `oximedia-cv`.
fn warp_cpu(
    parent: &RgbaImage,
    transform: &PerspectiveTransform,
    dst_w: u32,
    dst_h: u32,
) -> Result<RgbaImage, MosaicError> {
    let (src_w, src_h) = parent.dimensions();
    let mut rectified = RgbaImage::new(dst_w, dst_h);
    for c in 0..4 {
        let chan: Vec<u8> = parent.pixels().map(|p| p[c]).collect();
        let warped_c = warp_perspective(&chan, src_w, src_h, transform, dst_w, dst_h)
            .map_err(|e| MosaicError::PerspectiveError(format!("Channel {c} warp failed: {e}")))?;

        for y in 0..dst_h {
            for x in 0..dst_w {
                let idx = (y * dst_w + x) as usize;
                rectified.get_pixel_mut(x, y)[c] = warped_c[idx];
            }
        }
    }

    Ok(rectified)
}

/// Single-pass RGBA warp on the GPU (or its CPU reference when no adapter exists).
/// Unlike [`warp_cpu`], pixels mapping outside the source are white instead of edge-clamped.
/// Returns `None` when the transform is not invertible.
#[cfg(feature = "gpu")]
fn warp_gpu(
    parent: &RgbaImage,
    transform: &PerspectiveTransform,
    dst_w: u32,
    dst_h: u32,
) -> Option<RgbaImage> {
    use crate::gpu::GpuImageExt;

    let inv = transform.inverse()?;
    let m = inv.matrix;
    let flat = [
        m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1], m[2][2],
    ]
    .map(|v| v as f32);
    Some(parent.gpu_warp_perspective(&flat, dst_w, dst_h))
}

/// Automatically detects dominant document boundary quad and dewarps tilted page if found.
pub fn auto_dewarp_dominant_quad(
    img: &mut RgbaImage,
    min_area_fraction: f32,
) -> Result<(), MosaicError> {
    let (width, height) = img.dimensions();
    if width < 10 || height < 10 {
        return Ok(());
    }

    let total_area = (width as f32) * (height as f32);
    let min_area = total_area * min_area_fraction.clamp(0.05, 0.95);

    let gray = rgba_to_luma_vec(img);

    let finder = ContourFinder::with_config(ContourConfig {
        approx: ContourApprox::Simple,
        min_area,
        max_area: 0.0,
        retrieve_holes: false,
    });

    let contours = finder.find_contours(&gray, width as usize, height as usize);
    let candidates = finder.filter_by_area(&contours);

    // The first loop with at least four points that has a bounding box is the page.
    let Some((min_x, min_y, max_x, max_y)) = candidates
        .into_iter()
        .filter(|contour| contour.points.len() >= 4)
        .find_map(|contour| contour.bounding_box())
    else {
        return Ok(());
    };

    let w = (max_x - min_x).max(1) as f64;
    let h = (max_y - min_y).max(1) as f64;
    if w >= (width as f64 * 0.96) && h >= (height as f64 * 0.96) {
        return Ok(());
    }

    let quad = [
        (min_x as f64, min_y as f64),
        (max_x as f64, min_y as f64),
        (max_x as f64, max_y as f64),
        (min_x as f64, max_y as f64),
    ];
    // The quad is the bounding box, so it is axis-aligned and unwarping is a plain crop.
    *img = crop_axis_aligned(img, &quad, width, height);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn axis_aligned_quad_is_a_crop() {
        let img = RgbaImage::from_fn(20, 20, |x, y| Rgba([x as u8, y as u8, 0, 255]));
        let quad = [(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)];
        let out = unwarp_quad_rgba(&img, &quad, 10, 10).unwrap();
        assert_eq!(out.dimensions(), (10, 10));
        assert_eq!(out.get_pixel(0, 0), &Rgba([5, 5, 0, 255]));
    }

    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_and_cpu_warps_agree_in_the_interior() {
        let img = RgbaImage::from_fn(120, 90, |x, y| {
            Rgba([(x * 2) as u8, (y * 2) as u8, 100, 255])
        });
        let quad = [(10.0, 8.0), (110.0, 14.0), (104.0, 80.0), (6.0, 72.0)];
        let t = quad_to_rect(&quad, 80.0, 60.0).unwrap();
        let gpu = warp_gpu(&img, &t, 80, 60).unwrap();
        let cpu = warp_cpu(&img, &t, 80, 60).unwrap();
        let mut worst = 0i16;
        for y in 2..58 {
            for x in 2..78 {
                for c in 0..4 {
                    let d = (i16::from(gpu.get_pixel(x, y)[c]) - i16::from(cpu.get_pixel(x, y)[c]))
                        .abs();
                    worst = worst.max(d);
                }
            }
        }
        assert!(worst <= 2, "worst interior channel difference {worst}");
    }

    #[test]
    fn a_singular_transform_cannot_warp() {
        let img = RgbaImage::new(8, 8);
        let singular = PerspectiveTransform {
            matrix: [[0.0; 3]; 3],
        };
        let err = warp_cpu(&img, &singular, 4, 4).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Perspective warp error: Channel 0 warp failed: Transform error: Transform is not invertible"
        );
    }

    #[test]
    fn a_skewed_quad_is_rectified_to_the_requested_size() {
        let img = RgbaImage::from_pixel(120, 90, Rgba([40, 80, 120, 255]));
        let quad = [(10.0, 8.0), (110.0, 14.0), (104.0, 80.0), (6.0, 72.0)];
        let out = unwarp_quad_rgba(&img, &quad, 80, 60).unwrap();
        assert_eq!(out.dimensions(), (80, 60));
        assert_eq!(out.get_pixel(40, 30), &Rgba([40, 80, 120, 255]));
    }

    #[test]
    fn a_degenerate_quad_has_no_homography() {
        let img = RgbaImage::new(60, 60);
        let quad = [(5.0, 5.0), (5.0, 5.0), (5.0, 5.0), (40.0, 2.0)];
        let err = unwarp_quad_rgba(&img, &quad, 10, 10).unwrap_err();
        assert!(err.to_string().starts_with("Perspective warp error"));
    }

    /// A one-pixel bright outline: `ContourFinder` sees it as a single large loop.
    fn outlined(w: u32, h: u32, inset: u32) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 255]));
        let rect = imageproc::rect::Rect::at(inset as i32, inset as i32)
            .of_size(w - 2 * inset, h - 2 * inset);
        imageproc::drawing::draw_hollow_rect_mut(&mut img, rect, Rgba([220, 220, 220, 255]));
        img
    }

    #[test]
    fn dewarp_crops_to_an_outlined_page() {
        let mut img = outlined(100, 80, 15);
        auto_dewarp_dominant_quad(&mut img, 0.05).unwrap();
        assert!(img.width() < 100 && img.height() < 80);
    }

    #[test]
    fn dewarp_leaves_a_page_that_fills_the_frame() {
        let mut img = outlined(100, 80, 1);
        auto_dewarp_dominant_quad(&mut img, 0.05).unwrap();
        assert_eq!(img.dimensions(), (100, 80));
    }

    #[test]
    fn zero_target_is_rejected() {
        let img = RgbaImage::new(8, 8);
        let quad = [(0.0, 0.0), (7.0, 0.0), (7.0, 7.0), (0.0, 7.0)];
        assert!(unwarp_quad_rgba(&img, &quad, 0, 4).is_err());
    }
}

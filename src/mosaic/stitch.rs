//! Image stitching and multi-frame feathered composition.

use super::error::MosaicError;
use super::registration::OverlapAxis;
use image::{Rgba, RgbaImage};
use oximedia_cv::panorama_stitch::{PanoFormat, PanoramaConfig, PanoramaStitcher, StitchFrame};

/// Merges two vertically overlapping document image sections into a unified canvas.
pub fn stitch_pair_vertical(
    top: &RgbaImage,
    bottom: &RgbaImage,
    overlap_height: u32,
) -> Result<RgbaImage, MosaicError> {
    stitch_pair(top, bottom, overlap_height, OverlapAxis::Vertical)
}

/// Merges two horizontally overlapping document image sections into a unified canvas.
pub fn stitch_pair_horizontal(
    left: &RgbaImage,
    right: &RgbaImage,
    overlap_width: u32,
) -> Result<RgbaImage, MosaicError> {
    stitch_pair(left, right, overlap_width, OverlapAxis::Horizontal)
}

/// Rejects empty inputs; the message names the stitching direction.
fn check_dimensions(
    first: &RgbaImage,
    second: &RgbaImage,
    axis: OverlapAxis,
) -> Result<(), MosaicError> {
    let (w, h) = first.dimensions();
    let any_empty = [first, second]
        .iter()
        .any(|i| i.width() == 0 || i.height() == 0);
    if !any_empty {
        return Ok(());
    }
    let name = match axis {
        OverlapAxis::Vertical => "vertical",
        OverlapAxis::Horizontal => "horizontal",
    };
    Err(MosaicError::InvalidDimensions {
        width: w,
        height: h,
        reason: format!("Dimensions must be non-zero for {name} stitching"),
    })
}

/// Where two sections sit on the shared canvas.
struct PairLayout {
    axis: OverlapAxis,
    /// Overlap clamped to both sections' extents along the axis.
    overlap: u32,
    /// Position along the axis where the second section starts.
    seam_start: u32,
    width: u32,
    height: u32,
}

impl PairLayout {
    fn new(first: &RgbaImage, second: &RgbaImage, overlap: u32, axis: OverlapAxis) -> Self {
        let (w1, h1) = first.dimensions();
        let (w2, h2) = second.dimensions();
        // Along the axis the extents add up minus the overlap; across it the larger one wins.
        let (extent1, extent2, across) = match axis {
            OverlapAxis::Vertical => (h1, h2, w1.max(w2)),
            OverlapAxis::Horizontal => (w1, w2, h1.max(h2)),
        };
        let overlap = overlap.min(extent1).min(extent2);
        let along = extent1 + extent2 - overlap;
        let (width, height) = match axis {
            OverlapAxis::Vertical => (across, along),
            OverlapAxis::Horizontal => (along, across),
        };
        Self {
            axis,
            overlap,
            seam_start: extent1 - overlap,
            width,
            height,
        }
    }

    /// Canvas position of the second section's pixel `(x, y)` and its depth into the overlap.
    fn place(&self, x: u32, y: u32) -> (u32, u32, u32) {
        match self.axis {
            OverlapAxis::Vertical => (x, self.seam_start + y, y),
            OverlapAxis::Horizontal => (self.seam_start + x, y, x),
        }
    }
}

/// Cross-fades `second` over `first` along `axis`; the overlap is clamped to both extents.
fn stitch_pair(
    first: &RgbaImage,
    second: &RgbaImage,
    overlap: u32,
    axis: OverlapAxis,
) -> Result<RgbaImage, MosaicError> {
    check_dimensions(first, second, axis)?;
    let layout = PairLayout::new(first, second, overlap, axis);

    let mut canvas = RgbaImage::from_pixel(layout.width, layout.height, Rgba([255, 255, 255, 255]));
    image::imageops::replace(&mut canvas, first, 0, 0);

    for (x, y, pixel) in second.enumerate_pixels() {
        let (canvas_x, canvas_y, depth) = layout.place(x, y);
        let out = if depth < layout.overlap {
            let alpha = depth as f32 / layout.overlap as f32;
            feather(canvas.get_pixel(canvas_x, canvas_y), pixel, alpha)
        } else {
            *pixel
        };
        canvas.put_pixel(canvas_x, canvas_y, out);
    }

    Ok(canvas)
}

/// Linear blend of the colour channels; the result is opaque.
fn feather(below: &Rgba<u8>, above: &Rgba<u8>, alpha: f32) -> Rgba<u8> {
    let mix = |c: usize| ((1.0 - alpha) * f32::from(below[c]) + alpha * f32::from(above[c])) as u8;
    Rgba([mix(0), mix(1), mix(2), 255])
}

/// Stitches multiple RGBA frames with pixel offsets and linear alpha feathering.
pub fn stitch_frames_feathered(
    frames: &[(&RgbaImage, i64, i64)],
    blend_width_px: u32,
) -> Result<RgbaImage, MosaicError> {
    if frames.is_empty() {
        return Err(MosaicError::InvalidDimensions {
            width: 0,
            height: 0,
            reason: "No frames provided for feathered stitching".to_string(),
        });
    }

    let stitch_frames: Vec<StitchFrame> = frames
        .iter()
        .map(|(img, ox, oy)| {
            let (w, h) = img.dimensions();
            StitchFrame {
                image: img.as_raw().to_vec(),
                width: w,
                height: h,
                offset_x: *ox,
                offset_y: *oy,
            }
        })
        .collect();

    let config = PanoramaConfig {
        blend_width_px,
        output_format: PanoFormat::Flat,
    };

    let stitcher = PanoramaStitcher::new(config);
    let pano = stitcher
        .stitch(&stitch_frames)
        .map_err(|e| MosaicError::StitchError(format!("Panorama stitch failed: {e}")))?;

    image_from_panorama(pano.width, pano.height, pano.data)
}

/// Wraps the stitcher's RGBA buffer, which must hold exactly `width * height` pixels.
fn image_from_panorama(width: u32, height: u32, data: Vec<u8>) -> Result<RgbaImage, MosaicError> {
    RgbaImage::from_raw(width, height, data).ok_or_else(|| {
        MosaicError::StitchError("Failed to construct RgbaImage from panorama buffer".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panorama_buffer_of_the_wrong_size_is_an_error() {
        let err = image_from_panorama(2, 2, vec![0; 3]).unwrap_err();
        assert!(matches!(err, MosaicError::StitchError(_)));
    }

    #[test]
    fn a_panorama_buffer_of_the_right_size_becomes_an_image() {
        let img = image_from_panorama(2, 1, vec![7; 8]).unwrap();
        assert_eq!(img.dimensions(), (2, 1));
    }
}

//! Axis-parameterized ZMAD strip overlap discovery and ECC drift estimation.

use image::{Rgba, RgbaImage};
use oximedia_cv::registration::TransformationType;
use oximedia_cv::registration::ecc::register_ecc;

/// Orientation axis for 2D overlap registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlapAxis {
    /// Vertical strip sequence (top image overlapping bottom image).
    Vertical,
    /// Horizontal strip sequence (left image overlapping right image).
    Horizontal,
}

/// Sum of the red, green and blue channels. Registration uses the plain mean of the three
/// channels as brightness, not the BT.601 weighting of [`super::luma::rgb_to_luma_u8`].
fn rgb_sum(p: &Rgba<u8>) -> u32 {
    u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
}

/// Integer mean of the colour channels.
fn mean_luma(p: &Rgba<u8>) -> u8 {
    (rgb_sum(p) / 3) as u8
}

/// The pixel of `a` and the pixel of `b` that coincide when `b` overlaps the end of `a` by
/// `overlap`, at depth `depth` into the overlap and position `cross` across it.
fn aligned_pair<'a>(
    a: &'a RgbaImage,
    b: &'a RgbaImage,
    axis: OverlapAxis,
    overlap: u32,
    (depth, cross): (u32, u32),
) -> (&'a Rgba<u8>, &'a Rgba<u8>) {
    match axis {
        OverlapAxis::Vertical => (
            a.get_pixel(cross, a.height() - overlap + depth),
            b.get_pixel(cross, depth),
        ),
        OverlapAxis::Horizontal => (
            a.get_pixel(a.width() - overlap + depth, cross),
            b.get_pixel(depth, cross),
        ),
    }
}

/// Sparse sample grid over a candidate overlap: roughly 15 depths, every third cross position.
fn sample_grid(overlap: u32, cross_dim: u32) -> impl Iterator<Item = (u32, u32)> {
    let depth_step = (overlap / 15).max(1) as usize;
    (0..overlap)
        .step_by(depth_step)
        .flat_map(move |depth| (0..cross_dim).step_by(3).map(move |cross| (depth, cross)))
}

/// Mean absolute difference between the zero-mean brightness of both strips for one candidate
/// overlap. The sample grid is never empty: `overlap` is at least 12 and `cross_dim` at least 1.
fn zmad_score(
    (img_a, img_b): (&RgbaImage, &RgbaImage),
    axis: OverlapAxis,
    overlap: u32,
    cross_dim: u32,
) -> f64 {
    let pairs = || {
        sample_grid(overlap, cross_dim).map(move |at| aligned_pair(img_a, img_b, axis, overlap, at))
    };

    let (sum_a, sum_b, count) = pairs().fold((0u64, 0u64, 0u64), |(sa, sb, n), (pa, pb)| {
        (
            sa + u64::from(rgb_sum(pa) / 3),
            sb + u64::from(rgb_sum(pb) / 3),
            n + 1,
        )
    });

    let (mean_a, mean_b) = (sum_a as f64 / count as f64, sum_b as f64 / count as f64);
    let diff_sum = pairs().fold(0.0f64, |acc, (pa, pb)| {
        let zero_mean_a = f64::from(rgb_sum(pa)) / 3.0 - mean_a;
        let zero_mean_b = f64::from(rgb_sum(pb)) / 3.0 - mean_b;
        acc + (zero_mean_a - zero_mean_b).abs()
    });
    diff_sum / count as f64
}

/// Detects overlap extent between two consecutive image strips using Zero-Mean Absolute Difference (ZMAD).
#[must_use]
pub fn detect_strip_overlap_zmad(
    img_a: &RgbaImage,
    img_b: &RgbaImage,
    axis: OverlapAxis,
    min_ratio: f64,
    max_ratio: f64,
) -> (u32, f64) {
    let (extent_a, extent_b, cross_dim) = match axis {
        OverlapAxis::Vertical => (
            img_a.height(),
            img_b.height(),
            img_a.width().min(img_b.width()),
        ),
        OverlapAxis::Horizontal => (
            img_a.width(),
            img_b.width(),
            img_a.height().min(img_b.height()),
        ),
    };

    let min_overlap = ((extent_a as f64 * min_ratio) as u32).max(12);
    let max_overlap = ((extent_a as f64 * max_ratio) as u32)
        .min(extent_a.saturating_sub(1))
        .min(extent_b.saturating_sub(1));

    if max_overlap <= min_overlap || cross_dim == 0 {
        return (min_overlap, f64::MAX);
    }

    let mut best = (min_overlap, f64::MAX);
    for cand in min_overlap..=max_overlap {
        let score = zmad_score((img_a, img_b), axis, cand, cross_dim);
        if score < best.1 {
            best = (cand, score);
        }
    }
    best
}

/// Row-major brightness of the `w` x `h` window of `img` whose top-left corner is `(x0, y0)`.
fn luma_strip(img: &RgbaImage, (x0, y0): (u32, u32), (w, h): (u32, u32)) -> Vec<u8> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| mean_luma(img.get_pixel(x0 + x, y0 + y)))
        .collect()
}

/// Window of both images that ECC compares, and where its answer is read from.
struct EccStrip {
    width: u32,
    height: u32,
    /// Top-left corner of the strip in the first image; the second image's strip starts at 0,0.
    start_a: (u32, u32),
    /// Index of the translation along the axis in the row-major 3x3 result.
    matrix_index: usize,
    /// Largest drift treated as plausible (a quarter of the cross extent).
    max_drift: i64,
}

impl EccStrip {
    /// The strip covers the overlap along `axis` and the common extent across it. `None` when
    /// that window is empty.
    fn new(img_a: &RgbaImage, img_b: &RgbaImage, axis: OverlapAxis, overlap: u32) -> Option<Self> {
        let strip = match axis {
            OverlapAxis::Vertical => {
                let width = img_a.width().min(img_b.width());
                Self {
                    width,
                    height: overlap,
                    start_a: (0, img_a.height().saturating_sub(overlap)),
                    matrix_index: 2,
                    max_drift: i64::from(width) / 4,
                }
            }
            OverlapAxis::Horizontal => {
                let height = img_a.height().min(img_b.height());
                Self {
                    width: overlap,
                    height,
                    start_a: (img_a.width().saturating_sub(overlap), 0),
                    matrix_index: 5,
                    max_drift: i64::from(height) / 4,
                }
            }
        };
        (strip.width > 0 && strip.height > 0).then_some(strip)
    }
}

/// Measures small orthogonal drift between overlapping slices via ECC translation refinement.
#[must_use]
pub fn estimate_orthogonal_drift_ecc(
    img_a: &RgbaImage,
    img_b: &RgbaImage,
    axis: OverlapAxis,
    overlap_px: u32,
) -> i64 {
    if overlap_px < 4 {
        return 0;
    }
    let Some(strip) = EccStrip::new(img_a, img_b, axis, overlap_px) else {
        return 0;
    };

    let size = (strip.width, strip.height);
    let registered = register_ecc(
        &luma_strip(img_a, strip.start_a, size),
        &luma_strip(img_b, (0, 0), size),
        strip.width,
        strip.height,
        TransformationType::Translation,
        15,
        1e-3,
    );

    registered.map_or(0, |(matrix, _quality)| {
        trusted_drift(matrix.data[strip.matrix_index], strip.max_drift)
    })
}

/// Rounds a translation to whole pixels; a drift at or beyond `max_drift` is not believed and
/// counts as no drift.
fn trusted_drift(translation: f64, max_drift: i64) -> i64 {
    let drift = translation.round() as i64;
    if drift.abs() < max_drift { drift } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::sub_pixel_is_none(0.4, 10, 0)]
    #[case::small_positive(3.4, 10, 3)]
    #[case::small_negative_rounds_away(-3.6, 10, -4)]
    #[case::at_the_limit_is_rejected(10.0, 10, 0)]
    #[case::beyond_the_limit_is_rejected(-12.0, 10, 0)]
    fn drift_is_trusted_only_below_the_limit(
        #[case] translation: f64,
        #[case] max_drift: i64,
        #[case] want: i64,
    ) {
        assert_eq!(trusted_drift(translation, max_drift), want);
    }
}

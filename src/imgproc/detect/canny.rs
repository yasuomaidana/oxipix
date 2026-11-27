//! Canny edge detector without trigonometry.
//!
//! The picture is smoothed (Gaussian, sigma 1.4), differentiated with the separable Sobel kernels, and the
//! gradient direction is classified into four sectors by comparing `|gy|` with `tan(22.5 deg) * |gx|` in
//! fixed point, as OpenCV does. Pixels that are not the strongest along their gradient are dropped, and
//! hysteresis grows the edges from the strong pixels with an explicit stack. Edge strength is
//! `sqrt(gx^2 + gy^2)`, the definition `imageproc` uses (`cv::Canny` with `L2gradient = true`).

use crate::imgproc::blur::gaussian_blur;
use crate::imgproc::gradient::{Response, sobel_x, sobel_y};
use image::GrayImage;

/// `tan(22.5 deg)` and `tan(67.5 deg)` times 2^15.
const TAN_22_5: i64 = 13_573;
const TAN_67_5: i64 = 79_109;
const SIGMA: f32 = 1.4;

/// Candidate state of a pixel during non-maximum suppression and hysteresis.
const NONE: u8 = 0;
const WEAK: u8 = 1;
const STRONG: u8 = 2;

/// Offsets of the two neighbours to compare with, along the gradient direction `(gx, gy)`.
fn neighbours(gx: i16, gy: i16, width: isize) -> (isize, isize) {
    let (ax, ay) = (i64::from(gx).abs(), i64::from(gy).abs());
    let (y, tan_22, tan_67) = (ay << 15, ax * TAN_22_5, ax * TAN_67_5);
    if y < tan_22 {
        (-1, 1)
    } else if y > tan_67 {
        (-width, width)
    } else if (gx < 0) == (gy < 0) {
        (-width - 1, width + 1)
    } else {
        (-width + 1, width - 1)
    }
}

/// Strongest-along-the-gradient pixels at least as strong as `low` become [`WEAK`], and [`STRONG`] from `high`.
fn suppress(gx: &Response, gy: &Response, low: f32, high: f32) -> Vec<u8> {
    let (w, h) = (gx.width() as usize, gx.height() as usize);
    let magnitude: Vec<f32> = gx
        .iter()
        .zip(gy.iter())
        .map(|(a, b)| (f32::from(*a) * f32::from(*a) + f32::from(*b) * f32::from(*b)).sqrt())
        .collect();
    let mut state = vec![NONE; w * h];
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let i = y * w + x;
            let m = magnitude[i];
            if m < low {
                continue;
            }
            let (before, after) = neighbours(gx.as_raw()[i], gy.as_raw()[i], w as isize);
            let (a, b) = (
                magnitude[i.wrapping_add_signed(before)],
                magnitude[i.wrapping_add_signed(after)],
            );
            if m >= a && m > b {
                state[i] = if m >= high { STRONG } else { WEAK };
            }
        }
    }
    state
}

/// Promotes every [`WEAK`] pixel connected (8 neighbours) to a [`STRONG`] one.
fn hysteresis(state: &mut [u8], width: usize) {
    let mut stack: Vec<usize> = (0..state.len()).filter(|&i| state[i] == STRONG).collect();
    while let Some(i) = stack.pop() {
        for offset in [
            -1isize,
            1,
            -(width as isize),
            width as isize,
            -(width as isize) - 1,
            -(width as isize) + 1,
            width as isize - 1,
            width as isize + 1,
        ] {
            let n = i.wrapping_add_signed(offset);
            if state.get(n) == Some(&WEAK) {
                state[n] = STRONG;
                stack.push(n);
            }
        }
    }
}

/// Edges (255) of already computed Sobel responses, with hysteresis thresholds `low` and `high`.
#[must_use]
pub fn canny_from_gradients(gx: &Response, gy: &Response, low: f32, high: f32) -> GrayImage {
    let (low, high) = (low.min(high), low.max(high));
    let mut state = suppress(gx, gy, low, high);
    hysteresis(&mut state, gx.width() as usize);
    let data = state
        .iter()
        .map(|s| if *s == STRONG { 255 } else { 0 })
        .collect();
    GrayImage::from_raw(gx.width(), gx.height(), data).expect("one byte per pixel")
}

/// Canny edges of `image`: Gaussian smoothing (sigma 1.4), Sobel, non-maximum suppression, hysteresis.
#[must_use]
pub fn canny(image: &GrayImage, low: f32, high: f32) -> GrayImage {
    let smooth = gaussian_blur(image, SIGMA);
    canny_from_gradients(&sobel_x(&smooth), &sobel_y(&smooth), low, high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;
    use rstest::{fixture, rstest};

    /// A bright disc and a bright square on a dark background, with a gentle gradient.
    #[fixture]
    fn shapes() -> GrayImage {
        GrayImage::from_fn(96, 80, |x, y| {
            let (dx, dy) = (f32::from(x as u8) - 30.0, f32::from(y as u8) - 38.0);
            let disc = dx * dx + dy * dy < 22.0 * 22.0;
            let square = (56..88).contains(&x) && (20..60).contains(&y);
            Luma([if disc || square {
                210
            } else {
                30 + (x / 4) as u8
            }])
        })
    }

    /// Share of the edge pixels of `a` that have an edge pixel of `b` within one pixel (Chebyshev distance).
    fn near(a: &GrayImage, b: &GrayImage) -> (usize, usize) {
        let (w, h) = a.dimensions();
        let has_edge_near = |x: u32, y: u32| {
            (y.saturating_sub(1)..=(y + 1).min(h - 1)).any(|v| {
                (x.saturating_sub(1)..=(x + 1).min(w - 1)).any(|u| b.get_pixel(u, v).0[0] > 0)
            })
        };
        let edges: Vec<_> = a
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0[0] > 0)
            .collect();
        (
            edges
                .iter()
                .filter(|(x, y, _)| has_edge_near(*x, *y))
                .count(),
            edges.len(),
        )
    }

    #[rstest]
    fn the_edges_follow_those_of_imageproc_within_a_pixel(shapes: GrayImage) {
        let image = shapes;
        let ours = canny(&image, 40.0, 120.0);
        let theirs = imageproc::edges::canny(&image, 40.0, 120.0);
        for (a, b) in [(&ours, &theirs), (&theirs, &ours)] {
            let (matched, total) = near(a, b);
            assert!(total > 100, "edges were found: {total}");
            assert!(
                matched as f32 / total as f32 > 0.9,
                "agreement {matched}/{total}"
            );
        }
    }

    #[test]
    fn flat_pictures_and_tiny_pictures_have_no_edges() {
        for (w, h) in [(20, 20), (2, 2), (1, 5), (0, 0)] {
            let edges = canny(&GrayImage::from_pixel(w, h, Luma([90])), 10.0, 30.0);
            assert!(edges.iter().all(|v| *v == 0));
        }
    }

    #[rstest]
    fn higher_thresholds_never_find_more_and_swapped_ones_are_reordered(shapes: GrayImage) {
        let image = shapes;
        let count = |low, high| canny(&image, low, high).iter().filter(|v| **v > 0).count();
        assert!(count(40.0, 120.0) >= count(80.0, 240.0));
        assert_eq!(count(120.0, 40.0), count(40.0, 120.0));
        assert!(count(0.0, 0.0) >= count(40.0, 120.0));
    }

    #[rstest]
    #[case(100, 10, (-1, 1))]
    #[case(10, 100, (-8, 8))]
    #[case(60, 60, (-9, 9))]
    #[case(60, -60, (-7, 7))]
    #[case(-60, -60, (-9, 9))]
    fn the_gradient_direction_picks_the_comparison_neighbours(
        #[case] gx: i16,
        #[case] gy: i16,
        #[case] want: (isize, isize),
    ) {
        assert_eq!(neighbours(gx, gy, 8), want);
    }

    #[test]
    fn weak_edges_survive_only_next_to_strong_ones() {
        let mut state = vec![NONE; 25];
        state[6] = STRONG;
        state[7] = WEAK;
        state[8] = WEAK;
        state[20] = WEAK;
        hysteresis(&mut state, 5);
        assert_eq!((state[7], state[8], state[20]), (STRONG, STRONG, WEAK));
    }
}

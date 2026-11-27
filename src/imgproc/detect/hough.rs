//! Hough transforms: straight lines (standard) and circles (gradient method), following OpenCV.
//!
//! Lines vote with precomputed sine and cosine tables at one pixel and one degree resolution and keep the
//! accumulator cells that are local maxima of their four neighbours. Circles let every edge pixel vote only
//! along its gradient direction, which turns the three-dimensional search into a two-dimensional one; the
//! radius of each centre is then read from the distances to the edge pixels around it. Gradient directions
//! are taken from a Gaussian-smoothed copy (sigma 2).

use super::canny::canny;
use crate::imgproc::blur::gaussian_blur;
use crate::imgproc::gradient::{sobel_x, sobel_y};
use image::GrayImage;

/// Lines `(rho, theta)` supported by more than `threshold` edge pixels (non-zero pixels of `edges`), strongest
/// first. `rho` is in pixels and `theta` in radians within `0..pi`.
#[must_use]
pub fn hough_lines(edges: &GrayImage, threshold: u32) -> Vec<(f32, f32)> {
    let (w, h) = (edges.width() as usize, edges.height() as usize);
    let angles = 180usize;
    let rho_count = (w + h) * 2 + 1;
    let (sin, cos): (Vec<f32>, Vec<f32>) = (0..angles)
        .map(|n| (n as f32).to_radians().sin_cos())
        .unzip();
    let stride = rho_count + 2;
    let mut accumulator = vec![0u32; (angles + 2) * stride];
    for i in (0..edges.as_raw().len()).filter(|&i| edges.as_raw()[i] != 0) {
        let (x, y) = ((i % w) as f32, (i / w) as f32);
        for n in 0..angles {
            let r = (x * cos[n] + y * sin[n]).round() as isize + ((rho_count - 1) / 2) as isize;
            accumulator[(n + 1) * stride + r as usize + 1] += 1;
        }
    }
    let mut found: Vec<(u32, f32, f32)> = Vec::new();
    for n in 0..angles {
        for r in 0..rho_count {
            let base = (n + 1) * stride + r + 1;
            let v = accumulator[base];
            if v > threshold
                && v > accumulator[base - 1]
                && v >= accumulator[base + 1]
                && v > accumulator[base - stride]
                && v >= accumulator[base + stride]
            {
                found.push((
                    v,
                    r as f32 - ((rho_count - 1) / 2) as f32,
                    (n as f32).to_radians(),
                ));
            }
        }
    }
    found.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.total_cmp(&b.2))
    });
    found
        .into_iter()
        .map(|(_, rho, theta)| (rho, theta))
        .collect()
}

/// A circle found by [`hough_circles`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    /// Centre column.
    pub x: f32,
    /// Centre row.
    pub y: f32,
    /// Radius in pixels.
    pub radius: f32,
}

/// Settings for [`hough_circles`].
#[derive(Clone, Copy, Debug)]
pub struct CircleParams {
    /// Smallest radius searched.
    pub min_radius: u32,
    /// Largest radius searched.
    pub max_radius: u32,
    /// Smallest distance between two centres.
    pub min_distance: f32,
    /// High Canny threshold (the low one is half of it).
    pub canny_high: f32,
    /// Votes a centre needs, and edge pixels a radius needs.
    pub votes: u32,
}

/// Centre candidates: accumulator cells above `votes` that are local maxima, strongest first.
fn centres(accumulator: &[u32], w: usize, h: usize, votes: u32) -> Vec<(u32, usize, usize)> {
    let mut found = Vec::new();
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let i = y * w + x;
            let v = accumulator[i];
            if v > votes
                && v > accumulator[i - 1]
                && v >= accumulator[i + 1]
                && v > accumulator[i - w]
                && v >= accumulator[i + w]
            {
                found.push((v, x, y));
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0).then((a.2, a.1).cmp(&(b.2, b.1))));
    found
}

/// The radius with the most edge pixels (counting its two neighbouring integer radii) around `centre`.
fn best_radius(
    edges: &[(u32, u32)],
    centre: (usize, usize),
    p: &CircleParams,
) -> Option<(f32, u32)> {
    let mut bins = vec![0u32; p.max_radius as usize + 3];
    for &(x, y) in edges {
        let d =
            ((x as f32 - centre.0 as f32).powi(2) + (y as f32 - centre.1 as f32).powi(2)).sqrt();
        if d >= p.min_radius as f32 - 1.0 && d <= p.max_radius as f32 + 1.0 {
            bins[(d.round() as usize).min(p.max_radius as usize + 2)] += 1;
        }
    }
    (p.min_radius as usize..=p.max_radius as usize)
        .map(|r| (r, bins[r - 1] + bins[r] + bins[r + 1]))
        .max_by_key(|(r, count)| (*count, std::cmp::Reverse(*r)))
        .filter(|(_, count)| *count >= p.votes)
        .map(|(r, count)| (r as f32, count))
}

/// Adds one vote per radius along the gradient line of the edge pixel at `(x, y)`, in both directions.
fn vote(
    accumulator: &mut [u32],
    size: (usize, usize),
    at: (usize, usize),
    direction: (f32, f32),
    p: &CircleParams,
) {
    let (w, h) = size;
    for sign in [1.0f32, -1.0] {
        for r in p.min_radius..=p.max_radius {
            let cx = (at.0 as f32 + sign * r as f32 * direction.0).round();
            let cy = (at.1 as f32 + sign * r as f32 * direction.1).round();
            if cx >= 0.0 && cy >= 0.0 && (cx as usize) < w && (cy as usize) < h {
                accumulator[cy as usize * w + cx as usize] += 1;
            }
        }
    }
}

/// The centre accumulator and the list of edge pixels of `image`.
fn accumulate(image: &GrayImage, p: &CircleParams) -> (Vec<u32>, Vec<(u32, u32)>) {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let edges = canny(image, p.canny_high / 2.0, p.canny_high);
    // Directions from a smoothed copy: on a raw rasterised circle the 3x3 gradient angle is too coarse for the votes to meet.
    let smooth = gaussian_blur(image, 2.0);
    let (gx, gy) = (sobel_x(&smooth), sobel_y(&smooth));
    let mut accumulator = vec![0u32; w * h];
    let mut points = Vec::new();
    for i in (0..edges.as_raw().len()).filter(|&i| edges.as_raw()[i] != 0) {
        let (x, y) = (i % w, i / w);
        points.push((x as u32, y as u32));
        let (dx, dy) = (f32::from(gx.as_raw()[i]), f32::from(gy.as_raw()[i]));
        // A zero gradient (which cannot happen on a Canny edge) would vote on the pixel itself.
        let norm = dx.hypot(dy).max(f32::MIN_POSITIVE);
        vote(&mut accumulator, (w, h), (x, y), (dx / norm, dy / norm), p);
    }
    (accumulator, points)
}

/// Circles of `image` by the gradient method; strongest centres first, no two closer than `min_distance`.
///
/// # Panics
///
/// Panics when `min_radius` is 0 or larger than `max_radius`.
#[must_use]
pub fn hough_circles(image: &GrayImage, p: &CircleParams) -> Vec<Circle> {
    assert!(
        p.min_radius > 0 && p.min_radius <= p.max_radius,
        "need 0 < min_radius <= max_radius"
    );
    let (w, h) = (image.width() as usize, image.height() as usize);
    let (accumulator, points) = accumulate(image, p);
    let mut circles: Vec<Circle> = Vec::new();
    for (_, x, y) in centres(&accumulator, w, h, p.votes) {
        let apart = circles
            .iter()
            .all(|c| (c.x - x as f32).hypot(c.y - y as f32) >= p.min_distance);
        let radius = apart.then(|| best_radius(&points, (x, y), p)).flatten();
        circles.extend(radius.map(|(radius, _)| Circle {
            x: x as f32,
            y: y as f32,
            radius,
        }));
    }
    circles
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    fn line_picture() -> GrayImage {
        let mut image = GrayImage::new(80, 60);
        for x in 0..80 {
            image.put_pixel(x, 30, Luma([255]));
        }
        for y in 0..60 {
            image.put_pixel(20, y, Luma([255]));
        }
        image
    }

    #[test]
    fn a_horizontal_and_a_vertical_line_are_found() {
        let lines = hough_lines(&line_picture(), 50);
        let near = |rho: f32, theta: f32| {
            lines
                .iter()
                .any(|(r, t)| (r - rho).abs() <= 1.0 && (t - theta).abs() < 0.02)
        };
        assert!(near(30.0, std::f32::consts::FRAC_PI_2), "{lines:?}");
        assert!(near(20.0, 0.0), "{lines:?}");
        assert!(hough_lines(&line_picture(), 500).is_empty());
        assert!(hough_lines(&GrayImage::new(0, 0), 1).is_empty());
    }

    /// Bright discs with soft edges on a dark background.
    fn discs(circles: &[(f32, f32, f32)]) -> GrayImage {
        GrayImage::from_fn(140, 110, |x, y| {
            let inside = circles
                .iter()
                .any(|(cx, cy, r)| ((x as f32 - cx).hypot(y as f32 - cy)) <= *r);
            Luma([if inside { 220 } else { 20 }])
        })
    }

    #[test]
    fn two_discs_are_found_with_their_radii() {
        let found = hough_circles(
            &discs(&[(40.0, 50.0, 20.0), (100.0, 60.0, 28.0)]),
            &CircleParams {
                min_radius: 15,
                max_radius: 35,
                min_distance: 20.0,
                canny_high: 100.0,
                votes: 60,
            },
        );
        let has = |cx: f32, cy: f32, r: f32| {
            found.iter().any(|c| {
                (c.x - cx).abs() <= 2.0 && (c.y - cy).abs() <= 2.0 && (c.radius - r).abs() <= 2.0
            })
        };
        assert!(has(40.0, 50.0, 20.0), "{found:?}");
        assert!(has(100.0, 60.0, 28.0), "{found:?}");
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn flat_pictures_have_no_circles_and_bad_radii_panic() {
        let params = CircleParams {
            min_radius: 5,
            max_radius: 9,
            min_distance: 5.0,
            canny_high: 60.0,
            votes: 10,
        };
        assert!(hough_circles(&GrayImage::new(30, 30), &params).is_empty());
        let result = std::panic::catch_unwind(|| {
            hough_circles(
                &GrayImage::new(5, 5),
                &CircleParams {
                    min_radius: 0,
                    ..params
                },
            )
        });
        assert!(result.is_err());
    }
}

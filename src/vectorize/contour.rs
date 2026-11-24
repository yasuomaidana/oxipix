//! Contour boundary tracing, polyline simplification, and SVG path synthesis.

use crate::luma::rgba_to_luma_vec;
use image::RgbaImage;

/// Single vectorized contour path.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgVectorPath {
    /// Control and vertex points along the traced contour.
    pub points: Vec<(f32, f32)>,
    /// Whether the contour forms a closed geometric loop.
    pub is_closed: bool,
}

impl SvgVectorPath {
    /// Serializes this vectorized contour into an SVG path specification `d="..."`.
    #[must_use]
    pub fn to_svg_path_data(&self) -> String {
        if self.points.is_empty() {
            return String::new();
        }

        let mut d = String::new();
        for (i, pt) in self.points.iter().enumerate() {
            if i == 0 {
                d.push_str(&format!("M {:.2} {:.2}", pt.0, pt.1));
            } else {
                d.push_str(&format!(" L {:.2} {:.2}", pt.0, pt.1));
            }
        }

        if self.is_closed {
            d.push_str(" Z");
        }

        d
    }
}

/// Vectorized graphic representation containing traced contour paths.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorizedGraphic {
    /// Pixel width of original raster.
    pub width: u32,
    /// Pixel height of original raster.
    pub height: u32,
    /// Vectorized SVG paths.
    pub paths: Vec<SvgVectorPath>,
}

impl VectorizedGraphic {
    /// Synthesizes a standalone SVG document representation.
    #[must_use]
    pub fn to_svg_string(&self) -> String {
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}">"#,
            self.width, self.height
        );
        svg.push('\n');

        for path in &self.paths {
            let d = path.to_svg_path_data();
            if !d.is_empty() {
                svg.push_str(&format!(
                    r#"  <path d="{d}" fill="none" stroke="black" stroke-width="1.5" />"#
                ));
                svg.push('\n');
            }
        }

        svg.push_str("</svg>");
        svg
    }
}

/// Operational parameters for vectorization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VectorizeConfig {
    /// Target grid downsampling factor for contour extraction (default: 4.0).
    pub grid_step: f32,
    /// RDP simplification tolerance (epsilon in pixels, default: 2.0).
    pub rdp_epsilon: f32,
}

impl Default for VectorizeConfig {
    fn default() -> Self {
        Self {
            grid_step: 4.0,
            rdp_epsilon: 2.0,
        }
    }
}

/// Vectorizes an RGBA image into an SVG graphic using Otsu thresholding,
/// Moore-Neighbor contour tracing, and RDP simplification.
#[must_use]
pub fn vectorize_rgba_to_svg(img: &RgbaImage, config: VectorizeConfig) -> VectorizedGraphic {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return VectorizedGraphic {
            width,
            height,
            paths: Vec::new(),
        };
    }

    let gray = rgba_to_luma_vec(img);

    let thresh = otsu_threshold(&gray);
    let step = config.grid_step.max(1.0);
    let grid_w = ((width as f32 / step).ceil() as usize).max(1);
    let grid_h = ((height as f32 / step).ceil() as usize).max(1);

    let mut binary_grid = vec![false; grid_w * grid_h];
    for gy in 0..grid_h {
        let py = ((gy as f32 * step) as u32).min(height - 1);
        for gx in 0..grid_w {
            let px = ((gx as f32 * step) as u32).min(width - 1);
            let val = gray[(py * width + px) as usize];
            if val <= thresh {
                binary_grid[gy * grid_w + gx] = true;
            }
        }
    }

    let raw_contours = trace_moore_neighbor_contours(&binary_grid, grid_w, grid_h, step, step);

    let mut paths = Vec::with_capacity(raw_contours.len());
    for raw in raw_contours {
        let simplified = simplify_rdp(&raw, config.rdp_epsilon);
        if simplified.len() >= 3 {
            paths.push(SvgVectorPath {
                points: simplified,
                is_closed: true,
            });
        }
    }

    VectorizedGraphic {
        width,
        height,
        paths,
    }
}

/// Calculates the optimal global binarization threshold via Otsu's method.
#[must_use]
pub fn otsu_threshold(gray: &[u8]) -> u8 {
    if gray.is_empty() {
        return 128;
    }

    let mut hist = [0u64; 256];
    for &b in gray {
        hist[b as usize] += 1;
    }

    let total = gray.len() as f64;
    let mut sum = 0.0;
    for (t, count) in hist.iter().enumerate() {
        sum += (t as f64) * (*count as f64);
    }

    let mut sum_b = 0.0;
    let mut w_b = 0.0;
    let mut max_variance = 0.0;
    let mut best_thresh = 128u8;

    for (t, count) in hist.iter().enumerate() {
        w_b += *count as f64;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }

        sum_b += (t as f64) * (*count as f64);
        let m_b = sum_b / w_b;
        let m_f = (sum - sum_b) / w_f;

        let between_variance = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if between_variance > max_variance {
            max_variance = between_variance;
            best_thresh = t as u8;
        }
    }

    best_thresh
}

/// Neighbour offsets in the order the Moore tracer probes them, starting north and going clockwise.
const MOORE_DIRS: [(isize, isize); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// A contour is cut off after this many steps (and never exceeds the grid's cell count).
const MAX_CONTOUR_STEPS: usize = 1000;

/// Row-major boolean grid.
struct Grid<'a> {
    cells: &'a [bool],
    width: usize,
    height: usize,
}

impl Grid<'_> {
    fn is_set(&self, x: usize, y: usize) -> bool {
        self.cells[y * self.width + x]
    }

    /// A set cell on the grid edge or with an unset 4-neighbour.
    fn is_boundary(&self, x: usize, y: usize) -> bool {
        let on_edge = x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height;
        on_edge
            || !self.is_set(x - 1, y)
            || !self.is_set(x + 1, y)
            || !self.is_set(x, y - 1)
            || !self.is_set(x, y + 1)
    }

    /// First set neighbour of `(x, y)` probing clockwise from `dir`. Returns it with the direction
    /// to resume probing from.
    fn next_neighbor(&self, (x, y): (usize, usize), dir: usize) -> Option<((usize, usize), usize)> {
        (0..MOORE_DIRS.len()).find_map(|step| {
            let probe = (dir + step) % MOORE_DIRS.len();
            let nx = usize::try_from(x as isize + MOORE_DIRS[probe].0).ok()?;
            let ny = usize::try_from(y as isize + MOORE_DIRS[probe].1).ok()?;
            let in_bounds = nx < self.width && ny < self.height;
            (in_bounds && self.is_set(nx, ny)).then_some(((nx, ny), (probe + 6) % MOORE_DIRS.len()))
        })
    }

    /// Walks the boundary from `start`, marking every cell it visits, until it returns to `start`,
    /// runs out of neighbours or hits the step limit.
    fn trace(
        &self,
        start: (usize, usize),
        scale: (f32, f32),
        visited: &mut [bool],
    ) -> Vec<(f32, f32)> {
        let mut contour = Vec::new();
        let mut at = start;
        let mut dir = 0usize;

        for _ in 0..(self.width * self.height).min(MAX_CONTOUR_STEPS) {
            visited[at.1 * self.width + at.0] = true;
            contour.push((at.0 as f32 * scale.0, at.1 as f32 * scale.1));

            let Some((next, next_dir)) = self.next_neighbor(at, dir) else {
                break;
            };
            (at, dir) = (next, next_dir);
            if at == start && contour.len() > 2 {
                break;
            }
        }
        contour
    }
}

/// Traces contours across a downsampled 2D boolean grid using the Moore-Neighbor algorithm.
#[must_use]
pub fn trace_moore_neighbor_contours(
    grid: &[bool],
    gw: usize,
    gh: usize,
    sx: f32,
    sy: f32,
) -> Vec<Vec<(f32, f32)>> {
    let grid = Grid {
        cells: grid,
        width: gw,
        height: gh,
    };
    let mut visited = vec![false; gw * gh];
    let mut contours = Vec::new();

    for y in 0..gh {
        for x in 0..gw {
            if !grid.is_set(x, y) || visited[y * gw + x] || !grid.is_boundary(x, y) {
                continue;
            }
            let contour = grid.trace((x, y), (sx, sy), &mut visited);
            if contour.len() >= 3 {
                contours.push(contour);
            }
        }
    }

    contours
}

/// Simplifies a polygonal chain of 2D points using the Ramer-Douglas-Peucker (RDP) algorithm.
#[must_use]
pub fn simplify_rdp(points: &[(f32, f32)], epsilon: f32) -> Vec<(f32, f32)> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut max_dist = 0.0f32;
    let mut index = 0usize;
    let start = points[0];
    let end = points[points.len() - 1];

    for (i, &pt) in points.iter().enumerate().skip(1).take(points.len() - 2) {
        let dist = perpendicular_distance(pt, start, end);
        if dist > max_dist {
            max_dist = dist;
            index = i;
        }
    }

    if max_dist > epsilon {
        let mut rec1 = simplify_rdp(&points[0..=index], epsilon);
        let rec2 = simplify_rdp(&points[index..], epsilon);
        rec1.pop();
        rec1.extend(rec2);
        rec1
    } else {
        vec![start, end]
    }
}

/// Computes perpendicular distance from point `pt` to the line segment between `l1` and `l2`.
#[must_use]
pub fn perpendicular_distance(pt: (f32, f32), l1: (f32, f32), l2: (f32, f32)) -> f32 {
    let dx = l2.0 - l1.0;
    let dy = l2.1 - l1.1;
    let line_len_sq = dx * dx + dy * dy;

    if line_len_sq < 1e-6 {
        let px = pt.0 - l1.0;
        let py = pt.1 - l1.1;
        return (px * px + py * py).sqrt();
    }

    let num = ((l2.0 - l1.0) * (l1.1 - pt.1) - (l1.0 - pt.0) * (l2.1 - l1.1)).abs();
    num / line_len_sq.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn square_on_white() -> RgbaImage {
        RgbaImage::from_fn(64, 64, |x, y| {
            if (16..48).contains(&x) && (16..48).contains(&y) {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        })
    }

    #[test]
    fn otsu_separates_two_levels() {
        let mut gray = vec![20u8; 500];
        gray.extend(vec![220u8; 500]);
        let t = otsu_threshold(&gray);
        assert!((20..220).contains(&t), "threshold {t}");
    }

    #[test]
    fn square_is_traced_to_one_closed_path() {
        let g = vectorize_rgba_to_svg(
            &square_on_white(),
            VectorizeConfig {
                grid_step: 1.0,
                rdp_epsilon: 1.5,
            },
        );
        assert_eq!(g.paths.len(), 1);
        assert!(g.paths[0].is_closed);
        assert!(g.paths[0].points.len() <= 8, "RDP should reduce a square");
        assert!(g.to_svg_string().contains("<path"));
    }

    #[test]
    fn rdp_collapses_collinear_points() {
        let pts = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)];
        assert_eq!(simplify_rdp(&pts, 0.1).len(), 2);
    }

    #[test]
    fn an_empty_path_serializes_to_nothing_and_is_left_out_of_the_document() {
        let empty = SvgVectorPath {
            points: Vec::new(),
            is_closed: true,
        };
        assert_eq!(empty.to_svg_path_data(), "");
        let graphic = VectorizedGraphic {
            width: 4,
            height: 4,
            paths: vec![empty],
        };
        assert!(!graphic.to_svg_string().contains("<path"));
    }

    #[test]
    fn an_open_path_has_no_close_command() {
        let open = SvgVectorPath {
            points: vec![(0.0, 0.0), (1.5, 2.0)],
            is_closed: false,
        };
        assert_eq!(open.to_svg_path_data(), "M 0.00 0.00 L 1.50 2.00");
    }

    #[test]
    fn a_single_set_cell_yields_no_contour() {
        let mut grid = vec![false; 9];
        grid[4] = true;
        assert!(trace_moore_neighbor_contours(&grid, 3, 3, 1.0, 1.0).is_empty());
    }

    #[test]
    fn empty_image_yields_no_paths() {
        let g = vectorize_rgba_to_svg(&RgbaImage::new(0, 0), VectorizeConfig::default());
        assert!(g.paths.is_empty());
    }
}

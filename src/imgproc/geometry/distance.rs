//! Distance transforms of a binary mask: for every non-zero pixel, the distance to the nearest zero pixel.
//!
//! The Euclidean transform is exact (Felzenszwalb and Huttenlocher's lower-envelope algorithm, two 1-D passes);
//! the city-block and chessboard transforms are two-pass chamfer sweeps. Mask pixels that are zero have
//! distance 0. A mask without a zero pixel has no distances: every value is a very large number.

use image::GrayImage;

const FAR: f32 = 1e20;

/// Squared distance to the nearest zero of `f` (0 where `f` is 0, `FAR` elsewhere) along one line: the lower
/// envelope of the parabolas `(q - p)^2 + f[p]`.
fn envelope(f: &[f32], out: &mut [f32], v: &mut Vec<usize>, z: &mut Vec<f32>) {
    let n = f.len();
    v.clear();
    v.resize(n, 0);
    z.clear();
    z.resize(n + 1, 0.0);
    z[0] = -FAR;
    z[1] = FAR;
    let mut k = 0usize;
    for q in 1..n {
        let s = loop {
            let p = v[k];
            let s =
                ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
            if s > z[k] || k == 0 {
                break s;
            }
            k -= 1;
        };
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = FAR;
    }
    let mut k = 0;
    for (q, slot) in out.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        *slot = (q as f32 - v[k] as f32).powi(2) + f[v[k]];
    }
}

/// Exact Euclidean distance transform, row-major `f32` values.
#[must_use]
pub fn euclidean(mask: &GrayImage) -> Vec<f32> {
    let (w, h) = (mask.width() as usize, mask.height() as usize);
    let mut grid: Vec<f32> = mask
        .as_raw()
        .iter()
        .map(|p| if *p == 0 { 0.0 } else { FAR })
        .collect();
    let (mut v, mut z) = (Vec::new(), Vec::new());
    let mut line = vec![0.0f32; h.max(w)];
    let mut result = vec![0.0f32; h.max(w)];
    for x in 0..w {
        for y in 0..h {
            line[y] = grid[y * w + x];
        }
        envelope(&line[..h], &mut result[..h], &mut v, &mut z);
        for y in 0..h {
            grid[y * w + x] = result[y];
        }
    }
    for row in grid.chunks_exact_mut(w.max(1)) {
        line[..w].copy_from_slice(row);
        envelope(&line[..w], &mut result[..w], &mut v, &mut z);
        row.copy_from_slice(&result[..w]);
    }
    grid.iter().map(|d| d.sqrt()).collect()
}

/// One chamfer sweep over `d` (row-major, `size = (width, height)`): each cell becomes the smaller of itself and
/// one more than its neighbours at `offsets`, visiting the cells in reading order or, if `reverse`, backwards.
fn sweep(d: &mut [u32], size: (usize, usize), offsets: &[(isize, isize)], reverse: bool) {
    let (w, h) = size;
    for n in 0..w * h {
        let i = if reverse { w * h - 1 - n } else { n };
        let (x, y) = ((i % w) as isize, (i / w) as isize);
        let neighbours = offsets.iter().filter_map(|(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            (nx >= 0 && ny >= 0 && nx < w as isize && ny < h as isize)
                .then(|| d[ny as usize * w + nx as usize] + 1)
        });
        if let Some(best) = neighbours.min() {
            d[i] = d[i].min(best);
        }
    }
}

/// Two-pass chamfer transform with unit steps; `diagonal` adds the diagonal neighbours (chessboard distance).
fn chamfer(mask: &GrayImage, diagonal: bool) -> Vec<f32> {
    let size = (mask.width() as usize, mask.height() as usize);
    let mut d: Vec<u32> = mask
        .as_raw()
        .iter()
        .map(|p| if *p == 0 { 0 } else { u32::MAX / 2 })
        .collect();
    let forward: &[(isize, isize)] = if diagonal {
        &[(-1, 0), (0, -1), (-1, -1), (1, -1)]
    } else {
        &[(-1, 0), (0, -1)]
    };
    let backward: Vec<(isize, isize)> = forward.iter().map(|(dx, dy)| (-dx, -dy)).collect();
    sweep(&mut d, size, forward, false);
    sweep(&mut d, size, &backward, true);
    d.iter().map(|v| *v as f32).collect()
}

/// City-block (L1) distance transform.
#[must_use]
pub fn city_block(mask: &GrayImage) -> Vec<f32> {
    chamfer(mask, false)
}

/// Chessboard (L-infinity) distance transform.
#[must_use]
pub fn chessboard(mask: &GrayImage) -> Vec<f32> {
    chamfer(mask, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use image::Luma;
    use rstest::rstest;

    /// A mask with scattered zero pixels, so most pixels are foreground.
    fn mask(w: u32, h: u32, seed: u32) -> GrayImage {
        let n = noise(w, h, seed);
        GrayImage::from_fn(w, h, |x, y| {
            Luma([if n.get_pixel(x, y).0[0] < 12 { 0 } else { 255 }])
        })
    }

    /// Brute-force distance from every pixel to the nearest zero pixel under `metric`.
    fn brute(mask: &GrayImage, metric: impl Fn(f32, f32) -> f32) -> Vec<f32> {
        let zeros: Vec<(f32, f32)> = mask
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0[0] == 0)
            .map(|(x, y, _)| (x as f32, y as f32))
            .collect();
        mask.enumerate_pixels()
            .map(|(x, y, _)| {
                zeros
                    .iter()
                    .map(|(a, b)| metric((x as f32 - a).abs(), (y as f32 - b).abs()))
                    .fold(f32::INFINITY, f32::min)
            })
            .collect()
    }

    #[rstest]
    #[case(23, 17, 1)]
    #[case(40, 9, 2)]
    #[case(1, 6, 3)]
    fn every_transform_equals_the_brute_force_distance(
        #[case] w: u32,
        #[case] h: u32,
        #[case] seed: u32,
    ) {
        let m = mask(w, h, seed);
        if !m.contains(&0) {
            return;
        }
        let close = |a: &[f32], b: &[f32]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3);
        assert!(close(
            &euclidean(&m),
            &brute(&m, |dx, dy| (dx * dx + dy * dy).sqrt())
        ));
        assert!(close(&city_block(&m), &brute(&m, |dx, dy| dx + dy)));
        assert!(close(&chessboard(&m), &brute(&m, f32::max)));
    }

    #[test]
    fn a_mask_without_background_is_infinitely_far_and_empty_masks_are_empty() {
        assert!(
            euclidean(&GrayImage::from_pixel(4, 4, Luma([255])))
                .iter()
                .all(|d| *d > 1e9)
        );
        assert!(euclidean(&GrayImage::new(0, 0)).is_empty());
        assert!(city_block(&GrayImage::new(0, 0)).is_empty());
    }
}

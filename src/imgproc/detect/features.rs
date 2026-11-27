//! FAST-9 corners, BRIEF descriptors and Hamming matching.
//!
//! FAST rejects most pixels with a test on the four compass pixels of the 16-pixel ring, then finds a run of
//! nine bright (or dark) pixels around the circle with a bit mask, so arcs that wrap from index 15 back to 0
//! count. Corners are thinned with 3x3 non-max suppression. BRIEF compares 5x5 box means at fixed pairs of
//! points inside a 31x31 patch, read from an integral image, and packs the 256 results into four words.

use image::GrayImage;

/// The 16 ring offsets `(dx, dy)` of radius 3, clockwise from the bottom.
const RING: [(isize, isize); 16] = [
    (0, 3),
    (1, 3),
    (2, 2),
    (3, 1),
    (3, 0),
    (3, -1),
    (2, -2),
    (1, -3),
    (0, -3),
    (-1, -3),
    (-2, -2),
    (-3, -1),
    (-3, 0),
    (-3, 1),
    (-2, 2),
    (-1, 3),
];

/// Keypoints must be this many pixels inside the picture for the 31x31 patch to fit.
pub const MARGIN: u32 = 17;

/// A detected corner and its strength (sum of the ring differences above the threshold).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Corner {
    /// Column.
    pub x: u32,
    /// Row.
    pub y: u32,
    /// Corner strength.
    pub score: u32,
}

/// `true` when `mask` (16 bits, circular) has nine consecutive set bits.
fn has_run(mask: u32) -> bool {
    let doubled = mask | (mask << 16);
    (1..9).fold(doubled, |run, shift| run & (doubled >> shift)) & 0xFFFF != 0
}

/// Corner score of the pixel at index `i`, or 0 when it is not a FAST-9 corner.
fn corner_score(data: &[u8], i: usize, offsets: &[isize; 16], threshold: i32) -> u32 {
    let centre = i32::from(data[i]);
    let ring: [i32; 16] =
        std::array::from_fn(|k| i32::from(data[i.wrapping_add_signed(offsets[k])]));
    let bright = [0, 4, 8, 12]
        .iter()
        .filter(|&&k| ring[k] > centre + threshold)
        .count();
    let dark = [0, 4, 8, 12]
        .iter()
        .filter(|&&k| ring[k] < centre - threshold)
        .count();
    if bright < 2 && dark < 2 {
        return 0;
    }
    let (mut bright_mask, mut dark_mask) = (0u32, 0u32);
    for (k, v) in ring.iter().enumerate() {
        bright_mask |= u32::from(*v > centre + threshold) << k;
        dark_mask |= u32::from(*v < centre - threshold) << k;
    }
    let sum = |mask: u32| {
        ring.iter()
            .enumerate()
            .filter(|(k, _)| mask >> k & 1 == 1)
            .map(|(_, v)| (v - centre).unsigned_abs() - threshold as u32)
            .sum::<u32>()
    };
    match (has_run(bright_mask), has_run(dark_mask)) {
        (true, _) => sum(bright_mask),
        (_, true) => sum(dark_mask),
        _ => 0,
    }
}

/// FAST-9 corners with a ring difference above `threshold`, before non-max suppression.
#[must_use]
pub fn fast9(image: &GrayImage, threshold: u8) -> Vec<Corner> {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let offsets: [isize; 16] = std::array::from_fn(|k| RING[k].0 + RING[k].1 * w as isize);
    let mut corners = Vec::new();
    for y in 3..h.saturating_sub(3) {
        for x in 3..w.saturating_sub(3) {
            let score = corner_score(image.as_raw(), y * w + x, &offsets, i32::from(threshold));
            if score > 0 {
                corners.push(Corner {
                    x: x as u32,
                    y: y as u32,
                    score,
                });
            }
        }
    }
    corners
}

/// Keeps the corners that beat all eight neighbours (equal scores resolved by position).
#[must_use]
pub fn suppress_non_max(corners: &[Corner], width: u32, height: u32) -> Vec<Corner> {
    let mut plane = vec![0u32; width as usize * height as usize];
    corners
        .iter()
        .for_each(|c| plane[(c.y * width + c.x) as usize] = c.score);
    let beats = |c: &Corner| {
        (-1i64..=1)
            .flat_map(|dy| (-1i64..=1).map(move |dx| (dx, dy)))
            .filter(|d| *d != (0, 0))
            .all(|(dx, dy)| {
                let (nx, ny) = (i64::from(c.x) + dx, i64::from(c.y) + dy);
                if nx < 0 || ny < 0 || nx >= i64::from(width) || ny >= i64::from(height) {
                    return true;
                }
                let other = plane[(ny * i64::from(width) + nx) as usize];
                other < c.score || (other == c.score && (ny, nx) > (i64::from(c.y), i64::from(c.x)))
            })
    };
    corners.iter().copied().filter(beats).collect()
}

/// The `max` strongest corners, strongest first.
#[must_use]
pub fn strongest(mut corners: Vec<Corner>, max: usize) -> Vec<Corner> {
    if corners.len() > max {
        corners.select_nth_unstable_by(max, |a, b| b.score.cmp(&a.score));
        corners.truncate(max);
    }
    corners.sort_by(|a, b| b.score.cmp(&a.score).then((a.y, a.x).cmp(&(b.y, b.x))));
    corners
}

/// A BRIEF test: two points `(x0, y0, x1, y1)` inside the 31x31 patch.
pub type Pair = (u32, u32, u32, u32);

/// A 256-bit descriptor.
pub type Descriptor = [u64; 4];

/// Summed-area table of a picture.
struct Integral {
    width: usize,
    height: usize,
    sums: Vec<u32>,
}

impl Integral {
    fn new(image: &GrayImage) -> Self {
        let (width, height) = (image.width() as usize, image.height() as usize);
        let stride = width + 1;
        let mut sums = vec![0u32; stride * (height + 1)];
        for (y, row) in image.as_raw().chunks_exact(width.max(1)).enumerate() {
            let mut run = 0u32;
            for (x, p) in row.iter().enumerate() {
                run += u32::from(*p);
                sums[(y + 1) * stride + x + 1] = sums[y * stride + x + 1] + run;
            }
        }
        Self {
            width,
            height,
            sums,
        }
    }

    /// Mean (rounded down) of the 5x5 box centred on `(x, y)`, clipped to the picture.
    fn mean5(&self, x: usize, y: usize) -> u32 {
        let (x0, y0) = (x.saturating_sub(2), y.saturating_sub(2));
        let (x1, y1) = ((x + 3).min(self.width), (y + 3).min(self.height));
        let at = |px: usize, py: usize| self.sums[py * (self.width + 1) + px];
        (at(x1, y1) + at(x0, y0) - at(x1, y0) - at(x0, y1)) / ((x1 - x0) * (y1 - y0)) as u32
    }
}

/// Descriptors of the keypoints (at least [`MARGIN`] pixels from every edge): bit `i` is set when the mean
/// around the first point of pair `i` is smaller than around the second. At most 256 pairs are used.
#[must_use]
pub fn brief(image: &GrayImage, points: &[(u32, u32)], pairs: &[Pair]) -> Vec<Descriptor> {
    let integral = Integral::new(image);
    points
        .iter()
        .map(|&(x, y)| {
            let (left, top) = (x as usize - 15 + 1, y as usize - 15 + 1);
            let mut words = [0u64; 4];
            for (i, &(x0, y0, x1, y1)) in pairs.iter().take(256).enumerate() {
                let a = integral.mean5(left + x0 as usize, top + y0 as usize);
                let b = integral.mean5(left + x1 as usize, top + y1 as usize);
                words[i / 64] |= u64::from(a < b) << (i % 64);
            }
            words
        })
        .collect()
}

/// Number of differing bits.
#[must_use]
pub fn hamming(a: &Descriptor, b: &Descriptor) -> u32 {
    a.iter().zip(b).map(|(x, y)| (x ^ y).count_ones()).sum()
}

/// For each descriptor of `query`, its nearest neighbour in `train` when within `max_distance` bits:
/// `(query index, train index, distance)`.
#[must_use]
pub fn match_nearest(
    query: &[Descriptor],
    train: &[Descriptor],
    max_distance: u32,
) -> Vec<(usize, usize, u32)> {
    query
        .iter()
        .enumerate()
        .filter_map(|(i, q)| {
            let (j, d) = train
                .iter()
                .enumerate()
                .map(|(j, t)| (j, hamming(q, t)))
                .min_by_key(|(_, d)| *d)?;
            (d <= max_distance).then_some((i, j, d))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use image::Luma;

    /// A flat grey picture with a ring of 16 pixels around `(10, 10)` set to `value` on the given indices.
    fn ring_picture(bright: impl Iterator<Item = usize>, value: u8) -> GrayImage {
        let mut image = GrayImage::from_pixel(21, 21, Luma([100]));
        for i in bright {
            image.put_pixel(
                (10 + RING[i].0) as u32,
                (10 + RING[i].1) as u32,
                Luma([value]),
            );
        }
        image
    }

    #[test]
    fn arcs_of_nine_are_found_even_when_they_wrap_around_the_ring() {
        let has_centre =
            |image: &GrayImage| fast9(image, 20).iter().any(|c| (c.x, c.y) == (10, 10));
        assert!(has_centre(&ring_picture(3..12, 200)));
        assert!(has_centre(&ring_picture((12..16).chain(0..5), 200)));
        assert!(has_centre(&ring_picture((8..16).chain(0..1), 200)));
        assert!(has_centre(&ring_picture(3..12, 20)), "dark arcs count too");
        assert!(!has_centre(&ring_picture(3..11, 200)), "eight is too few");
        assert!(
            !has_centre(&ring_picture((0..4).chain(8..12), 200)),
            "two runs of four are not one of nine"
        );
        assert!(!has_centre(&ring_picture(0..0, 200)));
        assert!(
            !has_centre(&ring_picture(3..12, 110)),
            "below the threshold"
        );
    }

    #[test]
    fn suppression_thins_clusters_and_strongest_orders_them() {
        let image = noise(64, 48, 9);
        let all = fast9(&image, 25);
        let thin = suppress_non_max(&all, 64, 48);
        assert!(!thin.is_empty() && thin.len() < all.len());
        let top = strongest(thin.clone(), 10);
        assert_eq!(top.len(), 10);
        assert!(top.windows(2).all(|w| w[0].score >= w[1].score));
        assert_eq!(strongest(thin.clone(), usize::MAX).len(), thin.len());
        let edge = [
            Corner {
                x: 0,
                y: 0,
                score: 5,
            },
            Corner {
                x: 1,
                y: 0,
                score: 5,
            },
        ];
        assert_eq!(suppress_non_max(&edge, 4, 4), vec![edge[0]]);
    }

    fn pairs() -> Vec<Pair> {
        let mut s = 0x5EED_CAFE_u32;
        let mut next = || {
            s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (s >> 16) % 31
        };
        (0..256).map(|_| (next(), next(), next(), next())).collect()
    }

    #[test]
    fn descriptors_are_shift_stable_and_matching_recovers_the_shift() {
        let image = noise(120, 100, 4);
        let shifted = image::imageops::crop_imm(&image, 12, 7, 108, 93).to_image();
        let points: Vec<(u32, u32)> = vec![(40, 40), (70, 50), (90, 30)];
        let moved: Vec<(u32, u32)> = points.iter().map(|(x, y)| (x - 12, y - 7)).collect();
        let (a, b) = (
            brief(&image, &points, &pairs()),
            brief(&shifted, &moved, &pairs()),
        );
        assert!(a.iter().zip(&b).all(|(p, q)| hamming(p, q) == 0));
        let matches = match_nearest(&a, &b, 0);
        assert_eq!(matches, vec![(0, 0, 0), (1, 1, 0), (2, 2, 0)]);
        assert!(match_nearest(&a, &[], 10).is_empty());
        assert!(match_nearest(&a, &[[u64::MAX; 4]], 3).is_empty());
        assert!(
            brief(&image, &points, &pairs()[..8])
                .iter()
                .all(|d| d[1] == 0)
        );
    }
}

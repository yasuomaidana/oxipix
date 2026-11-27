//! Grey-scale morphology with a rectangular window: erosion, dilation, opening, closing and the hats.
//!
//! A window minimum or maximum is computed with a doubling table instead of a loop over the window:
//! after `j` rounds `cur[x]` holds the extreme of `x..x + 2^j`, and one more combination of two entries
//! covers any window length. That is `O(log window)` whole-row passes, each a plain element-wise `min` or
//! `max` of two slices, which compiles to 16 byte lanes per instruction. Pixels outside the picture do
//! not take part (OpenCV's default border for `erode` and `dilate`).

use image::GrayImage;

/// Which extreme a window pass keeps.
#[derive(Clone, Copy)]
enum Op {
    Min,
    Max,
}

impl Op {
    /// The value that never wins, used to pad the picture.
    fn neutral(self) -> u8 {
        match self {
            Op::Min => u8::MAX,
            Op::Max => u8::MIN,
        }
    }

    /// `out[i] = op(a[i], b[i])` for the whole slice.
    fn zip(self, a: &[u8], b: &[u8], out: &mut [u8]) {
        let pairs = a.iter().zip(b).zip(out);
        match self {
            Op::Min => pairs.for_each(|((x, y), o)| *o = *x.min(y)),
            Op::Max => pairs.for_each(|((x, y), o)| *o = *x.max(y)),
        }
    }
}

/// Extreme over every window of `len` consecutive items of `line`, written to `out[..line.len() + 1 - len]`.
/// `cur` and `scratch` are work buffers reused between rows.
fn window(op: Op, line: &[u8], len: usize, buffers: (&mut Vec<u8>, &mut [u8]), out: &mut [u8]) {
    let (cur, scratch) = buffers;
    let count = line.len() + 1 - len;
    cur.clear();
    cur.extend_from_slice(line);
    let mut span = 1;
    while span * 2 <= len {
        let n = cur.len() - span;
        op.zip(&cur[..n], &cur[span..], &mut scratch[..n]);
        cur[..n].copy_from_slice(&scratch[..n]);
        span *= 2;
    }
    op.zip(
        &cur[..count],
        &cur[len - span..len - span + count],
        &mut out[..count],
    );
}

/// One extreme filter along the rows (`horizontal`) or columns of a `width` x `height` byte image.
fn pass(op: Op, data: &[u8], size: (usize, usize), radius: usize, horizontal: bool) -> Vec<u8> {
    let (width, height) = size;
    let len = 2 * radius + 1;
    if horizontal {
        let mut out = vec![0u8; width * height];
        let mut padded = vec![op.neutral(); width + 2 * radius];
        let mut scratch = vec![0u8; padded.len()];
        let mut cur = Vec::with_capacity(padded.len());
        for (src, dst) in data.chunks_exact(width).zip(out.chunks_exact_mut(width)) {
            padded[radius..radius + width].copy_from_slice(src);
            window(op, &padded, len, (&mut cur, &mut scratch), dst);
        }
        return out;
    }
    // Columns: treat every row as one lane, so the passes run over whole rows.
    let mut rows = vec![op.neutral(); width * (height + 2 * radius)];
    rows[radius * width..(radius + height) * width].copy_from_slice(data);
    let mut cur = rows;
    let mut scratch = vec![0u8; cur.len()];
    let mut span = 1;
    while span * 2 <= len {
        let n = (cur.len() / width - span) * width;
        op.zip(&cur[..n], &cur[span * width..], &mut scratch[..n]);
        cur[..n].copy_from_slice(&scratch[..n]);
        span *= 2;
    }
    let mut out = vec![0u8; width * height];
    let start = (len - span) * width;
    op.zip(
        &cur[..width * height],
        &cur[start..start + width * height],
        &mut out,
    );
    out
}

fn filter(op: Op, image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    let (w, h) = image.dimensions();
    if w == 0 || h == 0 {
        return image.clone();
    }
    let size = (w as usize, h as usize);
    let across = pass(op, image.as_raw(), size, rx as usize, true);
    let down = pass(op, &across, size, ry as usize, false);
    GrayImage::from_raw(w, h, down).expect("the buffer has width * height bytes")
}

/// Erosion: the minimum over the `(2 * rx + 1) x (2 * ry + 1)` window around every pixel.
#[must_use]
pub fn erode(image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    filter(Op::Min, image, rx, ry)
}

/// Dilation: the maximum over the `(2 * rx + 1) x (2 * ry + 1)` window around every pixel.
#[must_use]
pub fn dilate(image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    filter(Op::Max, image, rx, ry)
}

/// Opening: erosion then dilation; removes bright specks smaller than the window.
#[must_use]
pub fn open(image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    dilate(&erode(image, rx, ry), rx, ry)
}

/// Closing: dilation then erosion; fills dark holes smaller than the window.
#[must_use]
pub fn close(image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    erode(&dilate(image, rx, ry), rx, ry)
}

/// `a - b` per pixel, stopping at 0, written into a new image.
fn subtract(a: &GrayImage, b: &GrayImage) -> GrayImage {
    let data = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| x.saturating_sub(*y))
        .collect();
    GrayImage::from_raw(a.width(), a.height(), data).expect("same size as the inputs")
}

/// White top-hat: the picture minus its opening, the bright specks the opening removed.
#[must_use]
pub fn top_hat(image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    subtract(image, &open(image, rx, ry))
}

/// Black top-hat: the closing minus the picture, the dark holes the closing filled.
#[must_use]
pub fn black_hat(image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    subtract(&close(image, rx, ry), image)
}

/// Morphological gradient: dilation minus erosion, the outline of bright regions.
#[must_use]
pub fn gradient(image: &GrayImage, rx: u32, ry: u32) -> GrayImage {
    subtract(&dilate(image, rx, ry), &erode(image, rx, ry))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise as seeded;
    use image::Luma;
    use rstest::rstest;

    fn noise(w: u32, h: u32) -> GrayImage {
        seeded(w, h, 0x9E37_79B9)
    }

    /// Direct window extreme, ignoring the pixels outside the picture.
    fn naive(image: &GrayImage, rx: u32, ry: u32, min: bool) -> GrayImage {
        GrayImage::from_fn(image.width(), image.height(), |x, y| {
            let (w, h) = image.dimensions();
            let window = (y.saturating_sub(ry)..=(y + ry).min(h - 1))
                .flat_map(|v| (x.saturating_sub(rx)..=(x + rx).min(w - 1)).map(move |u| (u, v)));
            let values = window.map(|(u, v)| image.get_pixel(u, v).0[0]);
            Luma([if min { values.min() } else { values.max() }.unwrap()])
        })
    }

    #[rstest]
    #[case(17, 13, 0, 0)]
    #[case(17, 13, 1, 1)]
    #[case(40, 31, 2, 2)]
    #[case(40, 31, 3, 1)]
    #[case(9, 7, 4, 4)]
    #[case(5, 5, 9, 9)]
    #[case(1, 1, 2, 2)]
    fn erode_and_dilate_match_the_direct_window(
        #[case] w: u32,
        #[case] h: u32,
        #[case] rx: u32,
        #[case] ry: u32,
    ) {
        let image = noise(w, h);
        assert_eq!(erode(&image, rx, ry), naive(&image, rx, ry, true));
        assert_eq!(dilate(&image, rx, ry), naive(&image, rx, ry, false));
    }

    #[test]
    fn composites_are_built_from_erode_and_dilate() {
        let image = noise(33, 21);
        let opened = open(&image, 2, 2);
        assert_eq!(opened, dilate(&erode(&image, 2, 2), 2, 2));
        assert_eq!(close(&image, 2, 2), erode(&dilate(&image, 2, 2), 2, 2));
        assert_eq!(top_hat(&image, 2, 2), subtract(&image, &opened));
        assert_eq!(
            gradient(&image, 1, 1),
            subtract(&dilate(&image, 1, 1), &erode(&image, 1, 1))
        );
        assert_eq!(
            black_hat(&image, 2, 2),
            subtract(&close(&image, 2, 2), &image)
        );
    }

    #[test]
    fn hats_pick_out_specks_and_holes() {
        let mut image = GrayImage::from_pixel(15, 15, Luma([100]));
        image.put_pixel(7, 7, Luma([200]));
        assert_eq!(top_hat(&image, 1, 1).get_pixel(7, 7).0[0], 100);
        assert_eq!(top_hat(&image, 1, 1).get_pixel(0, 0).0[0], 0);
        let mut holes = GrayImage::from_pixel(15, 15, Luma([100]));
        holes.put_pixel(7, 7, Luma([20]));
        assert_eq!(black_hat(&holes, 1, 1).get_pixel(7, 7).0[0], 80);
    }

    #[test]
    fn an_empty_picture_is_returned_unchanged() {
        let empty = GrayImage::new(0, 4);
        assert_eq!(erode(&empty, 2, 2), empty);
    }
}

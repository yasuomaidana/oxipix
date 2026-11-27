//! Template matching by FFT: the same four scores as `imageproc::template_matching`, in `O(N log N)`.
//!
//! A direct search costs `positions x template pixels` multiply-adds. Here the cross-correlation of the
//! picture with the template comes from one product in the frequency domain, and the energy of every
//! window comes from integral images, so
//!
//! `sum (I - T)^2 = sum I^2 - 2 sum I T + sum T^2`
//!
//! is evaluated in `O(1)` per position. Both inputs are shifted by the picture mean before the transforms,
//! which cancels in the correlation and keeps the `f32` transforms precise.

use image::{GrayImage, ImageBuffer, Luma};
use realfft::{RealFftPlanner, num_complex::Complex};
use rustfft::FftPlanner;

/// Scores for every position where the template fits inside the picture.
pub type Scores = ImageBuffer<Luma<f32>, Vec<f32>>;

/// How the template is compared with each window; the definitions are those of `imageproc`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Sum of squared differences; lower is better.
    Sse,
    /// [`Method::Sse`] divided by `sqrt(sum I^2 * sum T^2)`; lower is better.
    SseNormalized,
    /// Cross-correlation `sum I T`; higher is better.
    Ccorr,
    /// [`Method::Ccorr`] divided by `sqrt(sum I^2 * sum T^2)`; higher is better.
    CcorrNormalized,
}

/// Smallest length of at least `n` whose only prime factors are 2, 3 and 5 (fast transform sizes).
fn fast_len(n: usize) -> usize {
    (n..)
        .find(|&m| {
            let mut rest = m;
            for p in [2, 3, 5] {
                while rest % p == 0 {
                    rest /= p;
                }
            }
            rest == 1
        })
        .expect("the search is unbounded")
}

/// Forward 2-D transform of a real `height` x `width` plane: one real transform per row, then one complex
/// transform per column of the half spectrum. Returns `height x (width / 2 + 1)` values.
fn forward(plane: &mut [f32], width: usize, height: usize) -> Vec<Complex<f32>> {
    let cols = width / 2 + 1;
    let row_fft = RealFftPlanner::<f32>::new().plan_fft_forward(width);
    let mut spectrum = vec![Complex::new(0.0, 0.0); height * cols];
    for (row, out) in plane
        .chunks_exact_mut(width)
        .zip(spectrum.chunks_exact_mut(cols))
    {
        row_fft.process(row, out).expect("lengths match the plan");
    }
    columns(&mut spectrum, cols, height, false);
    spectrum
}

/// In-place complex transform of every column of a `height x cols` array.
fn columns(spectrum: &mut [Complex<f32>], cols: usize, height: usize, inverse: bool) {
    let mut planner = FftPlanner::<f32>::new();
    let fft = if inverse {
        planner.plan_fft_inverse(height)
    } else {
        planner.plan_fft_forward(height)
    };
    let mut column = vec![Complex::new(0.0, 0.0); height];
    for c in 0..cols {
        for (y, slot) in column.iter_mut().enumerate() {
            *slot = spectrum[y * cols + c];
        }
        fft.process(&mut column);
        for (y, value) in column.iter().enumerate() {
            spectrum[y * cols + c] = *value;
        }
    }
}

/// Inverse of [`forward`], scaled so that `inverse(forward(x)) == x`.
fn inverse(mut spectrum: Vec<Complex<f32>>, width: usize, height: usize) -> Vec<f32> {
    let cols = width / 2 + 1;
    columns(&mut spectrum, cols, height, true);
    let row_ifft = RealFftPlanner::<f32>::new().plan_fft_inverse(width);
    let mut plane = vec![0.0f32; width * height];
    for (row, out) in spectrum
        .chunks_exact_mut(cols)
        .zip(plane.chunks_exact_mut(width))
    {
        // The imaginary parts of the first and (even width) last bins are rounding noise.
        row[0].im = 0.0;
        if width.is_multiple_of(2) {
            row[cols - 1].im = 0.0;
        }
        row_ifft.process(row, out).expect("lengths match the plan");
    }
    let scale = 1.0 / (width * height) as f32;
    plane.iter_mut().for_each(|v| *v *= scale);
    plane
}

/// `image - shift` copied into the top-left corner of a zeroed `height x width` plane.
fn shifted_plane(image: &GrayImage, shift: f32, width: usize, height: usize) -> Vec<f32> {
    let mut plane = vec![0.0f32; width * height];
    let w = image.width() as usize;
    for (src, dst) in image
        .as_raw()
        .chunks_exact(w)
        .zip(plane.chunks_exact_mut(width))
    {
        for (d, s) in dst.iter_mut().zip(src) {
            *d = f32::from(*s) - shift;
        }
    }
    plane
}

/// `sum of (image - shift)(template - shift)` for every position, by one product in the frequency domain.
fn correlate(image: &GrayImage, template: &GrayImage, shift: f32) -> (Vec<f32>, usize) {
    let width = fast_len(image.width() as usize);
    let height = fast_len(image.height() as usize);
    let mut a = shifted_plane(image, shift, width, height);
    let mut b = shifted_plane(template, shift, width, height);
    let (fa, fb) = (
        forward(&mut a, width, height),
        forward(&mut b, width, height),
    );
    let product = fa.iter().zip(&fb).map(|(x, y)| x * y.conj()).collect();
    (inverse(product, width, height), width)
}

/// Summed-area tables of the pixel values and of their squares, `(width + 1) x (height + 1)` each.
fn integrals(image: &GrayImage) -> (Vec<u64>, Vec<u64>) {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let stride = w + 1;
    let (mut sum, mut square) = (vec![0u64; stride * (h + 1)], vec![0u64; stride * (h + 1)]);
    for (y, row) in image.as_raw().chunks_exact(w).enumerate() {
        let (mut run, mut run_sq) = (0u64, 0u64);
        for (x, p) in row.iter().enumerate() {
            let v = u64::from(*p);
            (run, run_sq) = (run + v, run_sq + v * v);
            sum[(y + 1) * stride + x + 1] = sum[y * stride + x + 1] + run;
            square[(y + 1) * stride + x + 1] = square[y * stride + x + 1] + run_sq;
        }
    }
    (sum, square)
}

/// Sum over the `w x h` window with top-left corner `(x, y)` of a table with row length `stride`.
fn window_sum(table: &[u64], stride: usize, at: (usize, usize), size: (usize, usize)) -> f64 {
    let (x, y) = at;
    let (w, h) = size;
    let total = table[(y + h) * stride + x + w] + table[y * stride + x]
        - table[y * stride + x + w]
        - table[(y + h) * stride + x];
    total as f64
}

fn mean(image: &GrayImage) -> f32 {
    let total: u64 = image.as_raw().iter().map(|p| u64::from(*p)).sum();
    (total as f64 / image.as_raw().len() as f64) as f32
}

/// Scores of `template` against every window of `image`, or `None` when the template is empty or larger
/// than the picture. The result is `(width - tw + 1) x (height - th + 1)`.
#[must_use]
pub fn match_template(image: &GrayImage, template: &GrayImage, method: Method) -> Option<Scores> {
    let ((w, h), (tw, th)) = (image.dimensions(), template.dimensions());
    if tw == 0 || th == 0 || tw > w || th > h {
        return None;
    }
    let shift = mean(image);
    let (corr, corr_width) = correlate(image, template, shift);
    let (sum, square) = integrals(image);
    let (t_sum, t_square) = {
        let (s, q) = integrals(template);
        let end = s.len() - 1;
        (s[end] as f64, q[end] as f64)
    };
    let (n, m) = ((tw * th) as f64, f64::from(shift));
    let stride = w as usize + 1;
    let size = (tw as usize, th as usize);
    let scores = (0..=(h - th) as usize).flat_map(|y| (0..=(w - tw) as usize).map(move |x| (x, y)));
    let data = scores
        .map(|(x, y)| {
            let (win, win_sq) = (
                window_sum(&sum, stride, (x, y), size),
                window_sum(&square, stride, (x, y), size),
            );
            // sum I T from the shifted correlation: (I' + m)(T' + m) expanded.
            let raw = f64::from(corr[y * corr_width + x])
                + m * (win - n * m)
                + m * (t_sum - n * m)
                + n * m * m;
            score(method, raw, win_sq, t_square)
        })
        .collect();
    ImageBuffer::from_raw(w - tw + 1, h - th + 1, data)
}

/// The score of one window from its correlation `raw`, window energy and template energy.
fn score(method: Method, raw: f64, window_energy: f64, template_energy: f64) -> f32 {
    let norm = (window_energy * template_energy).sqrt();
    // An all-zero window or template correlates to exactly 0; the transform leaves rounding noise there.
    let raw = if norm > 0.0 { raw } else { 0.0 };
    let normalise = |v: f64| if norm > 0.0 { v / norm } else { v };
    let sse = (window_energy - 2.0 * raw + template_energy).max(0.0);
    match method {
        Method::Sse => sse as f32,
        Method::SseNormalized => normalise(sse) as f32,
        Method::Ccorr => raw as f32,
        Method::CcorrNormalized => normalise(raw) as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imgproc::testing::noise;
    use rstest::rstest;

    /// `(sum (T - I)^2, sum I^2, sum T^2, sum I T)` of one window, evaluated directly in f64.
    fn sums(image: &GrayImage, template: &GrayImage, at: (u32, u32)) -> (f64, f64, f64, f64) {
        let mut acc = (0.0, 0.0, 0.0, 0.0);
        for (u, v, t) in template.enumerate_pixels() {
            let i = f64::from(image.get_pixel(at.0 + u, at.1 + v).0[0]);
            let t = f64::from(t.0[0]);
            acc = (
                acc.0 + (t - i).powi(2),
                acc.1 + i * i,
                acc.2 + t * t,
                acc.3 + i * t,
            );
        }
        acc
    }

    /// The definition of every method, evaluated directly in f64.
    fn direct(image: &GrayImage, template: &GrayImage, method: Method) -> Vec<f64> {
        let ((w, h), (tw, th)) = (image.dimensions(), template.dimensions());
        let positions = (0..=h - th).flat_map(|y| (0..=w - tw).map(move |x| (x, y)));
        positions
            .map(|at| {
                let (sse, ii, tt, it) = sums(image, template, at);
                let norm = (ii * tt).sqrt();
                let div = |v: f64| if norm > 0.0 { v / norm } else { v };
                match method {
                    Method::Sse => sse,
                    Method::SseNormalized => div(sse),
                    Method::Ccorr => it,
                    Method::CcorrNormalized => div(it),
                }
            })
            .collect()
    }

    #[rstest]
    #[case(37, 29, 7, 5)]
    #[case(48, 30, 16, 16)]
    #[case(31, 17, 31, 17)]
    #[case(20, 20, 1, 1)]
    #[case(1, 1, 1, 1)]
    #[case(64, 9, 8, 3)]
    fn scores_match_the_definition(
        #[case] w: u32,
        #[case] h: u32,
        #[case] tw: u32,
        #[case] th: u32,
        #[values(
            Method::Sse,
            Method::SseNormalized,
            Method::Ccorr,
            Method::CcorrNormalized
        )]
        method: Method,
    ) {
        let (image, template) = (noise(w, h, 7), noise(tw, th, 99));
        let fast = match_template(&image, &template, method).unwrap();
        let slow = direct(&image, &template, method);
        assert_eq!(fast.dimensions(), (w - tw + 1, h - th + 1));
        let scale = slow.iter().fold(1.0f64, |a, v| a.max(v.abs()));
        for (f, s) in fast.as_raw().iter().zip(&slow) {
            assert!(
                (f64::from(*f) - s).abs() <= 2e-4 * scale,
                "{f} vs {s} (scale {scale})"
            );
        }
    }

    #[test]
    fn the_best_match_is_where_the_template_was_cut() {
        let image = noise(60, 40, 3);
        let template = image::imageops::crop_imm(&image, 21, 13, 12, 9).to_image();
        let scores = match_template(&image, &template, Method::Sse).unwrap();
        let best = scores
            .enumerate_pixels()
            .min_by(|a, b| a.2.0[0].total_cmp(&b.2.0[0]))
            .unwrap();
        assert_eq!((best.0, best.1), (21, 13));
        assert!(best.2.0[0] < 1.0);
    }

    #[test]
    fn flat_pictures_have_no_norm_and_keep_the_raw_score() {
        let flat = GrayImage::new(8, 8);
        let scores = match_template(&flat, &GrayImage::new(3, 3), Method::SseNormalized).unwrap();
        assert!(scores.iter().all(|v| *v == 0.0));
    }

    #[rstest]
    #[case(10, 10, 0, 3)]
    #[case(10, 10, 3, 0)]
    #[case(4, 10, 5, 3)]
    #[case(10, 4, 3, 5)]
    fn a_template_that_does_not_fit_has_no_scores(
        #[case] w: u32,
        #[case] h: u32,
        #[case] tw: u32,
        #[case] th: u32,
    ) {
        let template = GrayImage::new(tw, th);
        assert!(match_template(&GrayImage::new(w, h), &template, Method::Sse).is_none());
    }

    #[rstest]
    #[case(1, 1)]
    #[case(7, 8)]
    #[case(13, 15)]
    #[case(451, 480)]
    fn fast_lengths_only_have_small_prime_factors(#[case] n: usize, #[case] want: usize) {
        assert_eq!(fast_len(n), want);
    }
}

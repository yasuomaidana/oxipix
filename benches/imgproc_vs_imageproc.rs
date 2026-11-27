//! `oxipix::imgproc` against `imageproc` on the sizes the rust-cv-guide benchmarks use.
//!
//! Run with `cargo bench --bench imgproc_vs_imageproc`. The pictures are pseudo-random, so the numbers do
//! not depend on image content; the guide's `comparisons/opencv` harness measures real photographs against
//! OpenCV 5.1.0.

use criterion::{Criterion, criterion_group, criterion_main};
use image::{GrayImage, Luma, Rgb, RgbImage};
use imageproc::edges::canny as ip_canny;
use imageproc::filter::{
    Kernel, gaussian_blur_f32, laplacian_filter, median_filter, separable_filter_equal,
};
use imageproc::gradients::horizontal_sobel;
use imageproc::morphology::{
    Mask, grayscale_close, grayscale_dilate, grayscale_erode, grayscale_open,
};
use imageproc::template_matching::{MatchTemplateMethod, match_template};
use oxipix::imgproc::{
    bilateral, blur, canny, filter2d, gradient, median, morph, pyramid, template,
};
use std::hint::black_box;

/// A picture whose value at each pixel is a hash of its coordinates and `seed`.
fn noise(w: u32, h: u32, seed: u32) -> GrayImage {
    GrayImage::from_fn(w, h, |x, y| {
        let hash = (x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ seed)
            .wrapping_mul(0xC2B2_AE3D);
        Luma([(hash >> 24) as u8])
    })
}

fn bench(c: &mut Criterion) {
    let photo = noise(451, 300, 1);
    let rgb = RgbImage::from_fn(451, 300, |x, y| {
        Rgb([photo.get_pixel(x, y).0[0], (x % 251) as u8, (y % 241) as u8])
    });
    let coins = noise(303, 384, 2);
    let patch = image::imageops::crop_imm(&photo, 205, 100, 40, 40).to_image();

    let mut group = c.benchmark_group("template_matching_sse_451x300_40x40");
    group.bench_function("oxipix", |b| {
        b.iter(|| {
            template::match_template(black_box(&photo), black_box(&patch), template::Method::Sse)
        })
    });
    group.bench_function("imageproc", |b| {
        b.iter(|| {
            match_template(
                black_box(&photo),
                black_box(&patch),
                MatchTemplateMethod::SumOfSquaredErrors,
            )
        })
    });
    group.finish();

    let mut group = c.benchmark_group("median_5x5_rgb_451x300");
    group.bench_function("oxipix", |b| {
        b.iter(|| median::median_blur(black_box(&rgb), 2))
    });
    group.bench_function("imageproc", |b| {
        b.iter(|| median_filter(black_box(&rgb), 2, 2))
    });
    group.finish();

    let mask = Mask::square(2);
    let mut group = c.benchmark_group("erode_dilate_5x5_303x384");
    group.bench_function("oxipix_erode", |b| {
        b.iter(|| morph::erode(black_box(&coins), 2, 2))
    });
    group.bench_function("imageproc_erode", |b| {
        b.iter(|| grayscale_erode(black_box(&coins), &mask))
    });
    group.bench_function("oxipix_dilate", |b| {
        b.iter(|| morph::dilate(black_box(&coins), 2, 2))
    });
    group.bench_function("imageproc_dilate", |b| {
        b.iter(|| grayscale_dilate(black_box(&coins), &mask))
    });
    group.finish();

    let mut group = c.benchmark_group("open_close_7x7_303x384");
    let mask = Mask::square(3);
    group.bench_function("oxipix_open", |b| {
        b.iter(|| morph::open(black_box(&coins), 3, 3))
    });
    group.bench_function("imageproc_open", |b| {
        b.iter(|| grayscale_open(black_box(&coins), &mask))
    });
    group.bench_function("oxipix_close", |b| {
        b.iter(|| morph::close(black_box(&coins), 3, 3))
    });
    group.bench_function("imageproc_close", |b| {
        b.iter(|| grayscale_close(black_box(&coins), &mask))
    });
    group.finish();
    wave_two(c, &rgb, &photo);
}
/// Wave 2: blur, pyrDown, derivatives, Canny, bilateral and filter2D.
fn wave_two(c: &mut Criterion, rgb: &RgbImage, grey: &GrayImage) {
    let mut group = c.benchmark_group("smoothing_rgb_451x300");
    group.bench_function("oxipix_gaussian", |b| {
        b.iter(|| blur::gaussian_blur(black_box(rgb), 1.1))
    });
    group.bench_function("imageproc_gaussian", |b| {
        b.iter(|| gaussian_blur_f32(black_box(rgb), 1.1))
    });
    let taps = [0.2f32; 5];
    group.bench_function("oxipix_box", |b| {
        b.iter(|| blur::box_blur(black_box(rgb), 2))
    });
    group.bench_function("imageproc_box", |b| {
        b.iter(|| separable_filter_equal(black_box(rgb), &taps))
    });
    let binomial = [
        1.0f32 / 16.0,
        4.0 / 16.0,
        6.0 / 16.0,
        4.0 / 16.0,
        1.0 / 16.0,
    ];
    group.bench_function("oxipix_pyr_down", |b| {
        b.iter(|| pyramid::pyr_down(black_box(rgb)))
    });
    group.bench_function("imageproc_pyr_down", |b| {
        b.iter(|| {
            let blurred = separable_filter_equal(black_box(rgb), &binomial);
            RgbImage::from_fn(rgb.width().div_ceil(2), rgb.height().div_ceil(2), |x, y| {
                *blurred.get_pixel(2 * x, 2 * y)
            })
        })
    });
    group.finish();

    let mut group = c.benchmark_group("derivatives_gray_451x300");
    group.bench_function("oxipix_sobel_x", |b| {
        b.iter(|| gradient::sobel_x(black_box(grey)))
    });
    group.bench_function("imageproc_sobel_x", |b| {
        b.iter(|| horizontal_sobel(black_box(grey)))
    });
    group.bench_function("oxipix_laplacian", |b| {
        b.iter(|| gradient::laplacian(black_box(grey)))
    });
    group.bench_function("imageproc_laplacian", |b| {
        b.iter(|| laplacian_filter(black_box(grey)))
    });
    group.bench_function("oxipix_canny", |b| {
        b.iter(|| canny::canny(black_box(grey), 50.0, 150.0))
    });
    group.bench_function("imageproc_canny", |b| {
        b.iter(|| ip_canny(black_box(grey), 50.0, 150.0))
    });
    let box5 = Kernel::new(&[1.0f32 / 25.0; 25], 5, 5);
    let kernel = [1.0f32 / 25.0; 25];
    group.bench_function("oxipix_filter2d_5x5", |b| {
        b.iter(|| filter2d::filter2d(black_box(grey), &kernel, 5))
    });
    group.bench_function("imageproc_filter2d_5x5", |b| {
        b.iter(|| {
            box5.filter::<image::Luma<u8>, _, image::Luma<u8>>(black_box(grey), |c, v| *c = v as u8)
        })
    });
    group.finish();

    // imageproc 0.25 reads `window_size` differently from 0.27, so there is no like-for-like comparison here;
    // the guide's `comparisons/opencv` harness compares bilateral filters with imageproc 0.27 and OpenCV.
    c.bench_function("bilateral_rgb_451x300_oxipix", |b| {
        b.iter(|| bilateral::bilateral_filter(black_box(rgb), 2, 30.0, 2.0))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);

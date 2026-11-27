//! `oxipix::imgproc` against `imageproc` on the sizes the rust-cv-guide benchmarks use.
//!
//! Run with `cargo bench --bench imgproc_vs_imageproc`. The pictures are pseudo-random, so the numbers do
//! not depend on image content; the guide's `comparisons/opencv` harness measures real photographs against
//! OpenCV 5.1.0.

use criterion::{Criterion, criterion_group, criterion_main};
use image::{GrayImage, Luma, Rgb, RgbImage};
use imageproc::filter::median_filter;
use imageproc::morphology::{
    Mask, grayscale_close, grayscale_dilate, grayscale_erode, grayscale_open,
};
use imageproc::template_matching::{MatchTemplateMethod, match_template};
use oxipix::imgproc::{median, morph, template};
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
}

criterion_group!(benches, bench);
criterion_main!(benches);

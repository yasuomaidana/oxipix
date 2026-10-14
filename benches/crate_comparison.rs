//! High-performance Criterion benchmarks for sovereign oxipix operations.
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_resize(c: &mut Criterion) {
    let width = 800u32;
    let height = 600u32;
    let dst_w = 400u32;
    let dst_h = 300u32;
    let raw_rgba = vec![128u8; (width * height * 4) as usize];

    let mut group = c.benchmark_group("Resize_800x600_to_400x300");

    // 1. oxipix nearest neighbor resize
    group.bench_function("oxipix_nearest", |b| {
        b.iter(|| {
            let res = oxipix::ops::resize_nearest_rgba(
                black_box(&raw_rgba),
                oxipix::PixelWidth(width),
                oxipix::PixelHeight(height),
                oxipix::PixelWidth(dst_w),
                oxipix::PixelHeight(dst_h),
            );
            black_box(res).unwrap()
        })
    });

    // 2. oxipix SIMD Bilinear resize
    group.bench_function("oxipix_simd_bilinear", |b| {
        b.iter(|| {
            let res = oxipix::ops::resize_simd_rgba(
                black_box(&raw_rgba),
                oxipix::PixelWidth(width),
                oxipix::PixelHeight(height),
                oxipix::PixelWidth(dst_w),
                oxipix::PixelHeight(dst_h),
                oxipix::ops::FilterType::Triangle,
            );
            black_box(res).unwrap()
        })
    });

    // 3. oxipix SIMD Lanczos3 resize
    group.bench_function("oxipix_simd_lanczos3", |b| {
        b.iter(|| {
            let res = oxipix::ops::resize_simd_rgba(
                black_box(&raw_rgba),
                oxipix::PixelWidth(width),
                oxipix::PixelHeight(height),
                oxipix::PixelWidth(dst_w),
                oxipix::PixelHeight(dst_h),
                oxipix::ops::FilterType::Lanczos3,
            );
            black_box(res).unwrap()
        })
    });

    group.finish();
}

fn bench_threshold(c: &mut Criterion) {
    let width = 800u32;
    let height = 600u32;
    let mut gray_raw = vec![40u8; (width * height / 2) as usize];
    gray_raw.resize((width * height) as usize, 210u8);

    let mut group = c.benchmark_group("Otsu_Threshold_800x600");

    // oxipix otsu threshold
    group.bench_function("oxipix_otsu", |b| {
        b.iter(|| {
            let t = oxipix::ops::otsu_threshold(black_box(&gray_raw));
            black_box(t)
        })
    });

    // oxipix Bradley-Roth adaptive threshold
    group.bench_function("oxipix_bradley_roth_adaptive", |b| {
        let mut binary = vec![0u8; (width * height) as usize];
        b.iter(|| {
            let res = oxipix::simd::binarize::bradley_roth_threshold(
                black_box(&gray_raw),
                width as usize,
                height as usize,
                black_box(&mut binary),
            );
            black_box(res).unwrap()
        })
    });

    group.finish();
}

fn bench_crop_subview(c: &mut Criterion) {
    let width = 800u32;
    let height = 600u32;

    let frame = oxipix::FrameBuffer::<oxipix::Rgba8, oxipix::state::Uninitialized>::allocate(
        oxipix::PixelWidth(width),
        oxipix::PixelHeight(height),
    )
    .unwrap()
    .fill_bytes(128);

    let mut group = c.benchmark_group("ROI_Crop_200x200");

    // oxipix zero-copy sub-view
    group.bench_function("oxipix_zero_copy_subview", |b| {
        b.iter(|| {
            let view = frame
                .sub_view(black_box(100), black_box(100), 200, 200)
                .unwrap();
            black_box(view.width().get())
        })
    });

    group.finish();
}

fn bench_barcode(c: &mut Criterion) {
    let mut group = c.benchmark_group("Barcode_QR_Engine");

    let text = "https://github.com/cool-japan/oxipix";

    group.bench_function("oxipix_qr_encode_256x256", |b| {
        b.iter(|| {
            let rgba = oxipix::barcode::qr_encode_rgba(black_box(text), 256, 256).unwrap();
            black_box(rgba.len())
        })
    });

    let rgba = oxipix::barcode::qr_encode_rgba(text, 256, 256).unwrap();
    group.bench_function("oxipix_qr_decode_256x256", |b| {
        b.iter(|| {
            let decoded = oxipix::barcode::decode_barcode_rgba(black_box(&rgba), 256, 256).unwrap();
            black_box(decoded.len())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_resize,
    bench_threshold,
    bench_crop_subview,
    bench_barcode,
);
criterion_main!(benches);

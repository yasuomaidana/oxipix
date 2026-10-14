use criterion::{criterion_group, criterion_main, Criterion};
use rxing::Writer;
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

    // 2. image crate nearest neighbor resize
    let img_rgba = image::RgbaImage::from_raw(width, height, raw_rgba.clone()).unwrap();
    group.bench_function("image_crate_nearest", |b| {
        b.iter(|| {
            let res = image::imageops::resize(
                black_box(&img_rgba),
                dst_w,
                dst_h,
                image::imageops::FilterType::Nearest,
            );
            black_box(res)
        })
    });

    // 3. fast_image_resize
    let src_fir = fast_image_resize::images::Image::from_vec_u8(
        width,
        height,
        raw_rgba,
        fast_image_resize::PixelType::U8x4,
    ).unwrap();
    let mut dst_fir = fast_image_resize::images::Image::new(
        dst_w,
        dst_h,
        fast_image_resize::PixelType::U8x4,
    );
    let mut resizer = fast_image_resize::Resizer::new();
    let options = fast_image_resize::ResizeOptions::new()
        .resize_alg(fast_image_resize::ResizeAlg::Nearest);

    group.bench_function("fast_image_resize_nearest", |b| {
        b.iter(|| {
            resizer.resize(
                black_box(&src_fir),
                black_box(&mut dst_fir),
                black_box(&options),
            ).unwrap();
            black_box(dst_fir.buffer().len())
        })
    });

    group.finish();
}

fn bench_threshold(c: &mut Criterion) {
    let width = 800u32;
    let height = 600u32;
    // Bimodal synthetic distribution
    let mut gray_raw = vec![40u8; (width * height / 2) as usize];
    gray_raw.resize((width * height) as usize, 210u8);

    let mut group = c.benchmark_group("Otsu_Threshold_800x600");

    // 1. oxipix otsu threshold
    group.bench_function("oxipix_otsu", |b| {
        b.iter(|| {
            let t = oxipix::ops::otsu_threshold(black_box(&gray_raw));
            black_box(t)
        })
    });

    // 2. imageproc otsu_level
    let gray_img = image::GrayImage::from_raw(width, height, gray_raw.clone()).unwrap();
    group.bench_function("imageproc_otsu", |b| {
        b.iter(|| {
            let t = imageproc::contrast::otsu_level(black_box(&gray_img));
            black_box(t)
        })
    });

    group.finish();
}

fn bench_crop_subview(c: &mut Criterion) {
    let width = 800u32;
    let height = 600u32;
    let raw_rgba = vec![255u8; (width * height * 4) as usize];

    let mut group = c.benchmark_group("ROI_Crop_200x200");

    // 1. oxipix zero-copy SubPixView
    let oxipix_fb = oxipix::FrameBuffer::<oxipix::Rgba8, _>::allocate(
        oxipix::PixelWidth(width),
        oxipix::PixelHeight(height),
    ).unwrap().fill_bytes(255);

    group.bench_function("oxipix_zero_copy_subview", |b| {
        b.iter(|| {
            let view = black_box(&oxipix_fb)
                .sub_view(100, 100, 200, 200)
                .unwrap();
            let r0 = view.row(0).unwrap();
            black_box(r0.len())
        })
    });

    // 2. image crate crop with .to_image() clone
    let img_rgba = image::RgbaImage::from_raw(width, height, raw_rgba).unwrap();
    group.bench_function("image_crop_to_image_clone", |b| {
        b.iter(|| {
            let sub = image::imageops::crop_imm(black_box(&img_rgba), 100, 100, 200, 200);
            let owned = sub.to_image();
            black_box(owned.len())
        })
    });

    group.finish();
}

fn bench_rxing_barcode(c: &mut Criterion) {
    let mut group = c.benchmark_group("Barcode_QR_Generation");

    let text = "https://github.com/cool-japan/oxipix";
    let writer = rxing::qrcode::QRCodeWriter::default();

    group.bench_function("rxing_qr_encode_256x256", |b| {
        b.iter(|| {
            let matrix = writer.encode(
                black_box(text),
                &rxing::BarcodeFormat::QR_CODE,
                256,
                256,
            ).unwrap();
            black_box(matrix.getWidth())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_resize,
    bench_threshold,
    bench_crop_subview,
    bench_rxing_barcode,
);
criterion_main!(benches);

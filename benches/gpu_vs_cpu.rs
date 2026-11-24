use criterion::{Criterion, criterion_group, criterion_main};
use image::{Rgba, RgbaImage};
use oxipix::gpu::GpuImageExt;

fn bench(c: &mut Criterion) {
    let img = RgbaImage::from_fn(1600, 1200, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255])
    });
    let k = [1.0 / 9.0; 9];
    let h = [0.95, 0.05, 10.0, -0.03, 0.97, 8.0, 0.0002, 0.0001, 1.0];
    c.bench_function("oxipix_gpu_filter3x3_2MP", |b| {
        b.iter(|| img.gpu_filter3x3(&k))
    });
    c.bench_function("oxipix_cpu_filter3x3_2MP", |b| {
        b.iter(|| oxipix::gpu::cpu::convolve3x3(img.as_raw(), 1600, 1200, &k))
    });
    c.bench_function("imageproc_filter3x3_2MP_baseline", |b| {
        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        b.iter(|| imageproc::filter::filter3x3::<_, _, u8>(&rgb, &[1.0f32 / 9.0; 9]))
    });
    c.bench_function("oxipix_gpu_warp_2MP", |b| {
        b.iter(|| img.gpu_warp_perspective(&h, 1600, 1200))
    });
    c.bench_function("oxipix_cpu_warp_2MP", |b| {
        b.iter(|| oxipix::gpu::cpu::warp_perspective(img.as_raw(), 1600, 1200, 1600, 1200, &h))
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);

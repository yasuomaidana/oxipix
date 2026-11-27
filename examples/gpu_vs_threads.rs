//! Where the GPU kernels win: `gpu` 3x3 filter and perspective warp (with upload and readback) against the
//! `imgproc` kernels on one thread and on all threads, at 640x480, 1920x1080 and 3840x2160 (RGBA).
//! Run with `cargo run --release --example gpu_vs_threads --features gpu,parallel`.

use image::{Rgba, RgbaImage};
use oxipix::gpu::GpuImageExt;
use oxipix::imgproc::{filter2d, warp};
use std::time::Instant;

fn median_ms(mut f: impl FnMut()) -> f64 {
    for _ in 0..3 {
        f();
    }
    let mut v: Vec<f64> = (0..11)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let k = [1.0f32 / 9.0; 9];
    let h = [1.0f32, 0.05, 2.0, 0.0, 1.0, 1.0, 0.00002, 0.0, 1.0];
    let one = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let all = rayon::ThreadPoolBuilder::new().build().unwrap();
    println!(
        "{:<8} {:>11} | {:>9} {:>9} {:>9} | {:>9} {:>9} {:>9}",
        "size", "pixels", "gpu 3x3", "cpu 1t", "cpu all", "gpu warp", "cpu 1t", "cpu all"
    );
    for (name, w, hh) in [
        ("small", 640u32, 480u32),
        ("medium", 1920, 1080),
        ("big", 3840, 2160),
    ] {
        let img = RgbaImage::from_fn(w, hh, |x, y| {
            Rgba([(x % 251) as u8, (y % 241) as u8, ((x + y) % 239) as u8, 255])
        });
        let g_f = median_ms(|| {
            std::hint::black_box(img.gpu_filter3x3(&k));
        });
        let g_w = median_ms(|| {
            std::hint::black_box(img.gpu_warp_perspective(&h, w, hh));
        });
        let c = |pool: &rayon::ThreadPool| {
            pool.install(|| {
                (
                    median_ms(|| {
                        std::hint::black_box(filter2d::filter2d(&img, &k, 3));
                    }),
                    median_ms(|| {
                        std::hint::black_box(warp::warp_perspective(&img, &h, (w, hh)));
                    }),
                )
            })
        };
        let (f1, w1) = c(&one);
        let (fa, wa) = c(&all);
        println!(
            "{name:<8} {:>11} | {g_f:>8.2}ms {f1:>7.2}ms {fa:>7.2}ms | {g_w:>8.2}ms {w1:>7.2}ms {wa:>7.2}ms",
            w * hh
        );
    }
}

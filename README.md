# oxipix

Document-capture building blocks on top of the [`image`](https://github.com/image-rs/image) crate.
oxipix does not replace `image` or `rxing`; it adds what they deliberately don't have.

| Feature | Module | What it gives you |
|---|---|---|
| `gpu` (opt-in) | `oxipix::gpu` | wgpu 3x3 convolution and bilinear perspective warp on `ImageBuffer<Rgba<u8>, _>`, via `GpuImageExt`. Pipelines are compiled once per process. Falls back to a CPU reference when no adapter exists. |
| `mosaic` (default) | `oxipix::mosaic` | Strip overlap detection (ZMAD), ECC drift estimation, quad unwarping, feathered stitching. Uses the GPU warp when `gpu` is on. |
| `vectorize` (default) | `oxipix::vectorize` | Otsu thresholding, Moore-neighbour contour tracing, RDP simplification, SVG path output. |

```toml
[dependencies]
oxipix = { git = "https://github.com/yasuomaidana/oxipix", features = ["gpu"] }
```

```rust
use image::RgbaImage;
use oxipix::gpu::GpuImageExt;

let page: RgbaImage = image::open("receipt.png")?.to_rgba8();
let blurred = page.gpu_blur3x3();
```

## Measured performance

`cargo bench --features gpu --bench gpu_vs_cpu` on an Apple M3, 1600x1200 RGBA, including upload and readback:

| Operation | GPU | CPU reference |
|---|---|---|
| 3x3 filter | 3.8 ms | 27 ms (`imageproc::filter3x3` on RGB: 21 ms) |
| Perspective warp | 3.6 ms | 21 ms |

Small images are faster on the CPU because of transfer overhead; measure before enabling `gpu` for thumbnails.

## Relationship to upstream

CPU-side improvements (SIMD colour conversion, barcode binarizer speedups) are developed as opt-in
features on forks of `image` and `rxing` and proposed upstream. See [`docs/FORKS.md`](docs/FORKS.md).
The earlier plan to replace those crates is archived in [`docs/archive`](docs/archive).

## Status

See [`DEV_STATUS.md`](DEV_STATUS.md). License: MIT OR Apache-2.0.

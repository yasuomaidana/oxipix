# 04 — Benchmark methodology

Performance claims in the README and the documentation site must be reproducible from a command in this file. A number
that cannot be reproduced does not go in the documentation.

## 1. What is measured today

`benches/gpu_vs_cpu.rs` (criterion, release profile) on a **1600 x 1200 RGBA** image (1.92 MP) generated in memory.
Every GPU figure **includes upload and readback**, because that is what a caller pays.

```bash
cargo bench --features gpu --bench gpu_vs_cpu
```

Result of the run recorded on 2026-11-24 (Apple M3, wgpu 30, Metal; criterion median of 100 samples):

| Benchmark | What it runs | Median |
|---|---|---|
| `oxipix_gpu_filter3x3_2MP` | `gpu_filter3x3` with a box kernel, on the GPU | 4.2 ms |
| `oxipix_cpu_filter3x3_2MP` | `gpu::cpu::convolve3x3`, the single-threaded reference | 25.8 ms |
| `imageproc_filter3x3_2MP_baseline` | `imageproc::filter::filter3x3` on **RGB** (3 channels, no alpha) | 15.1 ms |
| `oxipix_gpu_warp_2MP` | `gpu_warp_perspective`, 1600 x 1200 output | 4.7 ms |
| `oxipix_cpu_warp_2MP` | `gpu::cpu::warp_perspective`, the single-threaded reference | 19.9 ms |

Reading the table:

- The GPU path is about 3.6x faster than `imageproc` and 6x faster than oxipix's own CPU reference for the 3x3 filter,
  and about 4x faster than the CPU reference for the warp, **at this image size on this machine**.
- The CPU reference is a specification (it defines what the shaders must produce), not an optimised kernel. The fair CPU
  comparison for the filter is `imageproc`, and note that it processes 3 channels where oxipix processes 4.
- Small images are faster on the CPU because of transfer overhead. Measure at your own sizes before enabling `gpu`.
- Repeated runs on the same machine vary by several percent; an earlier run recorded 3.8 / 27 / 21 / 3.6 / 21 ms.

## 2. Rules for any comparison

1. **Check parity before timing.** Two implementations are only compared once their outputs agree within a stated
   tolerance (maximum absolute channel difference, PSNR, or IoU for shapes). Different borders, interpolation or
   rounding make a comparison meaningless.
2. **Report conversion cost separately.** Moving an `image::RgbaImage` into another library's container (for example
   an OpenCV `Mat`, with its BGR channel order) is part of the wrapper's cost to the caller but not of its kernel. Both
   numbers are shown.
3. **Same work.** Same input, same output size, same channel count, same interpolation. When a library cannot do the
   same work (for example no alpha), say so next to the number.
4. **Record the environment.** CPU, GPU, OS, rustc, crate versions, and for OpenCV its version and build flags.
5. **Warm and steady.** Criterion warm-up and at least 100 samples; GPU benchmarks use the shared context so pipeline
   compilation is excluded.
6. **Report losses.** Cases where another library is faster are published with the same prominence.

## 3. Planned: comparison with the `opencv` crate

Specified in [`plans/OPENCV_STYLE_DOCS_PLAN.md`](plans/OPENCV_STYLE_DOCS_PLAN.md) §4 and decided in
[ADR-0003](adr/0003-opencv-comparison-in-its-own-workspace.md). It runs from a separate workspace so that libopencv and
libclang never touch this crate.

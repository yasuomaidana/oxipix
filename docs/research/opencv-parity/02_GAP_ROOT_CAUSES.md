# 02: Gap root causes, current measurements and feasibility spike

## 1. Method

- **Fixtures and operations** are those of `rust-cv-guide/comparisons/opencv` (`bench.cpp`): the same images, sizes,
  parameters and timing rule. That rule is 3 warm-ups, then at least `iters` samples and at least 200 ms, capped at
  5000 samples, and the median is reported. Every Rust call allocates its output. It is compared with OpenCV's
  `__fresh` timing, which also releases its output first.
- **OpenCV** was rebuilt for this study from the guide's `bench.cpp`. The only change is that the thread count is
  read from `OCV_THREADS` (default 1). The build reports the KleidiCV 26.09 and carotene HALs, a NEON baseline and
  TBB. Results: [`results/small/opencv_5.1_t1.json`](results/small/opencv_5.1_t1.json) and
  [`results/small/opencv_5.1_t8.json`](results/small/opencv_5.1_t8.json).
- **Rust** was built with fat LTO, one codegen unit and `-C target-cpu=native`:
  - oxipix `main` at `412b476`, feature `imgproc`. The OXI-97 `parallel` feature was taken from `ced2ceb` (open PR #8).
  - kornia-imgproc at `f60a856` (HEAD on 2026-11-27). crates.io 0.2.0 gives the same numbers within noise:
    [`results/small/kornia_0.2.0_and_spike_t1.txt`](results/small/kornia_0.2.0_and_spike_t1.txt).
  - the spike kernels: [`results/harness/kbench_spike.rs.txt`](results/harness/kbench_spike.rs.txt).
- **Parity** means the output digest (size and mean sample) matches OpenCV's before any ratio is trusted, as in the
  guide. Where the two differ, the row says so.

To reproduce, build the OpenCV bench as the guide's `run.sh` does, then run
`RAYON_NUM_THREADS={1,8} <obench|kbench> <assets dir> results/<size>/opencv_5.1_t{1,8}.json`, where the medium and large asset dirs are produced by `scale_fixtures`.

## 2. Results: ratio of Rust time to OpenCV time (lower is better; below 1.0x, Rust is faster)

| Operation | OpenCV 1 thr (ms) | Guide PERF_QUEUE best gap | oxipix 412b476 | kornia f60a856 | spike | **best Rust now** | OpenCV 8 thr (ms) | oxipix + parallel (OXI-97) | kornia 8 thr |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `template_matching` (TM_SQDIFF) | 1.633 | 75.3x | 1.26x | - | - | **1.26x** | 0.890 | 2.29x | - |
| `smoothing_median` (5x5 RGB) | 0.406 | 41.3x | 7.45x | 102.04x | - | **7.45x** | 0.095 | 32.58x | 99.10x |
| `erosion` (5x5) | 0.010 | 28.1x | 2.96x | 681.60x | 2.35x | **2.35x** | 0.010 | 2.97x | 139.71x |
| `pyramids` (3x pyrDown) | 0.045 | 19.4x | 2.72x | 7.54x | - | **2.72x** | 0.038 | 3.40x | 5.79x |
| `hough_circle` | 3.746 | 16.2x | 0.23x | - | - | **0.23x** (count not checked) | 1.384 | 0.64x | - |
| `opening_closing_hats` (top-hat 7x7) | 0.028 | 14.7x | 2.20x | - | - | **2.20x** | 0.029 | 2.19x | - |
| `smoothing_gaussian` (5x5, sigma 1.1) | 0.198 | 9.1x | 1.10x | 0.54x | 0.85x | **0.54x** | 0.081 | 1.48x | 0.88x |
| `smoothing_bilateral` | 0.806 | 7.3x | 3.29x | - | - | **3.29x** | 0.435 | 1.73x | - |
| `homography` (warpPerspective) | 0.588 | 7.1x | 3.41x | 2.60x | - | **2.60x** | 0.132 | 4.74x | 2.78x |
| `canny` (blur 1.4 + Canny) | 2.172 | 6.9x | 0.77x | 0.62x | - | **0.62x** | 0.679 | 2.31x | 1.08x |
| `warp_affine` | 0.266 | 6.3x | 2.57x | 0.96x | - | **0.96x** | 0.067 | 4.05x | 1.65x |
| `adding_images` (addWeighted) | 0.038 | 5.8x | 1.48x | - | - | **1.48x** | 0.021 | 2.71x | - |
| `smoothing_box` (5x5) | 0.239 | 5.7x | 0.91x | 0.45x | - | **0.45x** | 0.239 | 0.50x | 0.31x |
| `feature_detection_fast` (FAST-9 t=30, NMS) | 0.588 | 5.6x | 4.24x | - | - | **4.24x** | 0.571 | 4.32x | - |
| `sobel` (x, 3x3) | 0.142 | 5.5x | 0.62x | - | - | **0.62x** | 0.141 | 0.62x | - |
| `basic_linear_transform` (convertTo) | 0.063 | 5.4x | 1.66x | - | - | **1.66x** | 0.024 | 4.28x | - |
| `filter_2d` (5x5 box, f32) | 0.662 | 4.3x | 1.17x | - | - | **1.17x** | 0.662 | 0.40x | - |
| `distance_transform` (L2) | 0.231 | 3.3x | 4.74x | - | - | **4.74x** (different work) | 0.248 | 4.44x | - |
| `hough_lines` | 6.418 | 3.2x | 0.92x | - | - | **0.92x** | 2.240 | 2.64x | - |
| `psnr` | 0.147 | 2.7x | 0.95x | - | - | **0.95x** | 0.093 | 1.50x | - |
| `ssim` | 3.081 | 2.0x | 1.81x | - | - | **1.81x** | 3.092 | 1.79x | - |
| `threshold` (Otsu) | 0.031 | 1.5x | 0.92x | - | - | **0.92x** | 0.032 | 0.87x | - |
| `equalize` (equalizeHist) | 0.253 | 1.2x | 0.92x | 1.91x | - | **0.92x** | 0.520 | 0.46x | 0.42x |

"-" means the backend has no comparable operation in the run: either kornia has only an `f32` or single-channel
variant, or the harness does not exercise it. The `find_contours` probe (contours only, no approxPolyDP) gave kornia
0.48x (0.075 ms against 0.155 ms). It is not in the table because the OpenCV row also includes approxPolyDP.

### Parity notes

The digest is the output size plus the mean sample. It is a cheap check, not a full comparison; the golden-output gate in [04](04_ARCHITECTURE.md) §9 replaces it.

- **Digest matches OpenCV to three decimals:**
  - box blur, median, bilateral, `filter2D`, Otsu, erosion, top-hat and `warpPerspective`, for every Rust backend that ran them;
  - Gaussian, for the spike only (oxipix and kornia: 115.310 against 115.305);
  - pyrDown, for kornia only (oxipix: 115.309 against 115.276).
- **Digest differs, so the row is not like for like and its ratio cannot count as parity yet:**

  | Operation | Rust | OpenCV | Notes |
  |---|---|---|---|
  | `addWeighted` | 57.821 | 57.545 | |
  | `convertTo` | 156.531 | 156.491 | |
  | `equalizeHist` | 127.903 | 127.731 | |
  | Sobel | 39.657 | 39.602 | |
  | Canny | 13.857 (oxipix), 13.818 (kornia) | 13.523 | see below |
  | `warpAffine` | 41.108 (oxipix), 41.203 (kornia) | 41.256 | see below |

  The likely causes are borders (oxipix replicates the edge, OpenCV uses `REFLECT_101`), rounding, and JPEG decoding (the two logo fixtures are JPEGs, decoded by different libraries).
- **Canny:** both sides use the L2 gradient norm. The differences are:
  - oxipix blurs with 7 taps (`ceil(2 * 1.4)` = radius 3) where OpenCV uses 9;
  - oxipix skips the 1-pixel frame in non-maximum suppression (`detect/canny.rs:47-48`);
  - the borders differ.

  The fix can use `gaussian_blur_radius(image, 1.4, 4)`.
- **`warpAffine`:** `imgproc::warp` already uses a constant 0 border. The two differences are that it rejects any source position outside `[0, w-1]`, where OpenCV blends the edge with the border colour (`geometry/warp.rs:19-21`), and that it uses f32 coordinates where OpenCV uses `INTER_BITS` fixed point.
- **`distanceTransform`:** OpenCV runs `DIST_L2` with `DIST_MASK_3`, a 3x3 chamfer approximation (`distransform.cpp`). oxipix computes the exact Euclidean transform. The ratio compares different work.
- **FAST-9:** 2097 corners (oxipix) against 2091 (OpenCV), close enough.
- **Hough circles:** the OpenCV probe records no circle count. The radius limits (15 to 40 px) and the homography corner points are not scaled for the medium and large fixtures, so those rows measure time only (§6).

## 3. Root causes, per operation

OpenCV's choice of algorithm comes from reading the 5.1.0 source and the KleidiCV HAL adapters.

| Operation | What OpenCV / KleidiCV does | What the slowest Rust crates did | Status in the best Rust crate | Where to fix |
|---|---|---|---|---|
| matchTemplate SQDIFF | DFT cross-correlation in tiles (`templmatch.cpp`, `crossCorr`), with window energies from integral images | imageproc: direct `positions x template` sum | oxipix: FFT plus integral images (1.26x). The remainder is `rustfft` planning and allocation per call. | oxipix: cache the FFT plan and scratch buffers |
| medianBlur 5x5 | KleidiCV NEON sorting network on each channel (`median_blur_sorting_network_5x5.h`); OpenCV's own path uses `medianBlur_SortNet` for k ≤ 5 and the O(1) histogram above | imageproc: Huang sliding histogram; kornia: generic path for 3 channels (its NEON path is single-channel only) | oxipix already splits the channels (`filter/median.rs:113-130`) and runs `[u8; 16]` min/max lanes. But its 5x5 network has 234 exchanges (`generated/median_networks.rs:17`), where known median-of-25 networks use about 100; it reloads 25 lanes for every 16 pixels without reusing sorted columns; and it copies the padded plane and gathers each channel with `step_by` (7.45x) | oxipix: a smaller network, reused column sorts, fewer copies. kornia: upstream PR for 3 channels |
| erode / dilate / top-hat | KleidiCV `morphology_neon` (separable row and column min/max); OpenCV's fallback is a vectorised separable filter | imageproc: through a distance transform; kornia: generic padded window loop | oxipix: separable passes in O(log window) (2.2x to 3x), with about six buffers allocated per call, including a padded copy of the whole image (`filter/morph.rs:63-85`); the spike reaches 2.35x | Fuse the passes, reuse a scratch arena, explicit u8x16 |
| pyrDown | KleidiCV `blur_and_downsample` (one fused pass) | kornia: f32 path for 3 channels | oxipix: filters only the kept rows, but computes every column and then drops the odd ones (`filter/pyramid.rs:41-49`) (2.72x) | oxipix: fuse the column pass and the decimation, fixed-point 16-bit lanes |
| GaussianBlur | KleidiCV `gaussian_blur_fixed` (8-bit fixed point, separable) | imageproc: f32 generic | kornia 0.54x; spike 0.85x; oxipix 1.10x | Already at parity. oxipix can take the spike's u16-to-u32 vertical pass. |
| bilateralFilter | Weight tables plus vectorised gathers (`bilateral_filter.simd.hpp`) | imageproc/oximedia: per-pixel `exp` | oxipix: weight tables (3.29x) | Vectorise across 16 pixels with weights per radius ring |
| warpPerspective / warpAffine | KleidiCV `warp_perspective_stripe` / remap with fixed-point coordinates (`INTER_BITS`) | oxipix: per-pixel f32 projection | kornia: Q10 fixed point, incremental per row (0.96x affine) | oxipix: take kornia's approach (or call kornia) |
| Canny | KleidiCV `canny` | imageproc: f32 | kornia 0.62x, oxipix 0.77x | At parity once the blur tap count matches |
| FAST-9 | SIMD test of the 16-pixel ring (`fast.cpp`, `fast_score.cpp`) | oximedia: wrong ring; imageproc: scalar | oxipix: scalar ring test with wrap-around (4.24x) | oxipix: test brighter and darker pixels for 16 candidates at once |
| addWeighted / convertTo | Universal intrinsics, fixed point | f32 per pixel | oxipix 1.48x / 1.66x | oxipix: Q15 fixed point in u16 lanes |
| distanceTransform | 3x3 chamfer (approximate) | different work | see the parity notes | Fix the probe |
| SSIM | Five `GaussianBlur` passes on f32 | – | oxipix 1.81x | oxipix: share the private f32 11-tap blur in `color/quality.rs:50` and fuse the products |

**Most gaps were algorithmic** (DFT matching, separable morphology, sorting networks), and they are closed. What is
left is SIMD shape (channel splitting, fixed point, 16 lanes) and fixed per-call costs on small images (allocation,
borders). Rust has no language barrier here. Both the spike and kornia beat KleidiCV on separable filters with safe or
`wide` code.

## 4. Feasibility spike

Two kernels were written in the style a kernel macro would generate: safe slices, no intrinsics, no `unsafe`, and
LLVM's auto-vectoriser left to do the SIMD.

| Kernel | Spike | OpenCV + KleidiCV | Ratio | Digest |
|---|---:|---:|---:|---|
| Gaussian 5x5, sigma 1.1, RGB, REFLECT_101, 8-bit fixed point per pass | 0.168 ms | 0.198 ms | **0.85x** | 115.305 = 115.305 |
| Erode 5x5, rectangle, separable min | 0.0243 ms | 0.0103 ms | 2.35x | 60.824 = 60.824 |

So safe Rust that auto-vectorises can beat KleidiCV's NEON Gaussian on this image. The erosion runs in about 24 µs,
where the two passes and the intermediate buffer are a real share of the cost. OpenCV's 10 µs comes from a fused
NEON kernel with no temporary image. Closing that gap needs an explicit-SIMD variant or a fused kernel, which the
architecture in [04](04_ARCHITECTURE.md) provides for.

## 5. Threading

At 8 threads, OpenCV (TBB) speeds up median 4.3x, Canny 3.2x, `warpAffine` 4.0x, `warpPerspective` 4.4x and Hough
2.7x to 2.9x. Most morphology, Sobel and threshold operations do not speed up, because the images are too small.
`equalizeHist` slows down by 2x. The OXI-97 `parallel` feature speeds up box (1.8x), Gaussian (1.8x), `filter2D` (2.9x), bilateral
(3.5x), `warpAffine` (2.5x) and `warpPerspective` (3.2x). It does not yet cover median, Canny, Hough or pyrDown, which is where the 8-thread
gaps widen. **A size threshold for each operation is required.** OpenCV's own `equalizeHist` regression shows the
cost of parallelising small images.

## 6. Medium and large images

The guide fixtures are small (0.07 to 0.5 MP), so fixed per-call costs dominate them. Every fixture was also upscaled
(Lanczos3, same names and formats, JPEG at quality 95) to a 1920 px width ("medium", about 2 to 3.7 MP) and a 3840 px
width ("large", about 8 to 14.7 MP), with [`results/harness/scale_fixtures.rs.txt`](results/harness/scale_fixtures.rs.txt).
OpenCV and the Rust harnesses read the same files. Raw results: `results/medium/` and `results/large/` (small: `results/small/`).

Each cell gives OpenCV's time, oxipix's ratio, and the best Rust ratio with its crate. All are single-threaded unless
the column says otherwise.

| Operation | small | medium | large | large, 8 thr: OpenCV ms / best Rust (crate) |
|---|---|---|---|---|
| `template_matching` | 1.633 / 1.26x / 1.26x (oxipix) | 26.158 / 1.52x / 1.52x (oxipix) | 104.075 / 1.70x / 1.70x (oxipix) | 51.129 / 3.40x (oxipix+par) |
| `smoothing_median` | 0.406 / 7.45x / 7.45x (oxipix) | 6.573 / 8.12x / 8.12x (oxipix) | 25.635 / 8.45x / 8.45x (oxipix) | 5.232 / 12.14x (oxipix+par) |
| `erosion` | 0.010 / 2.96x / 2.35x (spike) | 0.238 / 2.74x / 2.25x (spike) | 0.921 / 3.10x / 2.44x (spike) | 0.919 / 3.40x (oxipix+par) |
| `pyramids` | 0.045 / 2.72x / 2.72x (oxipix) | 0.733 / 2.96x / 2.96x (oxipix) | 3.664 / 2.61x / 2.61x (oxipix) | 0.966 / 2.38x (oxipix+par) |
| `hough_circle` (radii not scaled) | 3.746 / 0.23x / 0.23x (oxipix) | 9.723 / 1.63x / 1.63x (oxipix) | 18.345 / 2.65x / 2.65x (oxipix) | 23.777 / 1.50x (oxipix+par) |
| `opening_closing_hats` | 0.028 / 2.20x / 2.20x (oxipix) | 0.645 / 2.08x / 2.08x (oxipix) | 2.628 / 2.43x / 2.43x (oxipix) | 2.513 / 2.63x (oxipix+par) |
| `smoothing_gaussian` | 0.198 / 1.10x / 0.54x (kornia) | 3.432 / 1.16x / 0.49x (kornia) | 13.704 / 1.19x / 0.49x (kornia) | 3.582 / 0.68x (kornia) |
| `smoothing_bilateral` | 0.806 / 3.29x / 3.29x (oxipix) | 14.700 / 3.29x / 3.29x (oxipix) | 59.382 / 3.28x / 3.28x (oxipix) | 13.125 / 3.68x (oxipix+par) |
| `homography` (points not scaled) | 0.588 / 3.41x / 2.60x (kornia) | 5.271 / 3.52x / 2.58x (kornia) | 35.616 / 2.48x / 1.80x (kornia) | 5.885 / 2.01x (kornia) |
| `canny` | 2.172 / 0.77x / 0.62x (kornia) | 8.425 / 0.84x / 0.60x (kornia) | 22.505 / 1.05x / 0.69x (kornia) | 7.570 / 0.94x (kornia) |
| `warp_affine` | 0.266 / 2.57x / 0.96x (kornia) | 3.613 / 2.70x / 1.02x (kornia) | 15.895 / 2.62x / 1.00x (kornia) | 3.565 / 1.52x (kornia) |
| `adding_images` | 0.038 / 1.48x / 1.48x (oxipix) | 1.355 / 1.73x / 1.73x (oxipix) | 5.437 / 1.50x / 1.50x (oxipix) | 1.428 / 5.70x (oxipix+par) |
| `smoothing_box` | 0.239 / 0.91x / 0.45x (kornia) | 4.324 / 0.99x / 0.39x (kornia) | 17.254 / 0.95x / 0.39x (kornia) | 17.226 / 0.14x (kornia) |
| `feature_detection_fast` | 0.588 / 4.24x / 4.24x (oxipix) | 0.871 / 11.35x / 11.35x (oxipix) | 1.753 / 19.86x / 19.86x (oxipix) | 1.756 / 20.16x (oxipix+par) |
| `sobel` | 0.142 / 0.62x / 0.62x (oxipix) | 0.665 / 0.69x / 0.69x (oxipix) | 2.780 / 0.81x / 0.81x (oxipix) | 2.099 / 0.97x (oxipix+par) |
| `basic_linear_transform` | 0.063 / 1.66x / 1.66x (oxipix) | 0.657 / 2.30x / 2.30x (oxipix) | 2.652 / 1.62x / 1.62x (oxipix) | 0.675 / 6.36x (oxipix+par) |
| `filter_2d` | 0.662 / 1.17x / 1.17x (oxipix) | 11.922 / 1.19x / 1.19x (oxipix) | 48.468 / 1.18x / 1.18x (oxipix) | 10.996 / 1.28x (oxipix+par) |
| `distance_transform` (different work) | 0.231 / 4.74x / 4.74x (oxipix) | 6.323 / 4.33x / 4.33x (oxipix) | 24.857 / 4.56x / 4.56x (oxipix) | 24.797 / 4.20x (oxipix+par) |
| `hough_lines` | 6.418 / 0.92x / 0.92x (oxipix) | 17.192 / 0.94x / 0.94x (oxipix) | 34.203 / 0.93x / 0.93x (oxipix) | 16.439 / 1.62x (oxipix+par) |
| `psnr` | 0.147 / 0.95x / 0.95x (oxipix) | 0.716 / 1.05x / 1.05x (oxipix) | 3.074 / 0.93x / 0.93x (oxipix) | 2.290 / 1.25x (oxipix+par) |
| `ssim` | 3.081 / 1.81x / 1.81x (oxipix) | 16.727 / 1.68x / 1.68x (oxipix) | 70.251 / 1.67x / 1.67x (oxipix) | 38.138 / 3.13x (oxipix+par) |
| `threshold` | 0.031 / 0.92x / 0.92x (oxipix) | 0.788 / 0.97x / 0.97x (oxipix) | 3.438 / 0.99x / 0.99x (oxipix) | 3.443 / 0.98x (oxipix+par) |
| `equalize` | 0.253 / 0.92x / 0.92x (oxipix) | 1.676 / 0.71x / 0.71x (oxipix) | 8.410 / 0.59x / 0.59x (oxipix) | 1.929 / 1.37x (kornia) |

Counts (best Rust crate, single-threaded):

| Size | ≤ 1.0x | ≤ 1.5x | ≤ 3x | oxipix alone ≤ 1.0x | 8 threads, ≤ 1.5x |
|---|---:|---:|---:|---:|---:|
| small | 10 | 13 | 19 | 8 | 8 |
| medium | 7 | 10 | 19 | 6 | 9 |
| large | 8 | 11 | 19 | 6 | 9 |

What size changes:

- **FAST gets much worse as images grow**: 4.2x, then 11.4x, then 19.9x. OpenCV's SIMD ring test scales, and oxipix's
  scalar test does not. It becomes the top priority of phase 2.
- **Median (about 8x), bilateral (3.3x), the distance transform and morphology (2 to 3x) do not depend on size.** These
  are per-pixel cost problems, not overheads.
- **Template matching drifts from 1.26x to 1.70x.** That matches FFT plans and buffers being rebuilt per call, plus FFT
  size growth.
- **`equalizeHist` and `homography` get better at large sizes.** Their fixed costs are amortised.
- **The Hough circle row is not comparable** above small, because its radius limits were not scaled (the probe fix in
  phase 0).
- **At 8 threads, the ops that OXI-97 does not parallelise fall behind**: `addWeighted` 5.7x, `convertTo` 6.4x, SSIM
  3.1x, template matching 3.4x. The parallel layer needs to reach them.

## 7. Build profile

The numbers above use fat LTO and `-C target-cpu=native`. crates.io users get the default release profile, so oxipix
was also built without those options:
`oxipix_412b476_defaultprofile_t1.txt` in [`results/small/`](results/small/) and [`results/large/`](results/large/).

| Change from the tuned build | Operations |
|---|---|
| Within 5% (same speed) | most operations |
| Slower | `convertTo` +23%, `equalizeHist` +30% (small) / +31% (large), median +6% |
| Suspect | PSNR reads 0.09x in the default profile, which is not plausible. The call is probably being optimised away, so that row is not reported. |

On aarch64, NEON is part of the baseline target, so `native` adds little here. On x86_64 the gap will be larger (SSE2
baseline), and that is why G1 gates on the default profile.

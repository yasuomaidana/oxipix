# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `imgproc` (default feature): OpenCV-style kernels on 8-bit images that the compiler can vectorise.
  Wave 1: `imgproc::template::match_template` (FFT cross-correlation with integral-image window energies; the four
  `imageproc` scores), `imgproc::morph` (`erode`, `dilate`, `open`, `close`, `top_hat`, `black_hat`, `gradient` with a
  rectangular window in `O(log window)` whole-row passes) and `imgproc::median::median_blur` (selection network over
  16 pixels at a time for 3x3 and 5x5 windows, window selection for larger radii).
- `imgproc` wave 2: `blur` (`box_blur`, `gaussian_blur`, integer separable passes in 16-bit lanes), `pyramid::pyr_down`
  (only the kept rows are filtered), `gradient` (`sobel_x`, `sobel_y`, `laplacian` into `i16`, `scale_abs`,
  `normalize_abs`), `canny` (integer direction sectors, stack-based hysteresis), `bilateral::bilateral_filter`
  (spatial and colour weight tables, as OpenCV) and `filter2d::filter2d` (tap-ordered `f32` multiply-add).
- `imgproc` wave 3: `arith` (`add_weighted`, `convert_scale`), `threshold` (histogram, Otsu, binary threshold, fused
  HSV range test), `equalize::equalize_hist`, `quality` (`mse`, `psnr`, `ssim`), `warp` (affine and perspective with
  bilinear sampling, `homography`), `features` (FAST-9 with wrap-around arcs, non-max suppression, BRIEF through an
  integral image, Hamming matching), `hough` (standard line transform with OpenCV's peak rule, gradient-method
  circles) and `distance` (exact Euclidean, city-block and chessboard transforms). The modules are grouped in
  `filter`, `detect`, `geometry` and `color` and re-exported as `imgproc::<name>`.
- `parallel` feature (off by default, needs `imgproc`): `box_blur`, `gaussian_blur`, `pyr_down`, `median_blur`,
  `bilateral_filter`, `warp_affine`, `warp_perspective` and `filter2d` split their output rows into bands and run them on
  the rayon pool, but only for at least about a quarter of a megapixel of output, so small pictures stay on the calling
  thread. Results are identical with and without the feature (tests force both paths). The thread count is the
  caller's rayon pool (`ThreadPoolBuilder::install`).
- Benchmark `imgproc_vs_imageproc` and the dependencies `realfft` and `rustfft` (only with `imgproc`).

## [0.1.0] - 2026-11-24

First release on crates.io. Earlier `0.1.x` and `0.2.0` numbers were internal and were never published.

### Added

- `mosaic` (default feature): strip overlap detection (`detect_strip_overlap_zmad`), ECC drift estimation, quadrilateral
  unwarping (`unwarp_quad_rgba`, `auto_dewarp_dominant_quad`), pairwise and multi-frame feathered stitching, luma helpers.
- `vectorize` (default feature): Otsu thresholding, Moore-neighbour contour tracing, RDP simplification, SVG output.
- `gpu` (opt-in feature): wgpu 3x3 convolution and bilinear perspective warp through `GpuImageExt`, with a CPU fallback.
- Examples `unwarp_receipt`, `vectorize_svg` and `gpu_blur`; the README is the crate's rustdoc front page.
- Quality gates in CI: zero duplication, cyclomatic complexity 10 and cognitive 15, 500 lines per file, 100% line
  coverage, feature matrix, MSRV, rustdoc, cargo-deny and a package dry run.
- Golden-output tests that pin the behaviour of every public `mosaic` and `vectorize` function.

### Changed

- Minimum supported Rust version is 1.89.
- `thiserror` is an optional dependency, enabled by `mosaic` and `gpu`.

### Known limitations

- `estimate_orthogonal_drift_ecc` returned 0 for strips shifted by 1 to 3 px.
- `auto_dewarp_dominant_quad` finds an outlined page and crops to its bounding box.
- With `gpu`, `unwarp_quad_rgba` fills pixels outside the source with white; the CPU warp clamps to the edge.
- The Moore tracer stops a contour after 1000 steps.

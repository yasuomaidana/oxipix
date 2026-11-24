# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

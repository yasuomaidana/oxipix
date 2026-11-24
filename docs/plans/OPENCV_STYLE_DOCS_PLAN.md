# Plan: OpenCV-style documentation and performance comparison

| | |
|---|---|
| Status | Plan only. Nothing in this document is built yet. |
| Roadmap items | RM-DOCS, RM-BENCH ([`../03_ROADMAP.md`](../03_ROADMAP.md)) |
| Decisions | [ADR-0002](../adr/0002-docs-as-an-unpublished-package.md), [ADR-0003](../adr/0003-opencv-comparison-in-its-own-workspace.md) |

## 1. Goal

Documentation for oxipix as complete and as navigable as [docs.opencv.org](https://docs.opencv.org/4.x/): a reader can
learn a topic from a tutorial, look up any function in a reference, and see how it maps to what they used before. A
performance section shows, with reproducible numbers, where oxipix is faster than the [`opencv`](https://crates.io/crates/opencv)
wrapper crate, where it is not, and what each approach costs to build and ship.

## 2. Package layout

Nothing here is published (ADR-0002).

```text
website/                    Docusaurus site, package.json has "private": true
  docs/
    introduction/           install, features, platforms, MSRV
    tutorials/{core,gpu,mosaic,vectorize,layout}/
    reference/{gpu,mosaic,vectorize}/   one page per public item
    migration/opencv.md     function mapping, side-by-side code
    performance/            charts rendered from the committed result files
    bibliography.md
  static/bench/*.json       result files written by the comparison workflow
  snippets/                 Rust package, publish = false, workspace member
    examples/*.rs           one runnable example per tutorial
    tests/*.rs              asserts the numbers the tutorials print
comparisons/opencv/         separate workspace, publish = false, NOT a member of the root workspace
  benches/*.rs              criterion benchmarks, one per operation pair
  src/parity.rs             output-parity checks run before any timing
```

The tutorials show code by importing the files in `website/snippets/` (for example with Docusaurus's
`raw-loader` and `CodeBlock`), so every snippet on the site is compiled by `cargo check --examples` and every number a
tutorial prints is asserted in CI. Result images ("Result" sections) are produced by the same examples and committed.

`comparisons/opencv/` stays out of the root `Cargo.toml` workspace (`[workspace] exclude`) so the published crate, its
lockfile and the default CI never see `opencv`, libclang or libopencv.

## 3. Information architecture (modelled on docs.opencv.org)

### 3.1 Introduction

Install (`cargo add oxipix`, feature flags), the `image` features oxipix does not enable for you, supported
platforms and GPU backends, MSRV, and a "choosing between oxipix, `image`/`imageproc` and `opencv`" page.

### 3.2 Tutorials, one section per module

Every tutorial page follows OpenCV's template: **Goal** (what you will learn), **Theory** (the algorithm, with math and
figures), **Code** (the compiled snippet), **Explanation** (step by step), **Result** (images or printed output).

| Module | Tutorials |
|---|---|
| core | Images and buffers: `image::ImageBuffer` and oxipix; luma and per-channel mapping |
| gpu | Filtering on the GPU; perspective warp on the GPU; the CPU fallback; when the GPU pays off |
| mosaic | Straighten a receipt (quad unwarp); find strip overlap (ZMAD); estimate drift (ECC); stitch strips with feathering; a document-scanner pipeline end to end |
| vectorize | Otsu thresholding; Moore contour tracing; RDP simplification; raster to SVG |
| layout | Planned feature; page written when `LAY-1..4` land |

### 3.3 API reference, module to group to function

Mirrors OpenCV's "Modules" tree. Each public item gets one page with: signature, parameters, returns and errors, notes
and math, a **"OpenCV equivalent"** box, see-also links, a runnable example, and a link to the item on docs.rs.

A CI check enumerates the public items from the crate's rustdoc JSON and fails if any has no reference page, or if a
page names an item that no longer exists.

### 3.4 Migrating from `opencv`

A mapping table, then side-by-side code for each row. Rows are taken from this table (OpenCV names are the Rust crate's
snake_case functions); "closest" means the algorithms differ in the ways the notes say.

| oxipix | Closest OpenCV (`opencv` crate) | Notes |
|---|---|---|
| `GpuImageExt::gpu_filter3x3`, `GpuContext::convolve3x3` | `imgproc::filter_2d` (CUDA: `cudafilters`) | oxipix clamps to the edge and leaves alpha alone |
| `gpu_warp_perspective`, `GpuContext::warp_perspective` | `imgproc::warp_perspective` (CUDA: `cudawarping`) | oxipix takes the *inverse* homography and fills outside pixels with white |
| `mosaic::unwarp_quad_rgba` | `imgproc::get_perspective_transform` + `warp_perspective` | one call; crops when the quad is axis-aligned |
| `mosaic::auto_dewarp_dominant_quad` | `threshold`/`canny` -> `find_contours` -> `approx_poly_dp` -> `get_perspective_transform` -> `warp_perspective` | OpenCV's pipeline estimates a perspective quad; oxipix currently crops to a bounding box (RM-QUAD) |
| `mosaic::rgb_to_luma_u8`, `rgba_to_luma_vec` | `imgproc::cvt_color` (`COLOR_RGBA2GRAY`) | both use BT.601 weights |
| `mosaic::map_rgb_channels` | `core::split` / `merge`, `core::lut` | |
| `mosaic::detect_strip_overlap_zmad` | `imgproc::match_template` + `core::min_max_loc`, or `core::phase_correlate` | ZMAD is a sparse-sampled zero-mean absolute difference |
| `mosaic::estimate_orthogonal_drift_ecc` | `video::find_transform_ecc` (`MOTION_TRANSLATION`) | see the known limitation in [architecture §5](../01_ARCHITECTURE.md) |
| `mosaic::stitch_pair_vertical` / `horizontal` | `core::vconcat` / `hconcat` + `core::add_weighted` on the overlap | |
| `mosaic::stitch_frames_feathered` | `stitching::Stitcher` (scans mode) or `Detail_FeatherBlender` | |
| `vectorize::otsu_threshold` | `imgproc::threshold` with `THRESH_OTSU` | OpenCV returns the chosen threshold; oxipix's convention is `<=` |
| `vectorize::trace_moore_neighbor_contours` | `imgproc::find_contours` | OpenCV uses Suzuki-Abe border following, oxipix uses Moore-neighbour tracing |
| `vectorize::simplify_rdp` | `imgproc::approx_poly_dp` | both are Ramer-Douglas-Peucker |
| `vectorize::vectorize_rgba_to_svg`, `VectorizedGraphic::to_svg_string` | none | OpenCV has no SVG output |

Two things OpenCV does not give you that oxipix does, and one it does not need: GPU compute through wgpu (Metal,
Vulkan, DX12) without CUDA or a system OpenCV; one `cargo add` with no native toolchain; SVG output. OpenCV's own
OpenCL transparent API and CUDA modules are the comparable GPU routes and depend on how the installed OpenCV was built.

### 3.5 Performance, bibliography, contributing

The performance pages render the committed result files (section 4). The bibliography cites Otsu (1979),
Douglas-Peucker (1973), Evangelidis and Psarakis on ECC (2008), Moore-neighbour tracing, and the sources of the BT.601
weights.

## 4. Performance comparison with the `opencv` crate

### 4.1 Setup

- A separate workspace, `comparisons/opencv/` (ADR-0003). It depends on `oxipix` by path, `opencv` (0.102 at the time of
  writing, which needs a system OpenCV 4.x or 5.x, libclang and a C++ compiler), `image`, `imageproc` and `criterion`.
- One criterion benchmark per operation pair in the migration table, each measuring: oxipix on the GPU, oxipix on the
  CPU, `opencv`, and `imageproc` where it has an equivalent.
- Sizes: 640 x 480, 1600 x 1200 and 4000 x 3000, so the crossover where the GPU starts to pay off is visible.
- Inputs: generated in code with a fixed seed, plus a small fixed set of real document photographs checked into
  `comparisons/opencv/fixtures/` if licence-clean ones are available.

### 4.2 Fairness rules

These repeat the rules in [`../04_BENCHMARK_METHODOLOGY.md`](../04_BENCHMARK_METHODOLOGY.md) §2 and are enforced in
code:

1. `src/parity.rs` runs first and fails the benchmark when two outputs differ by more than the stated tolerance (PSNR
   for images, IoU for shapes). No timing of mismatched work is ever published.
2. The conversion cost (`RgbaImage` to `Mat`, BGR channel order, and back) is measured and reported separately from the
   kernel, because `opencv`'s API uses its own container and channel order. `Mat::new_rows_cols_with_data` can borrow
   the buffer without a copy; reading the result back to an `image` type is one copy.
3. Same interpolation, same border policy (set explicitly on both sides), same output size.
4. The OpenCV version, how it was built (CUDA, OpenCL, IPP), the CPU and GPU, OS and rustc are written into every result
   file.
5. Wins and losses are reported the same way.

### 4.3 What is measured beyond runtime

| Metric | How |
|---|---|
| Clean build time | `cargo build --release` from an empty target directory, wall clock, on the CI runner |
| Binary size | Release binary of the example, stripped |
| System requirements | Written list: libopencv, libclang, C++ toolchain versus none |
| Target matrix | Does it build for `wasm32-unknown-unknown`, `aarch64-linux-android`, `aarch64-apple-ios`, and a cross-compiled Linux target? Recorded as pass/fail with the error, per crate |
| GPU backends | Which GPU APIs each route can use (wgpu: Metal, Vulkan, DX12; OpenCV: depends on the CUDA/OpenCL build) |

The wasm and mobile rows for oxipix are also unverified today (see the roadmap) and are filled in by this work.

### 4.4 Running it

```bash
cd comparisons/opencv
cargo bench                        # needs OpenCV installed; see its README
python3 scripts/export_results.py  # writes website/static/bench/<date>-<machine>.json
```

A `workflow_dispatch` workflow runs it on an Ubuntu runner with `libopencv-dev`; macOS runs are done by hand and
committed with the same schema. The site's performance pages read these JSON files and draw bar charts per operation
and size, with the conversion cost stacked separately.

## 5. Phasing

| Phase | Deliverable | Done when |
|---|---|---|
| P1 | Scaffold and information architecture | The site builds; `website/snippets` is a workspace member; the reference-coverage check runs in CI; old 7-crate pages removed |
| P2 | Tutorials | Every module in section 3.2 has its pages; every snippet compiles; printed numbers are asserted |
| P3 | API reference | One page per public item with its OpenCV equivalent; the coverage check is green |
| P4 | Comparison | `comparisons/opencv` builds; parity checks pass; first result files committed; performance pages render them |
| P5 | Publishing | A workflow deploys the site (for example to GitHub Pages) on changes to `website/`; the README links to it |

## 6. Dependencies and risks

- RM-ECC and RM-QUAD change what two of the reference pages describe; write those pages after the fixes, or mark them
  with the current behaviour.
- The comparison needs a machine with OpenCV installed. If the hosted runner's OpenCV lacks CUDA or OpenCL, GPU rows for
  OpenCV are reported as "not available in this build" rather than omitted.
- `oximedia-cv` is young; a breaking release would need the snippets and the mapping updated, which the compiled
  snippets will surface.

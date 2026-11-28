# Pure-Rust OpenCV parity: summary and verdict

| | |
|---|---|
| Ticket | OXI-99 |
| Date | 2026-11-27 |
| Status | Research finding. The recommendation below is **proposed**: it becomes a decision only if [ADR-0005](../../adr/0005-opencv-parity-scope.md) is accepted. |
| Machine | Apple M3 (4 performance and 4 efficiency cores), macOS, Rust 1.89+. Tuned build (fat LTO, `-C target-cpu=native`), plus a check with the default release profile ([02](02_GAP_ROOT_CAUSES.md) §7). |
| OpenCV | 5.1.0 (Homebrew), with the KleidiCV 26.09 and carotene HALs, NEON baseline, TBB |

## Glossary

| Term | Meaning |
|---|---|
| HAL | OpenCV's Hardware Acceleration Layer: plug-in libraries that OpenCV calls first. **KleidiCV** is Arm's optimised image-processing library; **carotene** is the NEON library bundled with OpenCV. |
| REA | Reverse-engineering analysis with `dendrite retro`, the analysis tool of the author's dendrite toolchain. It indexes a source tree so the algorithms can be described without copying code. See [04](04_ARCHITECTURE.md) §10. |
| EGM | Elephant-Goldfish Model: a design draft is reviewed by agents that have no prior context. [05](05_EGM_REVIEW.md) holds the review of this design. |
| rust-cv-guide | [github.com/yasuomaidana/rust-cv-guide](https://github.com/yasuomaidana/rust-cv-guide), a sibling project that maps OpenCV tutorials to Rust crates and benchmarks them. Its `PERF_QUEUE.md` started this study. |
| Probe | The harness code that runs one operation on one library. |
| Ratio | Rust median time divided by OpenCV median time. Below 1.0x, Rust is faster. |
| Stripe | A contiguous range of output rows processed as one unit of work. |

## The question

Should oxipix become a pure-Rust OpenCV? The proposal was to build it with REA and EGM, as opt-in sub-crates, using
proc macros for repeated kernel patterns, opt-in multithreading, wgpu and WebAssembly. The rule: match OpenCV's
functionality at similar or better speed. Or are upstream changes to the existing crates enough?

## Findings

1. **OpenCV speed is reachable in Rust, and on most operations the gap is already small.** rust-cv-guide's
   `PERF_QUEUE.md` lists 23 operations, slower by 1.2x to 75x. Two crates changed that:
   - oxipix's new `imgproc` feature (OXI-94 to OXI-96, merged on 2026-11-27 as PRs #5 and #6);
   - kornia-rs, which the guide did not benchmark.

   Each operation was measured on one thread, with a newly allocated output on every call:

   | Image size | oxipix alone ≤ 1.0x | Best Rust crate ≤ 1.0x | Best ≤ 1.5x | Best ≤ 3x |
   |---|---:|---:|---:|---:|
   | small (guide fixtures, 0.07 to 0.5 MP) | 8 | 10 | 13 | 19 |
   | medium (1920 px wide) | 6 | 7 | 10 | 19 |
   | large (3840 px wide) | 6 | 8 | 11 | 19 |

   A 40-line safe-Rust prototype (the spike; no intrinsics, no `unsafe`) runs a 5x5 Gaussian at 0.85x of OpenCV's time.
   Its output mean matches OpenCV's.
2. **kornia-rs was missing from the comparison, and it is the strongest crate on some operations.** It is
   Apache-2.0, very active (at least 100 merged PRs in 90 days), and `kornia-imgproc` alone is about 100k lines.

   | | Operations |
   |---|---|
   | Faster than OpenCV at every size | box blur (0.39x to 0.45x), Gaussian (0.49x to 0.54x), Canny (0.60x to 0.69x) |
   | At parity | `warpAffine` |
   | 100x or more slower | 3-channel median, erosion |

   Its rayon dependency is mandatory, its GPU support is CUDA only, and it needs a 3-line patch to build for `wasm32`, with no wasm SIMD
   ([01](01_LANDSCAPE.md)).
3. **No single upstream crate meets the requirements: opt-in threads, wgpu, WebAssembly and OpenCV-level speed.**
   - imageproc is slow by design: generic `Pixel` code, and erosion through a distance transform.
   - oximedia-cv is the slowest, and has correctness bugs.
   - kornia-rs is the closest.

   **Filing PRs alone is not enough.**
4. **Replicating all of OpenCV is not needed.** Doing it would repeat the earlier "sovereign workspace" attempt: a
   seven-crate rewrite of image, imageproc and rxing that shipped stub decoders and ran slower than upstream, and was
   abandoned ([ADR-0001](../../adr/0001-single-crate-on-top-of-image.md)). About half of OpenCV's 17 modules (16
   main modules plus ml, now in contrib) are glue around problems that other crates already solve:
   - codecs, video I/O, DNN, GUI, ml and flann have several mature crates each;
   - most of calib and 3D is covered by kornia-rs.
5. **Portability favours oxipix's approach. Making kornia portable, or adopting a GPU framework, does not get there**
   ([06](06_PORTABILITY_AND_GPU_BACKENDS.md)).
   - **kornia and wasm.** kornia builds for wasm with a 3-line patch, but it has no wasm SIMD. Its box and Gaussian run
     16x to 17x slower there. oxipix's safe Rust, which the compiler vectorises, stays within 1.8x of native with
     `simd128` on 5 of 7 ops.
   - **kornia and non-CUDA GPUs.** A non-CUDA GPU path in kornia needs a storage redesign its maintainers have not
     approved: they removed CubeCL in July 2026.
   - **burn and candle.** burn-vision's image ops are tensor compositions or CPU fallbacks. candle has no non-CUDA,
     non-Metal GPU backend.
   - **oxipix's own GPU path.** With pooled buffers, oxipix's own WGSL wins on round trips. cubecl leads on resident
     compute by 1.2x to 1.5x through the browser's WGSL path, so it gets a defined evaluation protocol, run only when a
     trigger fires.
   - **Cross-compilation.** oxipix cross-checks on Linux (x86_64, aarch64), Windows, Android and wasm32. Its `gpu`
     feature builds for wasm with one wgpu feature.

## Proposed recommendation (pending ADR-0005): build the kernels, delegate the rest

| Area | Proposed action |
|---|---|
| imgproc (with features), photo, video background subtraction | **Build in oxipix** using the architecture in [04](04_ARCHITECTURE.md), continuing the `imgproc` work. Photo starts with what RM-ENHANCE (OXI-85, the receipt-enhancement roadmap item) needs. Video waits for a ticket with a consumer. |
| imgcodecs, videoio, dnn, ml, flann, highgui | **Delegate**: thin optional adapters over `image`/`zune`, `ffmpeg-next`/GStreamer, `burn`/`tract`/`ort`, `linfa`, `kiddo`, `minifb`/`egui`/`rerun`. |
| calib, geometry, stereo, ptcloud; Lucas-Kanade optical flow | **Reuse kornia-rs** and send fixes upstream. |
| GPU and wasm portability | Finish oxipix's own wgpu layer: async context for the browser, pooled buffers, a resident `GpuImage`, lavapipe CI. cubecl is evaluated only when a trigger fires. burn and candle are for DNN only ([06](06_PORTABILITY_AND_GPU_BACKENDS.md)). |
| Upstream | PRs to kornia-rs: 3-channel median, separable morphology, a `wasm32` build fix (3 lines, measured), optional rayon. Bug reports to oximedia-cv. kornia-rs and oxipix `imgproc` added as backends in the rust-cv-guide harness. |
| Guardrail | An operation is called "at parity" only after the golden-output and performance gates pass ([04](04_ARCHITECTURE.md) §9). |

[03](03_UPSTREAM_VS_NEW.md) gives the decision for each module, [04](04_ARCHITECTURE.md) the architecture (revision 2,
compared with OpenCV and KleidiCV), and [05](05_EGM_REVIEW.md) the EGM review and how each finding was resolved.

## Where Rust is still slower (best crate, one thread, large images)

| Operation | Best Rust / OpenCV (crate) | Likely cause |
|---|---:|---|
| FAST-9 | 19.86x (oxipix; 4.24x on small images) | Scalar ring test. OpenCV tests 16 pixels at once, and the gap grows with the image. |
| `medianBlur` 5x5 RGB | 8.45x (oxipix) | A 234-exchange network (about 100 is possible), sorted columns not reused, plane copies |
| `distanceTransform` | 4.56x (oxipix) | Not like for like: OpenCV runs the approximate 3x3 mask |
| `bilateralFilter` | 3.28x (oxipix) | Table lookups without vector gathers |
| erode, top-hat, pyrDown | 2.4x to 3.1x (oxipix or the spike) | Per-call allocations; pyrDown computes columns it then drops |
| `warpPerspective` | 1.80x (kornia) | KleidiCV uses a fixed-point remap |

At 8 threads (OpenCV with TBB; oxipix with the OXI-97 `parallel` feature, open as PR #8), the operations that OXI-97
does not parallelise yet fall further behind: median 12.1x, `addWeighted` 5.7x, `convertTo` 6.4x, SSIM 3.1x.

## Evidence and reproducibility

- **Raw results**: [`results/`](results/) holds the OpenCV JSON at 1 and 8 threads for the three sizes, and the
  oxipix, kornia-rs and spike timings.
- **Harness sources**: [`results/harness/`](results/harness/). They follow `rust-cv-guide/comparisons/opencv`:
  - 3 warm-ups;
  - at least `iters` samples (a count set for each operation) and at least 200 ms in total;
  - the median is reported;
  - a digest (the output size and mean sample) is compared with OpenCV's.
- **The fixture scaler**: `results/harness/scale_fixtures.rs.txt`.
- **REA analyses**: these come from the author's dendrite toolchain (not public), with
  `dendrite retro analyze <dir> --format source-tree`. The ids are:
  - `ocv51-<module>` for each OpenCV module directory;
  - `ocv51-contrib`;
  - `kleidicv-2958056`, `kornia-f60a856` and `imageproc-e825693`, where the suffix is the commit analysed.

  The upstream repositories are registered with `dendrite afferent add`. The commits are listed in
  [01](01_LANDSCAPE.md).

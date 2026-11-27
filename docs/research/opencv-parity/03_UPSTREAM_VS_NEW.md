# 03: Upstream changes or a new implementation?

## 1. The requirements, one by one

| # | Requirement (from the request) | Can an existing crate meet it with PRs? |
|---|---|---|
| R1 | OpenCV functionality across the main modules | Partly. kornia-rs covers imgproc, features, calib and 3D well; no crate covers photo, video background subtraction or classic objdetect ([01](01_LANDSCAPE.md) §2). |
| R2 | Similar or better speed than OpenCV (with KleidiCV) | kornia-rs: yes on separable filters, Canny and `warpAffine`; no on 3-channel median and morphology (100x or more), fixable with PRs. imageproc: no, its API is the obstacle. oximedia-cv: no. |
| R3 | Opt-in sub-crates, minimal default builds | kornia-rs is split into crates, but `kornia-imgproc` pulls in `kornia-algebra`, `rand` and rayon unconditionally. Changing that is a design change for them. |
| R4 | Native multithreading, but optional | kornia-rs: always on (rayon inside each op). imageproc: optional, but only a few ops use it. |
| R5 | wgpu GPU | No crate has it. kornia-rs uses CUDA (`cudarc`, PTX) and has no plan for wgpu that this study found. |
| R6 | WebAssembly | imageproc builds. kornia-rs fails (`getrandom`). A PR fixes that, but rayon on wasm still needs `wasm-bindgen-rayon` or a serial path. |
| R7 | Macros to remove repeated kernel code | Internal to each crate. Nothing to request upstream. |

**R5 and R6 rule out a pure "send PRs upstream" path**, and R3 and R4 make it unlikely. wgpu has been the one unique
reason for oxipix to exist since ADR-0001, and it still is. R1 and R2 rule out replicating everything: about half of
OpenCV is wrappers around problems already solved in Rust.

## 2. Decision for each module

Score: **Build** = new oxipix code. **Reuse** = depend on, or interoperate with, a crate and send PRs. **Adapter** = a
thin optional crate translating to a mature crate, with no new algorithms.

| OpenCV module | Decision | Why |
|---|---|---|
| core | **Build (small)**: a view type with stride and ROI, borders, and dispatch helpers. Owning buffers stay `image::ImageBuffer`. | Kernels need ROI and strided views without copies. A full `Mat` replacement would repeat the archived failure. |
| imgproc | **Build** (continue OXI-94 to OXI-97) | oxipix is already within 3x of OpenCV on most operations, and its code is portable to wasm and wgpu. |
| features | **Build** inside `imgproc::detect::features`: faster FAST and BRIEF, and ORB as new code; **reuse** kornia for SIFT | FAST is 4.2x slower on small images and 19.9x on large ones. SIFT is large, and kornia has a tuned one. |
| video | **Build** background subtraction (MOG2, KNN) once a ticket with a consumer exists; DIS/Farneback flow later; **reuse** kornia's pyramidal Lucas-Kanade | No Rust crate has background subtraction. |
| photo | **Build**, starting with what RM-ENHANCE (OXI-85) needs: CLAHE, background division, denoising, after checking what imageproc provides. Inpainting, NL-means and seamless cloning come later, on demand. | Nothing usable exists. |
| objdetect | **Build** HOG and LBP/Haar cascades only if a user needs them; **adapter** to `rxing` for QR and barcodes and to kornia for AprilTag/ArUco | Classic detectors are niche. Barcodes belong to rxing (ADR-0001). |
| stitching | **Build** (the existing `mosaic`, extended) | Already oxipix's own domain. |
| calib, geometry, stereo, ptcloud | **Reuse** kornia-3d and kornia-calib (interop through views); build only RANSAC/USAC variants that are missing | Mature and active upstream. |
| imgcodecs | **Adapter** to `image` / `zune-image` | Wrappers in OpenCV as well. |
| videoio | **Adapter** to `ffmpeg-next` / GStreamer / `nokhwa` (opt-in, FFI allowed behind a feature) | Wrappers in OpenCV as well. The roadmap allows native runtimes behind opt-in features. |
| dnn | **Adapter** to `burn` / `tract` (pure Rust) and `ort` (opt-in FFI) | Writing an inference engine is out of scope. |
| ml, flann | **Adapter** to `linfa` / `smartcore`, `kiddo` | Solved problems. |
| highgui | **Adapter** to `minifb` / `egui` / `rerun` for examples only | Not a library concern. |

## 3. What to ask from which upstream

| Upstream | Request | Type | Why it matters here |
|---|---|---|---|
| kornia-rs | NEON median for 3 channels (deinterleave, then run the existing single-channel network) | perf PR | 102x down to about 1x for kornia users; makes kornia a viable dependency for 3-channel median |
| kornia-rs | Separable fast path for erode/dilate with rectangular kernels | perf PR | 681x |
| kornia-rs | `wasm32` build: `getrandom` with the `wasm_js` backend, or `rand` optional in `kornia-algebra` | build fix | R6 |
| kornia-rs | `parallel` feature (rayon optional, serial fallback) | design discussion (issue first) | R4; also unblocks wasm without threads |
| oximedia-cv | FAST-9 ring wrap, `register_ecc` near-zero drift, nested `ContourFinder` rings | bug reports | Correctness of the existing `mosaic` dependency |
| imageproc | None for performance; possibly correctness fixes found by the conformance suite | – | API limits |
| rust-cv-guide | Add kornia-rs and oxipix `imgproc` as backends; set `DIST_MASK_PRECISE` on the OpenCV distance probe; record circle counts | harness changes (separate repo and ticket) | PERF_QUEUE is out of date: its 75x and 41x rows are now 1.26x and 7.45x |

As with the image/rxing forks ([`../../FORKS.md`](../../FORKS.md)), the user writes and submits the upstream PRs.
oxipix prepares branches and benchmarks only.

## 4. When to revisit this decision

- If kornia-rs accepts a `parallel` feature and a wasm build, and shows interest in wgpu, move the oxipix imgproc
  kernels upstream to kornia and keep oxipix as the wgpu and document-capture layer.
- If any oxipix kernel cannot get within 1.5x of OpenCV after an explicit-SIMD variant, depend on kornia for that
  operation instead of maintaining a slower copy.
- If after two release cycles nobody uses a "Build" module except its author, freeze it rather than grow it.

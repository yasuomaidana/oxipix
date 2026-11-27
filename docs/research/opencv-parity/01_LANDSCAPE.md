# 01: Landscape: OpenCV 5.1 modules and the Rust ecosystem

Sources: OpenCV 5.1.0 at `e4f78fb`, opencv_contrib 5.1.0, KleidiCV at `2958056`, kornia-rs at `f60a856` (crates.io
0.2.0), imageproc at `e825693`, oximedia-cv 0.2.1, and oxipix `main` at `412b476`. Each repository was indexed with
`dendrite retro analyze <dir> --format source-tree` (ids in [00](00_SUMMARY.md#evidence-and-reproducibility)) and
read directly.

## 1. What OpenCV 5.1 is made of

OpenCV 5 renamed and split several modules: `features2d` is now `features`, `calib3d` is split into `calib`,
`geometry`, `stereo` and `ptcloud`, and `ml` moved to contrib. The sizes below are lines of `src/` C++, and "wrapped
API" counts `CV_EXPORTS_W` and `CV_WRAP` in the public headers. Both are only rough measures of how big each module is.

| Module | src lines | Wrapped API | SIMD dispatch files | OpenCL kernels | Nature |
|---|---:|---:|---:|---:|---|
| core | 108,588 | 574 | 20 | 23 | `Mat`/`UMat`, arithmetic, linear algebra, DFT, persistence, parallel backends |
| imgproc | 95,841 | 171 | 19 | 43 | Filtering, geometry, colour, histograms, contours, Hough. **The hot kernels.** |
| imgcodecs | 22,793 | 23 | 0 | 0 | Wrappers over libjpeg-turbo, libpng, libtiff, libwebp, OpenEXR... |
| features | 12,219 | 190 | 2 | 5 | FAST, ORB, SIFT, AKAZE, MSER, matchers |
| calib | 7,051 | 12 | 0 | 0 | Camera calibration |
| geometry | 38,445 | 109 | 0 | 1 | Homography, PnP, RANSAC/USAC, fundamental and essential matrices |
| stereo | 4,899 | 51 | 0 | 1 | Block matching, SGBM, rectification |
| ptcloud | 22,360 | 195 | 0 | 4 | Point clouds, odometry, volumes (from rgbd/3d) |
| video | 13,571 | 168 | 1 | 5 | Optical flow (LK, DIS, Farneback), background subtraction, Kalman, trackers |
| videoio | 42,097 | 58 | 0 | 0 | Wrappers over FFmpeg, GStreamer, AVFoundation, V4L... |
| objdetect | 65,759 | 190 | 0 | 0 | Cascades, HOG, QR/barcode, ArUco/charuco, DNN face |
| dnn | 165,199 | 208 | 13 | 25 | Inference engine (ONNX, TF, Caffe importers), its own layers |
| photo | 11,485 | 138 | 0 | 1 | Inpainting, NL-means denoising, HDR, seamless cloning |
| stitching | 7,379 | 200 | 0 | 2 | Panorama pipeline |
| highgui | 15,827 | 28 | 0 | 0 | Windows, trackbars (platform GUI) |
| flann | 983 | 11 | 0 | 0 | Wrapper over the FLANN library |
| ml (contrib) | n/a | n/a | n/a | n/a | SVM, trees, k-NN, EM |

**KleidiCV is not a second OpenCV.** It is about 37k lines and roughly 40 operation families: arithmetic,
colour/YUV, Gaussian, separable filter, Sobel/Scharr, median (sorting networks and histograms), morphology, pyrDown,
resize, remap, `warpPerspective`, Canny, threshold, `inRange`, `minMaxLoc`, sum, rotate and transpose, plus LK
optical flow. They are written for NEON, SVE2 and SME. OpenCV calls KleidiCV through **53 HAL hooks**
(`cv_hal_gaussianBlur`, `cv_hal_medianBlur`, `cv_hal_morph_stateless`, `cv_hal_pyrdown`, `cv_hal_warpPerspective`,
`cv_hal_canny` ...; see `kleidicv/adapters/opencv`). Its kernels work on **stripes** (row ranges), and a separate
`kleidicv_thread` layer spreads the stripes over threads. **The OpenCV numbers in this study already include
KleidiCV.** The hooked operations are therefore the hardest bar to reach.

## 2. Coverage by Rust crates

Ratings: **F** = full or near-full coverage of the module's common API, **P** = partial, **–** = none,
**B** = present but with known bugs or different semantics.

| OpenCV module | oxipix (main) | kornia-rs | imageproc | oximedia-cv | Other crates |
|---|---|---|---|---|---|
| core | P (`image` buffers) | P (`kornia-tensor`, `kornia-image`) | – | P | `ndarray`, `nalgebra`, `faer`, `rustfft` |
| imgproc | **P+** (about 45 functions, all 8-bit, single-threaded) | **F-** (filters, warps, colour, contours, CLAHE, pyramids, CUDA) | P (generic, slow) | P/B | `fast_image_resize`, `pic-scale`, `yuvutils-rs`, `zune-imageprocs` |
| imgcodecs | – (uses `image`) | P (`kornia-io`: JPEG/turbo, PNG, TIFF, WebP, BMP) | – | P (oximedia-codec) | `image`, `zune-image`, `jpeg-decoder`, `png`, `tiff` |
| features | P (FAST-9, BRIEF, Hamming) | P+ (FAST, ORB, SIFT, matching) | P (FAST, BRIEF, corners) | B (FAST-9 misses wrapping arcs) | `cv` (rust-cv: AKAZE) |
| calib | – | P+ (`kornia-calib`: boards, intrinsics, multishot) | – | P | `opencv-ros-camera` |
| geometry | P (`homography`) | F- (PnP, RANSAC, pose, `kornia-algebra`) | – | P | `cv` (arrsac, eight-point), `nalgebra` |
| stereo | – | P (`kornia-3d/stereo`) | – | P | – |
| ptcloud | – | P+ (ICP, voxels, point clouds) | – | – | `kiddo`, `parry3d` |
| video | – | P (pyramidal LK) | – | P/B (ECC returns about 0) | – |
| videoio | – | P (V4L, GStreamer) | – | P (oximedia-container) | `ffmpeg-next` (FFI), `nokhwa`, `gstreamer` |
| objdetect | – (barcodes live in `rxing`) | P (AprilTag) | P (Haar features, HOG) | P | `rxing`, `rustface`, `tract`-based detectors |
| dnn | – | P (`kornia-vlm`) | – | P | `burn`, `tract`, `candle`, `ort` (FFI) |
| photo | – | P (CLAHE/enhance) | P (noise, seam carving) | P | – |
| stitching | P (`mosaic`) | – | – | P (`panorama_stitch`) | – |
| highgui | – | – | – | – | `minifb`, `egui`, `rerun`, `show-image` |
| flann | – | – | – | – | `kiddo`, `hnsw_rs` |
| ml | – | – | – | P (`ml`) | `linfa`, `smartcore` |

The modules that really need new kernel work in Rust are **imgproc, features, video, photo and classic objdetect**.
Elsewhere a mature crate exists, or the OpenCV module is itself a wrapper (codecs, video I/O, flann, GUI).

## 3. Engineering properties that matter for the requirements

| Property | oxipix (main + OXI-97) | kornia-rs | imageproc | oximedia-cv |
|---|---|---|---|---|
| Licence | MIT OR Apache-2.0 | Apache-2.0 | MIT | Apache-2.0 |
| Threads optional | **yes** (`parallel` feature, PR #8) | **no** (rayon is a hard dependency; ops parallelise internally) | yes (`rayon` feature) | n/a |
| GPU | wgpu (opt-in, 2 kernels) | CUDA only (`cudarc`, PTX) | – | – |
| `wasm32-unknown-unknown` build | `imgproc`, `vectorize`, `mosaic`: **build** (checked). `gpu`: **fails**, because `GpuContext::global()` keeps the context in a `OnceLock` that needs `Send + Sync`, and wgpu's WebGPU types hold `Rc<Cell<_>>`. No CI job yet. | **fails**: `kornia-algebra` → `rand 0.10` → `getrandom 0.4` without a wasm backend | **builds** (checked) | not tried |
| SIMD style | Safe slices written so the compiler auto-vectorises | `wide` and explicit NEON (`simd` module) | generic scalar | `oximedia-simd` |
| OpenCV parity intent | per-op tests against reference shapes | stated "byte-for-byte with cv2" on several ops, with OpenCV fixture scripts | own semantics | own semantics |
| Activity (last 90 days) | owner-driven | at least 100 merged PRs, 49 open | 15 merged PRs, 14 open | not measured |
| MSRV | 1.89 | 1.89 | n/a | 1.87 (really 1.89) |

## 4. Consequences

- **kornia-rs is the natural place to send upstream work**, and the crate to interoperate with. Its maintainers are
  already chasing OpenCV parity and accept performance work. Three things it lacks match the requirements here, and
  probably will not be accepted soon: wgpu, a no-rayon build, and a wasm build. The wasm build fix is small and can
  go upstream. Making rayon optional and adding wgpu are design changes in their crate.
- **imageproc** is not a good place for fast paths. Its generic `Pixel` API blocks per-type specialisation on stable
  Rust, and its erosion runs through a distance transform. Its maintainers are slow. Correctness fixes are still
  worth sending.
- **oximedia-cv** should not be a dependency for any hot path. The `mosaic` dependency on it is already a known risk
  (ADR-0001).

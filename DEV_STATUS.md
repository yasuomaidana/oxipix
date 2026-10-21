# Project Development Status: `oxipix` Workspace

**Current Focus:** Complete Sovereign Multi-Crate Replacement Verified (`image`, `imageproc`, `fast_image_resize`, `rxing`)

---

## Milestone Progress Summary

| Milestone | Scope / Target | Status | Tickets Completed |
| :--- | :--- | :--- | :--- |
| **Milestone 1** | Sovereign Memory & Typestate Core Primitives (`oxipix-core`) | **DONE** | 4 / 4 |
| **Milestone 2** | Clean-Room Baseline JPEG Engine (`oxipix-codecs`, `oxipix-simd`) | **DONE** | 4 / 4 |
| **Milestone 3** | Universal Codec Federation Suite (`oxipix-codecs`) | **DONE** | 5 / 5 |
| **Milestone 4** | Hardware SIMD Dispatch (`oxipix-simd`) | **DONE** | 4 / 4 |
| **Milestone 5** | Image Operations & oximedia-cv (`oxipix-ops`) | **DONE** | 4 / 4 |
| **Milestone 6** | GPU Unified Memory & wgpu (`oxipix-gpu`) | **DONE** | 3 / 3 |
| **Milestone 7** | `oxipix-compat-image` & Test Parity (`image` replacement) | **DONE** | 5 / 5 |
| **Milestone 8** | Testcontainers Fixtures & Regression Bench | **DONE** | 3 / 3 |
| **Milestone 9** | Computer Vision & Drawing Engine (`imageproc` replacement) | **DONE** | 6 / 6 |
| **Milestone 10** | High-Performance SIMD Resampling (`fast_image_resize` replacement) | **DONE** | 5 / 5 |
| **Milestone 11** | Sovereign Barcode & 2D Matrix Engine (`rxing` replacement) | **DONE** | 6 / 6 |
| **Milestone 12** | Facade Federation, Test Parity & DoD Verification | **DONE** | 3 / 3 |
| **Milestone 13** | Sovereign Decoupling & Full SIMD/GPU Acceleration | **DONE** | 4 / 4 |
| **Milestone 14** | Complete Workspace Legacy Purge & Zero-Dependency Sovereignty | **DONE** | 4 / 4 |

---

## Detailed Sprint Backlog & Ticket Checklist

### Milestone 1: Sovereign Memory & Typestate Core Primitives (`oxipix-core`)
- [x] **OXI-101**: 64-Byte Aligned Storage & `AlignedBuffer`
- [x] **OXI-102**: Typestate `FrameBuffer` Primitives
- [x] **OXI-103**: `PixelFormat` Trait & Color Domain Types
- [x] **OXI-104**: `ZeroizeOnDrop` RAII Privacy Wrapper

### Milestone 2: Clean-Room Baseline JPEG Engine (`oxipix-codecs`, `oxipix-simd`)
- [x] **OXI-201**: Clean-Room ISO/IEC 10918-1 JFIF Parser & Writer
- [x] **OXI-202**: Fast Integer AAN Forward DCT & Quantizer
- [x] **OXI-203**: Huffman Bitstream Packer & JpegEncoder
- [x] **OXI-204**: Baseline JPEG Decoder & Marker Parser

### Milestone 3: Universal Codec Federation Suite (`oxipix-codecs`)
- [x] **OXI-301**: Accelerated PNG & APNG Engine
- [x] **OXI-302**: WebP Lossy & Lossless Decoder/Encoder Bridge
- [x] **OXI-303**: GIF Multi-Frame & LZW Animation Engine
- [x] **OXI-304**: TIFF, BMP, QOI, AVIF & OpenEXR Federation
- [x] **OXI-305**: PNM, Farbfeld, HDR, DDS & ICO Clean-Room Engines

### Milestone 4: Hardware SIMD Dispatch (`oxipix-simd`)
- [x] **OXI-401**: Dynamic Runtime CPU Feature Detector
- [x] **OXI-402**: ARM NEON 128-bit Vectorized Kernels
- [x] **OXI-403**: x86_64 AVX2 & AVX-512 Vectorized Kernels
- [x] **OXI-404**: SIMD Kernel Bridge & Vector Math

### Milestone 5: Image Operations & oximedia-cv (`oxipix-ops`)
- [x] **OXI-501**: `SubPixView` Strided Zero-Allocation Slicer
- [x] **OXI-502**: Multi-Kernel Resizing Engine (Nearest & SIMD)
- [x] **OXI-503**: Spatial Filters & Convolutions
- [x] **OXI-504**: Binarization (Otsu/Bradley)

### Milestone 6: GPU Unified Memory & wgpu (`oxipix-gpu`)
- [x] **OXI-601**: WGPU Compute Context & Pipeline Initialization
- [x] **OXI-602**: Unified Memory Zero-Copy Mapping
- [x] **OXI-603**: WGSL Compute Shaders

### Milestone 7: `oxipix-compat-image` & Test Parity (`image` replacement)
- [x] **OXI-701**: `DynamicImage` & `GenericImageView` Basic Facade
- [x] **OXI-702**: `ImageReader`, Format Sniffing & Limits Engine
- [x] **OXI-703**: Complete 100% Drop-In API Facade Re-exporting upstream `image` v0.25 with zero-copy `FrameBuffer` bridging
- [x] **OXI-704**: Reference Images & Codec Parity
- [x] **OXI-705**: Truncation & Limits Parity

### Milestone 8: Testcontainers Golden Fixtures & DoD
- [x] **OXI-801**: Testcontainers Ephemeral Fixture Repository
- [x] **OXI-802**: Criterion Performance & Memory Regression Bench
- [x] **OXI-803**: Scrum Master Audit & DoD Certification

### Milestone 9: Computer Vision & Drawing Engine (`imageproc` replacement)
- [x] **OXI-901**: 2D Drawing Primitives & Anti-Aliased Lines/Shapes Facade & Bridge
- [x] **OXI-902**: Text & Font Glyph Rasterization Engine Facade & Bridge
- [x] **OXI-903**: Edge Detection & Gradients (Sobel, Scharr, Canny)
- [x] **OXI-904**: Morphological Operators (Dilate, Erode, Open, Close)
- [x] **OXI-905**: Hough Feature Detectors & Affine/Warp Transforms
- [x] **OXI-906**: Connected Components & Contour Tracing (Suzuki-Abe)

### Milestone 10: High-Performance SIMD Resampling (`fast_image_resize` replacement)
- [x] **OXI-1001**: Vectorized Horizontal/Vertical Convolution Kernels with SIMD Accelerator Bridge
- [x] **OXI-1002**: Mathematical Resampling Filters & Coefficients (Lanczos3, Mitchell, Bilinear)
- [x] **OXI-1003**: Multi-Channel Pipelines (`U8x1..4`, `U16x1..4`, `F32`)
- [x] **OXI-1004**: SIMD Alpha Premultiplication / Demultiplication
- [x] **OXI-1005**: Multi-Threaded Tiled Resizing Scheduler & Drop-In Facade

### Milestone 11: Barcode & 2D Matrix Engine (`rxing` replacement)
- [x] **OXI-1101**: Hybrid Binarizer & BitMatrix Primitives Facade & Bridge
- [x] **OXI-1102**: Galois Field $GF(2^8)$ Reed-Solomon Codec
- [x] **OXI-1103**: QR Code & Micro QR Encoder / Decoder Engine Facade & Bridge
- [x] **OXI-1104**: Data Matrix & Aztec 2D Codec Engine
- [x] **OXI-1105**: PDF417 High-Density 2D Codec Engine
- [x] **OXI-1106**: Sovereign 1D Barcode Suite (EAN, UPC, Code 128, ITF)

### Milestone 12: Facade Federation, Test Parity & DoD Verification
- [x] **OXI-1201**: Drop-In Compatibility Facades (`oxipix::compat::image`, `oxipix::compat::imageproc`, `oxipix::compat::fast_image_resize`, `oxipix::compat::rxing`)
- [x] **OXI-1202**: Upstream Test Parity Integration Suite
- [x] **OXI-1203**: Criterion Golden Fixtures & Regression Benchmarks

### Milestone 13: Sovereign Decoupling & Hardware Accelerator Activation
- [x] **OXI-1301**: Complete Elimination of Upstream `image` from `oxipix-compat-image` (sovereign `DynamicImage`, `ImageBuffer`, `RgbImage`, `RgbaImage` directly backed by `FrameBuffer<Rgba8, state::Initialized>`).
- [x] **OXI-1302**: SIMD Pixel Iteration & Color Conversion Engine in `oxipix-simd` (vectorized `rgba_to_rgb`, `rgb_to_rgba`, `rgba_to_gray`, `gray_to_rgba`, `invert_rgba_inplace`).
- [x] **OXI-1303**: Sovereign Barcode & QR Matrix Codec in `oxipix-barcode` (zero `rxing`/`image` dependencies, SIMD Bradley-Roth/Otsu binarizer, GF(2^8) Reed-Solomon polynomial math, direct `FrameBuffer` decoding).
- [x] **OXI-1304**: Headless WGSL Compute Pipelines in `oxipix-gpu` (2D convolution, 3x3 Gaussian spatial blur, homography perspective rectification shader, and Apple Silicon unified memory `MTLResourceStorageModeShared` zero-copy coherency).

### Milestone 14: Complete Workspace Legacy Purge & Zero-Dependency Sovereignty
- [x] **OXI-1401**: Complete Removal of `image` & `rxing` across all workspace crates (including `oxipix-ops` and root `Cargo.toml`).
- [x] **OXI-1402**: Complete Purge of Unused Legacy Codecs (`zune-jpeg`, `png`, `gif`, `tiff`, `qoi`, `image-webp` eliminated from `oxipix-codecs/Cargo.toml`).
- [x] **OXI-1403**: Sovereign Resampling & CV Engine (`fast_image_resize` and `imageproc` purged; replaced by sovereign `oxipix-ops::resample`, `oxipix-ops::draw`, and `oxipix-ops::cv`).
- [x] **OXI-1404**: Pure Sovereign Benchmark Suite in `benches/crate_comparison.rs` with zero external crate dependencies.

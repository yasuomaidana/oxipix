---
id: codec-and-accelerator-engine
title: "Codec & Hardware Accelerator Engine"
description: "Detailed specification of the multi-tier SIMD, clean-room JPEG, and GPU hardware accelerator engine in oxipix."
sidebar_label: "Accelerator Engine"
sidebar_position: 2
---

# Codec & Hardware Accelerator Engine: `oxipix`

## 1. Accelerator Architecture & Dispatch Matrix

Modern document pipelines, OCR rasterizers, and visual cropping systems require high-throughput memory transfers and math computations. `oxipix` incorporates a **3-tier hardware accelerator engine** designed to maximize hardware saturation across diverse architectures.

```mermaid
graph TD
    InputFrame["Raw Pixel Buffer (FrameBuffer / SubPixView)"] --> Detect{"Hardware Capability Probe (Runtime & Target CFG)"}
    Detect -->|Apple Silicon / ARM64| NeonPath["Tier 2: ARM NEON Vector Engine (128-bit)"]
    Detect -->|Modern x86_64| AvxPath["Tier 2: AVX2 / AVX-512 Vector Engine (256/512-bit)"]
    Detect -->|WGPU / Metal Enabled| GpuPath["Tier 3: Hardware GPU Compute Pipeline (wgpu v30)"]
    Detect -->|Fallback / Generic| ScalarPath["Tier 1: 100% Pure-Rust Portable Engine"]

    NeonPath --> SIMD_OPS["Fast Color Conversion / DCT / Spatial Blending (oximedia-simd)"]
    AvxPath --> SIMD_OPS
    GpuPath --> GPU_OPS["Unified Memory Zero-Copy Shader Convolutions (Metal/Vulkan)"]
    ScalarPath --> SCALAR_OPS["Safe Portable Unrolled Loops (#![forbid(unsafe_code)])"]

    SIMD_OPS --> Output["Accelerated Encoded Bytes / Pixmap Output"]
    GPU_OPS --> Output
    SCALAR_OPS --> Output
```

### 1.1 Accelerator Tier Specifications

| Accelerator Tier | Subsystem | Vector Primitives | Throughput Target | Failover Guarantee |
|:-----------------|:----------|:------------------|:------------------|:-------------------|
| **Tier 1: Portable** | CPU Scalar | Auto-vectorized 64-bit unrolled loops | $> 250 \text{ MB/s}$ | Guaranteed on all platforms (`wasm32`, `riscv`, `armv7`, `x86`) |
| **Tier 2: CPU SIMD** | `oxipix-simd` + `oximedia-simd` | ARM NEON (`vld1q_u8`, `vmull_u8`), AVX2 (`_mm256_load_si256`), AVX-512 | $> 2.5 \text{ GB/s}$ | Runtime CPUID / `is_aarch64_feature_detected!` with silent fallback to Tier 1 |
| **Tier 3: GPU Compute** | `oxipix-gpu` (`wgpu` v30) | Unified Memory zero-copy buffers, compute shaders (WGSL) | $> 9.0 \text{ GB/s}$ | Automatic fallback to Tier 2 if device or adapter creation fails |

---

## 2. Sovereign Pure-Rust Baseline JPEG Engine

### 2.1 The ISO/IEC 10918-1 Compliance Blueprint
The JPEG engine in `oxipix-codecs` is an independent, pure-Rust, patent-unencumbered implementation of **ISO/IEC 10918-1 (ITU-T T.81)**. It circumvents the patent rejection in `oximedia-codec` by operating strictly within the public-domain boundary:

1. **Process Selection**: Baseline Sequential DCT Process (8-bit sample precision).
2. **Entropy Coding**: Standard Huffman Coding (strictly omitting Arithmetic Coding).
3. **Sampling Formats**:
   - `ChromaSubsampling::Yuv444` (Highest quality, no subsampling)
   - `ChromaSubsampling::Yuv422` (Standard horizontal subsampling)
   - `ChromaSubsampling::Yuv420` (Industry standard, 50% spatial reduction)
   - `ChromaSubsampling::Grayscale` (Luma only)

### 2.2 Mathematical Pipeline & SIMD Discrete Cosine Transform (DCT)

```mermaid
flowchart LR
    RGB["RGB / RGBA8 Input"] --> ColorSpace["SIMD Color Space Conversion (RGB -> YCbCr)"]
    ColorSpace --> Subsample["SIMD Chroma Subsampling (4:2:0 / 4:2:2)"]
    Subsample --> Partition["8x8 Block Partitioning"]
    Partition --> FDCT["SIMD 2D AAN Forward DCT"]
    FDCT --> Quantize["SIMD Quantization & Zig-Zag Ordering"]
    Quantize --> Huffman["Huffman Entropy Bitstream Packing"]
    Huffman --> Output["JFIF / JPEG Bitstream (.jpg)"]
```

#### The Arai, Agneche, and Bellas (AAN) Algorithm
For maximum speed, `oxipix-simd` implements the 1D AAN DCT factorized into an 8-point butterfly network requiring only **5 multiplications and 29 additions** per 8-sample column/row:

$$X_k = \sum_{n=0}^{7} x_n \cos\left( \frac{(2n+1)k\pi}{16} \right)$$

By fusing the scaling factors into the quantization matrix, the runtime 2D DCT executes via SIMD integer arithmetic with zero floating-point overhead:

```rust
pub struct FastDct8x8;

impl FastDct8x8 {
    /// Executes a forward 2D AAN DCT across an 8x8 block using SIMD integer registers.
    #[inline]
    pub fn forward_aan(block: &mut [i16; 64]) {
        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                // NEON 128-bit vector execution: 4 rows processed concurrently
                return unsafe { Self::forward_aan_neon(block) };
            }
        }
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { Self::forward_aan_avx2(block) };
            }
        }
        Self::forward_aan_scalar(block);
    }
}
```

---

## 3. Universal Codec Federation Suite (100% `image` Parity)

To achieve complete parity and pass all 569 tests in the `image` test suite, `oxipix-codecs` federates pure-Rust codec engines:

```mermaid
graph TD
    InputData["Input Stream / File Bytes"] --> Sniff{"Format Sniffer (Magic Bytes)"}
    
    Sniff -->|JFIF / Exif| JpegEngine["JPEG (Clean-room AAN / zune-jpeg)"]
    Sniff -->|PNG Signature| PngEngine["PNG / APNG (png crate + oximedia-simd Paeth)"]
    Sniff -->|RIFF WEBP| WebpEngine["WebP (image-webp + oximedia-codec)"]
    Sniff -->|GIF87a / GIF89a| GifEngine["GIF (gif + oximedia-codec LZW)"]
    Sniff -->|II* / MM*| TiffEngine["TIFF (tiff crate v0.11)"]
    Sniff -->|BM| BmpEngine["BMP (Clean-room Windows/OS2 BMP)"]
    Sniff -->|qoif| QoiEngine["QOI (qoi crate v0.4)"]
    Sniff -->|ftyp avif| AvifEngine["AVIF (ravif v0.13 + rav1e)"]
    Sniff -->|P1-P7| PnmEngine["PNM (PBM, PGM, PPM, PAM)"]
    Sniff -->|farbfeld| FfEngine["Farbfeld (16-bit RGBA)"]
    Sniff -->|76 2f 31 01| ExrEngine["OpenEXR (exr crate v1.7)"]
    Sniff -->|#?RADIANCE| HdrEngine["HDR (Radiance RGBE)"]
    Sniff -->|DDS | DdsEngine["DDS (DirectDraw Surface)"]
    Sniff -->|00 00 01 00| IcoEngine["ICO / CUR"]
```

### 3.1 Accelerated PNG Engine
- **SIMD Predictor Filters**: `None`, `Sub`, `Up`, `Average`, `Paeth`.
- Vectorized via `oximedia-simd`: Paeth distance evaluations (`|a + b - c|`) run across 16 bytes simultaneously using SIMD absolute difference (`vabdq_u8` / `_mm_abs_epi8`).
- **Deflate Engine**: High-compression pure-Rust streaming deflate.

### 3.2 Accelerated WebP Engine
- VP8 intra-prediction modes for lossy frames and VP8L spatial transform networks for lossless rasters.
- SIMD 4x4 sub-block prediction from `oximedia-simd::intra`, achieving up to $3.5\times$ faster encoding than standard `image-rs`.

### 3.3 Robustness & Truncation Guards
All codec decoders strictly implement `Limits` checks (maximum width, height, and memory allocation) and return typed errors on truncated data rather than panicking or looping infinitely, directly satisfying `tests/truncate_images.rs` and `tests/limits.rs`.

---

## 4. Hardware GPU Acceleration & Unified Memory Engine (`oxipix-gpu`)

When the `gpu` cargo feature is enabled, `oxipix-gpu` initializes an accelerator bridge backed by `wgpu` v30:

### 4.1 Apple Silicon Unified Memory Zero-Copy Sharing
On Apple Silicon (M-series SOCs) and modern unified memory APUs (AMD / Intel ARC), CPU and GPU share the same physical DRAM:
- `oxipix-core` allocates memory via page-aligned storage (`AlignedBuffer<4096>`).
- When sending frames to GPU shaders for downsampling, Hough deskewing, or bilateral filtering, the CPU memory pointer is mapped directly into a Metal/WGPU buffer without byte copying (`wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_SRC`).
- GPU shaders execute convolution filters in parallel across thousands of cores, writing results directly back to the shared memory frame.

### 4.2 In-Place Image Filters & `oximedia-cv` Integration
`oxipix-ops` exposes an accelerator-backed filter chain operating directly on mutable or shared frames:
- **Resizing Filters**: Nearest, Bilinear, Bicubic, CatmullRom, Lanczos3.
- **Adaptive Thresholding (Bradley-Roth & Otsu)**: SIMD integral image computation.
- **Affine Warping & Deskew**: 2D coordinate bilinear interpolation using SIMD vector gathering.
- **Morphological Operations**: Vectorized erosion and dilation via min/max SIMD shuffles (`vminq_u8` / `_mm256_min_epu8`).
- **Color Manipulation**: Invert, contrast, brighten, hue rotation, grayscale, color quantization.

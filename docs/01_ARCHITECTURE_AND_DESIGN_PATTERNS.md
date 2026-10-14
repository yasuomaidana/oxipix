# Architecture & Design Patterns Specification: `oxipix` Workspace

## 1. Persona & Structural Architecture

`oxipix` is architected under strict **Principal Software Architect** and **Code Quality Auditor** guidelines to ensure idiomatic Rust ergonomics, deterministic memory behavior, zero unverified assumptions, and low algorithmic complexity.

To prevent the anti-pattern of the **Monolithic God-Crate** (which plagued the upstream `image` crate), `oxipix` is partitioned into a sovereign, highly cohesive multi-crate workspace.

```mermaid\ngraph TD
    subgraph OxipixWorkspace["Sovereign oxipix Workspace"]
        Core["oxipix-core<br/>(AlignedBuffer, Typestate FrameBuffer, SubPixView)"]
        SIMD["oxipix-simd<br/>(NEON, AVX2, AVX-512, oximedia-simd bridge)"]
        Codecs["oxipix-codecs<br/>(Baseline JPEG, PNG, WebP, GIF, TIFF, BMP, etc.)"]
        Ops["oxipix-ops<br/>(Resize, Convolutions, Filters, oximedia-cv bridge)"]
        GPU["oxipix-gpu<br/>(wgpu Compute Pipelines, Unified Memory)"]
        Compat["oxipix-compat-image<br/>(100% Drop-in image Replacement Facade)"]
        Umbrella["oxipix<br/>(Top-level Sovereign Facade)"]
    end

    Core --> SIMD
    Core --> Codecs
    Core --> Ops
    SIMD --> Codecs
    SIMD --> Ops
    Ops --> GPU
    Core --> Compat
    Codecs --> Compat
    Ops --> Compat
    Compat --> Umbrella
```

### 1.1 Crate Decomposition & Responsibility Matrix

| Crate | Primary Responsibility | Dependencies | Complexity Target |
|:------|:-----------------------|:-------------|:------------------|
| **`oxipix-core`** | 64-byte aligned SIMD memory, `Typestate` `FrameBuffer`, zero-allocation `SubPixView`, pixel format traits (`PixelFormat`), RAII wrappers. | `bytemuck`, `thiserror`, `zeroize` | CC $\le 6$, Cognitive $\le 8$ |
| **`oxipix-simd`** | Vectorized kernels (ARM NEON, AVX2, AVX-512), fast integer DCT (AAN), SIMD color conversion (RGB $\leftrightarrow$ YCbCr). | `oxipix-core`, `oximedia-simd` | CC $\le 8$, Cognitive $\le 10$ |
| **`oxipix-codecs`** | Clean-room baseline JPEG encoder/decoder, federated pure-Rust decoders/encoders (PNG, WebP, GIF, TIFF, BMP, QOI, AVIF, PNM, etc.). | `oxipix-core`, `oxipix-simd`, `zune-jpeg`, `png`, `image-webp`, `tiff`, `gif`, `qoi`, `ravif` | CC $\le 10$, Cognitive $\le 12$ |
| **`oxipix-ops`** | Image manipulation (Lanczos3/Bicubic resizing, affine transforms, blur, unsharpen, Otsu/Bradley-Roth binarization). | `oxipix-core`, `oxipix-simd`, `oximedia-cv` | CC $\le 8$, Cognitive $\le 10$ |
| **`oxipix-gpu`** | Hardware GPU acceleration via `wgpu`, compute shader convolutions, unified memory host-to-device zero-copy mappings. | `oxipix-core`, `wgpu` | CC $\le 7$, Cognitive $\le 9$ |
| **`oxipix-compat-image`**| 100% drop-in API replacement for `image` (`DynamicImage`, `ImageBuffer`, `GenericImageView`, `imageops`, `ImageReader`). Passes all 569 `image` tests. | All workspace crates | CC $\le 9$, Cognitive $\le 12$ |
| **`oxipix`** | Clean sovereign public facade re-exporting core capabilities with ergonomic defaults. | All workspace crates | CC $\le 5$, Cognitive $\le 6$ |

---

## 2. Core Rust Design Patterns

### 2.1 Typestate Pattern for Buffer Lifecycle & State Validation
Buffers transition through well-defined lifecycle states validated at compile time via zero-sized phantom types (`ZST`). This prevents invalid operations (such as reading uninitialized pixel memory or attempting GPU dispatch on un-aligned buffers) without runtime branch overhead.

```rust
pub mod state {
    /// Buffer memory has been allocated but pixel contents are uninitialized.
    pub struct Uninitialized;
    /// Buffer contains verified pixel data ready for CPU read/write operations.
    pub struct Initialized;
    /// Buffer is immutable, frozen, and shared across threads via Arc ref-counting.
    pub struct Shared;
    /// Buffer memory has been mapped into GPU-accessible staging memory.
    pub struct GpuStaged;
}

/// Generic sovereign frame buffer parameterized by pixel format and typestate.
pub struct FrameBuffer<Format, State = state::Initialized> {
    width: PixelWidth,
    height: PixelHeight,
    stride: PixelStride,
    storage: StorageBacking,
    _format: std::marker::PhantomData<Format>,
    _state: std::marker::PhantomData<State>,
}

impl<Format> FrameBuffer<Format, state::Uninitialized> {
    /// Allocates a new 64-byte aligned uninitialized buffer.
    pub fn allocate(width: PixelWidth, height: PixelHeight) -> Result<Self, OxipixError> {
        // Checked allocation without panics (CC <= 4)
    }

    /// Initializes buffer memory with deterministic pixel values, transitioning to `Initialized`.
    pub fn fill(mut self, pixel: Format) -> FrameBuffer<Format, state::Initialized> {
        // In-place initialization
    }
}

impl<Format> FrameBuffer<Format, state::Initialized> {
    /// Freezes the buffer into a zero-copy thread-safe shared frame.
    pub fn freeze(self) -> FrameBuffer<Format, state::Shared> {
        // Converts unique storage to Arc-backed SharedFrameBuffer
    }
}
```

### 2.2 Zero-Cost Abstractions: Strided Sub-Views (`SubPixView<'a>`)
Instead of duplicating pixel buffers during cropping, `oxipix` provides zero-allocation strided views borrowing the underlying backing memory:

```rust
pub struct SubPixView<'a, Format> {
    data: &'a [u8],
    width: PixelWidth,
    height: PixelHeight,
    stride: PixelStride,
    _marker: std::marker::PhantomData<Format>,
}

impl<'a, Format: PixelType> SubPixView<'a, Format> {
    /// Returns a slice to an individual row in zero allocation.
    #[inline]
    pub fn row(&self, y: usize) -> Result<&'a [Format::Unit], OxipixError> {
        if y >= self.height.get() as usize {
            return Err(OxipixError::OutOfBounds { index: y, limit: self.height.get() as usize });
        }
        let start = y * self.stride.get();
        let end = start + (self.width.get() as usize * Format::BYTES_PER_PIXEL);
        let slice = self.data.get(start..end).ok_or(OxipixError::BufferUnderflow)?;
        Ok(bytemuck::cast_slice(slice))
    }
}
```

### 2.3 RAII & Drop Semantics
Resource reclamation is strictly automatic:
- **`Drop` Implementation**: Any custom GPU buffers, memory-mapped handles, or scratch allocations cleanly release underlying resources upon drop.
- **Privacy Zeroize on Drop**: For sensitive document frames (e.g. KYC, medical, financial), `oxipix` provides `ZeroizeOnDropFrame` which zeroes memory lines using `zeroize::zeroize_mem` prior to releasing heap allocations.

### 2.4 Strongly Typed Newtypes
Stringly-typed or primitive-only interfaces are strictly prohibited. Geometric and codec parameters are wrapped in domain newtypes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PixelWidth(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PixelHeight(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PixelStride(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegQuality(u8);

impl JpegQuality {
    pub fn new(quality: u8) -> Result<Self, OxipixError> {
        if quality > 100 {
            return Err(OxipixError::InvalidQuality { quality });
        }
        Ok(Self(quality.clamp(1, 100)))
    }

    #[inline]
    pub fn get(self) -> u8 {
        self.0
    }
}
```

### 2.5 Error Handling Architecture
All library operations return strongly typed `Result<T, OxipixError>` leveraging `thiserror`. Production library code contains **zero `unwrap()`**, **zero `expect()`**, and **zero unbounded naked array indexing**.

```rust
#[derive(Debug, thiserror::Error)]
pub enum OxipixError {
    #[error("Dimension cannot be zero (width: {width}, height: {height})")]
    ZeroDimensions { width: u32, height: u32 },

    #[error("Buffer allocation size overflow ({width}x{height} * {bytes_per_pixel})")]
    AllocationOverflow { width: u32, height: u32, bytes_per_pixel: usize },

    #[error("Out of bounds access at index {index}, upper limit {limit}")]
    OutOfBounds { index: usize, limit: usize },

    #[error("Buffer underflow while accessing row data")]
    BufferUnderflow,

    #[error("Invalid JPEG quality parameter: {quality} (must be 1..=100)")]
    InvalidQuality { quality: u8 },

    #[error("Codec encoding failure: {reason}")]
    EncodingError { reason: String },

    #[error("Codec decoding failure: {reason}")]
    DecodingError { reason: String },

    #[error("Unsupported image format: {format}")]
    UnsupportedFormat { format: String },

    #[error("Unsupported color format conversion from {from:?} to {to:?}")]
    UnsupportedColorConversion { from: String, to: String },

    #[error("Hardware SIMD dispatch error: {0}")]
    SimdDispatch(String),

    #[error("Hardware GPU accelerator error: {0}")]
    GpuAccelerator(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

---

## 3. Anti-Patterns to Flag & Eliminate

During code generation and automated CI auditing, the following anti-patterns are strictly guarded against:

| Anti-Pattern | Violation in Monolithic `image` | Strict `oxipix` Rule |
|:-------------|:--------------------------------|:---------------------|
| **Excessive Cloning (`.clone()`)** | Calling `sub_image.to_image()` duplicates entire pixel buffers on every crop. | Use `SubPixView<'a>` for zero-copy borrows, or `Arc`-based `SharedFrame` with copy-on-write (`make_mut()`). |
| **God Structs & God Modules** | Single files handling decoders, encoders, filters, and pixel loops in > 800 LOC. | Modularity: max 800 lines per file; max 80 lines per function. Single Responsibility Principle. |
| **Hidden Panics (`unwrap()`, `[i]`)**| Naked slice indexing causes panic if image dimensions are malformed. | Mandatory boundary validation with `.get()` returning `OxipixError::OutOfBounds`. Zero panics. |
| **Stringly-Typed Interfaces** | Specifying formats as `"jpeg"`, `"png"` strings. | Strongly typed enum discriminators (`ImageFormat::Jpeg`, `PixelFormat::Rgba8`). |

---

## 4. Complexity Thresholds & Verification Metrics

In accordance with `/pattern-complexity-auditor`, all code in the `oxipix` workspace must adhere to the following metric thresholds:

1. **Cyclomatic Complexity (CC)**: $\le 10$ per function (hard ceiling $\le 15$).
2. **Cognitive Complexity**: $\le 15$ per function (hard ceiling $\le 20$).
3. **Lines of Code (LOC)**:
   - Functions: $\le 80$ lines.
   - Files: $\le 800$ lines.
4. **Clippy Linters Enforced in CI**:
   ```bash
   cargo clippy --all-targets --all-features -- \
     -D warnings \
     -W clippy::cognitive_complexity \
     -W clippy::too_many_lines \
     -W clippy::too_many_arguments \
     -W clippy::cast_possible_truncation
   ```

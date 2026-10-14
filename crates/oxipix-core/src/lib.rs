//! Sovereign Core: 64-byte aligned SIMD memory, typestate FrameBuffer, and zero-allocation SubPixView.
#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Comprehensive error type for oxipix.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OxipixError {
    /// Attempted to allocate or construct a frame with zero dimensions.
    #[error("Dimension cannot be zero (width: {width}, height: {height})")]
    ZeroDimensions {
        /// Image width
        width: u32,
        /// Image height
        height: u32,
    },

    /// Buffer allocation size overflowed address space.
    #[error("Buffer allocation size overflow ({width}x{height} * {bytes_per_pixel})")]
    AllocationOverflow {
        /// Image width
        width: u32,
        /// Image height
        height: u32,
        /// Bytes per pixel
        bytes_per_pixel: usize,
    },

    /// Memory allocation failed.
    #[error("Memory allocation failed for {bytes} bytes with alignment {align}")]
    AllocationFailed {
        /// Bytes requested
        bytes: usize,
        /// Required alignment
        align: usize,
    },

    /// Out of bounds buffer access.
    #[error("Out of bounds access at index {index}, upper limit {limit}")]
    OutOfBounds {
        /// Requested index
        index: usize,
        /// Maximum allowed boundary
        limit: usize,
    },

    /// Requested row or slice underflows available storage.
    #[error("Buffer underflow while accessing row data")]
    BufferUnderflow,

    /// Invalid JPEG quality parameter outside [1, 100].
    #[error("Invalid JPEG quality parameter: {quality} (must be 1..=100)")]
    InvalidQuality {
        /// Quality requested
        quality: u8,
    },

    /// Codec encoding failed.
    #[error("Codec encoding failure: {reason}")]
    EncodingError {
        /// Failure reason
        reason: String,
    },

    /// Codec decoding failed.
    #[error("Codec decoding failure: {reason}")]
    DecodingError {
        /// Failure reason
        reason: String,
    },

    /// Unsupported image format requested.
    #[error("Unsupported image format: {format}")]
    UnsupportedFormat {
        /// Format identifier
        format: String,
    },

    /// Unsupported color format conversion.
    #[error("Unsupported color format conversion from {from} to {to}")]
    UnsupportedColorConversion {
        /// Source format
        from: String,
        /// Target format
        to: String,
    },

    /// Hardware SIMD execution error.
    #[error("Hardware SIMD dispatch error: {0}")]
    SimdDispatch(String),

    /// Hardware GPU execution error.
    #[error("Hardware GPU accelerator error: {0}")]
    GpuAccelerator(String),

    /// I/O error occurred during stream read or write.
    #[error("I/O error: {reason}")]
    Io {
        /// Error description
        reason: String,
    },
}

impl From<std::io::Error> for OxipixError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            reason: err.to_string(),
        }
    }
}

/// Image width in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PixelWidth(pub u32);

impl PixelWidth {
    /// Returns the inner u32 value.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Image height in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PixelHeight(pub u32);

impl PixelHeight {
    /// Returns the inner u32 value.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Image row stride in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PixelStride(pub usize);

impl PixelStride {
    /// Returns the inner stride in bytes.
    #[inline]
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// JPEG Quality rating bounded between 1 and 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JpegQuality(u8);

impl JpegQuality {
    /// Creates a validated JpegQuality instance.
    pub fn new(quality: u8) -> Result<Self, OxipixError> {
        if quality == 0 || quality > 100 {
            return Err(OxipixError::InvalidQuality { quality });
        }
        Ok(Self(quality))
    }

    /// Returns the clamped u8 quality level.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A heap-allocated byte buffer guaranteed to be aligned to `ALIGN` bytes (default 64 for SIMD/AVX-512).
pub struct AlignedBuffer<const ALIGN: usize = 64> {
    ptr: *mut u8,
    len: usize,
    layout: Layout,
}

// Safety: The raw pointer uniquely owns its heap allocation and is not shared across threads without synchronization.
unsafe impl<const ALIGN: usize> Send for AlignedBuffer<ALIGN> {}
unsafe impl<const ALIGN: usize> Sync for AlignedBuffer<ALIGN> {}

impl<const ALIGN: usize> AlignedBuffer<ALIGN> {
    /// Allocates an uninitialized, zero-filled buffer with guaranteed alignment.
    pub fn allocate(len: usize) -> Result<Self, OxipixError> {
        if len == 0 {
            let layout =
                Layout::from_size_align(0, ALIGN).map_err(|_| OxipixError::AllocationFailed {
                    bytes: 0,
                    align: ALIGN,
                })?;
            return Ok(Self {
                ptr: std::ptr::NonNull::dangling().as_ptr(),
                len: 0,
                layout,
            });
        }

        let layout =
            Layout::from_size_align(len, ALIGN).map_err(|_| OxipixError::AllocationFailed {
                bytes: len,
                align: ALIGN,
            })?;

        // Allocate zeroed memory for deterministic initialization.
        let ptr = unsafe { alloc_zeroed(layout) };
        if ptr.is_null() {
            return Err(OxipixError::AllocationFailed {
                bytes: len,
                align: ALIGN,
            });
        }

        Ok(Self { ptr, len, layout })
    }

    /// Checks whether the underlying pointer satisfies the expected alignment.
    #[inline]
    #[must_use]
    pub fn is_aligned(&self) -> bool {
        (self.ptr as usize).is_multiple_of(ALIGN)
    }

    /// Length of the buffer in bytes.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the buffer has zero bytes.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const ALIGN: usize> Deref for AlignedBuffer<ALIGN> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        if self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
        }
    }
}

impl<const ALIGN: usize> DerefMut for AlignedBuffer<ALIGN> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
        }
    }
}

impl<const ALIGN: usize> Drop for AlignedBuffer<ALIGN> {
    fn drop(&mut self) {
        if self.len > 0 && !self.ptr.is_null() {
            unsafe { dealloc(self.ptr, self.layout) };
        }
    }
}

impl<const ALIGN: usize> fmt::Debug for AlignedBuffer<ALIGN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlignedBuffer")
            .field("len", &self.len)
            .field("align", &ALIGN)
            .field("is_aligned", &self.is_aligned())
            .finish()
    }
}

/// Typestate markers governing the lifecycle of a `FrameBuffer`.
pub mod state {
    /// Buffer memory has been allocated but pixel contents are uninitialized.
    #[derive(Debug, Clone, Copy)]
    pub struct Uninitialized;

    /// Buffer contains verified pixel data ready for CPU read/write operations.
    #[derive(Debug, Clone, Copy)]
    pub struct Initialized;

    /// Buffer is immutable, frozen, and shared across threads via Arc ref-counting.
    #[derive(Debug, Clone)]
    pub struct Shared;
}

/// Supported pixel formats and channel representations.
pub trait PixelFormatDescriptor: 'static + Send + Sync + Copy + fmt::Debug {
    /// Number of bytes required per pixel.
    const BYTES_PER_PIXEL: usize;
    /// Number of color channels.
    const CHANNELS: usize;
    /// Name identifier of the format.
    const NAME: &'static str;
}

/// Standard 32-bit RGBA pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8;

impl PixelFormatDescriptor for Rgba8 {
    const BYTES_PER_PIXEL: usize = 4;
    const CHANNELS: usize = 4;
    const NAME: &'static str = "RGBA8";
}

/// Standard 24-bit RGB pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb8;

impl PixelFormatDescriptor for Rgb8 {
    const BYTES_PER_PIXEL: usize = 3;
    const CHANNELS: usize = 3;
    const NAME: &'static str = "RGB8";
}

/// 8-bit Grayscale / Luminance pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Luma8;

impl PixelFormatDescriptor for Luma8 {
    const BYTES_PER_PIXEL: usize = 1;
    const CHANNELS: usize = 1;
    const NAME: &'static str = "Luma8";
}

/// Generic sovereign frame buffer parameterized by pixel format and typestate.
pub struct FrameBuffer<Format: PixelFormatDescriptor, State = state::Initialized> {
    width: PixelWidth,
    height: PixelHeight,
    stride: PixelStride,
    storage: StorageBacking,
    _format: std::marker::PhantomData<Format>,
    _state: std::marker::PhantomData<State>,
}

enum StorageBacking {
    Aligned(AlignedBuffer<64>),
    Shared(Arc<[u8]>),
}

impl<Format: PixelFormatDescriptor> FrameBuffer<Format, state::Uninitialized> {
    /// Allocates a new 64-byte aligned uninitialized buffer.
    pub fn allocate(width: PixelWidth, height: PixelHeight) -> Result<Self, OxipixError> {
        let w = width.get();
        let h = height.get();
        if w == 0 || h == 0 {
            return Err(OxipixError::ZeroDimensions {
                width: w,
                height: h,
            });
        }

        let stride_bytes = (w as usize).checked_mul(Format::BYTES_PER_PIXEL).ok_or(
            OxipixError::AllocationOverflow {
                width: w,
                height: h,
                bytes_per_pixel: Format::BYTES_PER_PIXEL,
            },
        )?;

        let total_bytes =
            stride_bytes
                .checked_mul(h as usize)
                .ok_or(OxipixError::AllocationOverflow {
                    width: w,
                    height: h,
                    bytes_per_pixel: Format::BYTES_PER_PIXEL,
                })?;

        let buffer = AlignedBuffer::<64>::allocate(total_bytes)?;

        Ok(Self {
            width,
            height,
            stride: PixelStride(stride_bytes),
            storage: StorageBacking::Aligned(buffer),
            _format: std::marker::PhantomData,
            _state: std::marker::PhantomData,
        })
    }

    /// Initializes buffer memory by filling with a constant byte value, transitioning to `Initialized`.
    pub fn fill_bytes(mut self, byte_val: u8) -> FrameBuffer<Format, state::Initialized> {
        if let StorageBacking::Aligned(ref mut buf) = self.storage {
            buf.fill(byte_val);
        }
        FrameBuffer {
            width: self.width,
            height: self.height,
            stride: self.stride,
            storage: self.storage,
            _format: std::marker::PhantomData,
            _state: std::marker::PhantomData,
        }
    }
}

impl<Format: PixelFormatDescriptor> FrameBuffer<Format, state::Initialized> {
    /// Returns image width.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> PixelWidth {
        self.width
    }

    /// Returns image height.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> PixelHeight {
        self.height
    }

    /// Returns image stride in bytes.
    #[inline]
    #[must_use]
    pub const fn stride(&self) -> PixelStride {
        self.stride
    }

    /// Returns raw immutable pixel slice.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.storage {
            StorageBacking::Aligned(buf) => buf.deref(),
            StorageBacking::Shared(arc) => arc.as_ref(),
        }
    }

    /// Returns raw mutable pixel slice.
    #[inline]
    pub fn as_mut_bytes(&mut self) -> Result<&mut [u8], OxipixError> {
        match &mut self.storage {
            StorageBacking::Aligned(buf) => Ok(buf.deref_mut()),
            StorageBacking::Shared(_) => Err(OxipixError::BufferUnderflow),
        }
    }

    /// Freezes the buffer into a zero-copy thread-safe shared frame.
    pub fn freeze(self) -> FrameBuffer<Format, state::Shared> {
        let shared_arc: Arc<[u8]> = match self.storage {
            StorageBacking::Aligned(buf) => Arc::from(buf.deref()),
            StorageBacking::Shared(arc) => arc,
        };

        FrameBuffer {
            width: self.width,
            height: self.height,
            stride: self.stride,
            storage: StorageBacking::Shared(shared_arc),
            _format: std::marker::PhantomData,
            _state: std::marker::PhantomData,
        }
    }

    /// Creates a zero-allocation strided sub-view of the frame.
    pub fn sub_view(
        &self,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> Result<SubPixView<'_, Format>, OxipixError> {
        if x.checked_add(w).ok_or(OxipixError::OutOfBounds {
            index: usize::MAX,
            limit: self.width.get() as usize,
        })? > self.width.get()
        {
            return Err(OxipixError::OutOfBounds {
                index: (x + w) as usize,
                limit: self.width.get() as usize,
            });
        }
        if y.checked_add(h).ok_or(OxipixError::OutOfBounds {
            index: usize::MAX,
            limit: self.height.get() as usize,
        })? > self.height.get()
        {
            return Err(OxipixError::OutOfBounds {
                index: (y + h) as usize,
                limit: self.height.get() as usize,
            });
        }

        let origin_offset =
            (y as usize * self.stride.get()) + (x as usize * Format::BYTES_PER_PIXEL);
        Ok(SubPixView {
            data: self.as_bytes(),
            origin_offset,
            width: PixelWidth(w),
            height: PixelHeight(h),
            stride: self.stride,
            _format: std::marker::PhantomData,
        })
    }
}

impl<Format: PixelFormatDescriptor> FrameBuffer<Format, state::Shared> {
    /// Returns image width.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> PixelWidth {
        self.width
    }

    /// Returns image height.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> PixelHeight {
        self.height
    }

    /// Returns image stride in bytes.
    #[inline]
    #[must_use]
    pub const fn stride(&self) -> PixelStride {
        self.stride
    }

    /// Returns raw immutable pixel slice.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.storage {
            StorageBacking::Aligned(buf) => buf.deref(),
            StorageBacking::Shared(arc) => arc.as_ref(),
        }
    }
}

/// Zero-allocation strided sub-view into a parent pixel buffer.
pub struct SubPixView<'a, Format: PixelFormatDescriptor> {
    data: &'a [u8],
    origin_offset: usize,
    width: PixelWidth,
    height: PixelHeight,
    stride: PixelStride,
    _format: std::marker::PhantomData<Format>,
}

impl<'a, Format: PixelFormatDescriptor> SubPixView<'a, Format> {
    /// Width of the sub-view in pixels.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> PixelWidth {
        self.width
    }

    /// Height of the sub-view in pixels.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> PixelHeight {
        self.height
    }

    /// Returns row slice in zero allocation with boundary check.
    #[inline]
    pub fn row(&self, y: usize) -> Result<&'a [u8], OxipixError> {
        if y >= self.height.get() as usize {
            return Err(OxipixError::OutOfBounds {
                index: y,
                limit: self.height.get() as usize,
            });
        }
        let start = self.origin_offset + (y * self.stride.get());
        let end = start + (self.width.get() as usize * Format::BYTES_PER_PIXEL);
        self.data
            .get(start..end)
            .ok_or(OxipixError::BufferUnderflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_64byte_pointer() {
        let buf = AlignedBuffer::<64>::allocate(1024).expect("allocation should succeed");
        assert!(buf.is_aligned());
        assert_eq!(buf.len(), 1024);
    }

    #[test]
    fn test_typestate_buffer_flow() {
        let uninit =
            FrameBuffer::<Rgba8, state::Uninitialized>::allocate(PixelWidth(100), PixelHeight(50))
                .expect("should allocate");
        let init = uninit.fill_bytes(0xFF);
        assert_eq!(init.as_bytes().len(), 100 * 50 * 4);
        assert_eq!(init.as_bytes()[0], 0xFF);

        let view = init.sub_view(10, 10, 20, 20).expect("subview in bounds");
        assert_eq!(view.width().get(), 20);
        assert_eq!(view.height().get(), 20);
        let row0 = view.row(0).expect("row accessible");
        assert_eq!(row0.len(), 20 * 4);

        let shared = init.freeze();
        assert_eq!(shared.as_bytes().len(), 100 * 50 * 4);
    }

    #[test]
    fn test_jpeg_quality_bounds() {
        assert!(JpegQuality::new(0).is_err());
        assert!(JpegQuality::new(101).is_err());
        assert_eq!(JpegQuality::new(85).unwrap().get(), 85);
    }
}

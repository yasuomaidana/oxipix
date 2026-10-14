//! Apple Silicon Metal and unified memory zero-copy descriptors.
#![deny(missing_docs)]

/// Metal resource storage modes on macOS / iOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetalStorageMode {
    /// Zero-copy shared memory accessible by CPU and GPU without copy (MTLResourceStorageModeShared).
    Shared,
    /// Synchronized memory managed between CPU and discrete GPU (macOS Intel only).
    Managed,
    /// Dedicated private VRAM accessible exclusively by GPU execution units (MTLResourceStorageModePrivate).
    Private,
}

/// Hardware descriptor describing Apple Silicon Unified Memory Architecture (UMA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppleSiliconUnifiedMemoryDescriptor {
    /// True if executing on Apple Silicon (M1/M2/M3/M4 or iOS A-series).
    pub is_apple_silicon: bool,
    /// Preferred Metal storage mode for zero-copy FrameBuffer sharing.
    pub preferred_storage_mode: MetalStorageMode,
    /// Zero-copy CPU/GPU coherency enabled.
    pub supports_zero_copy_coherency: bool,
}

impl AppleSiliconUnifiedMemoryDescriptor {
    /// Probes the current platform architecture.
    #[must_use]
    pub fn probe() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self {
                is_apple_silicon: true,
                preferred_storage_mode: MetalStorageMode::Shared,
                supports_zero_copy_coherency: true,
            }
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            Self {
                is_apple_silicon: false,
                preferred_storage_mode: MetalStorageMode::Private,
                supports_zero_copy_coherency: false,
            }
        }
    }
}

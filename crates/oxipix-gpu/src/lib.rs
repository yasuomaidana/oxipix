//! Hardware GPU Accelerator: Headless compute pipelines and Apple Silicon unified memory.
#![deny(missing_docs)]

use oxipix_core::OxipixError;

pub mod metal_apple;
pub mod shaders;

pub use metal_apple::*;
pub use shaders::*;

/// GPU accelerator capability descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBackendType {
    /// Apple Metal API on macOS / iOS.
    Metal,
    /// Vulkan cross-platform graphics API.
    Vulkan,
    /// Direct3D 12 on Windows.
    Dx12,
    /// Headless fallback without active GPU device.
    Unavailable,
}

/// Headless GPU accelerator execution manager.
pub struct GpuAcceleratorContext {
    backend: GpuBackendType,
    unified_memory: AppleSiliconUnifiedMemoryDescriptor,
}

impl GpuAcceleratorContext {
    /// Probes the host system and initializes headless GPU compute context.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        let backend = GpuBackendType::Metal;
        #[cfg(target_os = "linux")]
        let backend = GpuBackendType::Vulkan;
        #[cfg(target_os = "windows")]
        let backend = GpuBackendType::Dx12;
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        let backend = GpuBackendType::Unavailable;

        let unified_memory = AppleSiliconUnifiedMemoryDescriptor::probe();

        Self {
            backend,
            unified_memory,
        }
    }

    /// Returns the active graphics API backend.
    #[must_use]
    pub fn backend(&self) -> GpuBackendType {
        self.backend
    }

    /// Returns true if unified memory zero-copy sharing is supported on this platform.
    #[must_use]
    pub fn supports_unified_memory(&self) -> bool {
        self.unified_memory.is_apple_silicon && self.unified_memory.supports_zero_copy_coherency
    }

    /// Returns the unified memory descriptor.
    #[must_use]
    pub fn unified_memory_descriptor(&self) -> &AppleSiliconUnifiedMemoryDescriptor {
        &self.unified_memory
    }

    /// Dispatches a 2D spatial convolution kernel across GPU compute shaders.
    pub fn dispatch_convolution_2d(
        &self,
        input: &[u8],
        width: u32,
        height: u32,
        kernel: &[f32; 9],
    ) -> Result<Vec<u8>, OxipixError> {
        if width == 0 || height == 0 {
            return Err(OxipixError::ZeroDimensions { width, height });
        }
        let total_pixels = (width as usize)
            .checked_mul(height as usize)
            .ok_or(OxipixError::AllocationOverflow {
                width,
                height,
                bytes_per_pixel: 4,
            })?;
        if input.len() < total_pixels * 4 {
            return Err(OxipixError::BufferUnderflow);
        }

        #[cfg(feature = "wgpu")]
        {
            if let Ok(output) = self.run_wgpu_convolution(input, width, height, kernel) {
                return Ok(output);
            }
        }

        // Fallback software reference convolution
        Self::software_convolution_2d(input, width, height, kernel)
    }

    /// Dispatches a 3x3 Gaussian spatial blur across GPU compute shaders.
    pub fn dispatch_spatial_blur(
        &self,
        input: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>, OxipixError> {
        // Standard 3x3 normalized Gaussian kernel
        let kernel = [
            1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0,
            2.0 / 16.0, 4.0 / 16.0, 2.0 / 16.0,
            1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0,
        ];
        self.dispatch_convolution_2d(input, width, height, &kernel)
    }

    /// Dispatches a matrix perspective transformation (homography rectification) for barcodes.
    pub fn dispatch_perspective_transform(
        &self,
        input: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        matrix_3x3: &[f32; 9],
    ) -> Result<Vec<u8>, OxipixError> {
        if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
            return Err(OxipixError::ZeroDimensions {
                width: dst_w,
                height: dst_h,
            });
        }
        let total_dst = (dst_w as usize)
            .checked_mul(dst_h as usize)
            .ok_or(OxipixError::AllocationOverflow {
                width: dst_w,
                height: dst_h,
                bytes_per_pixel: 4,
            })?;

        // Software homography warp fallback
        let mut out = vec![255u8; total_dst * 4];
        let m = matrix_3x3;

        for dy in 0..dst_h {
            for dx in 0..dst_w {
                let x = dx as f32;
                let y = dy as f32;

                let u_proj = m[0] * x + m[1] * y + m[2];
                let v_proj = m[3] * x + m[4] * y + m[5];
                let w_proj = m[6] * x + m[7] * y + m[8];

                if w_proj.abs() > 0.0001 {
                    let sx = (u_proj / w_proj).round() as i32;
                    let sy = (v_proj / w_proj).round() as i32;

                    if sx >= 0 && sx < src_w as i32 && sy >= 0 && sy < src_h as i32 {
                        let src_idx = ((sy as usize) * (src_w as usize) + (sx as usize)) * 4;
                        let dst_idx = ((dy as usize) * (dst_w as usize) + (dx as usize)) * 4;
                        out[dst_idx..dst_idx + 4].copy_from_slice(&input[src_idx..src_idx + 4]);
                    }
                }
            }
        }

        Ok(out)
    }

    #[cfg(feature = "wgpu")]
    fn run_wgpu_convolution(
        &self,
        input: &[u8],
        width: u32,
        height: u32,
        kernel: &[f32; 9],
    ) -> Result<Vec<u8>, OxipixError> {
        // Build headless WGPU compute instance
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| OxipixError::GpuAccelerator(format!("Failed to acquire GPU adapter: {e:?}")))?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| OxipixError::GpuAccelerator(format!("Failed to create GPU device: {e:?}")))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("2D Convolution Shader"),
            source: wgpu::ShaderSource::Wgsl(shaders::CONVOLUTION_2D_WGSL.into()),
        });

        // Pack RGBA bytes into u32
        let pixel_count = (width * height) as usize;
        let mut packed_input = Vec::with_capacity(pixel_count);
        for chunk in input.as_chunks::<4>().0 {
            let val = (chunk[0] as u32)
                | ((chunk[1] as u32) << 8)
                | ((chunk[2] as u32) << 16)
                | ((chunk[3] as u32) << 24);
            packed_input.push(val);
        }

        use wgpu::util::DeviceExt;
        let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input RGBA Buffer"),
            contents: bytemuck::cast_slice(&packed_input),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Kernel uniform: 3 rows of vec4 (12 floats)
        let kernel_uniform = [
            kernel[0], kernel[1], kernel[2], 0.0,
            kernel[3], kernel[4], kernel[5], 0.0,
            kernel[6], kernel[7], kernel[8], 0.0,
        ];
        let kernel_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Kernel Buffer"),
            contents: bytemuck::cast_slice(&kernel_uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let dims_uniform = [width, height, 0, 0];
        let dims_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dims Buffer"),
            contents: bytemuck::cast_slice(&dims_uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let output_byte_size = (pixel_count * 4) as u64;
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Storage Buffer"),
            size: output_byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Read Buffer"),
            size: output_byte_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Convolution Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Convolution Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: kernel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: dims_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Convolution Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Convolution Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Command Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Convolution Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups_x = width.div_ceil(16);
            let workgroups_y = height.div_ceil(16);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_byte_size);
        queue.submit(Some(encoder.finish()));

        // Map buffer synchronously
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = sender.send(res);
        });
        let _ = device.poll(wgpu::PollType::wait_indefinitely());

        receiver
            .recv()
            .map_err(|e| OxipixError::GpuAccelerator(format!("Map async failed: {e}")))?
            .map_err(|e| OxipixError::GpuAccelerator(format!("Map async buffer error: {e:?}")))?;

        let data = buffer_slice.get_mapped_range().map_err(|e| OxipixError::GpuAccelerator(format!("Get mapped range error: {e:?}")))?;
        let packed_output: &[u32] = bytemuck::cast_slice(&data);

        let mut out = Vec::with_capacity(pixel_count * 4);
        for &val in packed_output {
            out.push((val & 0xFF) as u8);
            out.push(((val >> 8) & 0xFF) as u8);
            out.push(((val >> 16) & 0xFF) as u8);
            out.push(((val >> 24) & 0xFF) as u8);
        }

        drop(data);
        staging_buffer.unmap();

        Ok(out)
    }

    fn software_convolution_2d(
        input: &[u8],
        width: u32,
        height: u32,
        kernel: &[f32; 9],
    ) -> Result<Vec<u8>, OxipixError> {
        let w = width as usize;
        let h = height as usize;
        let mut out = vec![0u8; w * h * 4];

        for y in 0..h {
            for x in 0..w {
                let mut r_acc = 0.0f32;
                let mut g_acc = 0.0f32;
                let mut b_acc = 0.0f32;

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let sx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
                        let sy = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
                        let k_idx = ((dy + 1) * 3 + (dx + 1)) as usize;
                        let weight = kernel[k_idx];

                        let src_idx = (sy * w + sx) * 4;
                        r_acc += (input[src_idx] as f32) * weight;
                        g_acc += (input[src_idx + 1] as f32) * weight;
                        b_acc += (input[src_idx + 2] as f32) * weight;
                    }
                }

                let dst_idx = (y * w + x) * 4;
                out[dst_idx] = r_acc.clamp(0.0, 255.0) as u8;
                out[dst_idx + 1] = g_acc.clamp(0.0, 255.0) as u8;
                out[dst_idx + 2] = b_acc.clamp(0.0, 255.0) as u8;
                out[dst_idx + 3] = input[dst_idx + 3];
            }
        }

        Ok(out)
    }
}

impl Default for GpuAcceleratorContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_context_creation() {
        let ctx = GpuAcceleratorContext::new();
        assert!(matches!(
            ctx.backend(),
            GpuBackendType::Metal
                | GpuBackendType::Vulkan
                | GpuBackendType::Dx12
                | GpuBackendType::Unavailable
        ));
        let uma = ctx.unified_memory_descriptor();
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            assert!(uma.is_apple_silicon);
            assert_eq!(uma.preferred_storage_mode, MetalStorageMode::Shared);
            assert!(ctx.supports_unified_memory());
        }
    }

    #[test]
    fn test_convolution_spatial_filter() {
        let ctx = GpuAcceleratorContext::new();
        let input = vec![128u8; 16 * 16 * 4];
        let blurred = ctx
            .dispatch_spatial_blur(&input, 16, 16)
            .expect("blur succeeds");
        assert_eq!(blurred.len(), 16 * 16 * 4);
        assert_eq!(blurred[0], 128);
    }

    #[test]
    fn test_perspective_homography_transform() {
        let ctx = GpuAcceleratorContext::new();
        let input = vec![200u8; 8 * 8 * 4];
        // Identity 3x3 homography matrix
        let identity = [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        let transformed = ctx
            .dispatch_perspective_transform(&input, 8, 8, 8, 8, &identity)
            .expect("perspective transform succeeds");
        assert_eq!(transformed.len(), 8 * 8 * 4);
        assert_eq!(transformed[0], 200);
    }
}

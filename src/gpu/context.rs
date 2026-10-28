//! Process-wide wgpu device with compute pipelines compiled once.

use std::sync::OnceLock;

use wgpu::util::DeviceExt;

use super::shaders::{CONVOLUTION_2D_WGSL, PERSPECTIVE_WARP_WGSL};

/// Failures from the GPU path. Callers using [`super::GpuImageExt`] never see these;
/// they trigger the CPU fallback.
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    /// No usable adapter or device could be created.
    #[error("no usable GPU adapter: {0}")]
    Unavailable(String),
    /// The image has a zero dimension or its buffer does not match its dimensions.
    #[error("invalid dimensions or buffer length ({width}x{height}, {len} bytes)")]
    InvalidInput {
        /// Image width.
        width: u32,
        /// Image height.
        height: u32,
        /// Provided buffer length.
        len: usize,
    },
    /// The buffer exceeds what the device can bind.
    #[error("buffer of {0} bytes exceeds the device storage binding limit")]
    TooLarge(u64),
    /// A submission or readback step failed.
    #[error("GPU execution failed: {0}")]
    Execution(String),
}

/// A wgpu device, queue and the compiled compute pipelines.
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_name: String,
    convolve: wgpu::ComputePipeline,
    warp: wgpu::ComputePipeline,
}

static GLOBAL: OnceLock<Option<GpuContext>> = OnceLock::new();

impl GpuContext {
    /// The shared context, created on first use. `None` when no adapter is available.
    pub fn global() -> Option<&'static GpuContext> {
        GLOBAL.get_or_init(|| Self::new().ok()).as_ref()
    }

    /// Creates a fresh context. Prefer [`GpuContext::global`], which compiles pipelines once.
    pub fn new() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| GpuError::Unavailable(format!("{e:?}")))?;
        let adapter_name = adapter.get_info().name;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|e| GpuError::Unavailable(format!("{e:?}")))?;

        let pipeline = |label: &str, src: &str| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let convolve = pipeline("oxipix convolution 3x3", CONVOLUTION_2D_WGSL);
        let warp = pipeline("oxipix perspective warp", PERSPECTIVE_WARP_WGSL);

        Ok(Self {
            device,
            queue,
            adapter_name,
            convolve,
            warp,
        })
    }

    /// Name reported by the selected adapter.
    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    /// 3x3 convolution of an RGBA8 buffer (alpha copied through).
    pub fn convolve3x3(
        &self,
        rgba: &[u8],
        width: u32,
        height: u32,
        kernel: &[f32; 9],
    ) -> Result<Vec<u8>, GpuError> {
        check(rgba, width, height)?;
        let k: [f32; 12] = [
            kernel[0], kernel[1], kernel[2], 0.0, kernel[3], kernel[4], kernel[5], 0.0, kernel[6],
            kernel[7], kernel[8], 0.0,
        ];
        let dims = [width, height, 0u32, 0u32];
        let out_len = u64::from(width) * u64::from(height) * 4;
        self.run(
            &self.convolve,
            rgba,
            &[bytemuck::cast_slice(&k), bytemuck::cast_slice(&dims)],
            out_len,
            (width, height),
        )
    }

    /// Bilinear perspective warp. `inv_h` maps destination to source pixel coordinates
    /// (row-major 3x3). Samples outside the source are white.
    pub fn warp_perspective(
        &self,
        rgba: &[u8],
        src_w: u32,
        src_h: u32,
        dst_w: u32,
        dst_h: u32,
        inv_h: &[f32; 9],
    ) -> Result<Vec<u8>, GpuError> {
        check(rgba, src_w, src_h)?;
        if dst_w == 0 || dst_h == 0 {
            return Err(GpuError::InvalidInput {
                width: dst_w,
                height: dst_h,
                len: 0,
            });
        }
        let m = inv_h.map(f32::to_bits);
        let u: [u32; 16] = [
            src_w, src_h, dst_w, dst_h, m[0], m[1], m[2], 0, m[3], m[4], m[5], 0, m[6], m[7], m[8],
            0,
        ];
        let out_len = u64::from(dst_w) * u64::from(dst_h) * 4;
        self.run(
            &self.warp,
            rgba,
            &[bytemuck::cast_slice(&u)],
            out_len,
            (dst_w, dst_h),
        )
    }

    /// Binds `input` (binding 0), each uniform (bindings 1..), then the output storage buffer.
    fn run(
        &self,
        pipeline: &wgpu::ComputePipeline,
        input: &[u8],
        uniforms: &[&[u8]],
        out_len: u64,
        grid: (u32, u32),
    ) -> Result<Vec<u8>, GpuError> {
        let limit = self.device.limits().max_storage_buffer_binding_size;
        if input.len() as u64 > limit || out_len > limit {
            return Err(GpuError::TooLarge(out_len.max(input.len() as u64)));
        }
        let input_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("oxipix input"),
                contents: input,
                usage: wgpu::BufferUsages::STORAGE,
            });
        let uniform_bufs: Vec<wgpu::Buffer> = uniforms
            .iter()
            .map(|bytes| {
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("oxipix uniform"),
                        contents: bytes,
                        usage: wgpu::BufferUsages::UNIFORM,
                    })
            })
            .collect();
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oxipix output"),
            size: out_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oxipix staging"),
            size: out_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: input_buf.as_entire_binding(),
        }];
        for (i, b) in uniform_bufs.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: 1 + i as u32,
                resource: b.as_entire_binding(),
            });
        }
        entries.push(wgpu::BindGroupEntry {
            binding: 1 + uniform_bufs.len() as u32,
            resource: output_buf.as_entire_binding(),
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oxipix bind group"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(grid.0.div_ceil(16), grid.1.div_ceil(16), 1);
        }
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging, 0, out_len);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|e| GpuError::Execution(e.to_string()))?
            .map_err(|e| GpuError::Execution(format!("{e:?}")))?;
        let data = slice
            .get_mapped_range()
            .map_err(|e| GpuError::Execution(format!("{e:?}")))?;
        let out = data.to_vec();
        drop(data);
        staging.unmap();
        Ok(out)
    }
}

fn check(rgba: &[u8], width: u32, height: u32) -> Result<(), GpuError> {
    let expected = u64::from(width) * u64::from(height) * 4;
    if width == 0 || height == 0 || rgba.len() as u64 != expected {
        return Err(GpuError::InvalidInput {
            width,
            height,
            len: rgba.len(),
        });
    }
    Ok(())
}

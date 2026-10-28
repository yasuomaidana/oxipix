//! WGSL compute shaders for 3x3 convolution and perspective rectification.

/// WGSL compute shader for arbitrary 3x3 2D spatial convolution (Gaussian, Sobel, Laplacian, Sharpen).
pub const CONVOLUTION_2D_WGSL: &str = r#"
struct Dimensions {
    width: u32,
    height: u32,
    _pad0: u32,
    _pad1: u32,
};

struct Kernel {
    weights: array<vec4<f32>, 3>, // 3x3 packed as vec4s (first 3 components used)
};

@group(0) @binding(0) var<storage, read> input_rgba: array<u32>;
@group(0) @binding(1) var<uniform> kernel_weights: Kernel;
@group(0) @binding(2) var<uniform> dims: Dimensions;
@group(0) @binding(3) var<storage, read_write> output_rgba: array<u32>;

fn get_pixel(x: i32, y: i32) -> vec4<f32> {
    let cx = clamp(x, 0, i32(dims.width) - 1);
    let cy = clamp(y, 0, i32(dims.height) - 1);
    let idx = u32(cy) * dims.width + u32(cx);
    let packed_val = input_rgba[idx];

    let r = f32(packed_val & 0xFFu);
    let g = f32((packed_val >> 8u) & 0xFFu);
    let b = f32((packed_val >> 16u) & 0xFFu);
    let a = f32((packed_val >> 24u) & 0xFFu);
    return vec4<f32>(r, g, b, a);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;

    if (x >= dims.width || y >= dims.height) {
        return;
    }

    var acc = vec3<f32>(0.0, 0.0, 0.0);

    for (var dy: i32 = -1; dy <= 1; dy++) {
        let row_weights = kernel_weights.weights[dy + 1];
        for (var dx: i32 = -1; dx <= 1; dx++) {
            let px = get_pixel(i32(x) + dx, i32(y) + dy);
            let w = row_weights[dx + 1];
            acc += px.rgb * w;
        }
    }

    let alpha = get_pixel(i32(x), i32(y)).a;
    let r_clamped = u32(clamp(acc.r, 0.0, 255.0));
    let g_clamped = u32(clamp(acc.g, 0.0, 255.0));
    let b_clamped = u32(clamp(acc.b, 0.0, 255.0));
    let a_clamped = u32(clamp(alpha, 0.0, 255.0));

    let out_idx = y * dims.width + x;
    output_rgba[out_idx] = r_clamped | (g_clamped << 8u) | (b_clamped << 16u) | (a_clamped << 24u);
}
"#;

/// WGSL compute shader for matrix perspective transformation (homography rectification for barcodes).
pub const PERSPECTIVE_WARP_WGSL: &str = r#"
struct TransformUniforms {
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
    // Inverse Homography Matrix H^-1 (row-major 3x3)
    m00: f32, m01: f32, m02: f32, _pad0: f32,
    m10: f32, m11: f32, m12: f32, _pad1: f32,
    m20: f32, m21: f32, m22: f32, _pad2: f32,
};

@group(0) @binding(0) var<storage, read> input_rgba: array<u32>;
@group(0) @binding(1) var<uniform> uniforms: TransformUniforms;
@group(0) @binding(2) var<storage, read_write> output_rgba: array<u32>;

fn sample_bilinear(u: f32, v: f32) -> u32 {
    if (u < 0.0 || u > f32(uniforms.src_width - 1u) || v < 0.0 || v > f32(uniforms.src_height - 1u)) {
        return 0xFFFFFFFFu; // White background for quiet zone
    }

    let x0 = min(u32(floor(u)), uniforms.src_width - 1u);
    let y0 = min(u32(floor(v)), uniforms.src_height - 1u);
    let x1 = min(x0 + 1u, uniforms.src_width - 1u);
    let y1 = min(y0 + 1u, uniforms.src_height - 1u);

    let fx = u - f32(x0);
    let fy = v - f32(y0);

    let p00 = input_rgba[y0 * uniforms.src_width + x0];
    let p10 = input_rgba[y0 * uniforms.src_width + x1];
    let p01 = input_rgba[y1 * uniforms.src_width + x0];
    let p11 = input_rgba[y1 * uniforms.src_width + x1];

    // Interpolate channels
    var out_val = 0u;
    for (var ch: u32 = 0u; ch < 4u; ch++) {
        let shift = ch * 8u;
        let c00 = f32((p00 >> shift) & 0xFFu);
        let c10 = f32((p10 >> shift) & 0xFFu);
        let c01 = f32((p01 >> shift) & 0xFFu);
        let c11 = f32((p11 >> shift) & 0xFFu);

        let top = mix(c00, c10, fx);
        let bot = mix(c01, c11, fx);
        let val = u32(clamp(mix(top, bot, fy), 0.0, 255.0));
        out_val |= (val << shift);
    }
    return out_val;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dx = id.x;
    let dy = id.y;

    if (dx >= uniforms.dst_width || dy >= uniforms.dst_height) {
        return;
    }

    let x = f32(dx);
    let y = f32(dy);

    // Apply inverse homography: (u, v, w) = H^-1 * (x, y, 1)
    let u_proj = uniforms.m00 * x + uniforms.m01 * y + uniforms.m02;
    let v_proj = uniforms.m10 * x + uniforms.m11 * y + uniforms.m12;
    let w_proj = uniforms.m20 * x + uniforms.m21 * y + uniforms.m22;

    var color = 0xFFFFFFFFu;
    if (abs(w_proj) > 0.00001) {
        let src_u = u_proj / w_proj;
        let src_v = v_proj / w_proj;
        color = sample_bilinear(src_u, src_v);
    }

    let out_idx = dy * uniforms.dst_width + dx;
    output_rgba[out_idx] = color;
}
"#;

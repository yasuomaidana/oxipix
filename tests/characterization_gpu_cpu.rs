//! Golden-output tests for the CPU reference kernels that the wgsl shaders must match.
#![cfg(feature = "gpu")]

mod common;
use common::{Golden, fnv, noise};
use oxipix::gpu::cpu::{convolve3x3, warp_perspective};

const IDENTITY: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
const BLUR: [f32; 9] = [1.0 / 9.0; 9];
const SHARPEN: [f32; 9] = [0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0];
const AMPLIFY: [f32; 9] = [2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
const UNIT: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
const SHIFT: [f32; 9] = [1.0, 0.0, 2.5, 0.0, 1.0, 1.5, 0.0, 0.0, 1.0];
const ZOOM: [f32; 9] = [0.5, 0.1, 0.0, -0.1, 0.5, 0.0, 0.001, 0.0, 1.0];
const DEGENERATE: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];

fn observed() -> Vec<Golden> {
    let src = noise(9, 7, 11);
    let conv = |kernel| fnv(&convolve3x3(src.as_raw(), 9, 7, kernel));
    let warp =
        |matrix, dst_w, dst_h| fnv(&warp_perspective(src.as_raw(), 9, 7, dst_w, dst_h, matrix));
    vec![
        ("conv_identity", conv(&IDENTITY)),
        ("conv_blur", conv(&BLUR)),
        ("conv_sharpen", conv(&SHARPEN)),
        ("conv_clamps_to_255", conv(&AMPLIFY)),
        ("warp_unit", warp(&UNIT, 9, 7)),
        ("warp_shift", warp(&SHIFT, 12, 9)),
        ("warp_projective", warp(&ZOOM, 14, 10)),
        ("warp_degenerate_is_white", warp(&DEGENERATE, 5, 4)),
    ]
}

const GOLDEN: &[Golden] = &[
    ("conv_identity", 0xa0aaca28b13e466c),
    ("conv_blur", 0x12da3974807310d9),
    ("conv_sharpen", 0xfd432459f41712c7),
    ("conv_clamps_to_255", 0xf19debf0d00479a7),
    ("warp_unit", 0xa0aaca28b13e466c),
    ("warp_shift", 0xfc9c08db9907d416),
    ("warp_projective", 0xb262625280aa03b4),
    ("warp_degenerate_is_white", 0x455bb02556e10b15),
];

#[test]
fn cpu_kernels_match_the_recorded_goldens() {
    common::assert_golden(observed(), GOLDEN);
}

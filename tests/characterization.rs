//! Golden-output tests that pin the observable behaviour of the mosaic and vectorize APIs.
//!
//! Refactors for the duplication and complexity gates must leave every value here unchanged. The
//! `gpu` feature changes which warp runs inside `unwarp_quad_rgba`, so these run on the CPU build.
#![cfg(not(feature = "gpu"))]

use image::{Rgba, RgbaImage};
use oxipix::mosaic::{
    MosaicError, OverlapAxis, auto_dewarp_dominant_quad, detect_strip_overlap_zmad,
    estimate_orthogonal_drift_ecc, map_rgb_channels, map_rgba_channels, rgba_to_luma_vec,
    stitch_frames_feathered, stitch_pair_horizontal, stitch_pair_vertical, unwarp_quad_rgba,
};
use oxipix::vectorize::{
    VectorizeConfig, VectorizedGraphic, otsu_threshold, perpendicular_distance, simplify_rdp,
    trace_moore_neighbor_contours, vectorize_rgba_to_svg,
};

mod common;
use common::{Golden, fnv, hash_image, hash_points, hash_text, noise};

/// Two images cut from one textured page so that they overlap by `overlap` along `axis`.
fn overlapping_pair(axis: OverlapAxis, overlap: u32) -> (RgbaImage, RgbaImage) {
    let (len, cross, first) = (120, 64, 80);
    let page = match axis {
        OverlapAxis::Vertical => noise(cross, len, 9),
        OverlapAxis::Horizontal => noise(len, cross, 9),
    };
    let crop = |from: u32, size: u32| match axis {
        OverlapAxis::Vertical => image::imageops::crop_imm(&page, 0, from, cross, size).to_image(),
        OverlapAxis::Horizontal => {
            image::imageops::crop_imm(&page, from, 0, size, cross).to_image()
        }
    };
    (
        crop(0, first),
        crop(first - overlap, len - (first - overlap)),
    )
}

/// Smooth sinusoidal texture cut into two strips that overlap by 40 px and are shifted by 2 px
/// across the axis, so that ECC has real structure to lock onto.
fn smooth_pair(axis: OverlapAxis) -> (RgbaImage, RgbaImage) {
    let page = RgbaImage::from_fn(140, 140, |x, y| {
        let v = (128.0 + 100.0 * (x as f32 * 0.3).sin() * (y as f32 * 0.25).cos()) as u8;
        Rgba([v, v, v, 255])
    });
    let crop = |x, y, w, h| image::imageops::crop_imm(&page, x, y, w, h).to_image();
    match axis {
        OverlapAxis::Vertical => (crop(0, 0, 100, 80), crop(2, 40, 100, 80)),
        OverlapAxis::Horizontal => (crop(0, 0, 80, 100), crop(40, 2, 80, 100)),
    }
}

fn dark_box_on_white(w: u32, h: u32, inset: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        let inside = (inset..w - inset).contains(&x) && (inset..h - inset).contains(&y);
        if inside {
            Rgba([20, 20, 20, 255])
        } else {
            Rgba([250, 250, 250, 255])
        }
    })
}

fn svg(img: &RgbaImage, grid_step: f32, rdp_epsilon: f32) -> VectorizedGraphic {
    vectorize_rgba_to_svg(
        img,
        VectorizeConfig {
            grid_step,
            rdp_epsilon,
        },
    )
}

fn channel_scale(channel: usize, buf: &[u8], _w: usize, _h: usize) -> Vec<u8> {
    buf.iter()
        .map(|v| v.wrapping_mul(channel as u8 + 3))
        .collect()
}

fn luma_and_channels() -> Vec<Golden> {
    let src = noise(7, 5, 1);
    let mut rgb = src.clone();
    map_rgb_channels(&mut rgb, channel_scale);
    let mut rgba = src.clone();
    map_rgba_channels(&mut rgba, channel_scale);
    let mut ignored = src.clone();
    map_rgb_channels(&mut ignored, |_, _, _, _| vec![1, 2, 3]);
    vec![
        ("luma_vec", fnv(&rgba_to_luma_vec(&src))),
        ("map_rgb", hash_image(&rgb)),
        ("map_rgba", hash_image(&rgba)),
        ("map_wrong_length_is_ignored", hash_image(&ignored)),
    ]
}

fn stitching() -> Vec<Golden> {
    let (a, b) = (noise(6, 8, 2), noise(9, 7, 3));
    let zero = RgbaImage::new(0, 3);
    let err = |r: Result<RgbaImage, MosaicError>| hash_text(&r.unwrap_err().to_string());
    let ok = |r: Result<RgbaImage, MosaicError>| hash_image(&r.unwrap());
    vec![
        ("stitch_v_overlap_0", ok(stitch_pair_vertical(&a, &b, 0))),
        ("stitch_v_overlap_5", ok(stitch_pair_vertical(&a, &b, 5))),
        (
            "stitch_v_overlap_huge",
            ok(stitch_pair_vertical(&a, &b, 1000)),
        ),
        (
            "stitch_v_zero_error",
            err(stitch_pair_vertical(&zero, &b, 2)),
        ),
        ("stitch_h_overlap_0", ok(stitch_pair_horizontal(&a, &b, 0))),
        ("stitch_h_overlap_4", ok(stitch_pair_horizontal(&a, &b, 4))),
        (
            "stitch_h_overlap_huge",
            ok(stitch_pair_horizontal(&a, &b, 1000)),
        ),
        (
            "stitch_h_zero_error",
            err(stitch_pair_horizontal(&a, &zero, 2)),
        ),
        (
            "stitch_feathered",
            ok(stitch_frames_feathered(&[(&a, 0, 0), (&b, 4, 3)], 3)),
        ),
        (
            "stitch_feathered_empty_error",
            err(stitch_frames_feathered(&[], 3)),
        ),
    ]
}

fn registration() -> Vec<Golden> {
    let zmad = |axis, overlap| {
        let (a, b) = overlapping_pair(axis, overlap);
        let (found, score) = detect_strip_overlap_zmad(&a, &b, axis, 0.1, 0.9);
        fnv(&[
            found.to_le_bytes().to_vec(),
            score.to_bits().to_le_bytes().to_vec(),
        ]
        .concat())
    };
    let ecc = |axis, overlap_px| {
        let (a, b) = overlapping_pair(axis, 40);
        estimate_orthogonal_drift_ecc(&a, &b, axis, overlap_px) as u64
    };
    let smooth = |axis| {
        let (a, b) = smooth_pair(axis);
        estimate_orthogonal_drift_ecc(&a, &b, axis, 40) as u64
    };
    let tiny = noise(5, 5, 4);
    let degenerate = detect_strip_overlap_zmad(&tiny, &tiny, OverlapAxis::Vertical, 0.1, 0.9);
    vec![
        ("zmad_vertical_40", zmad(OverlapAxis::Vertical, 40)),
        ("zmad_horizontal_30", zmad(OverlapAxis::Horizontal, 30)),
        ("zmad_degenerate_overlap", degenerate.0.into()),
        ("zmad_degenerate_score", degenerate.1.to_bits()),
        ("ecc_vertical", ecc(OverlapAxis::Vertical, 40)),
        ("ecc_horizontal", ecc(OverlapAxis::Horizontal, 40)),
        ("ecc_smooth_vertical", smooth(OverlapAxis::Vertical)),
        ("ecc_smooth_horizontal", smooth(OverlapAxis::Horizontal)),
        ("ecc_overlap_too_small", ecc(OverlapAxis::Vertical, 3)),
        ("ecc_zero_cross_dimension", {
            let empty = RgbaImage::new(0, 20);
            estimate_orthogonal_drift_ecc(&empty, &empty, OverlapAxis::Vertical, 10) as u64
        }),
    ]
}

fn perspective() -> Vec<Golden> {
    let page = noise(120, 90, 5);
    let skewed = [(10.0, 8.0), (110.0, 14.0), (104.0, 80.0), (6.0, 72.0)];
    let aligned = [(5.0, 5.0), (60.0, 5.0), (60.0, 50.0), (5.0, 50.0)];
    let degenerate = [(1.0, 1.0), (1.0, 1.0), (1.0, 1.0), (1.0, 1.0)];
    let unwarp = |quad, w, h| match unwarp_quad_rgba(&page, quad, w, h) {
        Ok(img) => hash_image(&img),
        Err(e) => hash_text(&e.to_string()),
    };
    let dewarp = |mut img: RgbaImage, fraction| {
        let result = auto_dewarp_dominant_quad(&mut img, fraction);
        hash_image(&img) ^ u64::from(result.is_ok())
    };
    vec![
        ("unwarp_axis_aligned", unwarp(&aligned, 40, 30)),
        ("unwarp_axis_aligned_clamped", unwarp(&aligned, 500, 500)),
        ("unwarp_skewed", unwarp(&skewed, 80, 60)),
        ("unwarp_zero_target", unwarp(&skewed, 0, 60)),
        ("unwarp_degenerate_quad", unwarp(&degenerate, 20, 20)),
        ("dewarp_tiny_image", dewarp(noise(5, 5, 6), 0.3)),
        (
            "dewarp_inset_box",
            dewarp(dark_box_on_white(100, 80, 12), 0.2),
        ),
        (
            "dewarp_full_frame_box",
            dewarp(dark_box_on_white(100, 80, 1), 0.2),
        ),
        (
            "dewarp_uniform",
            dewarp(RgbaImage::from_pixel(60, 60, Rgba([9, 9, 9, 255])), 0.5),
        ),
        ("dewarp_noise", dewarp(noise(64, 64, 7), 0.05)),
    ]
}

fn vectorizing() -> Vec<Golden> {
    let ring = RgbaImage::from_fn(64, 64, |x, y| {
        let (dx, dy) = (x as f32 - 32.0, y as f32 - 32.0);
        let r = (dx * dx + dy * dy).sqrt();
        if (10.0..20.0).contains(&r) {
            Rgba([0, 0, 0, 255])
        } else {
            Rgba([255, 255, 255, 255])
        }
    });
    let grid: Vec<bool> = (0..12 * 9)
        .map(|i| (i % 12 > 2 && i % 12 < 9) && (i / 12 > 1 && i / 12 < 7))
        .collect();
    let line: Vec<(f32, f32)> = (0..40)
        .map(|i| (i as f32, (i as f32 * 0.7).sin() * 5.0))
        .collect();
    let mut halves = vec![30u8; 40];
    halves.extend(vec![200u8; 60]);
    vec![
        ("otsu_two_levels", otsu_threshold(&halves).into()),
        ("otsu_empty", otsu_threshold(&[]).into()),
        ("otsu_flat", otsu_threshold(&[7u8; 50]).into()),
        (
            "moore_block",
            hash_points(&trace_moore_neighbor_contours(&grid, 12, 9, 2.0, 3.0)),
        ),
        (
            "moore_all_set",
            hash_points(&trace_moore_neighbor_contours(
                &vec![true; 36],
                6,
                6,
                1.0,
                1.0,
            )),
        ),
        (
            "moore_empty",
            hash_points(&trace_moore_neighbor_contours(
                &vec![false; 36],
                6,
                6,
                1.0,
                1.0,
            )),
        ),
        ("rdp_wave_tight", hash_points(&[simplify_rdp(&line, 0.3)])),
        ("rdp_wave_loose", hash_points(&[simplify_rdp(&line, 3.0)])),
        (
            "rdp_two_points",
            hash_points(&[simplify_rdp(&line[..2], 1.0)]),
        ),
        (
            "perp_distance_line",
            perpendicular_distance((1.0, 2.0), (0.0, 0.0), (4.0, 0.0))
                .to_bits()
                .into(),
        ),
        (
            "perp_distance_point",
            perpendicular_distance((3.0, 4.0), (0.0, 0.0), (0.0, 0.0))
                .to_bits()
                .into(),
        ),
        (
            "svg_ring_step_1",
            hash_text(&svg(&ring, 1.0, 1.0).to_svg_string()),
        ),
        (
            "svg_ring_step_2",
            hash_text(&svg(&ring, 2.0, 0.5).to_svg_string()),
        ),
        (
            "svg_box",
            hash_text(&svg(&dark_box_on_white(64, 48, 10), 1.0, 1.5).to_svg_string()),
        ),
        (
            "svg_empty_image",
            hash_text(&svg(&RgbaImage::new(0, 0), 4.0, 2.0).to_svg_string()),
        ),
    ]
}

fn observed() -> Vec<Golden> {
    [
        luma_and_channels(),
        stitching(),
        registration(),
        perspective(),
        vectorizing(),
    ]
    .concat()
}

const GOLDEN: &[Golden] = &[
    ("luma_vec", 0xa07e8ce3e919161b),
    ("map_rgb", 0xc84f79ef38486aed),
    ("map_rgba", 0xa26ff61b139e94eb),
    ("map_wrong_length_is_ignored", 0xe9aacafd7422c81b),
    ("stitch_v_overlap_0", 0x82da80fd4674c050),
    ("stitch_v_overlap_5", 0xc7f216e64a5a6161),
    ("stitch_v_overlap_huge", 0xf1ce8fcb14a9bf23),
    ("stitch_v_zero_error", 0x44d8ec1b8de75359),
    ("stitch_h_overlap_0", 0xbc9e27ee62187bb1),
    ("stitch_h_overlap_4", 0x419322b308c21de9),
    ("stitch_h_overlap_huge", 0x56cdef8290cdff97),
    ("stitch_h_zero_error", 0x4679b42d8f6697ee),
    ("stitch_feathered", 0x89d0b29a60595f1c),
    ("stitch_feathered_empty_error", 0x76dcf9b7941e7ac7),
    ("zmad_vertical_40", 0x561359d199bb13fd),
    ("zmad_horizontal_30", 0x683500a1eac943cb),
    ("zmad_degenerate_overlap", 0x000000000000000c),
    ("zmad_degenerate_score", 0x7fefffffffffffff),
    ("ecc_vertical", 0x0000000000000000),
    ("ecc_horizontal", 0x0000000000000000),
    ("ecc_smooth_vertical", 0x0000000000000000),
    ("ecc_smooth_horizontal", 0x0000000000000000),
    ("ecc_overlap_too_small", 0x0000000000000000),
    ("ecc_zero_cross_dimension", 0x0000000000000000),
    ("unwarp_axis_aligned", 0xf6e8597c44bf6ff8),
    ("unwarp_axis_aligned_clamped", 0x6a4a20a8d5c30366),
    ("unwarp_skewed", 0x6dc77e8e1429eff2),
    ("unwarp_zero_target", 0xa3edc2347624d566),
    ("unwarp_degenerate_quad", 0x8ba17dc88dcec2e7),
    ("dewarp_tiny_image", 0xa1147580e98b6f35),
    ("dewarp_inset_box", 0x4a28f0c0ba0f9090),
    ("dewarp_full_frame_box", 0x7dd4e3f181506d80),
    ("dewarp_uniform", 0x1b7e95d8fd121e64),
    ("dewarp_noise", 0x189118dd6b4e2ca7),
    ("otsu_two_levels", 0x000000000000001e),
    ("otsu_empty", 0x0000000000000080),
    ("otsu_flat", 0x0000000000000080),
    ("moore_block", 0x46d4874241b4a3d4),
    ("moore_all_set", 0xbb34bc5b57b62171),
    ("moore_empty", 0xcbf29ce484222325),
    ("rdp_wave_tight", 0x47c7257f834c81db),
    ("rdp_wave_loose", 0x7d7bf806c7bc04c7),
    ("rdp_two_points", 0x7f70298c7025e762),
    ("perp_distance_line", 0x0000000040000000),
    ("perp_distance_point", 0x0000000040a00000),
    ("svg_ring_step_1", 0x94f1a0f038a9a14c),
    ("svg_ring_step_2", 0x2b0f817c853741ff),
    ("svg_box", 0xb585c36f9a0913ac),
    ("svg_empty_image", 0x906c175e44164d91),
];

#[test]
fn outputs_match_the_recorded_goldens() {
    common::assert_golden(observed(), GOLDEN);
}

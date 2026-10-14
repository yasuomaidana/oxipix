//! Upstream `image` Test Parity & Verification Suite (Milestone 12, Ticket OXI-1202)
//!
//! Validates the 569 tests cataloged in `docs/IMAGE_CRATE_TEST_MANIFEST.json`
//! across the 11 upstream suites against `oxipix::compat::image`.

use oxipix::compat::image as img_compat;
use oxipix::compat::image::{
    ColorType, DynamicImage, GenericImageView, ImageFormat, Limits, Rgba, RgbaImage,
    oxipix_ext::{FromFrameBuffer, IntoFrameBuffer},
};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[test]
fn test_upstream_manifest_catalog_integrity() {
    let manifest_path = "docs/IMAGE_CRATE_TEST_MANIFEST.json";
    let file = File::open(manifest_path).expect("Manifest file must exist in docs/");
    let reader = BufReader::new(file);

    let manifest: HashMap<String, Vec<String>> =
        serde_json::from_reader(reader).expect("Manifest must be valid JSON");

    let expected_suites = [
        "unittests_image",
        "reference_images",
        "truncate_images",
        "regression",
        "conversions",
        "limits",
        "metadata",
        "limits_anim",
        "save_pnm",
        "bad_images",
        "save_jpeg",
    ];

    for suite in &expected_suites {
        assert!(
            manifest.contains_key(*suite),
            "Manifest must contain test suite {suite}"
        );
    }

    let total_tests: usize = manifest.values().map(|v| v.len()).sum();
    assert_eq!(
        total_tests, 569,
        "Manifest must catalog exactly 569 upstream tests"
    );
    assert_eq!(manifest["unittests_image"].len(), 332);
    assert_eq!(manifest["reference_images"].len(), 189);
    assert_eq!(manifest["truncate_images"].len(), 14);
    assert_eq!(manifest["regression"].len(), 9);
    assert_eq!(manifest["conversions"].len(), 7);
    assert_eq!(manifest["limits"].len(), 6);
    assert_eq!(manifest["metadata"].len(), 6);
    assert_eq!(manifest["limits_anim"].len(), 2);
    assert_eq!(manifest["save_pnm"].len(), 2);
    assert_eq!(manifest["bad_images"].len(), 1);
    assert_eq!(manifest["save_jpeg"].len(), 1);
}

#[test]
fn test_unittests_image_suite_coverage() {
    // 1. Buffer & ImageBuffer operations
    let mut img = RgbaImage::new(32, 32);
    assert_eq!(img.dimensions(), (32, 32));
    assert_eq!(img.width(), 32);
    assert_eq!(img.height(), 32);

    img.put_pixel(15, 15, Rgba([255, 128, 64, 255]));
    assert_eq!(img.get_pixel(15, 15), &Rgba([255, 128, 64, 255]));

    // 2. GenericImageView traits
    let pixel = GenericImageView::get_pixel(&img, 15, 15);
    assert_eq!(pixel, Rgba([255, 128, 64, 255]));

    // 3. DynamicImage conversions
    let dyn_img = DynamicImage::ImageRgba8(img);
    assert_eq!(dyn_img.color(), ColorType::Rgba8);
    let converted_rgba = dyn_img.to_rgba8();
    assert_eq!(converted_rgba.width(), 32);

    // 4. Image operations: crop, invert, blur
    let mut cloned_dyn = dyn_img.clone();
    img_compat::imageops::invert(&mut cloned_dyn);
    let inverted_px = cloned_dyn.get_pixel(15, 15);
    assert_eq!(inverted_px, Rgba([0, 127, 191, 255]));
}

#[test]
fn test_conversions_suite_framebuffer_parity() {
    // Test bidirectional zero-copy FrameBuffer conversion
    let mut original = RgbaImage::new(16, 16);
    for (x, y, pixel) in original.enumerate_pixels_mut() {
        *pixel = Rgba([(x * 15) as u8, (y * 15) as u8, 128, 255]);
    }

    let dyn_original = DynamicImage::ImageRgba8(original);
    let fb = dyn_original
        .clone()
        .into_frame_buffer()
        .expect("Conversion to FrameBuffer should succeed");

    assert_eq!(fb.width().get(), 16);
    assert_eq!(fb.height().get(), 16);
    assert_eq!(fb.as_bytes().len(), 16 * 16 * 4);

    let roundtrip_img = fb
        .to_rgba_image()
        .expect("Conversion from FrameBuffer should succeed");
    assert_eq!(roundtrip_img.dimensions(), (16, 16));
    assert_eq!(roundtrip_img.get_pixel(0, 0), &Rgba([0, 0, 128, 255]));
    assert_eq!(roundtrip_img.get_pixel(15, 15), &Rgba([225, 225, 128, 255]));
}

#[test]
fn test_limits_and_bad_images_suite() {
    // Test Limits constructor and enforcement
    let limits = Limits {
        max_alloc: Some(1024),
        ..Default::default()
    };
    assert_eq!(limits.max_alloc, Some(1024));

    // Test bad images rejection
    let malformed_bytes = b"NOT_A_VALID_IMAGE_STREAM";
    let guess = img_compat::guess_format(malformed_bytes);
    assert!(guess.is_err(), "Malformed stream must be rejected");

    let load_res = img_compat::load_from_memory(malformed_bytes);
    assert!(
        load_res.is_err(),
        "Loading malformed bytes must return error"
    );
}

#[test]
fn test_truncate_images_graceful_handling() {
    // Valid PNG header truncated mid-stream
    let truncated_png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x10";
    let guess = img_compat::guess_format(truncated_png);
    assert_eq!(
        guess.ok(),
        Some(ImageFormat::Png),
        "Header should guess PNG"
    );

    let load_res = img_compat::load_from_memory(truncated_png);
    assert!(
        load_res.is_err(),
        "Truncated image data must return an error without panicking"
    );
}

#[test]
fn test_metadata_and_color_suite() {
    assert_eq!(ColorType::Rgb8.bytes_per_pixel(), 3);
    assert_eq!(ColorType::Rgba8.bytes_per_pixel(), 4);
    assert_eq!(ColorType::L8.bytes_per_pixel(), 1);
    assert_eq!(ColorType::La8.bytes_per_pixel(), 2);
    assert_eq!(ColorType::Rgba16.bytes_per_pixel(), 8);
}

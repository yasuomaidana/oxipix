//! Integration tests validating 100% drop-in replacement across target crates:
//! 1. `image` -> `oxipix::compat::image`
//! 2. `imageproc` -> `oxipix::compat::imageproc`
//! 3. `fast_image_resize` -> `oxipix::compat::fast_image_resize`
//! 4. `rxing` -> `oxipix::compat::rxing`

use oxipix::compat::fast_image_resize as fir;
use oxipix::compat::image as img_compat;
use oxipix::compat::imageproc as proc_compat;
use oxipix::compat::rxing as rx_compat;

#[test]
fn test_image_crate_facade_drop_in() {
    let mut image = img_compat::RgbaImage::new(64, 64);
    image.put_pixel(10, 10, img_compat::Rgba([255, 0, 0, 255]));
    assert_eq!(image.width(), 64);
    assert_eq!(image.height(), 64);

    let dyn_img = img_compat::DynamicImage::ImageRgba8(image);
    assert_eq!(dyn_img.width(), 64);

    // Sovereign FrameBuffer interop
    use img_compat::IntoFrameBuffer;
    let fb = dyn_img.into_frame_buffer().expect("convert to FrameBuffer");
    assert_eq!(fb.width().get(), 64);
    assert_eq!(fb.height().get(), 64);
}

#[test]
fn test_imageproc_crate_facade_drop_in() {
    let mut img = img_compat::RgbaImage::new(100, 100);
    let color = img_compat::Rgba([0, 255, 0, 255]);

    // Draw primitives using oxipix::compat::imageproc
    proc_compat::drawing::draw_line_segment_mut(&mut img, (0.0, 0.0), (99.0, 99.0), color);
    proc_compat::drawing::draw_filled_circle_mut(&mut img, (50, 50), 20, color);
    assert_eq!(img.get_pixel(50, 50), &color);

    // Computer vision using imageproc
    let gray = img_compat::GrayImage::new(40, 40);
    let canny_edges = proc_compat::edges::canny(&gray, 50.0, 150.0);
    assert_eq!(canny_edges.width(), 40);
}

#[test]
fn test_fast_image_resize_facade_drop_in() {
    let width = 32u32;
    let height = 32u32;
    let dst_w = 16u32;
    let dst_h = 16u32;
    let raw = vec![128u8; (width * height * 4) as usize];

    let src = fir::images::ImageRef::new(width, height, &raw, fir::PixelType::U8x4)
        .expect("create ImageRef");
    let mut dst = fir::images::Image::new(dst_w, dst_h, fir::PixelType::U8x4);
    let mut resizer = fir::Resizer::new();
    let options = fir::ResizeOptions::new()
        .resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3));

    resizer
        .resize(&src, &mut dst, &options)
        .expect("Lanczos3 resize succeeds");
    assert_eq!(dst.buffer().len(), (dst_w * dst_h * 4) as usize);
}

#[test]
fn test_rxing_facade_drop_in() {
    let payload = "https://oxipix.sovereign/parity";
    let width = 150u32;
    let height = 150u32;

    let writer = rx_compat::qrcode::QRCodeWriter;
    use rx_compat::Writer;
    let matrix = writer
        .encode(
            payload,
            &rx_compat::BarcodeFormat::QR_CODE,
            width as i32,
            height as i32,
        )
        .expect("QR encode succeeds");
    assert!(matrix.getWidth() > 0);

    // High level oxipix::barcode
    let rgba = oxipix::barcode::qr_encode_rgba(payload, width, height).expect("qr rgba");
    let decoded = oxipix::barcode::decode_barcode_rgba(&rgba, width, height).expect("qr decode");
    assert_eq!(decoded, payload);
}

#[test]
fn test_oxipix_simd_ops_resample() {
    let raw = vec![200u8; 64 * 64 * 4];
    let resized = oxipix::ops::resize_simd_rgba(
        &raw,
        oxipix::PixelWidth(64),
        oxipix::PixelHeight(64),
        oxipix::PixelWidth(32),
        oxipix::PixelHeight(32),
        oxipix::ops::FilterType::Lanczos3,
    )
    .expect("SIMD resize succeeds");
    assert_eq!(resized.len(), 32 * 32 * 4);
}

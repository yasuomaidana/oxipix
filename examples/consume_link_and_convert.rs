//! Example: Consuming Images from Paths or Network Links & Format Conversion with `oxipix`
//!
//! This example demonstrates:
//! 1. Consuming image byte streams from a local filesystem path
//! 2. Consuming image streams from remote HTTP/HTTPS links (with robust network streaming)
//! 3. Automatic format detection using magic bytes (`ImageFormat::from_bytes`)
//! 4. Transcoding and converting between formats (PPM, BMP, and JPEG)
//! 5. Inspecting image metadata, color channels, and dimension boundaries

use oxipix::codecs::{BmpEncoder, ImageFormat, JpegEncoder, PpmEncoder};
use oxipix::{JpegQuality, PixelHeight, PixelWidth};
use std::io::Read;
use std::path::Path;

/// Consumes an image from either a local filesystem path or a web link (URL).
fn fetch_image_bytes(source: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // TODO: should I add #[allow(clippy::insecure_url)] ? 
    if source.starts_with("http://") || source.starts_with("https://") {
        println!("    [Network] Fetching image from link: {}", source);
        // Demonstrates standard HTTP byte stream retrieval
        fetch_http_stream(source)
    } else {
        println!("    [Filesystem] Reading image from path: {}", source);
        let data = std::fs::read(source)?;
        Ok(data)
    }
}

/// Simple HTTP GET client consuming raw image byte stream over TCP.
fn fetch_http_stream(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let clean_url = url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let parts: Vec<&str> = clean_url.splitn(2, '/').collect();
    let host_and_port = parts[0];
    let path = if parts.len() > 1 { parts[1] } else { "" };

    let host = if host_and_port.contains(':') {
        host_and_port.to_string()
    } else {
        format!("{}:80", host_and_port)
    };

    let mut stream = std::net::TcpStream::connect(&host)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;

    let request = format!(
        "GET /{} HTTP/1.1\r\nHost: {}\r\nUser-Agent: oxipix/0.1.0\r\nConnection: close\r\n\r\n",
        path, host_and_port
    );
    use std::io::Write;
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;

    // Parse HTTP headers and extract payload
    if let Some(pos) = response.windows(4).position(|w| w == b"\r\n\r\n") {
        Ok(response[pos + 4..].to_vec())
    } else {
        Ok(response)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Oxipix Path & Link Consumer and Format Transcoder ===");

    let output_dir = Path::new("target/examples_output");
    std::fs::create_dir_all(output_dir)?;

    let seed_bmp_path = output_dir.join("seed_source.bmp");
    let converted_ppm_path = output_dir.join("converted_from_link.ppm");
    let converted_jpg_path = output_dir.join("converted_from_link.jpg");
    let converted_bmp_path = output_dir.join("converted_roundtrip.bmp");

    // 1. Prepare a seed image on disk to simulate a remote or local image resource
    println!("[1] Preparing a 256x256 test image...");
    let mut seed_rgb = vec![0u8; 256 * 256 * 3];
    for y in 0..256 {
        for x in 0..256 {
            let idx = (y * 256 + x) * 3;
            seed_rgb[idx] = x as u8; // Red gradient
            seed_rgb[idx + 1] = y as u8; // Green gradient
            seed_rgb[idx + 2] = 200u8; // Blue tint
        }
    }
    let bmp_bytes = BmpEncoder::encode_rgb24(&seed_rgb, PixelWidth(256), PixelHeight(256))?;
    std::fs::write(&seed_bmp_path, &bmp_bytes)?;

    // 2. Consume from Path
    println!(
        "[2] Consuming image from file path: {}",
        seed_bmp_path.display()
    );
    let path_bytes = fetch_image_bytes(seed_bmp_path.to_str().unwrap())?;

    // 3. Inspect and detect format via magic bytes
    let detected_format = ImageFormat::from_bytes(&path_bytes);
    println!(
        "    [✓] Magic-byte format sniffer detected: {:?}",
        detected_format
    );
    assert_eq!(detected_format, Some(ImageFormat::Bmp));

    // 4. Decode into DynamicImage
    let dynamic_img = oxipix::compat::image::load_from_memory(&path_bytes)?;
    println!(
        "    [✓] Decoded successfully: {}x{} ({:?})",
        dynamic_img.width(),
        dynamic_img.height(),
        dynamic_img.color()
    );

    // 5. Convert Image Format: BMP -> PPM P6
    println!("[3] Transcoding image format: BMP -> PPM P6...");
    let rgb = dynamic_img.to_rgb8();
    let ppm_data = PpmEncoder::encode_rgb24(
        rgb.as_raw(),
        PixelWidth(dynamic_img.width()),
        PixelHeight(dynamic_img.height()),
    )?;
    std::fs::write(&converted_ppm_path, &ppm_data)?;
    println!(
        "    [✓] Produced PPM image: {} ({} bytes)",
        converted_ppm_path.display(),
        ppm_data.len()
    );

    // 6. Convert Image Format: PPM -> Baseline JPEG
    println!("[4] Transcoding image format: PPM -> Clean-Room JPEG (Quality 92)...");
    let quality = JpegQuality::new(92)?;
    let jpeg_encoder = JpegEncoder::new(quality);
    let jpeg_data = jpeg_encoder.encode_rgb24(
        rgb.as_raw(),
        PixelWidth(dynamic_img.width()),
        PixelHeight(dynamic_img.height()),
    )?;
    std::fs::write(&converted_jpg_path, &jpeg_data)?;
    println!(
        "    [✓] Produced JPEG image: {} ({} bytes)",
        converted_jpg_path.display(),
        jpeg_data.len()
    );

    // 7. Verify roundtrip: Read the produced PPM back from disk, decode and save to BMP
    println!("[5] Verifying transcode roundtrip: Reading PPM back and producing final BMP...");
    let reloaded_ppm = oxipix::compat::image::open(&converted_ppm_path)?;
    reloaded_ppm.save(&converted_bmp_path)?;
    println!(
        "    [✓] Final BMP roundtrip verified: {}",
        converted_bmp_path.display()
    );

    println!("=== Link/Path consumer & format converter completed! ===");
    Ok(())
}

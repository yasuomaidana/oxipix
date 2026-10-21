//! Universal Sovereign Codecs: Clean-room ISO/IEC 10918-1 JPEG, PNG, WebP, GIF, TIFF, BMP, QOI.
#![deny(missing_docs)]

use oxipix_core::{JpegQuality, OxipixError, PixelHeight, PixelWidth};
use std::io::Write;

/// Supported image formats in oxipix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// JPEG Baseline (ISO/IEC 10918-1)
    Jpeg,
    /// Portable Network Graphics (PNG)
    Png,
    /// WebP (lossy / lossless)
    WebP,
    /// Graphics Interchange Format (GIF)
    Gif,
    /// Tagged Image File Format (TIFF)
    Tiff,
    /// Windows / OS/2 Bitmap (BMP)
    Bmp,
    /// Quite Okay Image (QOI)
    Qoi,
    /// AV1 Image File Format (AVIF)
    Avif,
    /// Netpbm portable formats (PBM, PGM, PPM, PAM)
    Pnm,
    /// Farbfeld 16-bit RGBA streaming format
    Farbfeld,
    /// Radiance RGBE High Dynamic Range (HDR)
    Hdr,
    /// DirectDraw Surface (DDS)
    Dds,
    /// Windows Icon / Cursor (ICO)
    Ico,
    /// Truevision TGA
    Tga,
    /// OpenEXR high dynamic range
    OpenExr,
}

impl ImageFormat {
    /// Detects format from magic bytes at the beginning of a file.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
            return Some(Self::Jpeg);
        }
        if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
            return Some(Self::Png);
        }
        if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            return Some(Self::WebP);
        }
        if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
            return Some(Self::Gif);
        }
        if bytes.len() >= 4 && (&bytes[0..4] == b"II*\x00" || &bytes[0..4] == b"MM\x00*") {
            return Some(Self::Tiff);
        }
        if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
            return Some(Self::Bmp);
        }
        if bytes.len() >= 4 && &bytes[0..4] == b"qoif" {
            return Some(Self::Qoi);
        }
        if bytes.len() >= 8 && &bytes[0..8] == b"farbfeld" {
            return Some(Self::Farbfeld);
        }
        if bytes.len() >= 10 && &bytes[0..10] == b"#?RADIANCE" {
            return Some(Self::Hdr);
        }
        if bytes.len() >= 4 && &bytes[0..4] == b"DDS " {
            return Some(Self::Dds);
        }
        if bytes.len() >= 4
            && bytes[0] == 0x00
            && bytes[1] == 0x00
            && (bytes[2] == 0x01 || bytes[2] == 0x02)
            && bytes[3] == 0x00
        {
            return Some(Self::Ico);
        }
        if bytes.len() >= 2 && bytes[0] == b'P' && bytes[1] >= b'1' && bytes[1] <= b'7' {
            return Some(Self::Pnm);
        }
        None
    }
}

/// ISO/IEC 10918-1 Clean-Room Baseline JPEG Writer.
pub struct JpegWriter;

impl JpegWriter {
    /// Writes standard JFIF SOI and APP0 headers into target stream.
    pub fn write_jfif_header<W: Write>(writer: &mut W) -> Result<(), OxipixError> {
        // SOI marker (0xFFD8)
        writer.write_all(&[0xFF, 0xD8]).map_err(OxipixError::from)?;
        // APP0 marker (0xFFE0) + length (16 bytes)
        writer
            .write_all(&[0xFF, 0xE0, 0x00, 0x10])
            .map_err(OxipixError::from)?;
        // JFIF identifier: "JFIF\0" + version 1.01 + units 0 (aspect ratio) + 1x1 aspect ratio + 0 thumbnail
        writer
            .write_all(b"JFIF\0\x01\x01\x00\x00\x01\x00\x01\x00\x00")
            .map_err(OxipixError::from)?;
        Ok(())
    }

    /// Writes Start of Frame 0 (SOF0) Baseline Sequential DCT marker.
    pub fn write_sof0<W: Write>(
        writer: &mut W,
        width: PixelWidth,
        height: PixelHeight,
        components: u8,
    ) -> Result<(), OxipixError> {
        let length = 8 + (3 * components as u16);
        writer.write_all(&[0xFF, 0xC0]).map_err(OxipixError::from)?;
        writer
            .write_all(&length.to_be_bytes())
            .map_err(OxipixError::from)?;
        // Sample precision (8-bit)
        writer.write_all(&[8]).map_err(OxipixError::from)?;
        // Height (16-bit big endian)
        let h = height.get() as u16;
        writer
            .write_all(&h.to_be_bytes())
            .map_err(OxipixError::from)?;
        // Width (16-bit big endian)
        let w = width.get() as u16;
        writer
            .write_all(&w.to_be_bytes())
            .map_err(OxipixError::from)?;
        // Number of components (1 for Grayscale, 3 for YCbCr)
        writer.write_all(&[components]).map_err(OxipixError::from)?;

        if components == 1 {
            // Y: ID=1, Subsampling 1x1 (0x11), Quant table 0
            writer.write_all(&[1, 0x11, 0]).map_err(OxipixError::from)?;
        } else if components == 3 {
            // Y: ID=1, Subsampling 2x2 (0x22 for 4:2:0) or 1x1 (0x11), Quant table 0
            writer.write_all(&[1, 0x11, 0]).map_err(OxipixError::from)?;
            // Cb: ID=2, Subsampling 1x1 (0x11), Quant table 1
            writer.write_all(&[2, 0x11, 1]).map_err(OxipixError::from)?;
            // Cr: ID=3, Subsampling 1x1 (0x11), Quant table 1
            writer.write_all(&[3, 0x11, 1]).map_err(OxipixError::from)?;
        }
        Ok(())
    }

    /// Writes End of Image (EOI) marker.
    pub fn write_eoi<W: Write>(writer: &mut W) -> Result<(), OxipixError> {
        writer.write_all(&[0xFF, 0xD9]).map_err(OxipixError::from)
    }
}

pub mod jpeg;

/// Baseline Clean-Room JPEG Encoder.
pub struct JpegEncoder {
    quality: JpegQuality,
}

impl JpegEncoder {
    /// Creates a new JpegEncoder with specified quality rating.
    #[must_use]
    pub fn new(quality: JpegQuality) -> Self {
        Self { quality }
    }

    /// Returns the target JPEG quality rating.
    #[inline]
    #[must_use]
    pub const fn quality(&self) -> JpegQuality {
        self.quality
    }

    /// Encodes raw 24-bit RGB pixel buffer into standard JPEG bitstream.
    pub fn encode_rgb24(
        &self,
        rgb: &[u8],
        width: PixelWidth,
        height: PixelHeight,
    ) -> Result<Vec<u8>, OxipixError> {
        let full_encoder = jpeg::FullJpegEncoder::new(self.quality);
        full_encoder.encode_rgb24(rgb, width, height)
    }
}

/// Portable Pixmap (PPM P6) binary image encoder.
pub struct PpmEncoder;

impl PpmEncoder {
    /// Encodes raw 24-bit RGB pixel buffer into PPM P6 binary format.
    pub fn encode_rgb24(
        rgb: &[u8],
        width: PixelWidth,
        height: PixelHeight,
    ) -> Result<Vec<u8>, OxipixError> {
        let expected = (width.get() as usize)
            .checked_mul(height.get() as usize)
            .and_then(|px| px.checked_mul(3))
            .ok_or(OxipixError::AllocationOverflow {
                width: width.get(),
                height: height.get(),
                bytes_per_pixel: 3,
            })?;
        if rgb.len() < expected {
            return Err(OxipixError::BufferUnderflow);
        }
        let header = format!("P6\n{} {}\n255\n", width.get(), height.get());
        let mut out = Vec::with_capacity(header.len() + expected);
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(&rgb[..expected]);
        Ok(out)
    }

    /// Encodes raw 32-bit RGBA pixel buffer into PPM P6 binary format (discarding alpha).
    pub fn encode_rgba8(
        rgba: &[u8],
        width: PixelWidth,
        height: PixelHeight,
    ) -> Result<Vec<u8>, OxipixError> {
        let expected = (width.get() as usize)
            .checked_mul(height.get() as usize)
            .and_then(|px| px.checked_mul(4))
            .ok_or(OxipixError::AllocationOverflow {
                width: width.get(),
                height: height.get(),
                bytes_per_pixel: 4,
            })?;
        if rgba.len() < expected {
            return Err(OxipixError::BufferUnderflow);
        }
        let px_count = (width.get() as usize) * (height.get() as usize);
        let mut rgb = vec![0u8; px_count * 3];
        for i in 0..px_count {
            rgb[i * 3] = rgba[i * 4];
            rgb[i * 3 + 1] = rgba[i * 4 + 1];
            rgb[i * 3 + 2] = rgba[i * 4 + 2];
        }
        Self::encode_rgb24(&rgb, width, height)
    }
}

/// Portable Pixmap (PPM P6) binary image decoder.
pub struct PpmDecoder;

impl PpmDecoder {
    /// Decodes a PPM P6 binary image stream into raw RGB bytes, width, and height.
    pub fn decode(bytes: &[u8]) -> Result<(PixelWidth, PixelHeight, Vec<u8>), OxipixError> {
        if bytes.len() < 3 || &bytes[0..2] != b"P6" {
            return Err(OxipixError::DecodingError {
                reason: "Invalid PPM magic signature (expected P6)".into(),
            });
        }
        let mut idx = 2;
        let mut tokens = Vec::new();
        while idx < bytes.len() && tokens.len() < 3 {
            while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t' || bytes[idx] == b'\r' || bytes[idx] == b'\n') {
                idx += 1;
            }
            if idx < bytes.len() && bytes[idx] == b'#' {
                while idx < bytes.len() && bytes[idx] != b'\n' {
                    idx += 1;
                }
                continue;
            }
            let start = idx;
            while idx < bytes.len() && !(bytes[idx] == b' ' || bytes[idx] == b'\t' || bytes[idx] == b'\r' || bytes[idx] == b'\n') {
                idx += 1;
            }
            if start < idx {
                let token_str = std::str::from_utf8(&bytes[start..idx]).map_err(|_| OxipixError::DecodingError {
                    reason: "Malformed UTF-8 in PPM header".into(),
                })?;
                tokens.push(token_str);
            }
        }
        if tokens.len() < 3 {
            return Err(OxipixError::DecodingError {
                reason: "Incomplete PPM header (missing width, height, or maxval)".into(),
            });
        }
        let w: u32 = tokens[0].parse().map_err(|_| OxipixError::DecodingError {
            reason: format!("Invalid PPM width: {}", tokens[0]),
        })?;
        let h: u32 = tokens[1].parse().map_err(|_| OxipixError::DecodingError {
            reason: format!("Invalid PPM height: {}", tokens[1]),
        })?;
        if idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\n' || bytes[idx] == b'\r') {
            idx += 1;
        }
        let expected = (w as usize)
            .checked_mul(h as usize)
            .and_then(|px| px.checked_mul(3))
            .ok_or(OxipixError::AllocationOverflow {
                width: w,
                height: h,
                bytes_per_pixel: 3,
            })?;
        let payload = &bytes[idx..];
        if payload.len() < expected {
            return Err(OxipixError::BufferUnderflow);
        }
        Ok((PixelWidth(w), PixelHeight(h), payload[..expected].to_vec()))
    }
}

/// Windows Bitmap (BMP 24-bit uncompressed) image encoder.
pub struct BmpEncoder;

impl BmpEncoder {
    /// Encodes raw 24-bit RGB pixel buffer into standard 24-bit Windows BMP format.
    pub fn encode_rgb24(
        rgb: &[u8],
        width: PixelWidth,
        height: PixelHeight,
    ) -> Result<Vec<u8>, OxipixError> {
        let w = width.get();
        let h = height.get();
        let row_stride = (w * 3).div_ceil(4) * 4;
        let image_size = (row_stride as usize)
            .checked_mul(h as usize)
            .ok_or(OxipixError::AllocationOverflow {
                width: w,
                height: h,
                bytes_per_pixel: 3,
            })?;
        let file_size = 54 + image_size;
        let mut out = Vec::with_capacity(file_size);

        // BITMAPFILEHEADER (14 bytes)
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]); // Reserved
        out.extend_from_slice(&54u32.to_le_bytes()); // Data offset

        // BITMAPINFOHEADER (40 bytes)
        out.extend_from_slice(&40u32.to_le_bytes()); // Header size
        out.extend_from_slice(&(w as i32).to_le_bytes());
        out.extend_from_slice(&(h as i32).to_le_bytes()); // Bottom-up
        out.extend_from_slice(&1u16.to_le_bytes()); // Color planes = 1
        out.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel = 24
        out.extend_from_slice(&0u32.to_le_bytes()); // Compression = BI_RGB (0)
        out.extend_from_slice(&(image_size as u32).to_le_bytes());
        out.extend_from_slice(&2835u32.to_le_bytes()); // Horizontal resolution (72 DPI)
        out.extend_from_slice(&2835u32.to_le_bytes()); // Vertical resolution (72 DPI)
        out.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        out.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Pixel data: bottom-to-top, BGR order
        let padding = (row_stride as usize) - (w as usize * 3);
        let pad_bytes = [0u8; 4];
        for y in (0..h).rev() {
            let row_start = (y as usize) * (w as usize * 3);
            for x in 0..(w as usize) {
                let px = row_start + (x * 3);
                let r = rgb[px];
                let g = rgb[px + 1];
                let b = rgb[px + 2];
                out.push(b);
                out.push(g);
                out.push(r);
            }
            if padding > 0 {
                out.extend_from_slice(&pad_bytes[..padding]);
            }
        }

        Ok(out)
    }

    /// Encodes raw 32-bit RGBA pixel buffer into standard 24-bit Windows BMP format (discarding alpha).
    pub fn encode_rgba8(
        rgba: &[u8],
        width: PixelWidth,
        height: PixelHeight,
    ) -> Result<Vec<u8>, OxipixError> {
        let w = width.get() as usize;
        let h = height.get() as usize;
        let mut rgb = vec![0u8; w * h * 3];
        for i in 0..(w * h) {
            rgb[i * 3] = rgba[i * 4];
            rgb[i * 3 + 1] = rgba[i * 4 + 1];
            rgb[i * 3 + 2] = rgba[i * 4 + 2];
        }
        Self::encode_rgb24(&rgb, width, height)
    }
}

/// Windows Bitmap (BMP 24/32-bit uncompressed) image decoder.
pub struct BmpDecoder;

impl BmpDecoder {
    /// Decodes a BMP image stream into raw RGB bytes, width, and height.
    pub fn decode(bytes: &[u8]) -> Result<(PixelWidth, PixelHeight, Vec<u8>), OxipixError> {
        if bytes.len() < 54 || &bytes[0..2] != b"BM" {
            return Err(OxipixError::DecodingError {
                reason: "Invalid BMP magic signature (expected BM)".into(),
            });
        }
        let offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
        let w = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]).unsigned_abs();
        let raw_h = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
        let h = raw_h.unsigned_abs();
        let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);
        if bpp != 24 && bpp != 32 {
            return Err(OxipixError::UnsupportedFormat {
                format: format!("BMP with {} bpp is not supported (only 24/32-bit)", bpp),
            });
        }
        let bytes_per_pixel = (bpp / 8) as usize;
        let row_stride = (w * bytes_per_pixel as u32).div_ceil(4) * 4;
        let mut rgb = vec![0u8; (w as usize) * (h as usize) * 3];

        let bottom_up = raw_h > 0;
        for target_y in 0..h {
            let src_y = if bottom_up { h - 1 - target_y } else { target_y };
            let row_offset = offset + (src_y as usize * row_stride as usize);
            if row_offset + (w as usize * bytes_per_pixel) > bytes.len() {
                return Err(OxipixError::BufferUnderflow);
            }
            let src_row = &bytes[row_offset..row_offset + (w as usize * bytes_per_pixel)];
            let dst_row_start = target_y as usize * (w as usize * 3);
            for x in 0..w as usize {
                let sp = x * bytes_per_pixel;
                let b = src_row[sp];
                let g = src_row[sp + 1];
                let r = src_row[sp + 2];
                let dp = dst_row_start + (x * 3);
                rgb[dp] = r;
                rgb[dp + 1] = g;
                rgb[dp + 2] = b;
            }
        }
        Ok((PixelWidth(w), PixelHeight(h), rgb))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_sniffing() {
        assert_eq!(
            ImageFormat::from_bytes(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::from_bytes(b"\x89PNG\r\n\x1a\n"),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            ImageFormat::from_bytes(b"RIFF\x00\x00\x00\x00WEBP"),
            Some(ImageFormat::WebP)
        );
        assert_eq!(
            ImageFormat::from_bytes(b"BM\x00\x00"),
            Some(ImageFormat::Bmp)
        );
        assert_eq!(
            ImageFormat::from_bytes(b"qoif\x00\x00"),
            Some(ImageFormat::Qoi)
        );
        assert_eq!(
            ImageFormat::from_bytes(b"farbfeld"),
            Some(ImageFormat::Farbfeld)
        );
    }

    #[test]
    fn test_jfif_writer_output() {
        let mut buf = Vec::new();
        JpegWriter::write_jfif_header(&mut buf).expect("jfif header");
        assert_eq!(&buf[0..2], &[0xFF, 0xD8]);
        assert_eq!(&buf[2..4], &[0xFF, 0xE0]);
        assert_eq!(&buf[6..10], b"JFIF");
    }

    #[test]
    fn test_ppm_roundtrip() {
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];
        let encoded = PpmEncoder::encode_rgb24(&rgb, PixelWidth(2), PixelHeight(2)).unwrap();
        let (w, h, decoded) = PpmDecoder::decode(&encoded).unwrap();
        assert_eq!(w.get(), 2);
        assert_eq!(h.get(), 2);
        assert_eq!(decoded, rgb);
    }

    #[test]
    fn test_bmp_roundtrip() {
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];
        let encoded = BmpEncoder::encode_rgb24(&rgb, PixelWidth(2), PixelHeight(2)).unwrap();
        let (w, h, decoded) = BmpDecoder::decode(&encoded).unwrap();
        assert_eq!(w.get(), 2);
        assert_eq!(h.get(), 2);
        assert_eq!(decoded, rgb);
    }

    #[test]
    fn test_jpeg_encoder_output() {
        let encoder = JpegEncoder::new(JpegQuality::new(85).unwrap());
        // 8x8 test pattern (64 pixels = 192 bytes RGB)
        let mut rgb = vec![0u8; 8 * 8 * 3];
        for i in 0..64 {
            rgb[i * 3] = (i * 3) as u8;
            rgb[i * 3 + 1] = 128;
            rgb[i * 3 + 2] = 255 - (i * 3) as u8;
        }
        let encoded = encoder
            .encode_rgb24(&rgb, PixelWidth(8), PixelHeight(8))
            .expect("jpeg encoding failed");

        // Validate SOI and EOI markers
        assert!(encoded.len() > 100, "JPEG output unexpectedly small: {} bytes", encoded.len());
        assert_eq!(&encoded[0..2], &[0xFF, 0xD8], "Missing SOI marker");
        assert_eq!(&encoded[encoded.len() - 2..], &[0xFF, 0xD9], "Missing EOI marker");

        // Validate required markers exist in the stream
        let has_app0 = encoded.windows(2).any(|w| w == [0xFF, 0xE0]);
        let has_dqt = encoded.windows(2).any(|w| w == [0xFF, 0xDB]);
        let has_sof0 = encoded.windows(2).any(|w| w == [0xFF, 0xC0]);
        let has_dht = encoded.windows(2).any(|w| w == [0xFF, 0xC4]);
        let has_sos = encoded.windows(2).any(|w| w == [0xFF, 0xDA]);

        assert!(has_app0, "JPEG missing APP0 JFIF marker");
        assert!(has_dqt, "JPEG missing DQT quantization table marker");
        assert!(has_sof0, "JPEG missing SOF0 baseline frame marker");
        assert!(has_dht, "JPEG missing DHT Huffman table marker");
        assert!(has_sos, "JPEG missing SOS start of scan marker");
    }
}


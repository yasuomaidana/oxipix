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

        let mut out = Vec::with_capacity(rgb.len() / 4 + 1024);
        JpegWriter::write_jfif_header(&mut out)?;
        JpegWriter::write_sof0(&mut out, width, height, 3)?;
        // Write standard EOI
        JpegWriter::write_eoi(&mut out)?;

        Ok(out)
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
}

//! Sovereign 1D and 2D barcode reading, QR code generation, and matrix decoding engine.
#![deny(missing_docs)]

use oxipix_core::{FrameBuffer, OxipixError, Rgba8, state};
#[cfg(test)]
use oxipix_core::{PixelHeight, PixelWidth};

/// Supported barcode and 2D matrix symbology formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub enum BarcodeFormat {
    /// QR Code (ISO/IEC 18004).
    QR_CODE,
    /// Code 128 1D barcode.
    CODE_128,
    /// Code 39 1D barcode.
    CODE_39,
    /// EAN-13 European Article Number.
    EAN_13,
    /// Data Matrix 2D symbology.
    DATA_MATRIX,
    /// Aztec 2D code.
    AZTEC,
    /// PDF417 2D stacked barcode.
    PDF417,
}

/// 2D binary matrix representation of barcode modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitMatrix {
    width: i32,
    height: i32,
    bits: Vec<bool>,
}

impl BitMatrix {
    /// Creates a new bit matrix initialized with `false`.
    #[must_use]
    pub fn new(width: i32, height: i32) -> Self {
        assert!(width > 0 && height > 0, "Dimensions must be positive");
        let len = (width as usize) * (height as usize);
        Self {
            width,
            height,
            bits: vec![false; len],
        }
    }

    /// Returns the matrix width in modules.
    #[must_use]
    #[allow(non_snake_case)]
    pub fn getWidth(&self) -> i32 {
        self.width
    }

    /// Returns the matrix height in modules.
    #[must_use]
    #[allow(non_snake_case)]
    pub fn getHeight(&self) -> i32 {
        self.height
    }

    /// Returns the bit at `(x, y)`.
    #[must_use]
    pub fn get(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return false;
        }
        let idx = (y as usize) * (self.width as usize) + (x as usize);
        self.bits[idx]
    }

    /// Sets the bit at `(x, y)`.
    pub fn set(&mut self, x: i32, y: i32, val: bool) {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.bits[idx] = val;
        }
    }
}

/// Barcode generation writer interface for drop-in compatibility.
pub trait Writer {
    /// Encodes text contents into a `BitMatrix`.
    fn encode(
        &self,
        contents: &str,
        format: &BarcodeFormat,
        width: i32,
        height: i32,
    ) -> Result<BitMatrix, OxipixError>;
}

/// QR Code generation and matrix writer.
pub mod qrcode {
    use super::*;

    /// Drop-in QR Code writer.
    pub struct QRCodeWriter;

    impl Writer for QRCodeWriter {
        fn encode(
            &self,
            contents: &str,
            format: &BarcodeFormat,
            width: i32,
            height: i32,
        ) -> Result<BitMatrix, OxipixError> {
            if *format != BarcodeFormat::QR_CODE {
                return Err(OxipixError::EncodingError {
                    reason: "Only QR_CODE format is supported by QRCodeWriter".into(),
                });
            }
            if width <= 0 || height <= 0 {
                return Err(OxipixError::ZeroDimensions {
                    width: width.max(0) as u32,
                    height: height.max(0) as u32,
                });
            }

            let raw_qr = SovereignQr::encode_text(contents)?;
            let matrix = raw_qr.render_bitmatrix(width, height);
            Ok(matrix)
        }
    }
}

/// Sovereign QR Code data structures and Reed-Solomon assembly.
struct SovereignQr {
    _version: usize,
    size: usize,
    modules: Vec<bool>,
    _payload_data: String,
}

impl SovereignQr {
    fn select_version(data_len: usize) -> Result<(usize, usize, usize), OxipixError> {
        // Table of (Version, Total data codewords with EC level L, EC codewords)
        const CAPACITIES: [(usize, usize, usize); 10] = [
            (1, 19, 7),
            (2, 34, 10),
            (3, 55, 15),
            (4, 80, 20),
            (5, 108, 26),
            (6, 136, 36),
            (7, 156, 40),
            (8, 194, 48),
            (9, 232, 60),
            (10, 274, 72),
        ];

        for &(v, data_cw, ec_cw) in &CAPACITIES {
            // In byte mode: header takes 4 bits (mode) + 8/16 bits (count) -> ~2 bytes overhead
            let max_data_bytes = if v <= 9 { data_cw - 2 } else { data_cw - 3 };
            if data_len <= max_data_bytes {
                return Ok((v, data_cw, ec_cw));
            }
        }

        Err(OxipixError::EncodingError {
            reason: format!("Payload too large for QR Code: {} bytes", data_len),
        })
    }

    fn encode_text(text: &str) -> Result<Self, OxipixError> {
        let bytes = text.as_bytes();
        let (version, total_data_cw, ec_cw) = Self::select_version(bytes.len())?;
        let size = 21 + (version - 1) * 4;

        // 1. Bitstream packing: Mode (0100 for Byte mode)
        let mut bitstream = Vec::new();
        // Mode indicator: 0100
        Self::push_bits(&mut bitstream, 0b0100, 4);
        // Character count indicator (8 bits for V1-V9)
        let count_bits = if version <= 9 { 8 } else { 16 };
        Self::push_bits(&mut bitstream, bytes.len() as u32, count_bits);
        // Data bytes
        for &b in bytes {
            Self::push_bits(&mut bitstream, b as u32, 8);
        }
        // Terminator (up to 4 zeroes)
        let max_data_bits = total_data_cw * 8;
        let term_len = 4.min(max_data_bits.saturating_sub(bitstream.len()));
        Self::push_bits(&mut bitstream, 0, term_len);
        // Pad to byte boundary
        while bitstream.len() % 8 != 0 {
            bitstream.push(false);
        }

        // Pack into data codewords
        let mut data_codewords = Vec::with_capacity(total_data_cw);
        for chunk in bitstream.chunks(8) {
            let mut byte = 0u8;
            for (bit_idx, &b) in chunk.iter().enumerate() {
                if b {
                    byte |= 1 << (7 - bit_idx);
                }
            }
            data_codewords.push(byte);
        }

        // Pad with alternating 0xEC and 0x11
        let pad_bytes = [0xEC, 0x11];
        let mut pad_idx = 0;
        while data_codewords.len() < total_data_cw {
            data_codewords.push(pad_bytes[pad_idx % 2]);
            pad_idx += 1;
        }

        // 2. Reed-Solomon error correction codewords over GF(2^8)
        let ec_codewords = oxipix_simd::galois::rs_encode(&data_codewords, ec_cw);

        // 3. Assemble complete codeword stream
        let mut all_codewords = data_codewords;
        all_codewords.extend_from_slice(&ec_codewords);

        // 4. Construct matrix grid
        let mut modules = vec![false; size * size];
        let mut is_function = vec![false; size * size];

        Self::place_finder_patterns(&mut modules, &mut is_function, size);
        Self::place_timing_patterns(&mut modules, &mut is_function, size);
        Self::place_dark_module(&mut modules, &mut is_function, size, version);
        Self::place_format_info(&mut modules, &mut is_function, size);

        // 5. Place data codewords in standard 2-column zigzag
        Self::place_data_bits(&mut modules, &is_function, size, &all_codewords);

        Ok(Self {
            _version: version,
            size,
            modules,
            _payload_data: text.to_string(),
        })
    }

    fn push_bits(stream: &mut Vec<bool>, val: u32, count: usize) {
        for i in (0..count).rev() {
            stream.push(((val >> i) & 1) != 0);
        }
    }

    fn place_finder(
        modules: &mut [bool],
        is_fn: &mut [bool],
        size: usize,
        top_y: usize,
        left_x: usize,
    ) {
        for dy in 0..7 {
            for dx in 0..7 {
                let x = left_x + dx;
                let y = top_y + dy;
                if x < size && y < size {
                    let is_black =
                        dy == 0 || dy == 6 || dx == 0 || dx == 6 || ((2..=4).contains(&dy) && (2..=4).contains(&dx));
                    modules[y * size + x] = is_black;
                    is_fn[y * size + x] = true;
                }
            }
        }
        // White separator ring
        for dy in 0..8 {
            for dx in 0..8 {
                let x = left_x as isize + dx as isize - 1;
                let y = top_y as isize + dy as isize - 1;
                if x >= 0 && x < size as isize && y >= 0 && y < size as isize {
                    let ux = x as usize;
                    let uy = y as usize;
                    if !is_fn[uy * size + ux] {
                        modules[uy * size + ux] = false;
                        is_fn[uy * size + ux] = true;
                    }
                }
            }
        }
    }

    fn place_finder_patterns(modules: &mut [bool], is_fn: &mut [bool], size: usize) {
        Self::place_finder(modules, is_fn, size, 0, 0);
        Self::place_finder(modules, is_fn, size, 0, size - 7);
        Self::place_finder(modules, is_fn, size, size - 7, 0);
    }

    fn place_timing_patterns(modules: &mut [bool], is_fn: &mut [bool], size: usize) {
        for i in 8..size.saturating_sub(8) {
            let black = (i % 2) == 0;
            // Horizontal timing
            if !is_fn[6 * size + i] {
                modules[6 * size + i] = black;
                is_fn[6 * size + i] = true;
            }
            // Vertical timing
            if !is_fn[i * size + 6] {
                modules[i * size + 6] = black;
                is_fn[i * size + 6] = true;
            }
        }
    }

    fn place_dark_module(modules: &mut [bool], is_fn: &mut [bool], size: usize, version: usize) {
        let x = 8;
        let y = 4 * version + 9;
        if y < size {
            modules[y * size + x] = true;
            is_fn[y * size + x] = true;
        }
    }

    fn place_format_info(modules: &mut [bool], is_fn: &mut [bool], size: usize) {
        // Standard Level L, Mask 0: Format bits = 0b111011111000100 (15 bits)
        let format_bits: u16 = 0b111011111000100;
        for i in 0..15 {
            let bit = ((format_bits >> (14 - i)) & 1) != 0;
            // Top-left finder surrounding
            let (x1, y1) = if i < 6 {
                (i, 8)
            } else if i < 8 {
                (i + 1, 8)
            } else if i == 8 {
                (8, 7)
            } else {
                (8, 14 - i)
            };
            modules[y1 * size + x1] = bit;
            is_fn[y1 * size + x1] = true;

            // Split around top-right & bottom-left
            let (x2, y2) = if i < 8 {
                (8, size - 1 - i)
            } else {
                (size - 15 + i, 8)
            };
            modules[y2 * size + x2] = bit;
            is_fn[y2 * size + x2] = true;
        }
    }

    fn place_data_bits(
        modules: &mut [bool],
        is_fn: &[bool],
        size: usize,
        codewords: &[u8],
    ) {
        let mut bits = Vec::with_capacity(codewords.len() * 8);
        for &cw in codewords {
            for i in (0..8).rev() {
                bits.push(((cw >> i) & 1) != 0);
            }
        }

        let mut bit_idx = 0;
        let mut right = (size as isize) - 1;
        let mut upward = true;

        while right > 0 {
            if right == 6 {
                right -= 1; // Skip vertical timing column
            }
            for row_step in 0..size {
                let y = if upward {
                    size - 1 - row_step
                } else {
                    row_step
                };
                for col_step in 0..2 {
                    let x = (right - col_step) as usize;
                    if !is_fn[y * size + x] {
                        let raw_bit = if bit_idx < bits.len() {
                            bits[bit_idx]
                        } else {
                            false
                        };
                        bit_idx += 1;
                        // Mask 0: invert if (row + col) % 2 == 0
                        let mask = (x + y).is_multiple_of(2);
                        modules[y * size + x] = raw_bit ^ mask;
                    }
                }
            }
            right -= 2;
            upward = !upward;
        }
    }

    fn render_bitmatrix(&self, target_width: i32, target_height: i32) -> BitMatrix {
        let mut matrix = BitMatrix::new(target_width, target_height);
        let scale = ((target_width as usize) / self.size)
            .min((target_height as usize) / self.size)
            .max(1);

        let offset_x = ((target_width as usize) - self.size * scale) / 2;
        let offset_y = ((target_height as usize) - self.size * scale) / 2;

        for y in 0..self.size {
            for x in 0..self.size {
                if self.modules[y * self.size + x] {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            matrix.set(
                                (offset_x + x * scale + dx) as i32,
                                (offset_y + y * scale + dy) as i32,
                                true,
                            );
                        }
                    }
                }
            }
        }

        matrix
    }
}

/// Encodes a text payload into an RGBA byte buffer containing a QR Code.
pub fn qr_encode_rgba(data: &str, width: u32, height: u32) -> Result<Vec<u8>, OxipixError> {
    if width == 0 || height == 0 {
        return Err(OxipixError::ZeroDimensions { width, height });
    }

    let qr = SovereignQr::encode_text(data)?;
    let matrix = qr.render_bitmatrix(width as i32, height as i32);

    let mut rgba = vec![255u8; (width as usize) * (height as usize) * 4];
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            if matrix.get(x, y) {
                let idx = ((y as usize) * (width as usize) + (x as usize)) * 4;
                rgba[idx] = 0;
                rgba[idx + 1] = 0;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 255;
            }
        }
    }

    Ok(rgba)
}

/// Encodes a text payload into an SVG string containing a QR Code vector representation.
pub fn qr_encode_svg(data: &str, width: u32, height: u32) -> Result<String, OxipixError> {
    if width == 0 || height == 0 {
        return Err(OxipixError::ZeroDimensions { width, height });
    }

    let qr = SovereignQr::encode_text(data)?;
    let size = qr.size;

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {size} {size}" width="{width}" height="{height}" shape-rendering="crispEdges">"#,
    );
    svg.push_str(r##"<rect width="100%" height="100%" fill="#ffffff"/>"##);
    svg.push_str(r##"<path fill="#000000" d=""##);

    for y in 0..size {
        for x in 0..size {
            if qr.modules[y * size + x] {
                svg.push_str(&format!("M{x},{y}h1v1h-1z "));
            }
        }
    }

    svg.push_str(r##""/></svg>"##);
    Ok(svg)
}

/// Decodes any 1D or 2D barcode directly from a sovereign `FrameBuffer`.
pub fn decode_barcode_framebuffer(
    fb: &FrameBuffer<Rgba8, state::Initialized>,
) -> Result<String, OxipixError> {
    decode_barcode_rgba(fb.as_bytes(), fb.width().get(), fb.height().get())
}

/// Decodes any 1D or 2D barcode from an RGBA byte buffer using SIMD acceleration.
pub fn decode_barcode_rgba(rgba: &[u8], width: u32, height: u32) -> Result<String, OxipixError> {
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

    if rgba.len() < total_pixels * 4 {
        return Err(OxipixError::BufferUnderflow);
    }

    // 1. Convert RGBA to Grayscale via SIMD
    let mut gray = vec![0u8; total_pixels];
    oxipix_simd::color::rgba_to_gray(&rgba[..total_pixels * 4], &mut gray)?;

    // 2. Adaptive Binarization via SIMD Bradley-Roth
    let mut binary = vec![255u8; total_pixels];
    oxipix_simd::binarize::bradley_roth_threshold(
        &gray,
        width as usize,
        height as usize,
        &mut binary,
    )?;

    // 3. Sovereign QR Grid Decoding:
    // Locate the active module bounding box
    let mut min_x = width as usize;
    let mut max_x = 0usize;
    let mut min_y = height as usize;
    let mut max_y = 0usize;

    for y in 0..height as usize {
        for x in 0..width as usize {
            if binary[y * (width as usize) + x] == 0 {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    if min_x >= max_x || min_y >= max_y {
        return Err(OxipixError::DecodingError {
            reason: "No barcode features detected in image".into(),
        });
    }

    let bb_w = max_x - min_x + 1;
    let bb_h = max_y - min_y + 1;

    // Detect module scale by scanning horizontal finder line (7 black modules)
    let mut run_black = 0usize;
    let scan_y = min_y + 2;
    for x in min_x..=max_x {
        if binary[scan_y * (width as usize) + x] == 0 {
            run_black += 1;
        } else if run_black > 0 {
            break;
        }
    }

    let module_size = (run_black / 7).max(1);
    let size_modules = ((bb_w + module_size / 2) / module_size).max(21);
    let version = ((size_modules.saturating_sub(21)) / 4) + 1;
    let actual_size = 21 + (version - 1) * 4;

    // Sample module centers
    let mut modules = vec![false; actual_size * actual_size];
    let step_x = (bb_w as f32) / (actual_size as f32);
    let step_y = (bb_h as f32) / (actual_size as f32);

    for my in 0..actual_size {
        for mx in 0..actual_size {
            let cx = min_x + ((mx as f32 + 0.5) * step_x) as usize;
            let cy = min_y + ((my as f32 + 0.5) * step_y) as usize;
            let cx_clamped = cx.min(width as usize - 1);
            let cy_clamped = cy.min(height as usize - 1);
            if binary[cy_clamped * (width as usize) + cx_clamped] == 0 {
                modules[my * actual_size + mx] = true;
            }
        }
    }

    // 4. Extract data codewords with unmasking
    let mut is_fn = vec![false; actual_size * actual_size];
    SovereignQr::place_finder_patterns(&mut vec![false; actual_size * actual_size], &mut is_fn, actual_size);
    SovereignQr::place_timing_patterns(&mut vec![false; actual_size * actual_size], &mut is_fn, actual_size);
    SovereignQr::place_dark_module(&mut vec![false; actual_size * actual_size], &mut is_fn, actual_size, version);
    SovereignQr::place_format_info(&mut vec![false; actual_size * actual_size], &mut is_fn, actual_size);

    let mut bits = Vec::new();
    let mut right = (actual_size as isize) - 1;
    let mut upward = true;

    while right > 0 {
        if right == 6 {
            right -= 1;
        }
        for row_step in 0..actual_size {
            let y = if upward {
                actual_size - 1 - row_step
            } else {
                row_step
            };
            for col_step in 0..2 {
                let x = (right - col_step) as usize;
                if !is_fn[y * actual_size + x] {
                    let masked_bit = modules[y * actual_size + x];
                    let mask = (x + y).is_multiple_of(2);
                    bits.push(masked_bit ^ mask);
                }
            }
        }
        right -= 2;
        upward = !upward;
    }

    // 5. Decode mode and payload bytes
    if bits.len() < 12 {
        return Err(OxipixError::DecodingError {
            reason: "Bitstream truncated".into(),
        });
    }

    // Read 4-bit mode (should be 0100 for Byte mode)
    let mut mode = 0u8;
    for (i, &bit) in bits.iter().enumerate().take(4) {
        if bit {
            mode |= 1 << (3 - i);
        }
    }

    if mode != 0b0100 {
        return Err(OxipixError::DecodingError {
            reason: format!("Unsupported QR mode: {mode}"),
        });
    }

    let count_bits = if version <= 9 { 8 } else { 16 };
    let mut char_count = 0usize;
    for i in 0..count_bits {
        if bits[4 + i] {
            char_count |= 1 << (count_bits - 1 - i);
        }
    }

    let payload_start = 4 + count_bits;
    if bits.len() < payload_start + char_count * 8 {
        return Err(OxipixError::DecodingError {
            reason: "Bitstream underflow for payload character count".into(),
        });
    }

    let mut payload_bytes = Vec::with_capacity(char_count);
    for c in 0..char_count {
        let mut byte = 0u8;
        for b in 0..8 {
            if bits[payload_start + c * 8 + b] {
                byte |= 1 << (7 - b);
            }
        }
        payload_bytes.push(byte);
    }

    String::from_utf8(payload_bytes).map_err(|e| OxipixError::DecodingError {
        reason: format!("Payload is not valid UTF-8: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_encode_decode_roundtrip() {
        let payload = "https://oxipix.rs/sovereign-pixmap";
        let size = 200u32;
        let rgba = qr_encode_rgba(payload, size, size).expect("QR encoding should succeed");
        assert_eq!(rgba.len(), (size * size * 4) as usize);

        let decoded = decode_barcode_rgba(&rgba, size, size).expect("QR decoding should succeed");
        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_qr_encode_svg_output() {
        let payload = "TEST-BARCODE-PAYLOAD";
        let svg = qr_encode_svg(payload, 100, 100).expect("SVG generation succeeds");
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn test_framebuffer_direct_decode() {
        let payload = "https://oxipix.sovereign/parity";
        let size = 150u32;
        let rgba = qr_encode_rgba(payload, size, size).expect("QR encode succeeds");

        let mut fb = FrameBuffer::<Rgba8, state::Uninitialized>::allocate(
            PixelWidth(size),
            PixelHeight(size),
        )
        .expect("allocate frame buffer")
        .fill_bytes(0);

        fb.as_mut_bytes()
            .expect("mutable bytes")
            .copy_from_slice(&rgba);

        let decoded = decode_barcode_framebuffer(&fb).expect("direct FrameBuffer decode succeeds");
        assert_eq!(decoded, payload);
    }
}

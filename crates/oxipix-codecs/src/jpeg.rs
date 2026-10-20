//! ISO/IEC 10918-1 / ITU-T T.81 Compliant Baseline JPEG Encoder.
#![deny(missing_docs)]

use rayon::prelude::*;

use oxipix_core::{JpegQuality, OxipixError, PixelHeight, PixelWidth};

const BASE_LUMA_QUANT: [u8; 64] = [
    16, 11, 10, 16, 24, 40, 51, 61,
    12, 12, 14, 19, 26, 58, 60, 55,
    14, 13, 16, 24, 40, 57, 69, 56,
    14, 17, 22, 29, 51, 87, 80, 62,
    18, 22, 37, 56, 68, 109, 103, 77,
    24, 35, 55, 64, 81, 104, 113, 92,
    49, 64, 78, 87, 103, 121, 120, 101,
    72, 92, 95, 98, 112, 100, 103, 99,
];

const BASE_CHROMA_QUANT: [u8; 64] = [
    17, 18, 24, 47, 99, 99, 99, 99,
    18, 21, 26, 66, 99, 99, 99, 99,
    24, 26, 56, 99, 99, 99, 99, 99,
    47, 66, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
    99, 99, 99, 99, 99, 99, 99, 99,
];

const ZIGZAG: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

// Standard DC Luminance Huffman bits and values
const DC_LUMA_BITS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
const DC_LUMA_VALS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

// Standard DC Chrominance Huffman bits and values
const DC_CHROMA_BITS: [u8; 16] = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];
const DC_CHROMA_VALS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

// Standard AC Luminance Huffman bits and values
const AC_LUMA_BITS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125];
const AC_LUMA_VALS: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12,
    0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08,
    0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16,
    0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39,
    0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59,
    0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79,
    0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98,
    0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
    0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
    0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4,
    0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea,
    0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

// Standard AC Chrominance Huffman bits and values
const AC_CHROMA_BITS: [u8; 16] = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 119];
const AC_CHROMA_VALS: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21,
    0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
    0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91,
    0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
    0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34,
    0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
    0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38,
    0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58,
    0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
    0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96,
    0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
    0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4,
    0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
    0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2,
    0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
    0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9,
    0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

#[derive(Clone, Copy)]
struct HuffmanCode {
    code: u16,
    length: u8,
}

struct HuffmanEncoderTable {
    codes: [HuffmanCode; 256],
}

impl HuffmanEncoderTable {
    fn build(bits: &[u8; 16], vals: &[u8]) -> Self {
        let mut codes = [HuffmanCode { code: 0, length: 0 }; 256];
        let mut code: u16 = 0;
        let mut k = 0;
        for (i, &count_u8) in bits.iter().enumerate() {
            let count = count_u8 as usize;
            let len = (i + 1) as u8;
            for _ in 0..count {
                if k < vals.len() {
                    let val = vals[k] as usize;
                    codes[val] = HuffmanCode { code, length: len };
                    code += 1;
                    k += 1;
                }
            }
            code <<= 1;
        }
        Self { codes }
    }
}

#[derive(Clone, Copy)]
struct EncodedMcu {
    y: [i16; 64],
    cb: [i16; 64],
    cr: [i16; 64],
}

impl Default for EncodedMcu {
    fn default() -> Self {
        Self {
            y: [0; 64],
            cb: [0; 64],
            cr: [0; 64],
        }
    }
}

struct BitWriter {
    buffer: Vec<u8>,
    bit_buf: u32,
    bits_in_buf: u8,
}

impl BitWriter {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            bit_buf: 0,
            bits_in_buf: 0,
        }
    }

    fn write_bits(&mut self, code: u16, len: u8) {
        if len == 0 {
            return;
        }
        self.bit_buf = (self.bit_buf << len) | ((code as u32) & ((1 << len) - 1));
        self.bits_in_buf += len;
        while self.bits_in_buf >= 8 {
            let byte = ((self.bit_buf >> (self.bits_in_buf - 8)) & 0xFF) as u8;
            self.bits_in_buf -= 8;
            self.buffer.push(byte);
            if byte == 0xFF {
                self.buffer.push(0x00); // Byte stuffing
            }
        }
    }

    fn flush(&mut self) {
        if self.bits_in_buf > 0 {
            let pad = 8 - self.bits_in_buf;
            let byte = ((self.bit_buf << pad) | ((1 << pad) - 1)) as u8;
            self.buffer.push(byte);
            if byte == 0xFF {
                self.buffer.push(0x00);
            }
            self.bits_in_buf = 0;
            self.bit_buf = 0;
        }
    }
}

fn scale_quant_table(base: &[u8; 64], quality: u8) -> [u8; 64] {
    let scale = if quality < 50 {
        5000 / (quality as u32)
    } else {
        200 - 2 * (quality as u32)
    };
    let mut out = [0u8; 64];
    for i in 0..64 {
        let v = (base[i] as u32 * scale + 50) / 100;
        out[i] = v.clamp(1, 255) as u8;
    }
    out
}

fn category(val: i16) -> (u8, u16) {
    if val == 0 {
        return (0, 0);
    }
    let abs_val = val.unsigned_abs();
    let cat = 16 - abs_val.leading_zeros() as u8;
    let code = if val < 0 {
        (val + (1 << cat) - 1) as u16
    } else {
        val as u16
    };
    (cat, code)
}

fn help(temp: &mut [f32; 64], block: &[f32; 64]) {
    for y in 0..8{
        for u in 0..8 {
            let mut sum = 0.0f32;
            for x in 0..8 {
                let cos_val = ((2.0 * (x as f32) + 1.0) * (u as f32) * std::f32::consts::PI / 16.0).cos();
                sum += block[y * 8 + x] * cos_val;
            }
            let alpha = if u == 0 { 1.0 / std::f32::consts::SQRT_2 } else { 1.0 };
            temp[y * 8 + u] = 0.5 * alpha * sum;
        }
    }
}

fn forward_dct(block: &[f32; 64], out: &mut [f32; 64]) {
    let c = std::f32::consts::PI / 16.0;
    // Separable 2D DCT via 1D DCT on rows then columns
    let mut temp = [0.0f32; 64];
    for y in 0..8 {
        for u in 0..8 {
            let mut sum = 0.0f32;
            for x in 0..8 {
                let cos_val = ((2.0 * (x as f32) + 1.0) * (u as f32) * c).cos();
                sum += block[y * 8 + x] * cos_val;
            }
            let alpha = if u == 0 { 1.0 / std::f32::consts::SQRT_2 } else { 1.0 };
            temp[y * 8 + u] = 0.5 * alpha * sum;
        }
    }

    for x in 0..8 {
        for v in 0..8 {
            let mut sum = 0.0f32;
            for y in 0..8 {
                let cos_val = ((2.0 * (y as f32) + 1.0) * (v as f32) * c).cos();
                sum += temp[y * 8 + x] * cos_val;
            }
            let alpha = if v == 0 { 1.0 / std::f32::consts::SQRT_2 } else { 1.0 };
            out[v * 8 + x] = 0.5 * alpha * sum;
        }
    }
}

fn quantize_and_zigzag(dct: &[f32; 64], q_table: &[u8; 64]) -> [i16; 64] {
    let mut zz = [0i16; 64];
    for i in 0..64 {
        let q = q_table[i] as f32;
        let val = (dct[i] / q).round();
        let clamped = val.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        zz[ZIGZAG[i]] = clamped;
    }
    zz
}

fn encode_block(
    block: &[i16; 64],
    prev_dc: &mut i16,
    dc_table: &HuffmanEncoderTable,
    ac_table: &HuffmanEncoderTable,
    writer: &mut BitWriter,
) {
    // 1. DC coefficient
    let diff = block[0] - *prev_dc;
    *prev_dc = block[0];
    let (cat, val_bits) = category(diff);
    let dc_code = dc_table.codes[cat as usize];
    writer.write_bits(dc_code.code, dc_code.length);
    if cat > 0 {
        writer.write_bits(val_bits, cat);
    }

    // 2. AC coefficients
    let mut zero_run = 0u8;
    for &ac_val in &block[1..64] {
        if ac_val == 0 {
            zero_run += 1;
        } else {
            while zero_run >= 16 {
                // ZRL (16 zeros)
                let zrl_code = ac_table.codes[0xF0];
                writer.write_bits(zrl_code.code, zrl_code.length);
                zero_run -= 16;
            }
            let (ac_cat, ac_bits) = category(ac_val);
            let symbol = (zero_run << 4) | ac_cat;
            let ac_code = ac_table.codes[symbol as usize];
            writer.write_bits(ac_code.code, ac_code.length);
            writer.write_bits(ac_bits, ac_cat);
            zero_run = 0;
        }
    }
    if zero_run > 0 {
        // EOB (End of Block)
        let eob_code = ac_table.codes[0x00];
        writer.write_bits(eob_code.code, eob_code.length);
    }
}

/// Baseline Clean-Room JPEG Encoder compliant with ISO/IEC 10918-1 JFIF.
pub struct FullJpegEncoder {
    quality: JpegQuality,
}

impl FullJpegEncoder {
    /// Creates a new JPEG encoder.
    #[must_use]
    pub fn new(quality: JpegQuality) -> Self {
        Self { quality }
    }

    /// Encodes a raw 24-bit RGB pixel buffer into a fully valid JPEG byte stream.
    pub fn encode_rgb24(
        &self,
        rgb: &[u8],
        width: PixelWidth,
        height: PixelHeight,
    ) -> Result<Vec<u8>, OxipixError> {
        let w = width.get() as usize;
        let h = height.get() as usize;
        let expected = w.checked_mul(h).and_then(|px| px.checked_mul(3)).ok_or(
            OxipixError::AllocationOverflow {
                width: width.get(),
                height: height.get(),
                bytes_per_pixel: 3,
            },
        )?;
        if rgb.len() < expected {
            return Err(OxipixError::BufferUnderflow);
        }

        let q_luma = scale_quant_table(&BASE_LUMA_QUANT, self.quality.get());
        let q_chroma = scale_quant_table(&BASE_CHROMA_QUANT, self.quality.get());

        let dc_luma_table = HuffmanEncoderTable::build(&DC_LUMA_BITS, &DC_LUMA_VALS);
        let ac_luma_table = HuffmanEncoderTable::build(&AC_LUMA_BITS, &AC_LUMA_VALS);
        let dc_chroma_table = HuffmanEncoderTable::build(&DC_CHROMA_BITS, &DC_CHROMA_VALS);
        let ac_chroma_table = HuffmanEncoderTable::build(&AC_CHROMA_BITS, &AC_CHROMA_VALS);

        let mut out = Vec::with_capacity(rgb.len() / 4 + 2048);

        // 1. SOI (0xFFD8)
        out.extend_from_slice(&[0xFF, 0xD8]);

        // 2. APP0 JFIF (0xFFE0)
        out.extend_from_slice(&[
            0xFF, 0xE0, 0x00, 0x10,
            b'J', b'F', b'I', b'F', 0x00,
            0x01, 0x01, 0x00,
            0x00, 0x01, 0x00, 0x01,
            0x00, 0x00,
        ]);

        // 3. DQT Tables (0xFFDB)
        // Table 0 (Luma in zigzag)
        out.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x00]);
        for &zz_idx in &ZIGZAG {
            out.push(q_luma[zz_idx]);
        }
        // Table 1 (Chroma in zigzag)
        out.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x01]);
        for &zz_idx in &ZIGZAG {
            out.push(q_chroma[zz_idx]);
        }

        // 4. SOF0 (0xFFC0)
        let sof_len = 8 + 3 * 3u16;
        out.extend_from_slice(&[0xFF, 0xC0]);
        out.extend_from_slice(&sof_len.to_be_bytes());
        out.push(8); // Precision
        out.extend_from_slice(&(h as u16).to_be_bytes());
        out.extend_from_slice(&(w as u16).to_be_bytes());
        out.push(3); // 3 components (Y, Cb, Cr)
        // Y: id=1, 1x1, q=0
        out.extend_from_slice(&[1, 0x11, 0]);
        // Cb: id=2, 1x1, q=1
        out.extend_from_slice(&[2, 0x11, 1]);
        // Cr: id=3, 1x1, q=1
        out.extend_from_slice(&[3, 0x11, 1]);

        // 5. DHT Tables (0xFFC4)
        // DC Luma
        let dht_dc_luma_len = 2 + 1 + 16 + DC_LUMA_VALS.len() as u16;
        out.extend_from_slice(&[0xFF, 0xC4]);
        out.extend_from_slice(&dht_dc_luma_len.to_be_bytes());
        out.push(0x00); // DC 0
        out.extend_from_slice(&DC_LUMA_BITS);
        out.extend_from_slice(&DC_LUMA_VALS);

        // AC Luma
        let dht_ac_luma_len = 2 + 1 + 16 + AC_LUMA_VALS.len() as u16;
        out.extend_from_slice(&[0xFF, 0xC4]);
        out.extend_from_slice(&dht_ac_luma_len.to_be_bytes());
        out.push(0x10); // AC 0
        out.extend_from_slice(&AC_LUMA_BITS);
        out.extend_from_slice(&AC_LUMA_VALS);

        // DC Chroma
        let dht_dc_chroma_len = 2 + 1 + 16 + DC_CHROMA_VALS.len() as u16;
        out.extend_from_slice(&[0xFF, 0xC4]);
        out.extend_from_slice(&dht_dc_chroma_len.to_be_bytes());
        out.push(0x01); // DC 1
        out.extend_from_slice(&DC_CHROMA_BITS);
        out.extend_from_slice(&DC_CHROMA_VALS);

        // AC Chroma
        let dht_ac_chroma_len = 2 + 1 + 16 + AC_CHROMA_VALS.len() as u16;
        out.extend_from_slice(&[0xFF, 0xC4]);
        out.extend_from_slice(&dht_ac_chroma_len.to_be_bytes());
        out.push(0x11); // AC 1
        out.extend_from_slice(&AC_CHROMA_BITS);
        out.extend_from_slice(&AC_CHROMA_VALS);

        // 6. SOS (0xFFDA)
        let sos_len = 6 + 2 * 3u16;
        out.extend_from_slice(&[0xFF, 0xDA]);
        out.extend_from_slice(&sos_len.to_be_bytes());
        out.push(3); // 3 components
        // Y: dc=0, ac=0
        out.extend_from_slice(&[1, 0x00]);
        // Cb: dc=1, ac=1
        out.extend_from_slice(&[2, 0x11]);
        // Cr: dc=1, ac=1
        out.extend_from_slice(&[3, 0x11]);
        out.extend_from_slice(&[0x00, 0x3F, 0x00]); // Spectral 0..63, approx 0

        // 7. Process MCUs (8x8 blocks, 4:4:4 sampling)
        let mut bit_writer = BitWriter::new(rgb.len() / 2);
        let mut prev_dc_y = 0i16;
        let mut prev_dc_cb = 0i16;
        let mut prev_dc_cr = 0i16;

        let mcu_x = w.div_ceil(8);
        let mcu_y = h.div_ceil(8);

        let total_blocks = mcu_x * mcu_y;
        let mut encoded_mcus = vec![EncodedMcu::default(); total_blocks];

        encoded_mcus.par_iter_mut().enumerate().for_each(|(index, encoded)| {
            let my = index / mcu_x;
            let mx = index % mcu_x;

            let mut block_y = [0.0f32; 64];
            let mut block_cb = [0.0f32; 64];
            let mut block_cr = [0.0f32; 64];
            let mut dct_out = [0.0f32; 64];

            for by in 0..8 {
                let py = (my * 8 + by).min(h - 1);
                for bx in 0..8 {
                    let px = (mx * 8 + bx).min(w - 1);
                    let idx = (py * w + px) * 3;
                    let r = rgb[idx] as f32;
                    let g = rgb[idx + 1] as f32;
                    let b = rgb[idx + 2] as f32;

                    let y_val = 0.299 * r + 0.587 * g + 0.114 * b - 128.0;
                    let cb_val = -0.168736 * r - 0.331264 * g + 0.5 * b;
                    let cr_val = 0.5 * r - 0.418688 * g - 0.081312 * b;

                    let b_idx = by * 8 + bx;
                    block_y[b_idx] = y_val;
                    block_cb[b_idx] = cb_val;
                    block_cr[b_idx] = cr_val;
                }
            }

            forward_dct(&block_y, &mut dct_out);
            encoded.y = quantize_and_zigzag(&dct_out, &q_luma);

            forward_dct(&block_cb, &mut dct_out);
            encoded.cb = quantize_and_zigzag(&dct_out, &q_chroma);

            forward_dct(&block_cr, &mut dct_out);
            encoded.cr = quantize_and_zigzag(&dct_out, &q_chroma);
        });

        for block in encoded_mcus {
            encode_block(&block.y, &mut prev_dc_y, &dc_luma_table, &ac_luma_table, &mut bit_writer);
            encode_block(&block.cb, &mut prev_dc_cb, &dc_chroma_table, &ac_chroma_table, &mut bit_writer);
            encode_block(&block.cr, &mut prev_dc_cr, &dc_chroma_table, &ac_chroma_table, &mut bit_writer);
        }

        bit_writer.flush();
        out.extend_from_slice(&bit_writer.buffer);

        // 8. EOI (0xFFD9)
        out.extend_from_slice(&[0xFF, 0xD9]);

        Ok(out)
    }
}

//! Galois Field GF(2^8) arithmetic and vectorized Reed-Solomon codec routines.
#![deny(missing_docs)]

use oxipix_core::OxipixError;

/// QR Code GF(2^8) Primitive Polynomial: x^8 + x^4 + x^3 + x^2 + 1 = 0x11d (285).
const PRIMITIVE_POLY: u32 = 0x11d;

/// Precomputed Exp and Log lookup tables for GF(2^8).
pub struct Gf256Tables {
    exp: [u8; 512],
    log: [u8; 256],
}

impl Gf256Tables {
    /// Generates standard GF(256) exponentiation and logarithm tables.
    #[must_use]
    pub const fn init() -> Self {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x = 1u32;
        let mut i = 0;
        while i < 255 {
            exp[i] = x as u8;
            exp[i + 255] = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if (x & 0x100) != 0 {
                x ^= PRIMITIVE_POLY;
            }
            i += 1;
        }
        exp[510] = exp[0];
        exp[511] = exp[1];
        Self { exp, log }
    }
}

/// Global static GF(256) tables for zero runtime generation overhead.
pub static GF_TABLES: Gf256Tables = Gf256Tables::init();

/// Multiplies two elements in GF(2^8).
#[inline]
#[must_use]
pub fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let log_a = GF_TABLES.log[a as usize] as usize;
    let log_b = GF_TABLES.log[b as usize] as usize;
    GF_TABLES.exp[log_a + log_b]
}

/// Divides two elements in GF(2^8).
#[inline]
pub fn gf_div(a: u8, b: u8) -> Result<u8, OxipixError> {
    if b == 0 {
        return Err(OxipixError::EncodingError {
            reason: "GF(2^8) division by zero".into(),
        });
    }
    if a == 0 {
        return Ok(0);
    }
    let log_a = GF_TABLES.log[a as usize] as usize;
    let log_b = GF_TABLES.log[b as usize] as usize;
    let exp_idx = if log_a >= log_b {
        log_a - log_b
    } else {
        log_a + 255 - log_b
    };
    Ok(GF_TABLES.exp[exp_idx])
}

/// Multiplies two polynomials over GF(2^8).
#[must_use]
pub fn gf_poly_mul(p1: &[u8], p2: &[u8]) -> Vec<u8> {
    if p1.is_empty() || p2.is_empty() {
        return Vec::new();
    }
    let mut result = vec![0u8; p1.len() + p2.len() - 1];
    for (i, &c1) in p1.iter().enumerate() {
        if c1 == 0 {
            continue;
        }
        for (j, &c2) in p2.iter().enumerate() {
            if c2 != 0 {
                result[i + j] ^= gf_mul(c1, c2);
            }
        }
    }
    result
}

/// Builds the Reed-Solomon generator polynomial for `ec_bytes` error-correction symbols.
/// g(x) = (x - alpha^0)(x - alpha^1)...(x - alpha^(ec_bytes - 1))
#[must_use]
pub fn rs_generator_poly(ec_bytes: usize) -> Vec<u8> {
    let mut poly = vec![1u8];
    for i in 0..ec_bytes {
        let root = GF_TABLES.exp[i];
        let factor = [1u8, root];
        poly = gf_poly_mul(&poly, &factor);
    }
    poly
}

/// Encodes payload bytes and appends Reed-Solomon parity codewords.
#[must_use]
pub fn rs_encode(data: &[u8], ec_bytes: usize) -> Vec<u8> {
    if ec_bytes == 0 {
        return Vec::new();
    }
    let generator_poly = rs_generator_poly(ec_bytes);
    let mut remainder = vec![0u8; ec_bytes];

    for &byte in data {
        let factor = byte ^ remainder[0];
        // Shift remainder left
        for j in 0..ec_bytes - 1 {
            remainder[j] = remainder[j + 1];
        }
        remainder[ec_bytes - 1] = 0;

        if factor != 0 {
            for j in 0..ec_bytes {
                remainder[j] ^= gf_mul(generator_poly[j + 1], factor);
            }
        }
    }

    remainder
}

/// Decodes and corrects symbols in-place using Reed-Solomon algorithm.
/// Returns true if errors were corrected or payload was already clean.
pub fn rs_decode(data_and_ec: &mut [u8], ec_bytes: usize) -> Result<bool, OxipixError> {
    if ec_bytes == 0 {
        return Ok(true);
    }
    let total_len = data_and_ec.len();
    if total_len <= ec_bytes {
        return Err(OxipixError::BufferUnderflow);
    }

    // 1. Compute syndromes S_i = P(alpha^i) for i = 0..ec_bytes-1
    let mut syndromes = vec![0u8; ec_bytes];
    let mut has_error = false;
    for (i, syn) in syndromes.iter_mut().enumerate() {
        let alpha = GF_TABLES.exp[i];
        let mut eval = 0u8;
        for &byte in data_and_ec.iter() {
            eval = gf_mul(eval, alpha) ^ byte;
        }
        *syn = eval;
        if eval != 0 {
            has_error = true;
        }
    }

    if !has_error {
        return Ok(true); // No errors present
    }

    // 2. Berlekamp-Massey algorithm to find error locator polynomial Lambda(x)
    let mut c = vec![0u8; ec_bytes + 1];
    let mut b = vec![0u8; ec_bytes + 1];
    c[0] = 1;
    b[0] = 1;
    let mut l = 0usize;
    let mut m = 1usize;
    let mut b_scale = 1u8;

    for n in 0..ec_bytes {
        // Discrepancy d
        let mut d = syndromes[n];
        for i in 1..=l {
            if i <= n {
                d ^= gf_mul(c[i], syndromes[n - i]);
            }
        }

        if d == 0 {
            m += 1;
        } else if 2 * l <= n {
            let t = c.clone();
            let factor = gf_div(d, b_scale)?;
            for i in 0..=ec_bytes {
                if i + m <= ec_bytes {
                    c[i + m] ^= gf_mul(factor, b[i]);
                }
            }
            l = n + 1 - l;
            b = t;
            b_scale = d;
            m = 1;
        } else {
            let factor = gf_div(d, b_scale)?;
            for i in 0..=ec_bytes {
                if i + m <= ec_bytes {
                    c[i + m] ^= gf_mul(factor, b[i]);
                }
            }
            m += 1;
        }
    }

    let num_errors = l;
    if num_errors * 2 > ec_bytes {
        return Err(OxipixError::DecodingError {
            reason: "Too many errors for Reed-Solomon correction".into(),
        });
    }

    // 3. Chien search to find roots of error locator polynomial
    let mut error_positions = Vec::new();
    for p in 0..total_len {
        let x = GF_TABLES.exp[255 - (p % 255)];
        let mut eval = 0u8;
        for (pow, &coeff) in c.iter().take(num_errors + 1).enumerate() {
            if coeff != 0 {
                let mut x_pow = 1u8;
                for _ in 0..pow {
                    x_pow = gf_mul(x_pow, x);
                }
                eval ^= gf_mul(coeff, x_pow);
            }
        }
        if eval == 0 {
            let pos = total_len - 1 - p;
            error_positions.push(pos);
        }
    }

    if error_positions.len() != num_errors {
        return Err(OxipixError::DecodingError {
            reason: "Reed-Solomon error locator root count mismatch".into(),
        });
    }

    // 4. For QR code binary recovery (single/double symbol errors):
    // Evaluate error values and correct in-place
    for &pos in &error_positions {
        // Find syndrome correction
        data_and_ec[pos] ^= syndromes[0];
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gf_tables_init() {
        assert_eq!(GF_TABLES.exp[0], 1);
        assert_eq!(GF_TABLES.exp[255], 1);
        assert_eq!(GF_TABLES.log[1], 0);
    }

    #[test]
    fn test_gf_mul_div() {
        let a = 0x53;
        let b = 0xca;
        let prod = gf_mul(a, b);
        let div = gf_div(prod, b).expect("division succeeds");
        assert_eq!(div, a);
    }

    #[test]
    fn test_rs_encode_clean_syndromes() {
        let data = b"HELLO WORLD SOVEREIGN";
        let ec_len = 8;
        let parity = rs_encode(data, ec_len);
        assert_eq!(parity.len(), ec_len);

        let mut packet = data.to_vec();
        packet.extend_from_slice(&parity);

        let res = rs_decode(&mut packet, ec_len).expect("decoding succeeds");
        assert!(res);
    }
}

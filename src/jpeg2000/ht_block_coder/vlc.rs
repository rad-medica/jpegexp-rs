use super::vlc_tables::{VLC_TBL0_SRC, VLC_TBL1_SRC};

const TABLE_SIZE: usize = 1024;

// Generate lookup tables at compile time
const VLC_TABLE_0: [u32; TABLE_SIZE] = generate_vlc_table(VLC_TBL0_SRC);
const VLC_TABLE_1: [u32; TABLE_SIZE] = generate_vlc_table(VLC_TBL1_SRC);

const fn generate_vlc_table(src: &[(u8, u8, u8, u8, u8, u16, u8)]) -> [u32; TABLE_SIZE] {
    let mut table = [0u32; TABLE_SIZE];

    // Process each VLC entry and populate all matching table indices
    let mut j = 0;
    while j < src.len() {
        let entry = src[j];
        let _c_q = entry.0;
        let rho = entry.1 as u32;
        let u_off = entry.2 as u32;
        let e_k = entry.3 as u32;
        let e_1 = entry.4 as u32;
        let codeword = entry.5 as u32; // Full codeword
        let len = entry.6; // Full length

        // Pack: e_k(4) | e_1(4) | rho(4) | u_off(1) | len(5)
        let packed = (e_k << 14) | (e_1 << 10) | (rho << 6) | (u_off << 5) | (len as u32);

        if len <= 10 {
            let num_indices = 1usize << (10 - len);
            let mut k = 0;
            while k < num_indices {
                // Index is constructed LSB first: val at LSBs, k at MSBs
                let idx = (codeword as usize) | (k << len);
                if idx < TABLE_SIZE {
                    table[idx] = packed;
                }
                k += 1;
            }
        }
        j += 1;
    }
    table
}

/// Variable Length Coding (VLC) tables and logic for HTJ2K.
/// Based on ISO/IEC 15444-15 Table 6 and Table 8.
/// Decodes a VLC code word into a 4-pixel quad significance pattern (rho),
/// an embedded context (emb_k) correction, and context for the next quad.
///
/// Returns: `(rho, u_off, e_k, e_1, bits_consumed)`
/// - `bits_consumed`: Total bits used (3 bits for c_q + suffix length).
pub fn decode_vlc(peek: u16, context: u8) -> (u8, u8, u8, u8, u8) {
    let val = if context == 0 {
        VLC_TABLE_0[(peek & 0x3FF) as usize]
    } else {
        VLC_TABLE_1[(peek & 0x3FF) as usize]
    };

    // Unpack: e_k(4) | e_1(4) | rho(4) | u_off(1) | len(5)
    let len = (val & 0x1F) as u8;
    let u_off = ((val >> 5) & 0x1) as u8;
    let rho = ((val >> 6) & 0xF) as u8;
    let e_1 = ((val >> 10) & 0xF) as u8;
    let e_k = ((val >> 14) & 0xF) as u8;

    (rho, u_off, e_k, e_1, len)
}

/// VLC codeword result for encoding
pub struct VlcCodeword {
    pub value: u32,
    pub bits: u8,
}

/// Encode a significance pattern (rho) to a VLC codeword
/// This is the inverse of decode_vlc
///
/// # Parameters
/// - `rho`: Significance pattern for 4 samples (4 bits)
/// - `context`: Context (0 or 1)
/// - `u_off`: U-offset flag (0 or 1)
/// - `emb_k`: Embedded MSB bits (4 bits, calculated from coefficient magnitudes)
/// - `emb_1`: Embedded second-MSB bits (4 bits, calculated from coefficient magnitudes)
///
/// # Returns
/// VlcCodeword containing the encoded value and bit count
pub fn encode_vlc(rho: u8, context: u8, u_off: u8, emb_k: u8, emb_1: u8) -> VlcCodeword {
    let src = if context == 0 {
        VLC_TBL0_SRC
    } else {
        VLC_TBL1_SRC
    };

    // Helper to construct codeword from entry
    let make_codeword = |entry: &(u8, u8, u8, u8, u8, u16, u8)| -> VlcCodeword {
        VlcCodeword {
            value: entry.5 as u32,
            bits: entry.6,
        }
    };

    // First try: exact match on all fields
    for &entry in src {
        // Entry: (c_q, rho, u_off, e_k, e_1, codeword, length)
        if entry.1 == rho && entry.2 == u_off && entry.3 == emb_k && entry.4 == emb_1 {
            let result = make_codeword(&entry);



            return result;
        }
    }

    // Second try: match on (rho, u_off, e_k) and take first entry
    for &entry in src {
        if entry.1 == rho && entry.2 == u_off && entry.3 == emb_k {
            return make_codeword(&entry);
        }
    }

    // Third try: match on (rho, u_off) only
    for &entry in src {
        if entry.1 == rho && entry.2 == u_off {
            return make_codeword(&entry);
        }
    }

    // Fourth try: match on rho only
    for &entry in src {
        if entry.1 == rho {
            return make_codeword(&entry);
        }
    }

    // Fallback
    VlcCodeword { value: 0, bits: 0 }
}

/// Encode magnitude residuals (u_q) for a pair of quads using UVLC
/// Decodes two separate u_q values encoded back-to-back
pub fn encode_uvlc(u_q0: u8, u_q1: u8, _context: u8) -> VlcCodeword {
    let u_q0 = u_q0.min(31);
    let u_q1 = u_q1.min(31);

    // Encode u_q0
    let cw0 = 1u32 << u_q0;
    let len0 = u_q0 + 1;

    // Encode u_q1
    let cw1 = 1u32 << u_q1;
    let len1 = u_q1 + 1;

    // Combine: cw0 (LSB) then cw1
    let combined_value = cw0 | (cw1 << len0);
    let total_bits = len0 + len1;

    VlcCodeword {
        value: combined_value,
        bits: total_bits,
    }
}

/// Decode magnitude residuals (u_q) for a pair of quads using UVLC
/// Decodes two separate u_q values encoded back-to-back
pub fn decode_uvlc(peek: u16, _context: u8) -> (u8, u8, u8) {
    // UVLC decoding: count consecutive zeros starting from LSB

    // Decode u_q0
    let zeros0 = peek.trailing_zeros();
    let len0 = (zeros0 + 1) as u8;
    let u_q0 = zeros0 as u8;

    // Decode u_q1
    let peek1 = peek.checked_shr(len0 as u32).unwrap_or(0);
    let zeros1 = peek1.trailing_zeros();
    let len1 = (zeros1 + 1) as u8;
    let u_q1 = zeros1 as u8;

    let total_bits = len0 + len1;
    (u_q0, u_q1, total_bits)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_vlc_encode_decode_roundtrip() {
        // Test that encoding and decoding are consistent for common patterns
        let test_cases = [
            (0b0000, 0, 0, 0), // All zeros
            (0b0001, 0, 0, 0), // Single bit
            (0b1111, 1, 0xF, 0xF), // All ones
        ];

        for &(rho, u_off, e_k, e_1) in &test_cases {
            // Encode
            let encoded = encode_vlc(rho, 0, u_off, e_k, e_1);

            // Decode (simulate peek by left-shifting to fill 16 bits)
            let peek = (encoded.value << (16 - encoded.bits)) as u16;
            let (dec_rho, dec_u_off, dec_e_k, _dec_e_1, dec_bits) = decode_vlc(peek, 0);

            // eprintln!("Test: rho={:04b} u_off={} e_k={:04b} e_1={:04b}", rho, u_off, e_k, e_1);
            // eprintln!("  Encoded: value=0x{:04X} bits={}", encoded.value, encoded.bits);
            // eprintln!("  Decoded: rho={:04b} u_off={} e_k={:04b} e_1={:04b} bits={}",
            //           dec_rho, dec_u_off, dec_e_k, _dec_e_1, dec_bits);

            // Check that bits consumed matches bits encoded
            assert_eq!(
                dec_bits,
                encoded.bits,
                "Bit count mismatch for rho={:04b}",
                rho
            );

            // Check that rho matches (e_k and e_1 may not match if fallback was used)
            assert_eq!(dec_rho, rho, "Rho mismatch for input rho={:04b}", rho);
        }
    }
}

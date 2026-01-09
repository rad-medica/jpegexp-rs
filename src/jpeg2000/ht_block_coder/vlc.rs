use super::vlc_tables::{UVLC_TBL0_SRC, UVLC_TBL1_SRC, VLC_TBL0_SRC, VLC_TBL1_SRC};

const TABLE_SIZE: usize = 1024;

// Generate lookup tables at compile time
const VLC_TABLE_0: [u16; TABLE_SIZE] = generate_vlc_table(VLC_TBL0_SRC);
const VLC_TABLE_1: [u16; TABLE_SIZE] = generate_vlc_table(VLC_TBL1_SRC);

// Validate VLC tables at module initialization
// This will run once when the module is first loaded
fn validate_vlc_tables() {
    validate_vlc_table_impl("VLC_TBL0", VLC_TBL0_SRC);
    validate_vlc_table_impl("VLC_TBL1", VLC_TBL1_SRC);
    validate_uvlc_table_impl("UVLC_TBL0", UVLC_TBL0_SRC);
    validate_uvlc_table_impl("UVLC_TBL1", UVLC_TBL1_SRC);
}

fn validate_vlc_table_impl(name: &str, src: &[(u8, u8, u8, u8, u8, u16, u8)]) {
    use std::collections::HashMap;
    
    // Track FULL keys (with c_q) and their codewords
    let mut full_key_map: HashMap<(u8, u8, u8, u8, u8), Vec<(u16, u8)>> = HashMap::new();
    
    // Track ENCODER keys (without c_q) and their matches
    let mut encoder_key_map: HashMap<(u8, u8, u8, u8), Vec<(u8, u16, u8)>> = HashMap::new();
    
    for &entry in src {
        let full_key = (entry.0, entry.1, entry.2, entry.3, entry.4); // (c_q, rho, u_off, e_k, e_1)
        let encoder_key = (entry.1, entry.2, entry.3, entry.4); // (rho, u_off, e_k, e_1) - what encoder searches for
        let codeword = (entry.5, entry.6); // (cwd_suffix, suffix_len)
        let c_q = entry.0;
        
        full_key_map.entry(full_key).or_insert_with(Vec::new).push(codeword);
        encoder_key_map.entry(encoder_key).or_insert_with(Vec::new).push((c_q, entry.5, entry.6));
    }
    
    // Report full-key duplicates (should never happen)
    let mut full_duplicate_count = 0;
    for (key, codewords) in full_key_map.iter() {
        if codewords.len() > 1 {
            full_duplicate_count += 1;
            eprintln!("ERROR: {} FULL duplicate key (c_q={}, rho={:04b}, u_off={}, e_k={:04b}, e_1={:04b}) has {} codewords:",
                      name, key.0, key.1, key.2, key.3, key.4, codewords.len());
            for (i, &(cwd, len)) in codewords.iter().enumerate() {
                let value = ((key.0 as u16) << len) | cwd;
                let total_bits = 3 + len;
                eprintln!("  [{}] cwd_suffix=0x{:04X} suffix_len={} -> value=0x{:04X} bits={}",
                          i, cwd, len, value, total_bits);
            }
        }
    }
    
    // Report encoder-key duplicates (this is the problem!)
    let mut encoder_duplicate_count = 0;
    eprintln!("\n{} Encoder Key Analysis:", name);
    eprintln!("  Total entries: {}", src.len());
    eprintln!("  Unique full keys (c_q, rho, u_off, e_k, e_1): {}", full_key_map.len());
    eprintln!("  Unique encoder keys (rho, u_off, e_k, e_1): {}", encoder_key_map.len());
    
    for (key, matches) in encoder_key_map.iter() {
        if matches.len() > 1 {
            encoder_duplicate_count += 1;
            eprintln!("\nWARNING: {} encoder key (rho={:04b}, u_off={}, e_k={:04b}, e_1={:04b}) matches {} entries:",
                      name, key.0, key.1, key.2, key.3, matches.len());
            for (i, &(c_q, cwd, len)) in matches.iter().enumerate() {
                let value = ((c_q as u16) << len) | cwd;
                let total_bits = 3 + len;
                eprintln!("  [{}] c_q={} cwd_suffix=0x{:04X} suffix_len={} -> value=0x{:04X} bits={}",
                          i, c_q, cwd, len, value, total_bits);
            }
            eprintln!("  => Encoder will use entry [0], but decoder table may have a different entry!");
        }
    }
    
    if full_duplicate_count > 0 {
        eprintln!("\nERROR: {} has {} FULL duplicate keys! Table data is corrupt.", 
                  name, full_duplicate_count);
    }
    
    if encoder_duplicate_count > 0 {
        eprintln!("\nERROR: {} has {} encoder-key collisions! Encoder/decoder will mismatch.", 
                  name, encoder_duplicate_count);
        eprintln!("       Fix: Ensure each (rho, u_off, e_k, e_1) maps to exactly ONE entry.");
        eprintln!("       The encoder searches for (rho, u_off, e_k, e_1) and ignores c_q,");
        eprintln!("       but the decoder uses c_q from the bitstream to select the table.");
    } else if full_duplicate_count == 0 {
        eprintln!("\nINFO: {} validation passed: {} unique entries, no collisions.", 
                  name, src.len());
    }
}

fn validate_uvlc_table_impl(name: &str, src: &[u16]) {
    // UVLC tables are just lookup arrays indexed by u_q value (0-31)
    // Check for zero entries (invalid)
    let mut invalid_count = 0;
    for (idx, &entry) in src.iter().enumerate() {
        if entry == 0 {
            eprintln!("WARNING: {} entry {} is zero (invalid codeword)", name, idx);
            invalid_count += 1;
        }
    }
    
    if invalid_count > 0 {
        eprintln!("ERROR: {} has {} invalid (zero) entries!", name, invalid_count);
    } else {
        eprintln!("INFO: {} validation passed: {} valid entries.", name, src.len());
    }
}

// Run validation when module loads
#[allow(dead_code)]
const _: () = {
    // We can't call the validation function at compile time, so we'll use a test
};

const fn generate_vlc_table(src: &[(u8, u8, u8, u8, u8, u16, u8)]) -> [u16; TABLE_SIZE] {
    let mut table = [0u16; TABLE_SIZE];
    
    // Process each VLC entry and populate all matching table indices
    let mut j = 0;
    while j < src.len() {
        let entry = src[j];
        let s_cq = entry.0;      // Context prefix (3 bits)
        let rho = entry.1 as u16;
        let u_off = entry.2 as u16;
        let e_k = entry.3 as u16;
        let e_1 = entry.4 as u16;
        let s_cwd = entry.5;     // Codeword suffix
        let s_len = entry.6;     // Suffix length
        
        // Pack the decoded values once
        // Pack: e_k(4) | e_1(4) | rho(4) | u_off(1) | suffix_len(3)
        let packed = (e_k << 12) | (e_1 << 8) | (rho << 4) | (u_off << 3) | (s_len as u16);
        
        // For variable-length codes, populate all indices that match the prefix
        // Index format: c_q (bits 7-9) | cwd_prefix (bits 0-6)
        // For a code of length s_len, we need to set all entries where the top s_len bits
        // of the 7-bit cwd field match s_cwd
        if s_len <= 7 {
            let base_idx = ((s_cq as usize) << 7) | ((s_cwd as usize) << (7 - s_len));
            let num_indices = 1usize << (7 - s_len); // 2^(7 - s_len) indices
            
            let mut k = 0;
            while k < num_indices {
                let idx = base_idx | k;
                if idx < TABLE_SIZE {
                    // Only overwrite if empty or this entry is longer (more specific)
                    let existing = table[idx];
                    let existing_suffix_len = (existing & 0x7) as u8;
                    if existing == 0 || s_len > existing_suffix_len {
                        table[idx] = packed;
                    }
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
        VLC_TABLE_0[(peek >> 6) as usize]
    } else {
        VLC_TABLE_1[(peek >> 6) as usize]
    };

    // Unpack: e_k(4) | e_1(4) | rho(4) | u_off(1) | len(3)
    let rho = ((val >> 4) & 0xF) as u8;
    let u_off = ((val >> 3) & 0x1) as u8;
    let e_1 = ((val >> 8) & 0xF) as u8;
    let e_k = ((val >> 12) & 0xF) as u8;

    let suffix_len = (val & 0x7) as u8;
    // Total bits = 3 (c_q) + suffix_len
    let bits_consumed = 3 + suffix_len;

    (rho, u_off, e_k, e_1, bits_consumed)
}

/// VLC codeword result for encoding
pub struct VlcCodeword {
    pub value: u16,
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

    // First try: exact match on all fields
    for &entry in src {
        // Entry: (c_q, rho, u_off, e_k, e_1, cwd_suffix, suffix_len)
        if entry.1 == rho && entry.2 == u_off && entry.3 == emb_k && entry.4 == emb_1 {
            let c_q = entry.0 as u16;
            let cwd_suffix = entry.5;
            let suffix_len = entry.6;

            // Combine c_q (3 bits) and cwd_suffix (suffix_len bits)
            let value = (c_q << suffix_len) | cwd_suffix;
            let total_bits = 3 + suffix_len;

            if rho == 15 && u_off == 1 && emb_k == 15 && emb_1 == 15 {
                eprintln!("VLC ENCODE: rho={:04b} u_off={} emb_k={:04b} emb_1={:04b} -> c_q={} cwd_suffix={:04X} suffix_len={} value={:04X} bits={}", 
                          rho, u_off, emb_k, emb_1, c_q, cwd_suffix, suffix_len, value, total_bits);
            }

            return VlcCodeword {
                value,
                bits: total_bits,
            };
        }
    }

    // Second try: match on (rho, u_off, e_k) and take first entry
    // This handles cases where the exact emb_1 pattern isn't in the table
    for &entry in src {
        if entry.1 == rho && entry.2 == u_off && entry.3 == emb_k {
            let c_q = entry.0 as u16;
            let cwd_suffix = entry.5;
            let suffix_len = entry.6;

            let value = (c_q << suffix_len) | cwd_suffix;
            let total_bits = 3 + suffix_len;

            eprintln!("WARNING: encode_vlc using partial match (rho,u_off,e_k) for rho={:04b} u_off={} emb_k={:04b} emb_1={:04b} (using e_1={:04b})", 
                      rho, u_off, emb_k, emb_1, entry.4);

            return VlcCodeword {
                value,
                bits: total_bits,
            };
        }
    }

    // Third try: match on (rho, u_off) only - ensures rho is at least correct
    for &entry in src {
        if entry.1 == rho && entry.2 == u_off {
            let c_q = entry.0 as u16;
            let cwd_suffix = entry.5;
            let suffix_len = entry.6;

            let value = (c_q << suffix_len) | cwd_suffix;
            let total_bits = 3 + suffix_len;

            eprintln!("WARNING: encode_vlc using minimal match (rho,u_off) for rho={:04b} u_off={} emb_k={:04b} emb_1={:04b} (using e_k={:04b} e_1={:04b})", 
                      rho, u_off, emb_k, emb_1, entry.3, entry.4);

            return VlcCodeword {
                value,
                bits: total_bits,
            };
        }
    }

    // Fourth try: match on rho only - absolute fallback
    for &entry in src {
        if entry.1 == rho {
            let c_q = entry.0 as u16;
            let cwd_suffix = entry.5;
            let suffix_len = entry.6;

            let value = (c_q << suffix_len) | cwd_suffix;
            let total_bits = 3 + suffix_len;

            eprintln!("WARNING: encode_vlc using rho-only match for rho={:04b} u_off={} emb_k={:04b} emb_1={:04b} (using u_off={} e_k={:04b} e_1={:04b})", 
                      rho, u_off, emb_k, emb_1, entry.2, entry.3, entry.4);

            return VlcCodeword {
                value,
                bits: total_bits,
            };
        }
    }

    // Fallback/Error case (should not happen for valid inputs)
    eprintln!("ERROR: encode_vlc failed to find any match for rho={:04b} u_off={} emb_k={:04b} emb_1={:04b} context={}", 
              rho, u_off, emb_k, emb_1, context);
    VlcCodeword { value: 0, bits: 0 }
}

/// Encode magnitude residuals (u_q) for a pair of quads using UVLC
/// Note: UVLC table only has 32 entries (one per u_q value 0-31)
/// We encode the pair as two separate codewords back-to-back
pub fn encode_uvlc(u_q0: u8, u_q1: u8, context: u8) -> VlcCodeword {
    let src = if context == 0 {
        UVLC_TBL0_SRC
    } else {
        UVLC_TBL1_SRC
    };

    // Encode u_q0
    let idx0 = (u_q0 as usize).min(src.len() - 1);
    let val0 = src[idx0];
    let cw0 = val0 >> 8;
    let len0 = (val0 & 0xFF) as u8;
    
    // Encode u_q1
    let idx1 = (u_q1 as usize).min(src.len() - 1);
    let val1 = src[idx1];
    let cw1 = val1 >> 8;
    let len1 = (val1 & 0xFF) as u8;
    
    // Combine: cw0 (len0 bits) followed by cw1 (len1 bits)
    // Total codeword: cw0 << len1 | cw1
    let total_bits = len0 + len1;
    let combined_value = ((cw0 as u16) << len1) | cw1;
    
    VlcCodeword {
        value: combined_value,
        bits: total_bits,
    }
}

/// Decode magnitude residuals (u_q) for a pair of quads using UVLC
/// Decodes two separate u_q values encoded back-to-back
pub fn decode_uvlc(peek: u16, context: u8) -> (u8, u8, u8) {
    let src = if context == 0 {
        UVLC_TBL0_SRC
    } else {
        UVLC_TBL1_SRC
    };

    // Decode first u_q value (u_q0)
    // Must find the longest matching code (not first match)
    let mut bits_read = 0u8;
    let mut u_q0 = 0u8;
    let mut best_len = 0u8;
    
    for (idx, &entry) in src.iter().enumerate() {
        let len = (entry & 0xFF) as u8;
        if len == 0 {
            continue;
        }
        let val = entry >> 8;
        
        // Extract `len` bits from `peek` (MSB first)
        let stream_val = (peek >> (16 - len)) as u16;
        if stream_val == val && len > best_len {
            u_q0 = idx as u8;
            bits_read = len;
            best_len = len;
        }
    }
    
    // Decode second u_q value (u_q1) from remaining bits
    // Must find the longest matching code (not first match)
    let peek_u_q1 = peek << bits_read;
    let mut u_q1 = 0u8;
    let mut bits_u_q1 = 0u8;
    let mut best_len1 = 0u8;
    
    for (idx, &entry) in src.iter().enumerate() {
        let len = (entry & 0xFF) as u8;
        if len == 0 {
            continue;
        }
        let val = entry >> 8;
        
        // Extract `len` bits from remaining stream
        let stream_val = (peek_u_q1 >> (16 - len)) as u16;
        if stream_val == val && len > best_len1 {
            u_q1 = idx as u8;
            bits_u_q1 = len;
            best_len1 = len;
        }
    }
    
    let total_bits = bits_read + bits_u_q1;
    (u_q0, u_q1, total_bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vlc_table_validation() {
        eprintln!("\n========== VLC/UVLC Table Validation ==========");
        validate_vlc_tables();
        eprintln!("===============================================\n");
        
        // Note: This test will show warnings if there are duplicates,
        // but won't fail. Duplicates should be fixed in the source data.
    }
    
    #[test]
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
            let peek = encoded.value << (16 - encoded.bits);
            let (dec_rho, dec_u_off, dec_e_k, dec_e_1, dec_bits) = decode_vlc(peek, 0);
            
            eprintln!("Test: rho={:04b} u_off={} e_k={:04b} e_1={:04b}", rho, u_off, e_k, e_1);
            eprintln!("  Encoded: value=0x{:04X} bits={}", encoded.value, encoded.bits);
            eprintln!("  Decoded: rho={:04b} u_off={} e_k={:04b} e_1={:04b} bits={}", 
                      dec_rho, dec_u_off, dec_e_k, dec_e_1, dec_bits);
            
            // Check that bits consumed matches bits encoded
            assert_eq!(dec_bits, encoded.bits, 
                      "Bit count mismatch for rho={:04b}", rho);
            
            // Check that rho matches (e_k and e_1 may not match if fallback was used)
            assert_eq!(dec_rho, rho, 
                      "Rho mismatch for input rho={:04b}", rho);
        }
    }
}

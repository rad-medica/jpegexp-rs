use super::vlc_tables::{UVLC_TBL0_SRC, UVLC_TBL1_SRC, VLC_TBL0_SRC, VLC_TBL1_SRC};

const TABLE_SIZE: usize = 1024;

// Generate lookup tables at compile time
const VLC_TABLE_0: [u16; TABLE_SIZE] = generate_vlc_table(VLC_TBL0_SRC);
const VLC_TABLE_1: [u16; TABLE_SIZE] = generate_vlc_table(VLC_TBL1_SRC);

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

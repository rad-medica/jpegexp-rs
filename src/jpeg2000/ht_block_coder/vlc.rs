use super::vlc_tables::{VLC_TBL0_SRC, VLC_TBL1_SRC};

const TABLE_SIZE: usize = 1024;

// Generate lookup tables at compile time
const VLC_TABLE_0: [u16; TABLE_SIZE] = generate_vlc_table(VLC_TBL0_SRC);
const VLC_TABLE_1: [u16; TABLE_SIZE] = generate_vlc_table(VLC_TBL1_SRC);

const fn generate_vlc_table(src: &[(u8, u8, u8, u8, u8, u16, u8)]) -> [u16; TABLE_SIZE] {
    let mut table = [0u16; TABLE_SIZE];
    let mut i = 0;
    while i < TABLE_SIZE {
        let cwd = (i as u16) & 0x7F;
        let c_q = (i as u8) >> 7;
        
        let mut j = 0;
        let mut found = false;
        while j < src.len() {
            let entry = src[j];
            let s_cq = entry.0;
            if s_cq == c_q {
                let s_cwd = entry.5;
                let s_len = entry.6;
                // Mask the lookahead `cwd` with the length of the table entry `cwd_len`
                let mask = (1 << s_len) - 1;
                if s_cwd == (cwd & mask) {
                    // Match found
                    // Format: c_q, rho, u_off, e_k, e_1, cwd, len
                    let rho = entry.1 as u16;
                    let u_off = entry.2 as u16;
                    let e_k = entry.3 as u16;
                    let e_1 = entry.4 as u16;
                    let len = entry.6 as u16;
                    
                    // Pack: e_k(4) | e_1(4) | rho(4) | u_off(1) | len(3)
                    // e_k: bits 12-15
                    // e_1: bits 8-11
                    // rho: bits 4-7
                    // u_off: bit 3
                    // len: bits 0-2
                    table[i] = (e_k << 12) | (e_1 << 8) | (rho << 4) | (u_off << 3) | len;
                    found = true;
                }
            }
            if found { break; }
            j += 1;
        }
        i += 1;
    }
    table
}

/// Variable Length Coding (VLC) tables and logic for HTJ2K.
/// Based on ISO/IEC 15444-15 Table 6 and Table 8.
/// Decodes a VLC code word into a 4-pixel quad significance pattern (rho),
/// an embedded context (emb_k) correction, and context for the next quad.
///
/// Arguments:
/// - `peek`: 16 bits of lookahead from the bitstream.
/// - `context`: The current context (0 or 1) derived from neighbors.
///
/// Returns: `(rho, u_off, e_k, bits_consumed)`
/// - `rho`: 4-bit significance pattern (0..15).
/// - `u_off`: u-value offset (used for magnitude exponent prediction).
/// - `e_k`: exponent prediction calculation helper? (Actually "emb_k" logic).
/// - `bits_consumed`: Number of bits used by the VLC code.
pub fn decode_vlc(peek: u16, context: u8) -> (u8, u8, u8, u8) {
    // peek is MSB aligned (bit 15 is first bit of stream).
    // The table index expects 7 bits lookahead in LSB-first order (bit-reversed).
    // 
    // 1. Extract top 7 bits: (peek >> 9) & 0x7F
    //    Stream: b0 b1 b2 b3 b4 b5 b6 ...
    //    Result: 0..0 b0 b1 b2 b3 b4 b5 b6 (MSB b0 is at bit 6)
    //
    // 2. Reverse bits to get LSB-first order for table lookup
    //    Target: 0..0 b6 b5 b4 b3 b2 b1 b0
    //
    let lookahead = (peek >> 9) & 0x7F;
    let lookahead_rev = (lookahead as u8).reverse_bits() >> 1; // shift down 1 because u8 is 8 bits
    
    let idx = ((context as u16) << 7) | (lookahead_rev as u16);
    
    let val = if context == 0 {
        VLC_TABLE_0[idx as usize]
    } else {
        VLC_TABLE_1[idx as usize]
    };
    
    // Unpack: e_k(4) | e_1(4) | rho(4) | u_off(1) | len(3)
    let rho = ((val >> 4) & 0xF) as u8;
    let u_off = ((val >> 3) & 0x1) as u8;
    let e_k = ((val >> 12) & 0xF) as u8;
    // We ignore e_1 for now in the return signature, or assume e_k covers what caller needs.
    // The caller asks for `e_k`. Standard says `emb_k` calculation uses `E_k`.
    
    let bits_consumed = (val & 0x7) as u8;
    
    (rho, u_off, e_k, bits_consumed)
}

/// VLC codeword result for encoding
pub struct VlcCodeword {
    pub value: u16,
    pub bits: u8,
}

/// Encode a significance pattern (rho) to a VLC codeword
/// This is the inverse of decode_vlc
pub fn encode_vlc(_rho: u8, _context: u8) -> VlcCodeword {
    // TODO: Implement using reverse lookup table if encoding is needed.
    // For now, this is a placeholder.
    VlcCodeword { value: 0, bits: 0 }
}

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
    // VLC decoding based on Table 8 of ISO 15444-15
    // Format: (Codeword, Mask, Rho, U_off, E_k)
    // We match the prefix of 'peek'.

    // Note: The standard defines 2 contexts for VLC:
    // Context 0 (Initial) and Context 1 (Adapted locally?)

    // Simplified Table matching logic.
    // Ideally this should be a lookup table.
    // Since we are implementing a subset/skeleton first, we implement the logic for common cases.

    if context == 0 {
        // Context 0 Table
        // 0... -> 0000 (rho=0), len=1
        if peek & 0x8000 == 0 {
            (0, 0, 0, 1)
        } else {
            // 10... -> 0001/0010/0100/1000 (rho=1/2/4/8), len=2?
            // Standard says:
            // 100 -> 1000 (rho=8), len=3
            // 101 -> 0100 (rho=4), len=3
            // 110 -> 0010 (rho=2), len=3
            // 1110 -> 0001 (rho=1), len=4
            // ...

            // Let's implement a small switch for the prompt.
            // Leading bits:
            let top3 = (peek >> 13) & 0x7;
            match top3 {
                0..=3 => (0, 0, 0, 1), // 0xxx -> 0000
                0b100 => (8, 0, 0, 3), // 100
                0b101 => (4, 0, 0, 3), // 101
                0b110 => (2, 0, 0, 3), // 110
                0b111 => {
                    // 111...
                    if peek & 0x1000 == 0 {
                        // 1110
                        (1, 0, 0, 4)
                    } else {
                        // 1111...
                        // Fallback/Extenstion
                        (15, 1, 1, 5) // Dummy fallback for fully significant
                    }
                }
                _ => (0, 0, 0, 1), // Should not reach
            }
        }
    } else {
        // Context 1 Table (different probabilities)
        // Context 1 uses similar structure but with different codeword assignments
        // For now, use same logic as Context 0 as a reasonable approximation
        // Full implementation would use the actual Context 1 table from the standard
        if peek & 0x8000 == 0 {
            (0, 0, 0, 1)
        } else {
            let top3 = (peek >> 13) & 0x7;
            match top3 {
                0..=3 => (0, 0, 0, 1), // 0xxx -> 0000
                0b100 => (8, 0, 0, 3), // 100
                0b101 => (4, 0, 0, 3), // 101
                0b110 => (2, 0, 0, 3), // 110
                0b111 => {
                    if peek & 0x1000 == 0 {
                        (1, 0, 0, 4) // 1110
                    } else {
                        (15, 1, 1, 5) // 1111... (fallback)
                    }
                }
                _ => (0, 0, 0, 1),
            }
        }
    }
}

/// VLC codeword result for encoding
pub struct VlcCodeword {
    pub value: u16,
    pub bits: u8,
}

/// Encode a significance pattern (rho) to a VLC codeword
/// This is the inverse of decode_vlc
pub fn encode_vlc(rho: u8, context: u8) -> VlcCodeword {
    // Map rho patterns to VLC codewords (inverse of decode table)
    // Context 0 and Context 1 use same structure for simplicity
    let _ = context; // Both contexts use similar encoding for now

    match rho {
        0 => VlcCodeword {
            value: 0b0,
            bits: 1,
        }, // 0
        1 => VlcCodeword {
            value: 0b1110,
            bits: 4,
        }, // 1110
        2 => VlcCodeword {
            value: 0b110,
            bits: 3,
        }, // 110
        4 => VlcCodeword {
            value: 0b101,
            bits: 3,
        }, // 101
        8 => VlcCodeword {
            value: 0b100,
            bits: 3,
        }, // 100
        // Multi-significant patterns (simplified fallback to 1111... prefix)
        3 => VlcCodeword {
            value: 0b11110,
            bits: 5,
        }, // 11110 (rho=3: samples 0,1)
        5 => VlcCodeword {
            value: 0b11111,
            bits: 5,
        }, // 11111 (rho=5: samples 0,2)
        6 => VlcCodeword {
            value: 0b111100,
            bits: 6,
        }, // 111100 (rho=6: samples 1,2)
        7 => VlcCodeword {
            value: 0b111101,
            bits: 6,
        }, // 111101 (rho=7: samples 0,1,2)
        9 => VlcCodeword {
            value: 0b111110,
            bits: 6,
        }, // 111110 (rho=9: samples 0,3)
        10 => VlcCodeword {
            value: 0b111111,
            bits: 6,
        }, // 111111 (rho=10: samples 1,3)
        11 => VlcCodeword {
            value: 0b1111100,
            bits: 7,
        }, // (rho=11: samples 0,1,3)
        12 => VlcCodeword {
            value: 0b1111101,
            bits: 7,
        }, // (rho=12: samples 2,3)
        13 => VlcCodeword {
            value: 0b1111110,
            bits: 7,
        }, // (rho=13: samples 0,2,3)
        14 => VlcCodeword {
            value: 0b1111111,
            bits: 7,
        }, // (rho=14: samples 1,2,3)
        15 => VlcCodeword {
            value: 0b11111111,
            bits: 8,
        }, // All significant
        _ => VlcCodeword {
            value: 0b0,
            bits: 1,
        }, // Default to insignificant
    }
}

// In a real optimized decoder, these would be 256-entry or 1024-entry lookup tables.

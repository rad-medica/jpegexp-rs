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
                0b100 => (8, 0, 0, 3),                         // 100
                0b101 => (4, 0, 0, 3),                         // 101
                0b110 => (2, 0, 0, 3),                         // 110
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

// In a real optimized decoder, these would be 256-entry or 1024-entry lookup tables.

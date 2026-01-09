/// Unit test to verify VLC encoding and decoding roundtrip
use jpegexp_rs::jpeg2000::ht_block_coder::vlc::{encode_vlc, decode_vlc};
use jpegexp_rs::jpeg2000::ht_block_coder::vlc_tables::{VLC_TBL0_SRC, VLC_TBL1_SRC};

#[test]
fn test_vlc_roundtrip_all_entries() {
    // Test all entries in VLC_TBL0_SRC for roundtrip consistency
    for (j, &entry) in VLC_TBL0_SRC.iter().enumerate() {
        let (c_q, rho, u_off, e_k, e_1, cwd_suffix, suffix_len) = entry;
        
        // Encode using the exact values from the table
        let encoded = encode_vlc(rho, 0, u_off, e_k, e_1);
        
        // Expected codeword: c_q (3 bits) + cwd_suffix (suffix_len bits)
        let expected_value = ((c_q as u16) << suffix_len) | cwd_suffix;
        let expected_bits = 3 + suffix_len;
        
        if encoded.value != expected_value {
            eprintln!("Entry {}: c_q={} rho={:04b} u_off={} e_k={:04b} e_1={:04b} cwd_suffix=0x{:02X} suffix_len={}",
                      j, c_q, rho, u_off, e_k, e_1, cwd_suffix, suffix_len);
            eprintln!("  Encoded: value=0x{:04X} bits={}", encoded.value, encoded.bits);
            eprintln!("  Expected: value=0x{:04X} bits={}", expected_value, expected_bits);
        }
        
        assert_eq!(encoded.value, expected_value, 
                   "Encode mismatch for c_q={} rho={:04b} u_off={} e_k={:04b} e_1={:04b}: got 0x{:04X}, expected 0x{:04X}",
                   c_q, rho, u_off, e_k, e_1, encoded.value, expected_value);
        assert_eq!(encoded.bits, expected_bits,
                   "Bit count mismatch for c_q={} rho={:04b}: got {}, expected {}",
                   c_q, rho, encoded.bits, expected_bits);
        
        // Decode: simulate bitstream with codeword in the top bits
        // The decoder uses (peek >> 6) which extracts bits 6-15 (10 bits)
        // A codeword of N bits occupies the top N bits of this 10-bit space
        // Remaining (10-N) bits can be anything (next codeword or padding)
        // For testing, we'll set remaining bits to 0
        let total_bits = 3 + suffix_len;
        let shift_amount = 16 - total_bits; // Left-align in 16-bit peek
        let peek = ((encoded.value as u16) << shift_amount);
        let (dec_rho, dec_u_off, dec_e_k, dec_e_1, dec_bits) = decode_vlc(peek, 0);
        
        if dec_rho != rho {
            eprintln!("Entry {}: DECODE MISMATCH", j);
            eprintln!("  Encoded: c_q={} rho={:04b} u_off={} e_k={:04b} e_1={:04b} value=0x{:04X} bits={}",
                      c_q, rho, u_off, e_k, e_1, encoded.value, encoded.bits);
            eprintln!("  Peek: 0x{:04X} (binary: {:016b})", peek, peek);
            eprintln!("  Index: {} (0x{:03X})", peek >> 6, peek >> 6);
            eprintln!("  Decoded: rho={:04b} u_off={} e_k={:04b} e_1={:04b} bits={}",
                      dec_rho, dec_u_off, dec_e_k, dec_e_1, dec_bits);
        }
        
        assert_eq!(dec_rho, rho, 
                   "Decode rho mismatch for peek=0x{:04X} (encoded c_q={} rho={:04b} u_off={} e_k={:04b} e_1={:04b}): got {:04b}, expected {:04b}",
                   peek, c_q, rho, u_off, e_k, e_1, dec_rho, rho);
        assert_eq!(dec_u_off, u_off,
                   "Decode u_off mismatch for peek=0x{:04X}: got {}, expected {}",
                   peek, dec_u_off, u_off);
        assert_eq!(dec_e_k, e_k,
                   "Decode e_k mismatch for peek=0x{:04X}: got {:04b}, expected {:04b}",
                   peek, dec_e_k, e_k);
        assert_eq!(dec_e_1, e_1,
                   "Decode e_1 mismatch for peek=0x{:04X}: got {:04b}, expected {:04b}",
                   peek, dec_e_1, e_1);
        assert_eq!(dec_bits, expected_bits,
                   "Decode bits mismatch for peek=0x{:04X}: got {}, expected {}",
                   peek, dec_bits, expected_bits);
    }
    
    println!("✓ All {} VLC_TBL0_SRC entries passed roundtrip test", VLC_TBL0_SRC.len());
}

#[test]
fn test_vlc_specific_case() {
    // Test the specific case from our HTJ2K failure:
    // Entry: (0, 0xF, 0x1, 0xF, 0x1, 0x33, 7)
    let rho = 0xF;
    let u_off = 1;
    let e_k = 0xF;
    let e_1 = 0x1;
    
    // Encode
    let encoded = encode_vlc(rho, 0, u_off, e_k, e_1);
    println!("Encoded: rho={:04b} u_off={} e_k={:04b} e_1={:04b} => value=0x{:04X} bits={}", 
             rho, u_off, e_k, e_1, encoded.value, encoded.bits);
    
    assert_eq!(encoded.value, 0x33, "Expected codeword 0x33");
    assert_eq!(encoded.bits, 10, "Expected 10 bits (3 + 7)");
    
    // Decode: place codeword in bits 6-15
    let peek = 0x0CF0; // 0x33 << 6 = 0xCCC, but actual stream has 0x0CF0
    println!("Peek value: 0x{:04X} (bits: {:016b})", peek, peek);
    println!("Index: {} (0x{:03X})", (peek >> 6), (peek >> 6));
    
    let (dec_rho, dec_u_off, dec_e_k, dec_e_1, dec_bits) = decode_vlc(peek, 0);
    println!("Decoded: rho={:04b} u_off={} e_k={:04b} e_1={:04b} bits={}", 
             dec_rho, dec_u_off, dec_e_k, dec_e_1, dec_bits);
    
    // These should match
    assert_eq!(dec_rho, rho, "rho mismatch");
    assert_eq!(dec_u_off, u_off, "u_off mismatch");
    assert_eq!(dec_e_k, e_k, "e_k mismatch");
    assert_eq!(dec_e_1, e_1, "e_1 mismatch");
    assert_eq!(dec_bits, encoded.bits, "bits mismatch");
}

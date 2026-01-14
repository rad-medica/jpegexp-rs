// Verify our context selection logic matches OpenJPEG's lookup tables

// OpenJPEG's zero-coding context lookup table (first 512 entries for orient=0)
const OPENJPEG_ZC_ORIENT0: &[u8] = &[
    0, 1, 3, 3, 1, 2, 3, 3, 5, 6, 7, 7, 6, 6, 7, 7, 0, 1, 3, 3, 1, 2, 3, 3, 5, 6, 7, 7, 6, 6, 7, 7,
    5, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 5, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    1, 2, 3, 3, 2, 2, 3, 3, 6, 6, 7, 7, 6, 6, 7, 7, 1, 2, 3, 3, 2, 2, 3, 3, 6, 6, 7, 7, 6, 6, 7, 7,
    6, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 6, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7, 3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7, 3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    1, 2, 3, 3, 2, 2, 3, 3, 6, 6, 7, 7, 6, 6, 7, 7, 1, 2, 3, 3, 2, 2, 3, 3, 6, 6, 7, 7, 6, 6, 7, 7,
    6, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 6, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    2, 2, 3, 3, 2, 2, 3, 3, 6, 6, 7, 7, 6, 6, 7, 7, 2, 2, 3, 3, 2, 2, 3, 3, 6, 6, 7, 7, 6, 6, 7, 7,
    6, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 6, 6, 7, 7, 6, 6, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7, 3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7, 3, 3, 4, 4, 3, 3, 4, 4, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
];

// Simple function to compute context like our implementation
fn compute_zc_context(h: u8, v: u8, d: u8, orient: u8) -> usize {
    match orient {
        0 | 2 => {
            // LL, LH - Prioritize H
            if h == 2 {
                8
            } else if h == 1 {
                if v >= 1 {
                    7
                } else if d >= 1 {
                    6
                } else {
                    5
                }
            } else if v == 2 {
                4
            } else if v == 1 {
                3
            } else if d >= 2 {
                2
            } else if d == 1 {
                1
            } else {
                0
            }
        }
        1 => {
            // HL - Prioritize V
            if v == 2 {
                8
            } else if v == 1 {
                if h >= 1 {
                    7
                } else if d >= 1 {
                    6
                } else {
                    5
                }
            } else if h == 2 {
                4
            } else if h == 1 {
                3
            } else if d >= 2 {
                2
            } else if d == 1 {
                1
            } else {
                0
            }
        }
        3 => {
            // HH
            let hv = h + v;
            if d >= 3 {
                8
            } else if d == 2 {
                if hv >= 1 {
                    7
                } else {
                    6
                }
            } else if d == 1 {
                if hv >= 2 {
                    5
                } else if hv == 1 {
                    4
                } else {
                    3
                }
            } else if hv >= 2 {
                2
            } else if hv == 1 {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

#[test]
fn verify_zc_context_orient0() {
    let mut mismatches = 0;
    
    // Test all combinations for orient=0
    // OpenJPEG uses 9-bit flag patterns where bits represent neighbors
    for flags in 0..512 {
        // Extract h, v, d from flag pattern
        // Bit layout: [d3 d2 v1 h1 h0 v0 d1 d0 center]
        // We need to match OpenJPEG's bit positions
        let h = ((flags >> 3) & 1) + ((flags >> 5) & 1);
        let v = ((flags >> 1) & 1) + ((flags >> 7) & 1);
        let d = (flags & 1) + ((flags >> 2) & 1) + ((flags >> 6) & 1) + ((flags >> 8) & 1);
        
        let our_ctx = compute_zc_context(h as u8, v as u8, d as u8, 0);
        let opj_ctx = OPENJPEG_ZC_ORIENT0[flags] as usize;
        
        if our_ctx != opj_ctx {
            if mismatches < 10 {
                println!("Mismatch at flags={:#011b}: h={} v={} d={} → ours={} openjpeg={}", 
                    flags, h, v, d, our_ctx, opj_ctx);
            }
            mismatches += 1;
        }
    }
    
    if mismatches > 0 {
        println!("\nTotal mismatches: {}/512", mismatches);
    } else {
        println!("\n✅ All 512 orient=0 contexts match OpenJPEG!");
    }
    
    assert_eq!(mismatches, 0, "Context selection doesn't match OpenJPEG");
}

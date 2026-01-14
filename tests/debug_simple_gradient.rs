// Debug test for simple gradient pattern

use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

#[test]
fn test_simple_2x2_gradient() {
    // Simplest possible gradient: 4 different values
    let data = [0i32, 1, 2, 3];
    
    println!("\n=== Testing 2x2 gradient: {:?} ===", data);
    
    // Encode
    let mut bpc = BitPlaneCoder::new(2, 2, &data);
    let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
    println!("Max bit-plane: {}", max_bp);
    
    let passes = bpc.encode_codeblock(max_bp, 0, 0);  // orient=0 (LL subband)
    bpc.mq.flush();
    let encoded = bpc.mq.get_buffer().to_vec();
    
    println!("Encoded to {} bytes in {} passes", encoded.len(), passes);
    println!("Encoded bytes: {:02x?}", &encoded[..encoded.len().min(20)]);
    
    // Decode
    let mut dec = BitPlaneCoder::new(2, 2, &[]);
    let decoded = dec.decode_codeblock(&encoded, max_bp, passes, 0).unwrap();
    
    println!("\nOriginal:  {:?}", data);
    println!("Decoded:   {:?}", decoded);
    
    // Check for errors
    let mut errors = vec![];
    for (i, (&orig, &dec_val)) in data.iter().zip(decoded.iter()).enumerate() {
        if orig != dec_val {
            errors.push((i, orig, dec_val));
        }
    }
    
    if errors.is_empty() {
        println!("✅ Perfect roundtrip!");
    } else {
        println!("\n❌ ERRORS:");
        for (i, orig, dec_val) in &errors {
            println!("  Position {}: {} → {}", i, orig, dec_val);
        }
    }
    
    assert_eq!(errors.len(), 0, "Should have perfect roundtrip for simple gradient");
}

#[test]
fn test_simple_4x4_gradient_horizontal() {
    // Horizontal gradient: each row has same value
    #[rustfmt::skip]
    let data = [
        0, 0, 0, 0,
        85, 85, 85, 85,
        170, 170, 170, 170,
        255, 255, 255, 255,
    ];
    
    println!("\n=== Testing 4x4 horizontal gradient ===");
    
    // Encode
    let mut bpc = BitPlaneCoder::new(4, 4, &data);
    let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
    
    let passes = bpc.encode_codeblock(max_bp, 0, 0);
    bpc.mq.flush();
    let encoded = bpc.mq.get_buffer().to_vec();
    
    // Decode
    let mut dec = BitPlaneCoder::new(4, 4, &[]);
    let decoded = dec.decode_codeblock(&encoded, max_bp, passes, 0).unwrap();
    
    // Check
    let mut errors = 0;
    for (i, (&orig, &dec_val)) in data.iter().zip(decoded.iter()).enumerate() {
        if orig != dec_val {
            if errors < 5 {
                println!("Mismatch at [{}]: {} → {}", i, orig, dec_val);
            }
            errors += 1;
        }
    }
    
    if errors == 0 {
        println!("✅ Perfect roundtrip!");
    } else {
        println!("\n❌ {} errors out of {}", errors, data.len());
    }
    
    assert_eq!(errors, 0, "Should have perfect roundtrip");
}

#[test]
fn test_simple_4x4_gradient_vertical() {
    // Vertical gradient: each column has same value
    #[rustfmt::skip]
    let data = [
        0, 85, 170, 255,
        0, 85, 170, 255,
        0, 85, 170, 255,
        0, 85, 170, 255,
    ];
    
    println!("\n=== Testing 4x4 vertical gradient ===");
    
    // Encode
    let mut bpc = BitPlaneCoder::new(4, 4, &data);
    let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
    
    let passes = bpc.encode_codeblock(max_bp, 0, 0);
    bpc.mq.flush();
    let encoded = bpc.mq.get_buffer().to_vec();
    
    // Decode
    let mut dec = BitPlaneCoder::new(4, 4, &[]);
    let decoded = dec.decode_codeblock(&encoded, max_bp, passes, 0).unwrap();
    
    // Check
    let mut errors = 0;
    for (i, (&orig, &dec_val)) in data.iter().zip(decoded.iter()).enumerate() {
        if orig != dec_val {
            if errors < 5 {
                println!("Mismatch at [{}]: {} → {}", i, orig, dec_val);
            }
            errors += 1;
        }
    }
    
    if errors == 0 {
        println!("✅ Perfect roundtrip!");
    } else {
        println!("\n❌ {} errors out of {}", errors, data.len());
    }
    
    assert_eq!(errors, 0, "Should have perfect roundtrip");
}

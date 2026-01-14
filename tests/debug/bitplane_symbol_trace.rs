/// Test to trace every symbol encoded by the bit-plane coder
/// This will help us find where our symbol sequence diverges from OpenJPEG

use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

#[test]
#[ignore]
fn trace_bitplane_symbols_4x4() {
    // Create a minimal 4x4 codeblock with known values
    // Using a simple pattern that should be easy to verify
    let data = vec![0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    println!("\n=== Tracing 4x4 Codeblock with single coefficient = 7 ===");
    println!("Data layout:");
    for y in 0..4 {
        print!("  ");
        for x in 0..4 {
            print!("{:3} ", data[y * 4 + x]);
        }
        println!();
    }

    let mut bpc = BitPlaneCoder::new(4, 4, &data);
    let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
    println!("\nMax bit-plane: {}", max_bp);
    println!("Binary representation of 7: {:08b}", 7);
    println!("Bit-planes: BP2=1, BP1=1, BP0=1\n");

    // Manually trace what should happen:
    // Cleanup pass at BP2 (bit-plane 2):
    //   - Scan all 16 pixels in stripe order (4 stripes of 4 pixels each)
    //   - Stripe 0 (y=0): x=0,1,2,3 all have bit=0 at BP2
    //   - Stripe 1 (y=1): x=0 bit=0, x=1 bit=1 (significant!), x=2,3 bit=0
    //   - When x=1,y=1 becomes significant, encode sign
    //   - Continue for remaining stripes

    println!("Expected symbol sequence for Cleanup pass at BP2:");
    println!("  Stripe 0 (y=0, x=0..3): All zeros, no significant neighbors");
    println!("    - Check if RLC applies (all 4 insignificant, no neighbors)");
    println!("    - For each x: encode AGG bit, then individual pixels");
    println!("  Stripe 1 (y=1, x=0..3): x=1 has bit=1");
    println!("    - x=0: encode ZC(0), result=0");
    println!("    - x=1: encode ZC(0), result=1, then SC(sign=0)");
    println!("    - x=2,3: encode ZC(0), result=0");
    println!("  Stripes 2-3: All zeros\n");

    // Now encode and see what we actually produce
    let passes = bpc.encode_codeblock(max_bp, 0, 0);
    bpc.mq.flush();
    let encoded = bpc.mq.get_buffer();

    println!("\nEncoding complete:");
    println!("  Passes: {}", passes);
    println!("  Encoded bytes: {}", encoded.len());
    println!("  Data: {:02X?}", encoded);

    // Decode to verify
    let mut bpc_dec = BitPlaneCoder::new(4, 4, &[]);
    let decoded = bpc_dec
        .decode_codeblock(encoded, max_bp, passes, 0)
        .unwrap();

    println!("\nDecoded result:");
    for y in 0..4 {
        print!("  ");
        for x in 0..4 {
            print!("{:3} ", decoded[y * 4 + x]);
        }
        println!();
    }

    let mut errors = 0;
    for (i, (&orig, &dec)) in data.iter().zip(decoded.iter()).enumerate() {
        if orig != dec {
            println!("ERROR at index {}: {} -> {}", i, orig, dec);
            errors += 1;
        }
    }

    assert_eq!(errors, 0, "Should have perfect roundtrip");
}

#[test]
#[ignore]
fn trace_bitplane_symbols_8x8_gradient() {
    // Create an 8x8 gradient similar to what appears in level 2 subbands
    let mut data = vec![0i32; 64];
    for y in 0..8 {
        for x in 0..8 {
            data[y * 8 + x] = (x + y * 2) as i32;
        }
    }

    println!("\n=== Tracing 8x8 Gradient Codeblock ===");
    println!("Data layout:");
    for y in 0..8 {
        print!("  ");
        for x in 0..8 {
            print!("{:3} ", data[y * 8 + x]);
        }
        println!();
    }

    let mut bpc = BitPlaneCoder::new(8, 8, &data);
    let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
    println!("\nMax bit-plane: {}", max_bp);

    let passes = bpc.encode_codeblock(max_bp, 0, 0);
    bpc.mq.flush();
    let encoded = bpc.mq.get_buffer();

    println!("\nEncoding complete:");
    println!("  Passes: {}", passes);
    println!("  Encoded bytes: {}", encoded.len());
    println!(
        "  First 20 bytes: {:02X?}",
        &encoded[..encoded.len().min(20)]
    );

    // Decode to verify
    let mut bpc_dec = BitPlaneCoder::new(8, 8, &[]);
    let decoded = bpc_dec
        .decode_codeblock(encoded, max_bp, passes, 0)
        .unwrap();

    let mut errors = 0;
    for (i, (&orig, &dec)) in data.iter().zip(decoded.iter()).enumerate() {
        if orig != dec {
            if errors < 5 {
                println!("ERROR at index {}: {} -> {}", i, orig, dec);
            }
            errors += 1;
        }
    }

    println!("\nRoundtrip: {} errors out of {}", errors, data.len());
    assert_eq!(errors, 0, "Should have perfect roundtrip");
}

#[test]
#[ignore]
fn trace_bitplane_symbols_32x32_hl_subband() {
    // Simulate the actual HL subband from level 2 that's causing issues
    // This is a 32x32 block from the gradient test image
    let mut data = vec![0i32; 1024];

    // Create a gradient pattern similar to what DWT produces for HL subband
    // HL captures horizontal high-frequency (vertical edges)
    for y in 0..32 {
        for x in 0..32 {
            // Simple gradient that produces horizontal edges
            data[y * 32 + x] = ((x as i32 - 16).abs() - 8).max(0);
        }
    }

    println!("\n=== Tracing 32x32 HL Subband Codeblock ===");
    println!("Data sample (top-left 8x8):");
    for y in 0..8 {
        print!("  ");
        for x in 0..8 {
            print!("{:3} ", data[y * 32 + x]);
        }
        println!();
    }

    let mut bpc = BitPlaneCoder::new(32, 32, &data);
    let max_bp = bpc.calculate_max_bit_plane().expect("Should have max_bp");
    println!("\nMax bit-plane: {}", max_bp);

    let passes = bpc.encode_codeblock(max_bp, 0, 1); // orientation=1 for HL
    bpc.mq.flush();
    let encoded = bpc.mq.get_buffer();

    println!("\nEncoding complete:");
    println!("  Passes: {}", passes);
    println!("  Encoded bytes: {}", encoded.len());
    println!(
        "  First 20 bytes: {:02X?}",
        &encoded[..encoded.len().min(20)]
    );

    // This is where we expect 65 bytes, but OpenJPEG produces 68 bytes
    println!("\n⚠️  Expected OpenJPEG length: 68 bytes");
    println!("⚠️  Our length: {} bytes", encoded.len());
    println!("⚠️  Difference: {} bytes", 68i32 - encoded.len() as i32);

    // Decode to verify our encoding is internally consistent
    let mut bpc_dec = BitPlaneCoder::new(32, 32, &[]);
    let decoded = bpc_dec
        .decode_codeblock(encoded, max_bp, passes, 1)
        .unwrap();

    let mut errors = 0;
    for (i, (&orig, &dec)) in data.iter().zip(decoded.iter()).enumerate() {
        if orig != dec {
            if errors < 5 {
                let y = i / 32;
                let x = i % 32;
                println!("ERROR at ({}, {}): {} -> {}", x, y, orig, dec);
            }
            errors += 1;
        }
    }

    println!("\nRoundtrip: {} errors out of {}", errors, data.len());
    assert_eq!(errors, 0, "Should have perfect roundtrip");
}

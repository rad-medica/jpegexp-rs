// Test to trace coefficient flow through encoder and decoder
// Goal: Find where gradient patterns break

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

/// Test that a simple 4x4 gradient roundtrips perfectly
#[test]
fn test_4x4_gradient_roundtrip() {
    // 4x4 horizontal gradient
    #[rustfmt::skip]
    let input: Vec<u8> = vec![
        0, 85, 170, 255,
        0, 85, 170, 255,
        0, 85, 170, 255,
        0, 85, 170, 255,
    ];
    
    let frame_info = FrameInfo {
        width: 4,
        height: 4,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    println!("\n=== Input ===");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:3} ", input[y * 4 + x]);
        }
        println!();
    }
    
    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(1);
    encoder.set_irreversible(false);
    
    let mut encoded = vec![0u8; 8192];
    let encode_result = encoder.encode(&input, &frame_info, &mut encoded);
    
    match encode_result {
        Ok(len) => {
            println!("\nEncoded {} bytes", len);
            
            // Decode
            let mut stream = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(&encoded[..len]);
            let mut decoder = jpegexp_rs::jpeg2000::decoder::J2kDecoder::new(&mut stream);
            
            let decode_result = decoder.decode();
            match decode_result {
                Ok(image) => {
                    let pixels = image.reconstruct_pixels();
                    match pixels {
                        Ok(decoded) => {
                            println!("\n=== Decoded ===");
                            for y in 0..4 {
                                for x in 0..4 {
                                    print!("{:3} ", decoded[y * 4 + x]);
                                }
                                println!();
                            }
                            
                            // Calculate errors
                            let mut total_error = 0i32;
                            let mut max_error = 0i32;
                            for i in 0..16 {
                                let err = (input[i] as i32 - decoded[i] as i32).abs();
                                total_error += err;
                                max_error = max_error.max(err);
                            }
                            let mae = total_error as f64 / 16.0;
                            
                            println!("\nMAE: {:.4}", mae);
                            println!("Max Error: {}", max_error);
                            
                            assert_eq!(max_error, 0, "Lossless encoding should have 0 error");
                        }
                        Err(e) => panic!("Failed to reconstruct: {}", e),
                    }
                }
                Err(e) => panic!("Failed to decode: {:?}", e),
            }
        }
        Err(e) => panic!("Failed to encode: {:?}", e),
    }
}

/// Test solid pattern (known to work)
#[test] 
fn test_4x4_solid_roundtrip() {
    let input: Vec<u8> = vec![127; 16];
    
    let frame_info = FrameInfo {
        width: 4,
        height: 4,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(1);
    encoder.set_irreversible(false);
    
    let mut encoded = vec![0u8; 8192];
    let len = encoder.encode(&input, &frame_info, &mut encoded).unwrap();
    
    let mut stream = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(&encoded[..len]);
    let mut decoder = jpegexp_rs::jpeg2000::decoder::J2kDecoder::new(&mut stream);
    
    let image = decoder.decode().unwrap();
    let decoded = image.reconstruct_pixels().unwrap();
    
    let max_error: i32 = input.iter()
        .zip(decoded.iter())
        .map(|(a, b)| (*a as i32 - *b as i32).abs())
        .max()
        .unwrap_or(0);
    
    println!("Solid pattern test: max_error = {}", max_error);
    assert_eq!(max_error, 0, "Solid pattern should have 0 error");
}

/// Debug test to trace coefficient flow
#[test]
fn test_debug_coefficient_flow() {
    use jpegexp_rs::jpeg2000::dwt::Dwt53;
    
    // Simple 4x4 gradient
    let input: Vec<i32> = vec![
        0, 85, 170, 255,
        0, 85, 170, 255,
        0, 85, 170, 255,
        0, 85, 170, 255,
    ];
    
    // Level shift (subtract 128 for 8-bit)
    let level_shifted: Vec<i32> = input.iter().map(|&v| v - 128).collect();
    
    println!("\n=== Level Shifted Input ===");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:4} ", level_shifted[y * 4 + x]);
        }
        println!();
    }
    
    // Forward DWT (matching encoder's apply_forward_dwt_2d)
    let width = 4;
    let height = 4;
    let mut result = level_shifted.clone();
    let current_w = width;
    let current_h = height;
    
    // 1 level decomposition
    // Row transform
    for y in 0..current_h {
        let row_start = y * width;
        let row: Vec<i32> = result[row_start..row_start + current_w].to_vec();
        
        let l_len = (current_w + 1) / 2;
        let h_len = current_w / 2;
        let mut out_l = vec![0i32; l_len];
        let mut out_h = vec![0i32; h_len];
        
        Dwt53::forward(&row, &mut out_l, &mut out_h);
        
        for (i, &v) in out_l.iter().enumerate() {
            result[row_start + i] = v;
        }
        for (i, &v) in out_h.iter().enumerate() {
            result[row_start + l_len + i] = v;
        }
    }
    
    println!("\n=== After Row DWT ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:4} ", result[y * width + x]);
        }
        println!();
    }
    
    // Column transform
    for x in 0..current_w {
        let col: Vec<i32> = (0..current_h).map(|y| result[y * width + x]).collect();
        
        let l_len = (current_h + 1) / 2;
        let h_len = current_h / 2;
        let mut out_l = vec![0i32; l_len];
        let mut out_h = vec![0i32; h_len];
        
        Dwt53::forward(&col, &mut out_l, &mut out_h);
        
        for (i, &v) in out_l.iter().enumerate() {
            result[i * width + x] = v;
        }
        for (i, &v) in out_h.iter().enumerate() {
            result[(l_len + i) * width + x] = v;
        }
    }
    
    println!("\n=== After Column DWT (full DWT result) ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:4} ", result[y * width + x]);
        }
        println!();
    }
    
    // Extract subbands as encoder would (get_ll_size logic)
    // For 1 level decomposition on 4x4:
    // - res=0 (LL): ll_w=2, ll_h=2
    // - res=1 (HL/LH/HH): ll_w=4, ll_h=4, prev_ll_w=2, prev_ll_h=2
    
    let ll_w = 2;
    let ll_h = 2;
    
    println!("\n=== Extracted Subbands ===");
    
    // LL: top-left 2x2
    println!("LL (2x2):");
    for y in 0..ll_h {
        for x in 0..ll_w {
            print!("{:4} ", result[y * width + x]);
        }
        println!();
    }
    
    // HL: top-right 2x2 (width 2, height 2, start_x=2, start_y=0)
    let hl_w = width - ll_w;  // 2
    let hl_h = ll_h;          // 2
    println!("HL ({}x{}, start at ({}, {})):", hl_w, hl_h, ll_w, 0);
    for y in 0..hl_h {
        for x in 0..hl_w {
            print!("{:4} ", result[y * width + (ll_w + x)]);
        }
        println!();
    }
    
    // LH: bottom-left 2x2 (width 2, height 2, start_x=0, start_y=2)
    let lh_w = ll_w;           // 2
    let lh_h = height - ll_h;  // 2
    println!("LH ({}x{}, start at ({}, {})):", lh_w, lh_h, 0, ll_h);
    for y in 0..lh_h {
        for x in 0..lh_w {
            print!("{:4} ", result[(ll_h + y) * width + x]);
        }
        println!();
    }
    
    // HH: bottom-right 2x2 (width 2, height 2, start_x=2, start_y=2)
    let hh_w = width - ll_w;   // 2
    let hh_h = height - ll_h;  // 2
    println!("HH ({}x{}, start at ({}, {})):", hh_w, hh_h, ll_w, ll_h);
    for y in 0..hh_h {
        for x in 0..hh_w {
            print!("{:4} ", result[(ll_h + y) * width + (ll_w + x)]);
        }
        println!();
    }
    
    // Now extract as separate vectors (matching extract_subband_coeffs)
    let mut ll = Vec::with_capacity(ll_w * ll_h);
    for y in 0..ll_h {
        for x in 0..ll_w {
            ll.push(result[y * width + x]);
        }
    }
    
    let mut hl = Vec::with_capacity(hl_w * hl_h);
    for y in 0..hl_h {
        for x in 0..hl_w {
            hl.push(result[y * width + (ll_w + x)]);
        }
    }
    
    let mut lh = Vec::with_capacity(lh_w * lh_h);
    for y in 0..lh_h {
        for x in 0..lh_w {
            lh.push(result[(ll_h + y) * width + x]);
        }
    }
    
    let mut hh = Vec::with_capacity(hh_w * hh_h);
    for y in 0..hh_h {
        for x in 0..hh_w {
            hh.push(result[(ll_h + y) * width + (ll_w + x)]);
        }
    }
    
    println!("\n=== As Vectors ===");
    println!("LL: {:?}", ll);
    println!("HL: {:?}", hl);
    println!("LH: {:?}", lh);
    println!("HH: {:?}", hh);
    
    // Now verify inverse DWT
    let mut reconstructed = vec![0i32; width * height];
    Dwt53::inverse_2d(&ll, &hl, &lh, &hh, width as u32, height as u32, &mut reconstructed);
    
    println!("\n=== After Inverse DWT ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:4} ", reconstructed[y * width + x]);
        }
        println!();
    }
    
    // Reverse level shift
    let final_output: Vec<i32> = reconstructed.iter().map(|&v| v + 128).collect();
    
    println!("\n=== After Level Shift Reversal ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", final_output[y * width + x]);
        }
        println!();
    }
    
    // Verify
    let max_error: i32 = input.iter()
        .zip(final_output.iter())
        .map(|(a, b)| (*a - *b).abs())
        .max()
        .unwrap_or(0);
    
    println!("\nMax reconstruction error: {}", max_error);
    assert_eq!(max_error, 0, "DWT roundtrip should be lossless");
}

// Test the bit-plane coder directly without full JPEG 2000 pipeline
use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

#[test]
fn test_bitplane_single_value() {
    // Test specific failing values
    let test_values = vec![91, 200, 218, 236, 255];
    
    for &val in &test_values {
        println!("\n=== Testing value {} (0b{:08b}) ===", val, val);
        
        // Create a 1x1 codeblock with this value
        let data = vec![val as i32];
        let mut coder = BitPlaneCoder::new(1, 1, &data);
        
        // Calculate max bit-plane
        let max_bp = coder.calculate_max_bit_plane().expect("Should have max BP");
        println!("Max bit-plane: {}", max_bp);
        
        // Encode
        let num_passes = coder.encode_codeblock(max_bp, 0, 0); // Orient 0 = LL
        println!("Number of passes: {}", num_passes);
        
        // Get encoded data
        coder.mq.flush();
        let encoded = coder.mq.get_buffer().to_vec();
        println!("Encoded bytes: {} bytes", encoded.len());
        println!("Hex: {}", encoded.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
        
        // Now decode it
        let mut decoder = BitPlaneCoder::new(1, 1, &[0]); // Empty data for decoder
        
        // Decode with proper parameters
        match decoder.decode_codeblock(&encoded, max_bp, num_passes, 0) {
            Ok(coefficients) => {
                println!("Decoded {} coefficients", coefficients.len());
                
                // Get the decoded coefficient
                let decoded_val = coefficients[0];
                println!("Decoded value: {} (0b{:08b})", decoded_val, decoded_val);
                
                if decoded_val == val as i32 {
                    println!("✓ PASS: Perfect match");
                } else {
                    let error = (val as i32 - decoded_val).abs();
                    println!("✗ FAIL: Error = {} (decoded as 0b{:08b})", error, decoded_val);
                    println!("  XOR: 0b{:08b}", (val as i32) ^ decoded_val);
                }
            },
            Err(e) => {
                println!("✗ DECODE ERROR: {:?}", e);
            }
        }
    }
}

#[test]
fn test_bitplane_8x8_gradient() {
    // Test the full 8x8 gradient in a single codeblock
    let width = 8;
    let height = 8;
    let mut data: Vec<i32> = vec![0; (width * height) as usize];
    
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height - 2)) as i32;
            data[(y * width + x) as usize] = val;
        }
    }
    
    println!("Original 8x8 gradient:");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", data[(y * width + x) as usize]);
        }
        println!();
    }
    
    let mut coder = BitPlaneCoder::new(width as u32, height as u32, &data);
    let max_bp = coder.calculate_max_bit_plane().expect("Should have max BP");
    println!("\nMax bit-plane: {}", max_bp);
    
    let num_passes = coder.encode_codeblock(max_bp, 0, 0);
    println!("Number of passes: {}", num_passes);
    
    coder.mq.flush();
    let encoded = coder.mq.get_buffer().to_vec();
    println!("Encoded: {} bytes", encoded.len());
    
    // Decode
    let dummy_data = vec![0; (width * height) as usize];
    let mut decoder = BitPlaneCoder::new(width as u32, height as u32, &dummy_data);
    
    match decoder.decode_codeblock(&encoded, max_bp, num_passes, 0) {
        Ok(coefficients) => {
            println!("Decoded {} coefficients\n", coefficients.len());
            
            println!("Decoded 8x8:");
            for y in 0..height {
                for x in 0..width {
                    let val = coefficients[(y * width + x) as usize];
                    print!("{:3} ", val);
                }
                println!();
            }
            
            // Compare
            println!("\nErrors:");
            let mut error_count = 0;
            for i in 0..(width * height) as usize {
                let orig = data[i];
                let dec = coefficients[i];
                if orig != dec {
                    error_count += 1;
                    println!("Pixel [{}]: {} (0b{:08b}) -> {} (0b{:08b}) ERROR", 
                             i, orig, orig, dec, dec);
                }
            }
            
            if error_count == 0 {
                println!("✓ All pixels match perfectly!");
            } else {
                println!("✗ {} pixels have errors", error_count);
            }
        },
        Err(e) => {
            println!("✗ DECODE ERROR: {:?}", e);
        }
    }
}

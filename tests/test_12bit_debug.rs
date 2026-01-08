use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_12bit_checkerboard_4x4_dwt1_debug() {
    std::env::set_var("J2K_DEBUG", "1");
    
    println!("\n================================================================================");
    println!("Testing 4x4 12-bit Checkerboard with DWT=1");
    println!("================================================================================\n");
    
    // Create 4x4 checkerboard: 0 and 4095 alternating
    let pixels_u16: Vec<u16> = (0..16)
        .map(|i| {
            let row = i / 4;
            let col = i % 4;
            if (row + col) % 2 == 0 {
                4095 // Max value for 12-bit
            } else {
                0
            }
        })
        .collect();
    
    println!("Original pixels:");
    for row in 0..4 {
        print!("  ");
        for col in 0..4 {
            print!("{:4} ", pixels_u16[row * 4 + col]);
        }
        println!();
    }
    println!();
    
    // Convert to bytes (little-endian)
    let mut pixels_bytes = vec![0u8; pixels_u16.len() * 2];
    for (i, &val) in pixels_u16.iter().enumerate() {
        pixels_bytes[i * 2] = (val & 0xFF) as u8;
        pixels_bytes[i * 2 + 1] = (val >> 8) as u8;
    }
    
    let frame_info = FrameInfo {
        width: 4,
        height: 4,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    // Encode
    let mut output = vec![0u8; 4 * 4 * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    println!("Encoding with DWT=1...");
    let result = encoder.encode(&pixels_bytes, &frame_info, &mut output);
    
    if let Err(e) = result {
        println!("❌ ENCODING FAILED: {:?}", e);
        panic!("Encoding failed");
    }
    
    let encoded_len = result.unwrap();
    output.truncate(encoded_len);
    println!("✅ Encoded successfully, size = {} bytes\n", output.len());
    
    // Decode
    println!("Decoding...");
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    
    let image = decoder.decode().expect("Failed to decode");
    println!("✅ Decoded image: {}x{} with {} components\n", 
             image.width, image.height, image.components.len());
    
    let decoded_bytes = image.reconstruct_pixels().expect("Failed to reconstruct");
    
    // Convert bytes back to u16
    let mut decoded_u16 = vec![0u16; 16];
    for i in 0..16 {
        decoded_u16[i] = (decoded_bytes[i * 2] as u16) | ((decoded_bytes[i * 2 + 1] as u16) << 8);
    }
    
    println!("Decoded pixels:");
    for row in 0..4 {
        print!("  ");
        for col in 0..4 {
            print!("{:4} ", decoded_u16[row * 4 + col]);
        }
        println!();
    }
    println!();
    
    // Calculate error
    let mut total_error = 0.0;
    let mut max_error = 0;
    
    for i in 0..16 {
        let original = pixels_u16[i];
        let decoded_val = decoded_u16[i];
        let error = (original as i32 - decoded_val as i32).abs();
        total_error += error as f64;
        max_error = max_error.max(error);
        
        if error > 0 {
            println!("  Position {}: original={} decoded={} error={}", 
                     i, original, decoded_val, error);
        }
    }
    
    let mae = total_error / 16.0;
    println!("\nMAE = {:.6}", mae);
    println!("Max Error = {}", max_error);
    
    if mae > 0.01 {
        println!("\n❌ TEST FAILED: MAE too high");
        panic!("MAE = {}, expected 0", mae);
    } else {
        println!("\n✅ TEST PASSED");
    }
}

#[test]
fn test_12bit_checkerboard_8x8_dwt1_debug() {
    std::env::set_var("J2K_DEBUG", "1");
    
    println!("\n================================================================================");
    println!("Testing 8x8 12-bit Checkerboard with DWT=1");
    println!("================================================================================\n");
    
    // Create 8x8 checkerboard
    let pixels_u16: Vec<u16> = (0..64)
        .map(|i| {
            let row = i / 8;
            let col = i % 8;
            if (row + col) % 2 == 0 {
                4095
            } else {
                0
            }
        })
        .collect();
    
    // Convert to bytes
    let mut pixels_bytes = vec![0u8; pixels_u16.len() * 2];
    for (i, &val) in pixels_u16.iter().enumerate() {
        pixels_bytes[i * 2] = (val & 0xFF) as u8;
        pixels_bytes[i * 2 + 1] = (val >> 8) as u8;
    }
    
    let frame_info = FrameInfo {
        width: 8,
        height: 8,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    // Encode
    let mut output = vec![0u8; 8 * 8 * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    println!("Encoding 8x8 checkerboard with DWT=1...");
    let result = encoder.encode(&pixels_bytes, &frame_info, &mut output);
    
    if let Err(e) = result {
        println!("❌ ENCODING FAILED: {:?}", e);
        panic!("Encoding failed");
    }
    
    let encoded_len = result.unwrap();
    output.truncate(encoded_len);
    println!("✅ Encoded successfully, size = {} bytes\n", output.len());
    
    // Decode
    println!("Decoding...");
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    
    let image = decoder.decode().expect("Failed to decode");
    let decoded_bytes = image.reconstruct_pixels().expect("Failed to reconstruct");
    
    // Convert bytes back to u16
    let mut decoded_u16 = vec![0u16; 64];
    for i in 0..64 {
        decoded_u16[i] = (decoded_bytes[i * 2] as u16) | ((decoded_bytes[i * 2 + 1] as u16) << 8);
    }
    
    // Calculate error
    let mut total_error = 0.0;
    let mut errors_found = Vec::new();
    
    for i in 0..64 {
        let original = pixels_u16[i];
        let decoded_val = decoded_u16[i];
        let error = (original as i32 - decoded_val as i32).abs();
        total_error += error as f64;
        
        if error > 0 {
            errors_found.push((i, original, decoded_val, error));
        }
    }
    
    let mae = total_error / 64.0;
    println!("\nMAE = {:.6}", mae);
    
    if !errors_found.is_empty() {
        println!("Found {} pixels with errors:", errors_found.len());
        for (i, orig, dec, err) in errors_found.iter().take(10) {
            println!("  Pos {}: orig={} dec={} err={}", i, orig, dec, err);
        }
        if errors_found.len() > 10 {
            println!("  ... and {} more", errors_found.len() - 10);
        }
    }
    
    if mae > 0.01 {
        println!("\n❌ TEST FAILED: MAE = {}", mae);
        panic!("MAE too high");
    } else {
        println!("\n✅ TEST PASSED");
    }
}

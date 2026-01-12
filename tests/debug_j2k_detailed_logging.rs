// Detailed logging test to find exact divergence point
// This test creates a minimal 4x4 gradient and logs every bit-plane coding operation
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

#[test]
#[ignore]
fn debug_4x4_gradient_detailed() {
    // Even smaller: 4x4 gradient
    let width = 4;
    let height = 4;
    let mut pixels: Vec<u8> = vec![0; (width * height) as usize];
    
    // Create specific values that trigger ±1 errors: 91, 200, 218, 236
    // Let's use a controlled pattern
    pixels[0] = 0;    // 0b00000000
    pixels[1] = 91;   // 0b01011011 - Known to fail
    pixels[2] = 127;  // 0b01111111
    pixels[3] = 255;  // 0b11111111
    
    pixels[4] = 64;   // 0b01000000
    pixels[5] = 128;  // 0b10000000
    pixels[6] = 192;  // 0b11000000
    pixels[7] = 200;  // 0b11001000 - Known to fail
    
    pixels[8] = 32;   // 0b00100000
    pixels[9] = 96;   // 0b01100000
    pixels[10] = 160; // 0b10100000
    pixels[11] = 218; // 0b11011010 - Known to fail
    
    pixels[12] = 16;  // 0b00010000
    pixels[13] = 48;  // 0b00110000
    pixels[14] = 80;  // 0b01010000
    pixels[15] = 236; // 0b11101100 - Known to fail
    
    println!("Test pattern 4x4 (includes known failing values):");
    for y in 0..height {
        for x in 0..width {
            let val = pixels[(y * width + x) as usize];
            print!("{:3} (0b{:08b})  ", val, val);
        }
        println!();
    }
    
    // Encode with our encoder
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(1); // Use fewer levels for 4x4 image
    
    let mut encoded = vec![0u8; (width * height * 4) as usize];
    let encoded_len = match encoder.encode(&pixels, &frame_info, &mut encoded) {
        Ok(len) => len,
        Err(e) => {
            panic!("Encoding failed with error: {:?}", e);
        }
    };
    encoded.truncate(encoded_len);
    
    println!("\n=== Encoded to {} bytes ===", encoded_len);
    
    // Write to file for OpenJPEG
    fs::write("test_gradient_4x4_rust.j2k", &encoded).unwrap();
    
    // Decode with OpenJPEG
    let status = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_gradient_4x4_rust.j2k", "-o", "test_gradient_4x4_rust_opj.pgm"])
        .output()
        .expect("Failed to run opj_decompress");
    
    if !status.status.success() {
        println!("OpenJPEG decode failed: {}", String::from_utf8_lossy(&status.stderr));
        panic!("OpenJPEG failed to decode");
    }
    
    // Read back the PGM
    let pgm_data = fs::read("test_gradient_4x4_rust_opj.pgm").unwrap();
    
    // Parse PGM (skip P5 header)
    let mut lines = pgm_data.split(|&b| b == b'\n');
    let _magic = lines.next().unwrap();
    let mut dims_line = lines.next().unwrap();
    while dims_line.starts_with(b"#") {
        dims_line = lines.next().unwrap();
    }
    let _maxval = lines.next().unwrap();
    let decoded_pixels: Vec<u8> = lines.flatten().copied().collect();
    
    println!("\n=== Decoded by OpenJPEG ===");
    for y in 0..height {
        for x in 0..width {
            if (y * width + x) < decoded_pixels.len() as u32 {
                let val = decoded_pixels[(y * width + x) as usize];
                print!("{:3} (0b{:08b})  ", val, val);
            }
        }
        println!();
    }
    
    // Compare bit-by-bit
    println!("\n=== Bit-by-bit comparison ===");
    let mut total_error = 0.0;
    let mut max_error = 0i32;
    let mut error_count = 0;
    
    for i in 0..(width * height).min(decoded_pixels.len() as u32) {
        let orig = pixels[i as usize];
        let dec = decoded_pixels[i as usize];
        let error = (orig as i32 - dec as i32).abs();
        total_error += error as f64;
        max_error = max_error.max(error);
        
        if error > 0 {
            error_count += 1;
            let xor = orig ^ dec;
            println!("Pixel [{}]: {} (0b{:08b}) -> {} (0b{:08b})", 
                     i, orig, orig, dec, dec);
            println!("           XOR: 0b{:08b} (bit positions that differ)", xor);
            
            // Show which bit positions differ
            for bit in 0..8 {
                if (xor & (1 << bit)) != 0 {
                    println!("           Bit {} differs (weight {})", bit, 1 << bit);
                }
            }
        }
    }
    
    let mae = total_error / (width * height) as f64;
    println!("\n=== Summary ===");
    println!("MAE: {:.4}", mae);
    println!("Max Error: {}", max_error);
    println!("Error count: {}/{}", error_count, width * height);
    println!("Pass rate: {:.1}%", 100.0 * (1.0 - error_count as f64 / (width * height) as f64));
    
    // Don't clean up - keep files for analysis
    println!("\nFiles kept for analysis:");
    println!("  test_gradient_4x4_rust.j2k");
    println!("  test_gradient_4x4_rust_opj.pgm");
    
    if mae > 0.0 {
        println!("\n⚠️  OpenJPEG decoded our output with errors (MAE={:.4})", mae);
        println!("This is expected - we're investigating the root cause.");
    }
}

#[test]
#[ignore]
fn debug_single_value_sweep() {
    // Test individual values around known failures (91, 200, 218, 236)
    let test_values = vec![
        90, 91, 92,  // Around first failure
        199, 200, 201, // Around second failure
        217, 218, 219, // Around third failure
        235, 236, 237, // Around fourth failure
    ];
    
    let width = 1;
    let height = 1;
    
    println!("=== Testing individual values ===\n");
    
    for &test_val in &test_values {
        let pixels = vec![test_val];
        
        let frame_info = FrameInfo {
            width,
            height,
            bits_per_sample: 8,
            component_count: 1,
        };
        
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false); // Lossless
        encoder.set_decomposition_levels(0); // No DWT for 1x1 pixel
        
        let mut encoded = vec![0u8; 1024];
        let encoded_len = encoder.encode(&pixels, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let filename = format!("test_single_{}.j2k", test_val);
        fs::write(&filename, &encoded).unwrap();
        
        // Decode with OpenJPEG
        let pgm_filename = format!("test_single_{}_opj.pgm", test_val);
        let status = Command::new("libs/bin/opj_decompress.exe")
            .args(&["-i", &filename, "-o", &pgm_filename])
            .output()
            .expect("Failed to run opj_decompress");
        
        if !status.status.success() {
            println!("Value {}: OpenJPEG decode FAILED", test_val);
            continue;
        }
        
        // Read back the PGM
        let pgm_data = fs::read(&pgm_filename).unwrap();
        let mut lines = pgm_data.split(|&b| b == b'\n');
        let _magic = lines.next().unwrap();
        let mut dims_line = lines.next().unwrap();
        while dims_line.starts_with(b"#") {
            dims_line = lines.next().unwrap();
        }
        let _maxval = lines.next().unwrap();
        let decoded_pixels: Vec<u8> = lines.flatten().copied().collect();
        
        let decoded = decoded_pixels[0];
        let error = (test_val as i32 - decoded as i32).abs();
        
        let status_str = if error == 0 { "✓ PASS" } else { "✗ FAIL" };
        println!("Value {:3} (0b{:08b}): decoded as {:3} (0b{:08b}) - {} (error={})",
                 test_val, test_val, decoded, decoded, status_str, error);
        
        if error > 0 {
            let xor = test_val ^ decoded;
            println!("           XOR: 0b{:08b}", xor);
            for bit in 0..8 {
                if (xor & (1 << bit)) != 0 {
                    println!("           Bit {} differs", bit);
                }
            }
        }
        
        // Clean up individual test files
        let _ = fs::remove_file(&filename);
        let _ = fs::remove_file(&pgm_filename);
    }
}

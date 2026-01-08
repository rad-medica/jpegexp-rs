//! Find the exact size where 12-bit checkerboard starts failing

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_12bit_checkerboard_size_progression() {
    println!("\n{}", "=".repeat(80));
    println!("Testing 12-bit Checkerboard at Different Sizes");
    println!("{}", "=".repeat(80));
    
    let sizes = vec![4, 8, 16, 32, 64];
    
    for &size in &sizes {
        let mae = test_size(size, size, 0); // DWT = 0
        let status = if mae == 0.0 { "✅ PASS" } else { "❌ FAIL" };
        println!("  {}x{} DWT=0: MAE={:.6} {}", size, size, mae, status);
    }
    
    println!("\nTesting with different DWT levels at 64x64:");
    for dwt in 0..=5 {
        let mae = test_size(64, 64, dwt);
        let status = if mae == 0.0 { "✅ PASS" } else { "❌ FAIL" };
        println!("  64x64 DWT={}: MAE={:.6} {}", dwt, mae, status);
    }
}

fn test_size(width: u32, height: u32, dwt_levels: u8) -> f64 {
    // Create checkerboard
    let mut pixels_u16 = vec![0u16; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let is_white = (x + y) % 2 == 0;
            pixels_u16[(y * width + x) as usize] = if is_white { 4095 } else { 0 };
        }
    }
    
    // Convert to bytes
    let mut pixels_bytes = vec![0u8; pixels_u16.len() * 2];
    for (i, &val) in pixels_u16.iter().enumerate() {
        pixels_bytes[i * 2] = (val & 0xFF) as u8;
        pixels_bytes[i * 2 + 1] = (val >> 8) as u8;
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    // Encode
    let mut encoded = vec![0u8; (width * height * 4) as usize];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(dwt_levels);
    
    let encoded_len = match encoder.encode(&pixels_bytes, &frame_info, &mut encoded) {
        Ok(len) => len,
        Err(e) => {
            println!("  {}x{} DWT={}: Encoding failed: {}", width, height, dwt_levels, e);
            return f64::MAX;
        }
    };
    encoded.truncate(encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = match decoder.decode() {
        Ok(img) => img,
        Err(e) => {
            println!("  {}x{} DWT={}: Decoding failed: {}", width, height, dwt_levels, e);
            return f64::MAX;
        }
    };
    
    let reconstructed_bytes = match image.reconstruct_pixels() {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("  {}x{} DWT={}: Reconstruction failed: {}", width, height, dwt_levels, e);
            return f64::MAX;
        }
    };
    
    let reconstructed_u16: Vec<u16> = reconstructed_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    
    // MAE
    let mae: f64 = pixels_u16.iter()
        .zip(reconstructed_u16.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).abs() as f64)
        .sum::<f64>() / pixels_u16.len() as f64;
    
    mae
}

#[test]
fn test_12bit_checkerboard_8x8_square_sizes() {
    println!("\n{}", "=".repeat(80));
    println!("Testing 12-bit Checkerboard with 8x8 Square Sizes (DWT > 0)");
    println!("{}", "=".repeat(80));
    println!("Hypothesis: Larger checkerboard squares should work\n");
    
    // Test different square sizes in checkerboard pattern
    for square_size in vec![1, 2, 4, 8, 16] {
        let width = 64u32;
        let height = 64u32;
        
        let mut pixels_u16 = vec![0u16; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let square_x = x / square_size;
                let square_y = y / square_size;
                let is_white = (square_x + square_y) % 2 == 0;
                pixels_u16[(y * width + x) as usize] = if is_white { 4095 } else { 0 };
            }
        }
        
        let mae = test_pattern_12bit(&pixels_u16, width, height, 3);
        let status = if mae == 0.0 { "✅ PASS" } else { "❌ FAIL" };
        println!("  Square size {}x{}: MAE={:.6} {}", square_size, square_size, mae, status);
    }
}

fn test_pattern_12bit(pixels_u16: &[u16], width: u32, height: u32, dwt_levels: u8) -> f64 {
    // Convert to bytes
    let mut pixels_bytes = vec![0u8; pixels_u16.len() * 2];
    for (i, &val) in pixels_u16.iter().enumerate() {
        pixels_bytes[i * 2] = (val & 0xFF) as u8;
        pixels_bytes[i * 2 + 1] = (val >> 8) as u8;
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    // Encode
    let mut encoded = vec![0u8; (width * height * 4) as usize];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(dwt_levels);
    
    let encoded_len = encoder.encode(&pixels_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = image.reconstruct_pixels().expect("Reconstruction failed");
    
    let reconstructed_u16: Vec<u16> = reconstructed_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    
    // MAE
    pixels_u16.iter()
        .zip(reconstructed_u16.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).abs() as f64)
        .sum::<f64>() / pixels_u16.len() as f64
}

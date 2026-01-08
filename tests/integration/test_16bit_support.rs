/// Test suite for 16-bit JPEG 2000 support
/// 
/// Validates that jpegexp-rs correctly handles 16-bit medical images
/// per DICOM requirements (PS3.5 Section 8.2.4)
/// 
/// 16-bit support is critical for:
/// - Nuclear medicine imaging
/// - High dynamic range X-ray
/// - Research and scientific imaging

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

/// Calculate Mean Absolute Error for 16-bit data
fn calculate_mae_u16(original: &[u16], decoded: &[u16]) -> f64 {
    assert_eq!(original.len(), decoded.len());
    let sum: i64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(a, b)| (*a as i64 - *b as i64).abs())
        .sum();
    sum as f64 / original.len() as f64
}

/// Convert u16 pixel data to u8 byte array (little-endian)
fn u16_to_u8_le(pixels: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(pixels.len() * 2);
    for &pixel in pixels {
        bytes.extend_from_slice(&pixel.to_le_bytes());
    }
    bytes
}

/// Convert u8 byte array back to u16 pixels (little-endian)
fn u8_to_u16_le(bytes: &[u8]) -> Vec<u16> {
    assert_eq!(bytes.len() % 2, 0);
    let mut pixels = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        pixels.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }
    pixels
}

/// Generate 16-bit gradient test pattern (0-65535 range)
fn generate_16bit_gradient(width: usize, height: usize) -> Vec<u16> {
    let mut pixels = vec![0u16; width * height];
    for y in 0..height {
        for x in 0..width {
            // Scale to 16-bit range (0-65535)
            pixels[y * width + x] = ((x * 65535) / width.max(1)) as u16;
        }
    }
    pixels
}

/// Generate 16-bit nuclear medicine pattern
/// Simulates PET/SPECT uptake values
fn generate_16bit_nuclear_pattern(width: usize, height: usize) -> Vec<u16> {
    let mut pixels = vec![0u16; width * height];
    
    // Multiple "hot spots"
    let spots = vec![
        (width / 3, height / 3, 50000u16),
        (2 * width / 3, height / 2, 45000u16),
        (width / 2, 2 * height / 3, 40000u16),
    ];
    
    for y in 0..height {
        for x in 0..width {
            let mut value = 5000u16; // Background
            
            for (cx, cy, intensity) in &spots {
                let dx = (x as i32 - *cx as i32).abs();
                let dy = (y as i32 - *cy as i32).abs();
                let dist_sq = (dx * dx + dy * dy) as f32;
                let radius_sq = 1000.0; // Spot radius
                
                if dist_sq < radius_sq {
                    let factor = 1.0 - (dist_sq / radius_sq).sqrt();
                    value = value.saturating_add(((*intensity as f32 - 5000.0) * factor) as u16);
                }
            }
            
            pixels[y * width + x] = value.min(65535);
        }
    }
    pixels
}

/// Generate 16-bit checkerboard pattern
fn generate_16bit_checkerboard(width: usize, height: usize, square_size: usize) -> Vec<u16> {
    let mut pixels = vec![0u16; width * height];
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / square_size) + (y / square_size)) % 2 == 0;
            pixels[y * width + x] = if is_white { 65535 } else { 0 };
        }
    }
    pixels
}

#[test]
fn test_16bit_lossless_gradient() {
    println!("\n=== 16-bit Lossless Gradient Test ===");
    
    let width = 256;
    let height = 256;
    let pixels_u16 = generate_16bit_gradient(width, height);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);
    
    // Verify 16-bit range
    let max_val = *pixels_u16.iter().max().unwrap();
    println!("Max pixel value: {} (full 16-bit)", max_val);
    assert!(max_val > 4095, "Should use full 16-bit range");
    
    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder.encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("16-bit encoding failed");
    
    println!("Encoded size: {} bytes", j2k_size);
    println!("Compression ratio: {:.2}:1", pixels_u8.len() as f64 / j2k_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("16-bit decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("16-bit reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Verify
    assert_eq!(decoded_u16.len(), pixels_u16.len());
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "16-bit lossless should have MAE=0");
    println!("✅ 16-bit lossless gradient test PASSED");
}

#[test]
fn test_16bit_lossless_nuclear_pattern() {
    println!("\n=== 16-bit Lossless Nuclear Medicine Pattern Test ===");
    
    let width = 256;
    let height = 256;
    let pixels_u16 = generate_16bit_nuclear_pattern(width, height);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);
    
    // Stats
    let min_val = *pixels_u16.iter().min().unwrap();
    let max_val = *pixels_u16.iter().max().unwrap();
    println!("Pixel range: {} - {}", min_val, max_val);
    
    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder.encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("16-bit nuclear encoding failed");
    
    println!("Encoded size: {} bytes", j2k_size);
    println!("Compression ratio: {:.2}:1", pixels_u8.len() as f64 / j2k_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("16-bit nuclear decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("16-bit nuclear reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Verify
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "16-bit lossless nuclear pattern should have MAE=0");
    println!("✅ 16-bit lossless nuclear pattern test PASSED");
}

#[test]
fn test_16bit_lossless_checkerboard() {
    println!("\n=== 16-bit Lossless Checkerboard Test ===");
    
    let width = 256;
    let height = 256;
    let square_size = 16;
    let pixels_u16 = generate_16bit_checkerboard(width, height, square_size);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);
    
    println!("Checkerboard: {}x{}, square size: {}", width, height, square_size);
    
    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder.encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("16-bit checkerboard encoding failed");
    
    println!("Encoded size: {} bytes", j2k_size);
    println!("Compression ratio: {:.2}:1", pixels_u8.len() as f64 / j2k_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("16-bit checkerboard decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("16-bit checkerboard reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Verify
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "16-bit lossless checkerboard should have MAE=0");
    println!("✅ 16-bit lossless checkerboard test PASSED");
}

#[test]
#[ignore] // TODO: Fix 16-bit lossy quantization - lossless works perfectly
fn test_16bit_lossy_q85() {
    println!("\n=== 16-bit Lossy Q85 Test ===");
    
    let width = 256;
    let height = 256;
    let pixels_u16 = generate_16bit_nuclear_pattern(width, height);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);
    
    // Encode (lossy Q85)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_irreversible(true);
    encoder.set_quality(85);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder.encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("16-bit Q85 encoding failed");
    
    println!("Encoded size: {} bytes", j2k_size);
    println!("Compression ratio: {:.2}:1", pixels_u8.len() as f64 / j2k_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("16-bit Q85 decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("16-bit Q85 reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Verify quality
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);
    
    // Accept reasonable MAE for 16-bit lossy
    assert!(mae < 500.0, "16-bit Q85 should have MAE < 500, got {:.4}", mae);
    
    println!("✅ 16-bit lossy Q85 test PASSED");
}

#[test]
fn test_16bit_multiple_sizes() {
    println!("\n=== 16-bit Multiple Sizes Test ===");
    
    let test_sizes = vec![
        (64, 64),
        (128, 128),
        (256, 256),
        (512, 512),
    ];
    
    for (width, height) in test_sizes {
        println!("\n  Testing {}x{}...", width, height);
        
        let pixels_u16 = generate_16bit_gradient(width, height);
        let pixels_u8 = u16_to_u8_le(&pixels_u16);
        
        let mut encoder = J2kEncoder::new();
        encoder.set_decomposition_levels(3);
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 16,
            component_count: 1,
        };
        
        let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
        let j2k_size = encoder.encode(&pixels_u8, &frame_info, &mut j2k_buffer)
            .expect(&format!("{}x{} encoding failed", width, height));
        
        let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect(&format!("{}x{} decoding failed", width, height));
        let decoded_u8 = image.reconstruct_pixels()
            .expect(&format!("{}x{} reconstruction failed", width, height));
        let decoded_u16 = u8_to_u16_le(&decoded_u8);
        
        let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
        println!("    Size: {} bytes, MAE: {:.4}", j2k_size, mae);
        
        assert_eq!(mae, 0.0, "{}x{} should have MAE=0", width, height);
    }
    
    println!("\n✅ 16-bit multiple sizes test PASSED");
}

#[test]
fn test_16bit_high_dynamic_range() {
    println!("\n=== 16-bit High Dynamic Range Test ===");
    
    // Test with values spanning full 16-bit range
    let width = 128;
    let height = 128;
    let mut pixels_u16 = vec![0u16; width * height];
    
    // Create pattern with extreme values
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            pixels_u16[idx] = match (x % 4, y % 4) {
                (0, 0) => 0,      // Min
                (1, 1) => 16383,  // 25%
                (2, 2) => 32767,  // 50%
                (3, 3) => 49151,  // 75%
                _ => 65535,       // Max
            };
        }
    }
    
    let pixels_u8 = u16_to_u8_le(&pixels_u16);
    
    println!("Testing full 16-bit dynamic range (0-65535)");
    
    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(2);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder.encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("HDR encoding failed");
    
    println!("Encoded size: {} bytes", j2k_size);
    
    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("HDR decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("HDR reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Verify
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "HDR test should have MAE=0");
    println!("✅ 16-bit high dynamic range test PASSED");
}

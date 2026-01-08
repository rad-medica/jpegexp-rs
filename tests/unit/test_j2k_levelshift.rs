//! Test JPEG 2000 Level Shift Logic
//!
//! This test verifies that our level shift implementation matches OpenJPEG's behavior.

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn calculate_mae(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        return f64::MAX;
    }
    let sum: u64 = a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

#[test]
fn test_level_shift_simple_8bit() {
    // Create a simple test pattern: solid gray (128)
    let width = 8u32;
    let height = 8u32;
    let gray_value = 128u8;
    
    let original = vec![gray_value; (width * height) as usize];
    
    println!("Original pixels (first 16): {:?}", &original[0..16]);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded to {} bytes", encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    println!("Reconstructed pixels (first 16): {:?}", &reconstructed[0..16]);
    
    let mae = calculate_mae(&original, &reconstructed);
    println!("MAE: {:.6}", mae);
    
    assert_eq!(mae, 0.0, "Level shift roundtrip should be perfect for solid color");
}

#[test]
fn test_level_shift_gradient_8bit() {
    // Create gradient: 0, 1, 2, ..., 63
    let width = 8u32;
    let height = 8u32;
    
    let original: Vec<u8> = (0..(width * height) as u8).collect();
    
    println!("Original pixels (first 16): {:?}", &original[0..16]);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded to {} bytes", encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    println!("Reconstructed pixels (first 16): {:?}", &reconstructed[0..16]);
    println!("Expected pixels (first 16): {:?}", &original[0..16]);
    
    let mae = calculate_mae(&original, &reconstructed);
    println!("MAE: {:.6}", mae);
    
    // Show differences
    for i in 0..16 {
        if original[i] != reconstructed[i] {
            println!("  Pixel {}: {} -> {} (diff: {})", 
                i, original[i], reconstructed[i], 
                (original[i] as i32 - reconstructed[i] as i32).abs());
        }
    }
    
    assert_eq!(mae, 0.0, "Level shift roundtrip should be perfect for gradient");
}

#[test]
fn test_level_shift_extremes_8bit() {
    // Test extreme values: 0, 255
    let width = 8u32;
    let height = 8u32;
    
    let mut original = vec![0u8; (width * height) as usize];
    // Fill half with 0, half with 255
    for i in 0..32 {
        original[i] = 0;
        original[i + 32] = 255;
    }
    
    println!("Original pixels (first 8, last 8): {:?} ... {:?}", 
        &original[0..8], &original[56..64]);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded to {} bytes", encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    println!("Reconstructed pixels (first 8, last 8): {:?} ... {:?}", 
        &reconstructed[0..8], &reconstructed[56..64]);
    
    let mae = calculate_mae(&original, &reconstructed);
    println!("MAE: {:.6}", mae);
    
    assert_eq!(mae, 0.0, "Level shift roundtrip should handle extremes correctly");
}

#[test]
fn test_level_shift_12bit() {
    // Test 12-bit: solid mid-gray (2048)
    let width = 8u32;
    let height = 8u32;
    let gray_value = 2048u16;
    
    let mut original = vec![0u8; (width * height * 2) as usize];
    for i in 0..(width * height) as usize {
        original[i * 2] = (gray_value & 0xFF) as u8;
        original[i * 2 + 1] = (gray_value >> 8) as u8;
    }
    
    println!("Original 12-bit value: {}", gray_value);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded to {} bytes", encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    // Check first pixel
    let recon_value = reconstructed[0] as u16 | ((reconstructed[1] as u16) << 8);
    println!("Reconstructed 12-bit value: {}", recon_value);
    println!("Difference: {}", (gray_value as i32 - recon_value as i32).abs());
    
    let mae = calculate_mae(&original, &reconstructed);
    println!("MAE: {:.6}", mae);
    
    assert_eq!(mae, 0.0, "Level shift roundtrip should be perfect for 12-bit");
}

/// Test to reproduce and fix JPEG 2000 16-bit endianness issue
///
/// Current Status: MAE ~19,491 (SEVERE)
/// Expected: MAE = 0 (perfect match)
///
/// This test validates that 16-bit grayscale images roundtrip correctly
/// through JPEG 2000 lossless compression.

use jpegexp_rs::FrameInfo;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;

fn generate_gradient_16bit(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 2);
    let max_value = 65535u16;
    
    for i in 0..(width * height) {
        let ratio = i as f64 / (width * height).max(1) as f64;
        let value = (ratio * max_value as f64) as u16;
        
        // Store as Little Endian (native on x86)
        pixels.push((value & 0xFF) as u8);        // Low byte
        pixels.push((value >> 8) as u8);          // High byte
    }
    
    pixels
}

fn calculate_mae_16bit(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    assert_eq!(a.len() % 2, 0);
    
    let count = a.len() / 2;
    let mut sum_abs_diff = 0u64;
    
    for i in 0..count {
        // Read as Little Endian (native format)
        let val_a = u16::from_le_bytes([a[i * 2], a[i * 2 + 1]]) as i32;
        let val_b = u16::from_le_bytes([b[i * 2], b[i * 2 + 1]]) as i32;
        sum_abs_diff += (val_a - val_b).abs() as u64;
    }
    
    sum_abs_diff as f64 / count as f64
}

#[test]
fn test_j2k_16bit_lossless_roundtrip_minimal() {
    // Use very small image to isolate issue
    let width = 4;
    let height = 4;
    
    // Generate test pattern
    let original_pixels = generate_gradient_16bit(width, height);
    
    println!("Original pixels (first 8 bytes): {:02X?}", &original_pixels[0..8]);
    
    // Create frame info
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 16,
    };
    
    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_quality(100); // Lossless
    
    let mut encoded = vec![0u8; width * height * 4]; // Oversized buffer
    let encoded_size = encoder.encode(&original_pixels, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_size);
    
    println!("Encoded size: {} bytes", encoded_size);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");
    
    println!("Decoded pixels (first 8 bytes): {:02X?}", &decoded_pixels[0..8]);
    
    // Compare
    assert_eq!(original_pixels.len(), decoded_pixels.len(), "Length mismatch");
    
    let mae = calculate_mae_16bit(&original_pixels, &decoded_pixels);
    
    println!("MAE: {:.4}", mae);
    
    // Print first few pixel comparisons for debugging
    for i in 0..4 {
        let orig = u16::from_le_bytes([original_pixels[i * 2], original_pixels[i * 2 + 1]]);
        let dec = u16::from_le_bytes([decoded_pixels[i * 2], decoded_pixels[i * 2 + 1]]);
        println!("Pixel {}: orig={} dec={} diff={}", i, orig, dec, (orig as i32 - dec as i32).abs());
    }
    
    assert!(mae < 0.001, "MAE {} is too high (expected 0.0 for lossless)", mae);
}

#[test]
fn test_j2k_16bit_lossless_roundtrip_256x256() {
    let width = 256;
    let height = 256;
    
    let original_pixels = generate_gradient_16bit(width, height);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 16,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_quality(100); // Lossless
    
    let mut encoded = vec![0u8; width * height * 4];
    let encoded_size = encoder.encode(&original_pixels, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_size);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");
    
    assert_eq!(original_pixels.len(), decoded_pixels.len());
    
    let mae = calculate_mae_16bit(&original_pixels, &decoded_pixels);
    
    println!("256x256 16-bit roundtrip MAE: {:.4}", mae);
    
    assert!(mae < 0.001, "MAE {} is too high for lossless compression", mae);
}

#[test]
fn test_j2k_16bit_constant_values() {
    // Test edge cases with constant values
    let test_values = [0u16, 1, 255, 256, 32767, 32768, 65534, 65535];
    
    for &value in &test_values {
        let width = 8;
        let height = 8;
        
        let mut pixels = Vec::with_capacity(width * height * 2);
        for _ in 0..(width * height) {
            pixels.push((value & 0xFF) as u8);
            pixels.push((value >> 8) as u8);
        }
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            component_count: 1,
            bits_per_sample: 16,
        };
        
        let mut encoder = J2kEncoder::new();
        encoder.set_quality(100);
        
        let mut encoded = vec![0u8; width * height * 4];
        let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded)
            .expect(&format!("Encoding failed for value {}", value));
        encoded.truncate(encoded_size);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect(&format!("Decoding failed for value {}", value));
        let decoded_pixels = image.reconstruct_pixels()
            .expect(&format!("Reconstruction failed for value {}", value));
        
        let mae = calculate_mae_16bit(&pixels, &decoded_pixels);
        
        assert!(
            mae < 0.001,
            "Constant value {} failed: MAE = {:.4}",
            value, mae
        );
    }
    
    println!("All constant value tests passed ✅");
}

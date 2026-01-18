//! Signed Pixel Data Support Tests
//!
//! Tests for DICOM Pixel Representation = 1 (signed/two's complement) support.
//! This is a HIGH PRIORITY requirement from docs/DICOM_J2K_REQUIREMENTS.md.
//!
//! DICOM Standard PS3.5 Section 8.1.1:
//! - Pixel Representation (0028,0103) = 1 indicates signed integers
//! - Two's complement representation for negative values
//! - Common in CT imaging (Hounsfield Units: -1024 to +3071)
//!
//! Test coverage:
//! - 8-bit signed (-128 to +127)
//! - 12-bit signed (-2048 to +2047)
//! - 16-bit signed (-32768 to +32767)
//! - CT Hounsfield Units medical use case
//! - Lossless roundtrip validation

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

/// Helper: Convert u16 to little-endian bytes
fn u16_to_u8_le(pixels: &[u16]) -> Vec<u8> {
    pixels
        .iter()
        .flat_map(|&p| p.to_le_bytes().to_vec())
        .collect()
}

/// Helper: Convert little-endian bytes to u16
fn u8_to_u16_le(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

/// Helper: Convert signed i16 to unsigned u16 (offset by 2^(depth-1))
fn signed_to_unsigned(signed: &[i16], depth: u8) -> Vec<u16> {
    let offset = 1i32 << (depth - 1);
    signed
        .iter()
        .map(|&val| (val as i32 + offset) as u16)
        .collect()
}

/// Helper: Convert unsigned u16 back to signed i16
fn unsigned_to_signed(unsigned: &[u16], depth: u8) -> Vec<i16> {
    let offset = 1i32 << (depth - 1);
    unsigned
        .iter()
        .map(|&val| (val as i32 - offset) as i16)
        .collect()
}

/// Calculate Mean Absolute Error between two i16 arrays
fn calculate_mae_signed(original: &[i16], decoded: &[i16]) -> f64 {
    assert_eq!(original.len(), decoded.len());
    let sum: f64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(o, d)| (*o as f64 - *d as f64).abs())
        .sum();
    sum / original.len() as f64
}

/// Generate 8-bit signed gradient (-128 to +127)
fn generate_8bit_signed_gradient(width: usize, height: usize) -> Vec<i16> {
    let mut pixels = Vec::with_capacity(width * height);
    for _y in 0..height {
        for x in 0..width {
            // Map [0, width-1] to [-128, 127]
            let value = ((x as f64 / (width - 1) as f64) * 255.0 - 128.0) as i16;
            pixels.push(value.clamp(-128, 127));
        }
    }
    pixels
}

/// Generate 12-bit signed gradient (-2048 to +2047)
fn generate_12bit_signed_gradient(width: usize, height: usize) -> Vec<i16> {
    let mut pixels = Vec::with_capacity(width * height);
    for _y in 0..height {
        for x in 0..width {
            // Map [0, width-1] to [-2048, 2047]
            let value = ((x as f64 / (width - 1) as f64) * 4095.0 - 2048.0) as i16;
            pixels.push(value.clamp(-2048, 2047));
        }
    }
    pixels
}

/// Generate 16-bit signed gradient (-32768 to +32767)
fn generate_16bit_signed_gradient(width: usize, height: usize) -> Vec<i16> {
    let mut pixels = Vec::with_capacity(width * height);
    for _y in 0..height {
        for x in 0..width {
            // Map [0, width-1] to [-32768, 32767]
            let value = ((x as f64 / (width - 1) as f64) * 65535.0 - 32768.0) as i16;
            pixels.push(value);
        }
    }
    pixels
}

/// Generate CT Hounsfield Units pattern
/// Typical CT range: -1024 (air) to +3071 (bone)
/// Standard mapping: -1024 = air, 0 = water, +1000 = soft tissue, +3000 = bone
fn generate_ct_hounsfield_pattern(width: usize, height: usize) -> Vec<i16> {
    let mut pixels = Vec::with_capacity(width * height);

    // Create a realistic CT pattern with different tissue densities
    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - width as f64 / 2.0;
            let dy = y as f64 - height as f64 / 2.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let max_distance = ((width * width + height * height) as f64).sqrt() / 2.0;

            // Simulate CT cross-section:
            // Center (bone): +2000 HU
            // Middle (soft tissue): +50 HU
            // Outer (air): -1000 HU
            let value = if distance < max_distance * 0.2 {
                // Bone center
                2000
            } else if distance < max_distance * 0.6 {
                // Soft tissue
                50
            } else {
                // Air/background
                -1000
            };

            pixels.push(value);
        }
    }
    pixels
}

#[test]
fn test_8bit_signed_lossless() {
    println!("\n=== 8-bit Signed Lossless Test ===");

    let width = 256;
    let height = 256;
    let signed_pixels = generate_8bit_signed_gradient(width, height);

    // Verify range
    let min = *signed_pixels.iter().min().unwrap();
    let max = *signed_pixels.iter().max().unwrap();
    println!("Signed pixel range: {} to {}", min, max);
    assert!(min >= -128 && min <= -100, "Min should be near -128");
    assert!(max >= 100 && max <= 127, "Max should be near 127");

    // Convert signed to unsigned for encoding
    let unsigned_pixels = signed_to_unsigned(&signed_pixels, 8);
    let pixels_u8 = unsigned_pixels.iter().map(|&p| p as u8).collect::<Vec<_>>();

    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder.encode(&pixels_u8, &frame_info, &mut dest).expect(
        "Encoding failed",
    );

    println!("Encoded size: {} bytes", encoded_size);

    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");

    // Convert back to signed
    let decoded_unsigned: Vec<u16> = decoded_u8.iter().map(|&p| p as u16).collect();
    let decoded_signed = unsigned_to_signed(&decoded_unsigned, 8);

    // Verify
    let mae = calculate_mae_signed(&signed_pixels, &decoded_signed);
    println!("MAE: {:.4}", mae);

    // Check compression ratio
    let original_size = width * height * 2; // i16 = 2 bytes per pixel
    let ratio = original_size as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0");
    println!("✅ 8-bit signed lossless test PASSED");
}

#[test]
fn test_12bit_signed_lossless() {
    println!("\n=== 12-bit Signed Lossless Test ===");

    let width = 256;
    let height = 256;
    let signed_pixels = generate_12bit_signed_gradient(width, height);

    // Verify range
    let min = *signed_pixels.iter().min().unwrap();
    let max = *signed_pixels.iter().max().unwrap();
    println!("Signed pixel range: {} to {}", min, max);
    assert!(min >= -2048 && min <= -2000, "Min should be near -2048");
    assert!(max >= 2000 && max <= 2047, "Max should be near 2047");

    // Convert signed to unsigned for encoding
    let unsigned_pixels = signed_to_unsigned(&signed_pixels, 12);
    let pixels_u8 = u16_to_u8_le(&unsigned_pixels);

    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder.encode(&pixels_u8, &frame_info, &mut dest).expect(
        "Encoding failed",
    );

    println!("Encoded size: {} bytes", encoded_size);

    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_unsigned = u8_to_u16_le(&decoded_u8);
    let decoded_signed = unsigned_to_signed(&decoded_unsigned, 12);

    // Verify
    let mae = calculate_mae_signed(&signed_pixels, &decoded_signed);
    println!("MAE: {:.4}", mae);

    let original_size = width * height * 2;
    let ratio = original_size as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0");
    println!("✅ 12-bit signed lossless test PASSED");
}

#[test]
fn test_16bit_signed_lossless() {
    println!("\n=== 16-bit Signed Lossless Test ===");

    let width = 256;
    let height = 256;
    let signed_pixels = generate_16bit_signed_gradient(width, height);

    // Verify range
    let min = *signed_pixels.iter().min().unwrap();
    let max = *signed_pixels.iter().max().unwrap();
    println!("Signed pixel range: {} to {}", min, max);
    assert!(min >= -32768 && min <= -32000, "Min should be near -32768");
    assert!(max >= 32000 && max < i16::MAX, "Max should be near 32767");

    // Convert signed to unsigned for encoding
    let unsigned_pixels = signed_to_unsigned(&signed_pixels, 16);
    let pixels_u8 = u16_to_u8_le(&unsigned_pixels);

    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 16,
        component_count: 1,
    };

    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder.encode(&pixels_u8, &frame_info, &mut dest).expect(
        "Encoding failed",
    );

    println!("Encoded size: {} bytes", encoded_size);

    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_unsigned = u8_to_u16_le(&decoded_u8);
    let decoded_signed = unsigned_to_signed(&decoded_unsigned, 16);

    // Verify
    let mae = calculate_mae_signed(&signed_pixels, &decoded_signed);
    println!("MAE: {:.4}", mae);

    let original_size = width * height * 2;
    let ratio = original_size as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0");
    println!("✅ 16-bit signed lossless test PASSED");
}

#[test]
fn test_ct_hounsfield_units() {
    println!("\n=== CT Hounsfield Units Test ===");
    println!("Testing medical CT use case (12-bit signed: -1024 to +3071 HU)");

    let width = 512;
    let height = 512;
    let signed_pixels = generate_ct_hounsfield_pattern(width, height);

    // Verify HU range
    let min = *signed_pixels.iter().min().unwrap();
    let max = *signed_pixels.iter().max().unwrap();
    println!("Hounsfield Units range: {} HU to {} HU", min, max);
    assert_eq!(min, -1000, "Should have air (-1000 HU)");
    assert_eq!(max, 2000, "Should have bone (2000 HU)");

    // Count tissue types
    let air_count = signed_pixels.iter().filter(|&&p| p == -1000).count();
    let tissue_count = signed_pixels.iter().filter(|&&p| p == 50).count();
    let bone_count = signed_pixels.iter().filter(|&&p| p == 2000).count();
    println!("Tissue distribution:");
    println!("  Air (-1000 HU): {} pixels", air_count);
    println!("  Soft tissue (+50 HU): {} pixels", tissue_count);
    println!("  Bone (+2000 HU): {} pixels", bone_count);

    // Convert to unsigned 12-bit (range: -2048 to +2047)
    let unsigned_pixels = signed_to_unsigned(&signed_pixels, 12);
    let pixels_u8 = u16_to_u8_le(&unsigned_pixels);

    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(5);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder.encode(&pixels_u8, &frame_info, &mut dest).expect(
        "Encoding failed",
    );

    println!("Encoded size: {} bytes", encoded_size);

    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_unsigned = u8_to_u16_le(&decoded_u8);
    let decoded_signed = unsigned_to_signed(&decoded_unsigned, 12);

    // Verify
    let mae = calculate_mae_signed(&signed_pixels, &decoded_signed);
    println!("MAE: {:.4} HU", mae);

    let original_size = width * height * 2;
    let ratio = original_size as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);

    // Verify tissue values preserved
    let decoded_air = decoded_signed.iter().filter(|&&p| p == -1000).count();
    let decoded_tissue = decoded_signed.iter().filter(|&&p| p == 50).count();
    let decoded_bone = decoded_signed.iter().filter(|&&p| p == 2000).count();

    assert_eq!(air_count, decoded_air, "Air pixels should be preserved");
    assert_eq!(
        tissue_count,
        decoded_tissue,
        "Tissue pixels should be preserved"
    );
    assert_eq!(bone_count, decoded_bone, "Bone pixels should be preserved");

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0 HU");
    println!("✅ CT Hounsfield Units test PASSED");
}

#[test]
fn test_signed_negative_values() {
    println!("\n=== Signed Negative Values Test ===");
    println!("Testing pure negative values");

    let width = 128;
    let height = 128;

    // Create image with only negative values (-2048 to -1)
    let signed_pixels: Vec<i16> = (0..width * height)
        .map(|i| -((i % 2048) as i16 + 1))
        .collect();

    let min = *signed_pixels.iter().min().unwrap();
    let max = *signed_pixels.iter().max().unwrap();
    println!("All negative range: {} to {}", min, max);
    assert!(max < 0, "All values should be negative");

    // Convert and encode
    let unsigned_pixels = signed_to_unsigned(&signed_pixels, 12);
    let pixels_u8 = u16_to_u8_le(&unsigned_pixels);

    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder.encode(&pixels_u8, &frame_info, &mut dest).expect(
        "Encoding failed",
    );

    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_unsigned = u8_to_u16_le(&decoded_u8);
    let decoded_signed = unsigned_to_signed(&decoded_unsigned, 12);

    let mae = calculate_mae_signed(&signed_pixels, &decoded_signed);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "All negative values should roundtrip losslessly");
    println!("✅ Signed negative values test PASSED");
}

#[test]
fn test_signed_zero_crossing() {
    println!("\n=== Signed Zero Crossing Test ===");
    println!("Testing values crossing zero boundary");

    let width = 256;
    let height = 256;

    // Create pattern that crosses zero: -128 to +127
    let signed_pixels: Vec<i16> = (0..width * height)
        .map(|i| ((i % 256) as i16) - 128)
        .collect();

    let min = *signed_pixels.iter().min().unwrap();
    let max = *signed_pixels.iter().max().unwrap();
    println!("Zero-crossing range: {} to {}", min, max);

    // Count negatives, zeros, positives
    let neg_count = signed_pixels.iter().filter(|&&p| p < 0).count();
    let zero_count = signed_pixels.iter().filter(|&&p| p == 0).count();
    let pos_count = signed_pixels.iter().filter(|&&p| p > 0).count();
    println!(
        "Distribution: {} negative, {} zero, {} positive",
        neg_count,
        zero_count,
        pos_count
    );

    // Convert and encode
    let unsigned_pixels = signed_to_unsigned(&signed_pixels, 8);
    let pixels_u8 = unsigned_pixels.iter().map(|&p| p as u8).collect::<Vec<_>>();

    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder.encode(&pixels_u8, &frame_info, &mut dest).expect(
        "Encoding failed",
    );

    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_unsigned: Vec<u16> = decoded_u8.iter().map(|&p| p as u16).collect();
    let decoded_signed = unsigned_to_signed(&decoded_unsigned, 8);

    let mae = calculate_mae_signed(&signed_pixels, &decoded_signed);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "Zero crossing should roundtrip losslessly");
    println!("✅ Signed zero crossing test PASSED");
}

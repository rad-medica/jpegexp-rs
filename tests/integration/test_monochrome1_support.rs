//! MONOCHROME1 Support Tests
//!
//! Tests for DICOM Photometric Interpretation MONOCHROME1 (inverse grayscale).
//! This is a MEDIUM PRIORITY requirement from docs/DICOM_J2K_REQUIREMENTS.md.
//!
//! DICOM Standard PS3.3 Section C.7.6.3.1.2:
//! - MONOCHROME1: Pixel value 0 is intended to be displayed as WHITE
//! - MONOCHROME2: Pixel value 0 is intended to be displayed as BLACK (standard)
//! - Inversion formula: inverted_pixel = max_value - pixel
//!
//! Common uses:
//! - X-ray radiography (historically displayed on film where exposed = black)
//! - Some CR/DR systems
//! - Legacy PACS systems
//!
//! Test coverage:
//! - 8-bit MONOCHROME1 lossless
//! - 12-bit MONOCHROME1 lossless
//! - 16-bit MONOCHROME1 lossless
//! - Inversion verification (white ↔ black swap)
//! - Roundtrip validation

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

/// Calculate Mean Absolute Error for u16 data
fn calculate_mae_u16(original: &[u16], decoded: &[u16]) -> f64 {
    assert_eq!(original.len(), decoded.len());
    let sum: i64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(a, b)| (*a as i64 - *b as i64).abs())
        .sum();
    sum as f64 / original.len() as f64
}

/// Convert u16 to little-endian bytes
fn u16_to_u8_le(pixels: &[u16]) -> Vec<u8> {
    pixels
        .iter()
        .flat_map(|&p| p.to_le_bytes().to_vec())
        .collect()
}

/// Convert little-endian bytes to u16
fn u8_to_u16_le(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

/// Invert pixels for MONOCHROME1 (0 = white, max = black)
fn invert_pixels(pixels: &[u16], max_value: u16) -> Vec<u16> {
    pixels.iter().map(|&p| max_value - p).collect()
}

/// Invert 8-bit pixels
fn invert_pixels_u8(pixels: &[u8], max_value: u8) -> Vec<u8> {
    pixels.iter().map(|&p| max_value - p).collect()
}

/// Generate gradient pattern (0 to max)
fn generate_gradient(width: usize, height: usize, max_value: u16) -> Vec<u16> {
    let mut pixels = Vec::with_capacity(width * height);
    for _y in 0..height {
        for x in 0..width {
            let value = ((x * max_value as usize) / width.max(1)) as u16;
            pixels.push(value.min(max_value));
        }
    }
    pixels
}

/// Generate X-ray chest pattern (simulating lung and bone structures)
fn generate_xray_chest_pattern(width: usize, height: usize) -> Vec<u16> {
    let mut pixels = vec![0u16; width * height];
    
    let cx = width / 2;
    let cy = height / 2;
    
    for y in 0..height {
        for x in 0..width {
            let dx = x as i32 - cx as i32;
            let dy = y as i32 - cy as i32;
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            let max_dist = ((cx * cx + cy * cy) as f32).sqrt();
            
            // Simulate X-ray appearance:
            // Center (lungs/air): High transmission = Dark on film = High pixel value in MONOCHROME2
            // Edges (ribs/bone): Low transmission = Bright on film = Low pixel value in MONOCHROME2
            let _normalized = (dist / max_dist).min(1.0);
            
            // MONOCHROME2 values (before inversion):
            // Lungs (center): 3000 (dark)
            // Soft tissue: 1500 (medium)
            // Bone/ribs (edges): 500 (bright)
            let value = if dist < max_dist * 0.3 {
                3000 // Lungs
            } else if dist < max_dist * 0.7 {
                1500 // Soft tissue
            } else {
                500 // Ribs
            };
            
            pixels[y * width + x] = value.min(4095);
        }
    }
    pixels
}

#[test]
fn test_monochrome1_8bit_lossless() {
    println!("\n=== MONOCHROME1 8-bit Lossless Test ===");
    
    let width = 256;
    let height = 256;
    let max_value = 255u16;
    
    // Create MONOCHROME2 gradient (0=black, 255=white)
    let mono2_pixels = generate_gradient(width, height, max_value);
    
    println!("MONOCHROME2 range: {} to {}", 
             *mono2_pixels.iter().min().unwrap(),
             *mono2_pixels.iter().max().unwrap());
    
    // Invert to MONOCHROME1 (0=white, 255=black)
    let mono1_pixels = invert_pixels(&mono2_pixels, max_value);
    
    println!("MONOCHROME1 range: {} to {}", 
             *mono1_pixels.iter().min().unwrap(),
             *mono1_pixels.iter().max().unwrap());
    
    // Verify inversion: first pixel should be near 255, last should be near 0
    assert!(mono1_pixels[0] >= 254, "First pixel should be near white (255), got {}", mono1_pixels[0]);
    assert!(mono1_pixels[mono1_pixels.len() - 1] <= 1, "Last pixel should be near black (0), got {}", mono1_pixels[mono1_pixels.len() - 1]);
    
    // Convert to u8 for encoding
    let pixels_u8 = invert_pixels_u8(
        &mono2_pixels.iter().map(|&p| p as u8).collect::<Vec<_>>(),
        max_value as u8
    );
    
    // Encode MONOCHROME1 pixels
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder
        .encode(&pixels_u8, &frame_info, &mut dest)
        .expect("Encoding failed");
    
    println!("Encoded size: {} bytes", encoded_size);
    
    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    
    // Invert back to MONOCHROME2 for comparison
    let decoded_inverted = invert_pixels_u8(&decoded_u8, max_value as u8);
    let decoded_u16: Vec<u16> = decoded_inverted.iter().map(|&p| p as u16).collect();
    
    // Verify against original MONOCHROME2
    let mae = calculate_mae_u16(&mono2_pixels, &decoded_u16);
    println!("MAE: {:.4}", mae);
    
    let ratio = (width * height) as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);
    
    assert_eq!(mae, 0.0, "MONOCHROME1 lossless should have MAE=0");
    println!("✅ MONOCHROME1 8-bit lossless test PASSED");
}

#[test]
fn test_monochrome1_12bit_lossless() {
    println!("\n=== MONOCHROME1 12-bit Lossless Test ===");
    
    let width = 256;
    let height = 256;
    let max_value = 4095u16;
    
    // Create MONOCHROME2 gradient
    let mono2_pixels = generate_gradient(width, height, max_value);
    
    println!("MONOCHROME2 range: {} to {}", 
             *mono2_pixels.iter().min().unwrap(),
             *mono2_pixels.iter().max().unwrap());
    
    // Invert to MONOCHROME1
    let mono1_pixels = invert_pixels(&mono2_pixels, max_value);
    
    println!("MONOCHROME1 range: {} to {}", 
             *mono1_pixels.iter().min().unwrap(),
             *mono1_pixels.iter().max().unwrap());
    
    // Verify inversion
    assert!(mono1_pixels[0] >= 4090, "First pixel should be near white (4095), got {}", mono1_pixels[0]);
    assert!(mono1_pixels[mono1_pixels.len() - 1] <= 20, "Last pixel should be near black (0), got {}", mono1_pixels[mono1_pixels.len() - 1]);
    
    // Encode MONOCHROME1 pixels
    let pixels_u8 = u16_to_u8_le(&mono1_pixels);
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder
        .encode(&pixels_u8, &frame_info, &mut dest)
        .expect("Encoding failed");
    
    println!("Encoded size: {} bytes", encoded_size);
    
    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Invert back to MONOCHROME2
    let decoded_mono2 = invert_pixels(&decoded_u16, max_value);
    
    // Verify
    let mae = calculate_mae_u16(&mono2_pixels, &decoded_mono2);
    println!("MAE: {:.4}", mae);
    
    let ratio = (width * height * 2) as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);
    
    assert_eq!(mae, 0.0, "MONOCHROME1 12-bit lossless should have MAE=0");
    println!("✅ MONOCHROME1 12-bit lossless test PASSED");
}

#[test]
fn test_monochrome1_16bit_lossless() {
    println!("\n=== MONOCHROME1 16-bit Lossless Test ===");
    
    let width = 256;
    let height = 256;
    let max_value = 65535u16;
    
    // Create MONOCHROME2 gradient
    let mono2_pixels = generate_gradient(width, height, max_value);
    
    println!("MONOCHROME2 range: {} to {}", 
             *mono2_pixels.iter().min().unwrap(),
             *mono2_pixels.iter().max().unwrap());
    
    // Invert to MONOCHROME1
    let mono1_pixels = invert_pixels(&mono2_pixels, max_value);
    
    println!("MONOCHROME1 range: {} to {}", 
             *mono1_pixels.iter().min().unwrap(),
             *mono1_pixels.iter().max().unwrap());
    
    // Verify inversion
    assert!(mono1_pixels[0] >= 65000, "First pixel should be near white (65535), got {}", mono1_pixels[0]);
    assert!(mono1_pixels[mono1_pixels.len() - 1] <= 300, "Last pixel should be near black (0), got {}", mono1_pixels[mono1_pixels.len() - 1]);
    
    // Encode MONOCHROME1 pixels
    let pixels_u8 = u16_to_u8_le(&mono1_pixels);
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder
        .encode(&pixels_u8, &frame_info, &mut dest)
        .expect("Encoding failed");
    
    println!("Encoded size: {} bytes", encoded_size);
    
    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Invert back to MONOCHROME2
    let decoded_mono2 = invert_pixels(&decoded_u16, max_value);
    
    // Verify
    let mae = calculate_mae_u16(&mono2_pixels, &decoded_mono2);
    println!("MAE: {:.4}", mae);
    
    let ratio = (width * height * 2) as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);
    
    assert_eq!(mae, 0.0, "MONOCHROME1 16-bit lossless should have MAE=0");
    println!("✅ MONOCHROME1 16-bit lossless test PASSED");
}

#[test]
fn test_monochrome1_xray_chest() {
    println!("\n=== MONOCHROME1 X-ray Chest Pattern Test ===");
    println!("Testing realistic X-ray radiography use case (12-bit)");
    
    let width = 512;
    let height = 512;
    let max_value = 4095u16;
    
    // Create MONOCHROME2 X-ray pattern
    let mono2_pixels = generate_xray_chest_pattern(width, height);
    
    let min = *mono2_pixels.iter().min().unwrap();
    let max = *mono2_pixels.iter().max().unwrap();
    println!("MONOCHROME2 X-ray range: {} to {}", min, max);
    
    // Count tissue types
    let lung_count = mono2_pixels.iter().filter(|&&p| p == 3000).count();
    let tissue_count = mono2_pixels.iter().filter(|&&p| p == 1500).count();
    let bone_count = mono2_pixels.iter().filter(|&&p| p == 500).count();
    println!("X-ray pattern:");
    println!("  Lungs (3000): {} pixels", lung_count);
    println!("  Soft tissue (1500): {} pixels", tissue_count);
    println!("  Ribs/bone (500): {} pixels", bone_count);
    
    // Invert to MONOCHROME1 (as displayed on X-ray film)
    let mono1_pixels = invert_pixels(&mono2_pixels, max_value);
    
    println!("MONOCHROME1 inverted:");
    println!("  Lungs: {} (dark on film)", 4095 - 3000);
    println!("  Soft tissue: {} (medium)", 4095 - 1500);
    println!("  Ribs: {} (bright on film)", 4095 - 500);
    
    // Encode MONOCHROME1
    let pixels_u8 = u16_to_u8_le(&mono1_pixels);
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(5);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    let mut dest = vec![0u8; pixels_u8.len() * 4];
    let encoded_size = encoder
        .encode(&pixels_u8, &frame_info, &mut dest)
        .expect("Encoding failed");
    
    println!("Encoded size: {} bytes", encoded_size);
    
    // Decode
    let mut reader = JpegStreamReader::new(&dest[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("Reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Invert back to MONOCHROME2
    let decoded_mono2 = invert_pixels(&decoded_u16, max_value);
    
    // Verify
    let mae = calculate_mae_u16(&mono2_pixels, &decoded_mono2);
    println!("MAE: {:.4}", mae);
    
    let ratio = (width * height * 2) as f64 / encoded_size as f64;
    println!("Compression ratio: {:.2}:1", ratio);
    
    // Verify tissue values preserved
    let decoded_lung = decoded_mono2.iter().filter(|&&p| p == 3000).count();
    let decoded_tissue = decoded_mono2.iter().filter(|&&p| p == 1500).count();
    let decoded_bone = decoded_mono2.iter().filter(|&&p| p == 500).count();
    
    assert_eq!(lung_count, decoded_lung, "Lung pixels should be preserved");
    assert_eq!(tissue_count, decoded_tissue, "Tissue pixels should be preserved");
    assert_eq!(bone_count, decoded_bone, "Bone pixels should be preserved");
    
    assert_eq!(mae, 0.0, "MONOCHROME1 X-ray should have MAE=0");
    println!("✅ MONOCHROME1 X-ray chest test PASSED");
}

#[test]
fn test_monochrome1_inversion_symmetry() {
    println!("\n=== MONOCHROME1 Inversion Symmetry Test ===");
    println!("Verifying that double inversion returns original");
    
    let width = 128;
    let height = 128;
    let max_value = 4095u16;
    
    // Create original MONOCHROME2 pattern
    let original = generate_gradient(width, height, max_value);
    
    // Invert to MONOCHROME1
    let mono1 = invert_pixels(&original, max_value);
    
    // Invert back to MONOCHROME2
    let restored = invert_pixels(&mono1, max_value);
    
    // Verify perfect restoration
    let mae = calculate_mae_u16(&original, &restored);
    println!("MAE after double inversion: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "Double inversion should restore original exactly");
    
    // Verify specific values
    assert_eq!(original[0], restored[0], "First pixel should match");
    assert_eq!(original[width * height - 1], restored[width * height - 1], "Last pixel should match");
    
    println!("✅ MONOCHROME1 inversion symmetry test PASSED");
}

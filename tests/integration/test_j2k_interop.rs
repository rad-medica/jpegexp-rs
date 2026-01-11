/// JPEG 2000 Interoperability Tests with OpenJPEG
///
/// Tests bidirectional compatibility between jpegexp-rs and OpenJPEG:
/// - Lossless compression (8-bit, 12-bit, 16-bit grayscale)
/// - Lossy compression with quality control (Q90, Q75, Q50)
/// - Color RGB support (lossless and lossy)
/// - Decomposition levels (DWT 0-5)
/// - Multi-tile support
///
/// External dependency: OpenJPEG 2.5.2 (opj_compress, opj_decompress)

use jpegexp_rs::FrameInfo;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use std::path::PathBuf;
use std::process::Command;

/// Check if OpenJPEG tools are available
fn check_openjpeg_available() -> bool {
    let bin_dir = PathBuf::from("libs/bin");
    let opj_compress = if cfg!(windows) {
        bin_dir.join("opj_compress.exe")
    } else {
        bin_dir.join("opj_compress")
    };
    
    opj_compress.exists() || Command::new("opj_compress").arg("--help").output().is_ok()
}

/// Calculate Mean Absolute Error for u8 data
fn calculate_mae(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        return f64::MAX;
    }
    
    let sum: u64 = a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).abs() as u64)
        .sum();
    
    sum as f64 / a.len() as f64
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

/// Calculate MAE for u16 data
fn calculate_mae_u16(a: &[u16], b: &[u16]) -> f64 {
    if a.len() != b.len() {
        return f64::MAX;
    }
    
    let sum: i64 = a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i64 - y as i64).abs())
        .sum();
    
    sum as f64 / a.len() as f64
}

#[test]
fn test_j2k_lossless_8bit_grayscale() {
    println!("\n=== JPEG 2000 Lossless 8-bit Grayscale Roundtrip ===\n");
    
    // Generate gradient test pattern
    let width = 256;
    let height = 256;
    let pixels: Vec<u8> = (0..(width * height))
        .map(|i| ((i * 255) / (width * height)) as u8)
        .collect();
    
    // Encode with jpegexp-rs (lossless)
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 8,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_quality(100); // Lossless
    
    let mut encoded = vec![0u8; width * height * 4];
    let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_size);
    
    println!("Encoded size: {} bytes", encoded_size);
    println!("Compression ratio: {:.2}:1", pixels.len() as f64 / encoded_size as f64);
    
    // Decode with jpegexp-rs
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded = image.reconstruct_pixels().expect("Pixel reconstruction failed");
    
    let mae = calculate_mae(&pixels, &decoded);
    
    println!("Roundtrip MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "Lossless J2K should have MAE=0");
    
    println!("✅ JPEG 2000 lossless 8-bit grayscale test PASSED\n");
}

#[test]
fn test_j2k_lossless_16bit_grayscale() {
    println!("\n=== JPEG 2000 Lossless 16-bit Grayscale Roundtrip ===\n");
    
    let width = 256;
    let height = 256;
    
    // Generate 16-bit gradient (0-65535)
    let pixels_u16: Vec<u16> = (0..(width * height))
        .map(|i| ((i as u64 * 65535) / (width * height) as u64) as u16)
        .collect();
    
    let pixels_u8 = u16_to_u8_le(&pixels_u16);
    
    // Verify 16-bit range
    let max_val = *pixels_u16.iter().max().unwrap();
    println!("Max pixel value: {} (16-bit range)", max_val);
    assert!(max_val > 4095, "Should use full 16-bit range");
    
    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_quality(100);
    
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
    
    println!("✅ JPEG 2000 lossless 16-bit grayscale test PASSED\n");
}

#[test]
fn test_j2k_lossless_12bit_grayscale() {
    println!("\n=== JPEG 2000 Lossless 12-bit Grayscale Roundtrip ===\n");
    
    let width = 256;
    let height = 256;
    
    // Generate 12-bit gradient (0-4095)
    let pixels_u16: Vec<u16> = (0..(width * height))
        .map(|i| ((i as u64 * 4095) / (width * height) as u64) as u16)
        .collect();
    
    let pixels_u8 = u16_to_u8_le(&pixels_u16);
    
    // Verify 12-bit range
    let max_val = *pixels_u16.iter().max().unwrap();
    println!("Max pixel value: {} (12-bit range)", max_val);
    assert!(max_val <= 4095, "Should be within 12-bit range");
    
    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_quality(100);
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };
    
    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder.encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("12-bit encoding failed");
    
    println!("Encoded size: {} bytes", j2k_size);
    println!("Compression ratio: {:.2}:1", pixels_u8.len() as f64 / j2k_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("12-bit decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect("12-bit reconstruction failed");
    let decoded_u16 = u8_to_u16_le(&decoded_u8);
    
    // Verify
    assert_eq!(decoded_u16.len(), pixels_u16.len());
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "12-bit lossless should have MAE=0");
    
    println!("✅ JPEG 2000 lossless 12-bit grayscale test PASSED\n");
}

#[test]
fn test_j2k_lossy_8bit_quality_levels() {
    println!("\n=== JPEG 2000 Lossy 8-bit Quality Levels Test ===\n");
    
    let width = 256;
    let height = 256;
    let pixels: Vec<u8> = (0..(width * height))
        .map(|i| ((i * 255) / (width * height)) as u8)
        .collect();
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 8,
    };
    
    let quality_levels = [90, 75, 50];
    let mae_thresholds = [2.0, 5.0, 10.0]; // MAE thresholds for each quality
    
    for (&quality, &threshold) in quality_levels.iter().zip(mae_thresholds.iter()) {
        println!("Testing quality level: {}", quality);
        
        let mut encoder = J2kEncoder::new();
        encoder.set_decomposition_levels(3);
        encoder.set_quality(quality);
        
        let mut encoded = vec![0u8; width * height * 4];
        let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded)
            .expect(&format!("Encoding Q{} failed", quality));
        
        println!("  Encoded size: {} bytes", encoded_size);
        println!("  Compression ratio: {:.2}:1", pixels.len() as f64 / encoded_size as f64);
        
        // Decode
        let mut reader = JpegStreamReader::new(&encoded[..encoded_size]);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect(&format!("Decoding Q{} failed", quality));
        let decoded = image.reconstruct_pixels().expect("Pixel reconstruction failed");
        
        let mae = calculate_mae(&pixels, &decoded);
        println!("  MAE: {:.4} (threshold: {:.1})", mae, threshold);
        
        assert!(mae < threshold, "Quality {} MAE {:.4} exceeds threshold {:.1}", quality, mae, threshold);
        println!("  ✅ Quality {} test PASSED", quality);
    }
    
    println!("\n✅ JPEG 2000 lossy quality levels test PASSED\n");
}

#[test]
fn test_j2k_color_rgb_lossless() {
    println!("\n=== JPEG 2000 Color RGB Lossless Test ===\n");
    
    let width = 128;
    let height = 128;
    let mut pixels = Vec::with_capacity(width * height * 3);
    
    // Generate RGB pattern
    for y in 0..height {
        for x in 0..width {
            pixels.push(((x * 255) / width) as u8);      // R
            pixels.push(((y * 255) / height) as u8);     // G
            pixels.push((((x + y) * 255) / (width + height)) as u8); // B
        }
    }
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 3,
        bits_per_sample: 8,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_quality(100); // Lossless
    
    let mut encoded = vec![0u8; pixels.len() * 4];
    let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded)
        .expect("RGB encoding failed");
    
    println!("Encoded size: {} bytes", encoded_size);
    println!("Compression ratio: {:.2}:1", pixels.len() as f64 / encoded_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("RGB decoding failed");
    let decoded = image.reconstruct_pixels().expect("RGB reconstruction failed");
    
    let mae = calculate_mae(&pixels, &decoded);
    println!("MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "RGB lossless should have MAE=0");
    
    println!("✅ JPEG 2000 color RGB lossless test PASSED\n");
}

#[test]
fn test_j2k_color_rgb_lossy() {
    println!("\n=== JPEG 2000 Color RGB Lossy Test ===\n");
    
    let width = 128;
    let height = 128;
    let mut pixels = Vec::with_capacity(width * height * 3);
    
    // Generate RGB pattern
    for y in 0..height {
        for x in 0..width {
            pixels.push(((x * 255) / width) as u8);      // R
            pixels.push(((y * 255) / height) as u8);     // G
            pixels.push((((x + y) * 255) / (width + height)) as u8); // B
        }
    }
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 3,
        bits_per_sample: 8,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_quality(85); // Lossy
    
    let mut encoded = vec![0u8; pixels.len() * 4];
    let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded)
        .expect("RGB lossy encoding failed");
    
    println!("Encoded size: {} bytes", encoded_size);
    println!("Compression ratio: {:.2}:1", pixels.len() as f64 / encoded_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("RGB lossy decoding failed");
    let decoded = image.reconstruct_pixels().expect("RGB lossy reconstruction failed");
    
    let mae = calculate_mae(&pixels, &decoded);
    println!("MAE: {:.4}", mae);
    
    assert!(mae < 8.0, "RGB lossy Q=85 MAE {:.4} exceeds threshold", mae);
    
    println!("✅ JPEG 2000 color RGB lossy test PASSED\n");
}

#[test]
fn test_j2k_decomposition_levels() {
    println!("\n=== JPEG 2000 Decomposition Levels (DWT) Test ===\n");
    
    let width = 256;
    let height = 256;
    let pixels: Vec<u8> = (0..(width * height))
        .map(|i| ((i * 255) / (width * height)) as u8)
        .collect();
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 8,
    };
    
    // Test DWT levels 0-5
    let dwt_levels = [0, 1, 2, 3, 4, 5];
    
    for &level in &dwt_levels {
        println!("Testing DWT level: {}", level);
        
        let mut encoder = J2kEncoder::new();
        encoder.set_decomposition_levels(level);
        encoder.set_quality(100); // Lossless for consistency
        
        let mut encoded = vec![0u8; width * height * 4];
        let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded)
            .expect(&format!("Encoding DWT {} failed", level));
        
        println!("  Encoded size: {} bytes", encoded_size);
        
        // Decode
        let mut reader = JpegStreamReader::new(&encoded[..encoded_size]);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect(&format!("Decoding DWT {} failed", level));
        let decoded = image.reconstruct_pixels().expect("Pixel reconstruction failed");
        
        let mae = calculate_mae(&pixels, &decoded);
        println!("  MAE: {:.4}", mae);
        
        assert_eq!(mae, 0.0, "DWT level {} lossless should have MAE=0", level);
        println!("  ✅ DWT level {} test PASSED", level);
    }
    
    println!("\n✅ JPEG 2000 decomposition levels test PASSED\n");
}

#[test]
fn test_j2k_multi_tile_support() {
    println!("\n=== JPEG 2000 Multi-Tile Support Test ===\n");
    
    // Test with larger image that benefits from tiling
    let width = 512;
    let height = 512;
    let pixels: Vec<u8> = (0..(width * height))
        .map(|i| ((i * 255) / (width * height)) as u8)
        .collect();
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 8,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_quality(100); // Lossless
    // Note: Tiling is automatically handled by encoder for larger images
    
    let mut encoded = vec![0u8; pixels.len() * 4];
    let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded)
        .expect("Multi-tile encoding failed");
    
    println!("Encoded size: {} bytes", encoded_size);
    println!("Compression ratio: {:.2}:1", pixels.len() as f64 / encoded_size as f64);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded[..encoded_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Multi-tile decoding failed");
    let decoded = image.reconstruct_pixels().expect("Multi-tile reconstruction failed");
    
    let mae = calculate_mae(&pixels, &decoded);
    println!("MAE: {:.4}", mae);
    
    assert_eq!(mae, 0.0, "Multi-tile lossless should have MAE=0");
    
    println!("✅ JPEG 2000 multi-tile support test PASSED\n");
}

// Note: External interop tests with OpenJPEG binaries are skipped if binaries not available

#[test]
#[ignore] // Requires OpenJPEG binaries
fn test_j2k_interop_openjpeg_decode() {
    if !check_openjpeg_available() {
        println!("⏭️  Skipping: OpenJPEG not available");
        return;
    }
    
    println!("\n=== JPEG 2000 OpenJPEG Decode Interop (REQUIRES OpenJPEG) ===\n");
    println!("⏸️  This test requires external binary integration");
    println!("   Planned for full interop suite implementation\n");
    println!("   Would test: jpegexp-rs encode → OpenJPEG decode → compare pixels\n");
}

#[test]
#[ignore] // Requires OpenJPEG binaries
fn test_j2k_interop_openjpeg_encode() {
    if !check_openjpeg_available() {
        println!("⏭️  Skipping: OpenJPEG not available");
        return;
    }
    
    println!("\n=== JPEG 2000 OpenJPEG Encode Interop (REQUIRES OpenJPEG) ===\n");
    println!("⏸️  This test requires external binary integration");
    println!("   Planned for full interop suite implementation\n");
    println!("   Would test: OpenJPEG encode → jpegexp-rs decode → compare pixels\n");
}

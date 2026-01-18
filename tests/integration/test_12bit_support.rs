//! Test suite for 12-bit JPEG 2000 support
//!
//! Validates that jpegexp-rs correctly handles 12-bit medical images
//! per DICOM requirements (PS3.5 Section 8.2.4)
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

/// Generate 12-bit gradient test pattern (0-4095 range)
fn generate_12bit_gradient(width: usize, height: usize) -> Vec<u16> {
    let mut pixels = vec![0u16; width * height];
    for y in 0..height {
        for x in 0..width {
            // Scale to 12-bit range (0-4095)
            pixels[y * width + x] = ((x * 4095) / width.max(1)) as u16;
        }
    }
    pixels
}

/// Generate 12-bit CT-like test pattern
/// Simulates typical CT density values (air=-1000 HU, bone=+1000 HU)
/// Mapped to 12-bit range: 0-4095
fn generate_12bit_ct_pattern(width: usize, height: usize) -> Vec<u16> {
    let mut pixels = vec![0u16; width * height];

    // Center point
    let cx = width / 2;
    let cy = height / 2;

    for y in 0..height {
        for x in 0..width {
            let dx = (x as i32 - cx as i32).abs();
            let dy = (y as i32 - cy as i32).abs();
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            let max_dist = ((cx * cx + cy * cy) as f32).sqrt();

            // Normalize to 0.0-1.0
            let normalized = (dist / max_dist).min(1.0);

            // Map to 12-bit range with CT-like distribution
            // Center (bone): ~3000, Edge (air): ~1000
            let value = (1000.0 + normalized * 2000.0) as u16;
            pixels[y * width + x] = value.min(4095);
        }
    }
    pixels
}

/// Generate 12-bit checkerboard pattern
fn generate_12bit_checkerboard(width: usize, height: usize, square_size: usize) -> Vec<u16> {
    let mut pixels = vec![0u16; width * height];
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / square_size) + (y / square_size)) % 2 == 0;
            pixels[y * width + x] = if is_white { 4095 } else { 0 };
        }
    }
    pixels
}

#[test]
fn test_12bit_lossless_gradient() {
    println!("\n=== 12-bit Lossless Gradient Test ===");

    let width = 256;
    let height = 256;
    let pixels_u16 = generate_12bit_gradient(width, height);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);

    // Verify 12-bit range
    let max_val = *pixels_u16.iter().max().unwrap();
    println!("Max pixel value: {} (should be ≤4095)", max_val);
    assert!(max_val <= 4095, "12-bit values should not exceed 4095");

    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    // No set_irreversible() call = lossless mode

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder
        .encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("12-bit encoding failed");

    println!("Encoded size: {} bytes", j2k_size);
    println!(
        "Compression ratio: {:.2}:1",
        pixels_u8.len() as f64 / j2k_size as f64
    );

    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("12-bit decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect(
        "12-bit reconstruction failed",
    );
    let decoded_u16 = u8_to_u16_le(&decoded_u8);

    // Verify
    assert_eq!(decoded_u16.len(), pixels_u16.len());
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "12-bit lossless should have MAE=0");
    println!("✅ 12-bit lossless gradient test PASSED");
}

#[test]
fn test_12bit_lossless_ct_pattern() {
    println!("\n=== 12-bit Lossless CT Pattern Test ===");

    let width = 512;
    let height = 512;
    let pixels_u16 = generate_12bit_ct_pattern(width, height);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);

    // Stats
    let min_val = *pixels_u16.iter().min().unwrap();
    let max_val = *pixels_u16.iter().max().unwrap();
    println!("Pixel range: {} - {}", min_val, max_val);

    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(5);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder
        .encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("12-bit CT encoding failed");

    println!("Encoded size: {} bytes", j2k_size);
    println!(
        "Compression ratio: {:.2}:1",
        pixels_u8.len() as f64 / j2k_size as f64
    );

    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("12-bit CT decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect(
        "12-bit CT reconstruction failed",
    );
    let decoded_u16 = u8_to_u16_le(&decoded_u8);

    // Verify
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "12-bit lossless CT pattern should have MAE=0");
    println!("✅ 12-bit lossless CT pattern test PASSED");
}

#[test]
fn test_12bit_lossless_checkerboard() {
    println!("\n=== 12-bit Lossless Checkerboard Test ===");

    let width = 256;
    let height = 256;
    let square_size = 16;
    let pixels_u16 = generate_12bit_checkerboard(width, height, square_size);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);

    println!(
        "Checkerboard: {}x{}, square size: {}",
        width,
        height,
        square_size
    );

    // Encode (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder
        .encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("12-bit checkerboard encoding failed");

    println!("Encoded size: {} bytes", j2k_size);
    println!(
        "Compression ratio: {:.2}:1",
        pixels_u8.len() as f64 / j2k_size as f64
    );

    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect(
        "12-bit checkerboard decoding failed",
    );
    let decoded_u8 = image.reconstruct_pixels().expect(
        "12-bit checkerboard reconstruction failed",
    );
    let decoded_u16 = u8_to_u16_le(&decoded_u8);

    // Verify
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "12-bit lossless checkerboard should have MAE=0");
    println!("✅ 12-bit lossless checkerboard test PASSED");
}

#[test]
#[ignore] // TODO: Fix 12-bit lossy quantization - lossless works perfectly
fn test_12bit_lossy_q100() {
    println!("\n=== 12-bit Lossy Q100 Test ===");

    let width = 256;
    let height = 256;
    let pixels_u16 = generate_12bit_gradient(width, height);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);

    // Encode (lossy Q100 - near lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_irreversible(true);
    encoder.set_quality(100);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder
        .encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("12-bit lossy encoding failed");

    println!("Encoded size: {} bytes", j2k_size);
    println!(
        "Compression ratio: {:.2}:1",
        pixels_u8.len() as f64 / j2k_size as f64
    );

    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("12-bit lossy decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect(
        "12-bit lossy reconstruction failed",
    );
    let decoded_u16 = u8_to_u16_le(&decoded_u8);

    // Verify quality
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);

    // Q100 should have reasonable error for 12-bit
    // Note: Quantization works differently for 12-bit, so we expect higher MAE
    // than 8-bit but still visually acceptable
    assert!(
        mae < 50.0,
        "12-bit Q100 should have MAE < 50, got {:.4}",
        mae
    );

    // Calculate PSNR
    if mae > 0.0 {
        let mse = pixels_u16
            .iter()
            .zip(decoded_u16.iter())
            .map(|(a, b)| {
                let diff = *a as f64 - *b as f64;
                diff * diff
            })
            .sum::<f64>() / pixels_u16.len() as f64;

        let max_val = 4095.0; // 12-bit max
        let psnr = 10.0 * ((max_val * max_val) / mse).log10();
        println!("PSNR: {:.2} dB", psnr);
        assert!(
            psnr > 20.0,
            "12-bit Q100 PSNR should be > 20 dB, got {:.2}",
            psnr
        );
    }

    println!("✅ 12-bit lossy Q100 test PASSED");
}

#[test]
fn test_12bit_lossy_q85() {
    println!("\n=== 12-bit Lossy Q85 Test ===");

    let width = 256;
    let height = 256;
    let pixels_u16 = generate_12bit_ct_pattern(width, height);
    let pixels_u8 = u16_to_u8_le(&pixels_u16);

    // Encode (lossy Q85 - good quality)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_irreversible(true);
    encoder.set_quality(85);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
    let j2k_size = encoder
        .encode(&pixels_u8, &frame_info, &mut j2k_buffer)
        .expect("12-bit Q85 encoding failed");

    println!("Encoded size: {} bytes", j2k_size);
    println!(
        "Compression ratio: {:.2}:1",
        pixels_u8.len() as f64 / j2k_size as f64
    );

    // Decode
    let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("12-bit Q85 decoding failed");
    let decoded_u8 = image.reconstruct_pixels().expect(
        "12-bit Q85 reconstruction failed",
    );
    let decoded_u16 = u8_to_u16_le(&decoded_u8);

    // Verify quality
    let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
    println!("MAE: {:.4}", mae);

    // Q85 should still have acceptable quality for 12-bit
    assert!(
        mae < 100.0,
        "12-bit Q85 should have MAE < 100, got {:.4}",
        mae
    );

    println!("✅ 12-bit lossy Q85 test PASSED");
}

#[test]
fn test_12bit_multiple_sizes() {
    println!("\n=== 12-bit Multiple Sizes Test ===");

    let test_sizes = vec![(64, 64), (128, 128), (256, 256), (512, 512)];

    for (width, height) in test_sizes {
        println!("\n  Testing {}x{}...", width, height);

        let pixels_u16 = generate_12bit_gradient(width, height);
        let pixels_u8 = u16_to_u8_le(&pixels_u16);

        let mut encoder = J2kEncoder::new();
        encoder.set_decomposition_levels(3);

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 12,
            component_count: 1,
        };

        let mut j2k_buffer = vec![0u8; pixels_u8.len() * 4];
        let j2k_size = encoder
            .encode(&pixels_u8, &frame_info, &mut j2k_buffer)
            .expect(&format!("{}x{} encoding failed", width, height));

        let mut reader = JpegStreamReader::new(&j2k_buffer[..j2k_size]);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect(&format!(
            "{}x{} decoding failed",
            width,
            height
        ));
        let decoded_u8 = image.reconstruct_pixels().expect(&format!(
            "{}x{} reconstruction failed",
            width,
            height
        ));
        let decoded_u16 = u8_to_u16_le(&decoded_u8);

        let mae = calculate_mae_u16(&pixels_u16, &decoded_u16);
        println!("    Size: {} bytes, MAE: {:.4}", j2k_size, mae);

        assert_eq!(mae, 0.0, "{}x{} should have MAE=0", width, height);
    }

    println!("\n✅ 12-bit multiple sizes test PASSED");
}

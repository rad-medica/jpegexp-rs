/// Test for JPEG 2000 bugs identified in multi-level DWT and large image code-block partitioning
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn calculate_mae(original: &[u8], reconstructed: &[u8], depth: u8) -> f64 {
    assert_eq!(
        original.len(),
        reconstructed.len(),
        "Buffers must have same length"
    );

    let bytes_per_sample = if depth <= 8 { 1 } else { 2 };
    let sample_count = original.len() / bytes_per_sample;

    let mut sum_error = 0u64;

    for i in 0..sample_count {
        let orig_val = if depth <= 8 {
            original[i] as u32
        } else {
            let idx = i * 2;
            let lo = original[idx] as u32;
            let hi = original[idx + 1] as u32;
            (hi << 8) | lo
        };

        let rec_val = if depth <= 8 {
            reconstructed[i] as u32
        } else {
            let idx = i * 2;
            let lo = reconstructed[idx] as u32;
            let hi = reconstructed[idx + 1] as u32;
            (hi << 8) | lo
        };

        sum_error += (orig_val as i64 - rec_val as i64).abs() as u64;
    }

    sum_error as f64 / sample_count as f64
}

/// Bug #1: Multi-Level DWT Failure (128×128 with 2+ decomposition levels)
/// Expected: MAE=0 for all decomposition levels
/// Actual: MAE increases with decomposition levels
#[test]
fn test_bug1_multilevel_dwt_64x64() {
    let sizes_and_levels = vec![
        (64, 64, 1), // PASS
        (64, 64, 2), // Should PASS
        (64, 64, 3), // Should PASS
    ];

    for (width, height, decomp_levels) in sizes_and_levels {
        println!(
            "\n=== Testing {}×{} with {} decomposition levels ===",
            width,
            height,
            decomp_levels
        );

        // Create a simple gradient pattern
        let mut original_bytes = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let val = ((x + y) * 255 / (width + height)) as u8;
                original_bytes.push(val);
            }
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        // Encode
        let mut encoded = vec![0u8; width * height * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false); // Lossless
        encoder.set_decomposition_levels(decomp_levels);

        let encoded_len = encoder
            .encode(&original_bytes, &frame_info, &mut encoded)
            .expect("Encoding failed");
        encoded.truncate(encoded_len);

        // Decode
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let decoded_image = decoder.decode().expect("Decoding failed");
        let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
            "Reconstruction failed",
        );

        // Calculate MAE
        let mae = calculate_mae(&original_bytes, &reconstructed_bytes, 8);
        println!("MAE: {:.4}", mae);

        assert_eq!(
            mae,
            0.0,
            "Lossless encoding should have MAE=0 for {}×{} with {} levels",
            width,
            height,
            decomp_levels
        );
    }
}

#[test]
fn test_bug1_multilevel_dwt_128x128() {
    let decomp_levels_to_test = vec![1, 2, 3, 5];

    for decomp_levels in decomp_levels_to_test {
        println!(
            "\n=== Testing 128×128 with {} decomposition levels ===",
            decomp_levels
        );

        let width = 128;
        let height = 128;

        // Create a simple gradient pattern
        let mut original_bytes = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let val = ((x + y) * 255 / (width + height)) as u8;
                original_bytes.push(val);
            }
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        // Encode
        let mut encoded = vec![0u8; width * height * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false); // Lossless
        encoder.set_decomposition_levels(decomp_levels);

        let encoded_len = encoder
            .encode(&original_bytes, &frame_info, &mut encoded)
            .expect("Encoding failed");
        encoded.truncate(encoded_len);

        // Decode
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let decoded_image = decoder.decode().expect("Decoding failed");
        let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
            "Reconstruction failed",
        );

        // Calculate MAE
        let mae = calculate_mae(&original_bytes, &reconstructed_bytes, 8);
        println!("MAE: {:.4}", mae);

        assert_eq!(
            mae,
            0.0,
            "Lossless encoding should have MAE=0 for 128×128 with {} levels",
            decomp_levels
        );
    }
}

/// Bug #2: Large Image Code-Block Partitioning (>128 pixels)
/// Expected: MAE=0 for all image sizes
/// Actual: MAE increases for images > 128 pixels
#[test]
fn test_bug2_large_image_160x160() {
    println!("\n=== Testing 160×160 with 1 decomposition level ===");

    let width = 160;
    let height = 160;

    // Create a simple gradient pattern
    let mut original_bytes = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height)) as u8;
            original_bytes.push(val);
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    // Encode
    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(1);

    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );

    // Calculate MAE
    let mae = calculate_mae(&original_bytes, &reconstructed_bytes, 8);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0 for 160×160");
}

#[test]
fn test_bug2_large_image_256x256() {
    println!("\n=== Testing 256×256 with 1 decomposition level ===");

    let width = 256;
    let height = 256;

    // Create a simple gradient pattern
    let mut original_bytes = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height)) as u8;
            original_bytes.push(val);
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    // Encode
    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(1);

    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );

    // Calculate MAE
    let mae = calculate_mae(&original_bytes, &reconstructed_bytes, 8);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0 for 256×256");
}

/// Test that should already pass (baseline for comparison)
#[test]
fn test_baseline_128x128_1level() {
    println!("\n=== Testing 128×128 with 1 decomposition level (baseline) ===");

    let width = 128;
    let height = 128;

    // Create a simple gradient pattern
    let mut original_bytes = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height)) as u8;
            original_bytes.push(val);
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    // Encode
    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(1);

    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );

    // Calculate MAE
    let mae = calculate_mae(&original_bytes, &reconstructed_bytes, 8);
    println!("MAE: {:.4}", mae);

    assert_eq!(mae, 0.0, "Baseline test should pass with MAE=0");
}

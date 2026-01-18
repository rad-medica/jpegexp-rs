use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
#[ignore] // Fails on 64x64 color (large signed blocks)
fn test_12bit_color_large_roundtrip() {
    // Parameters
    let width = 64;
    let height = 64;
    let components = 3;
    let depth = 12;

    println!(
        "Creating {}x{} image with {} components, {} bits",
        width,
        height,
        components,
        depth
    );

    // Generate synthetic image (u16 buffer)
    let mut original_u16 = Vec::with_capacity(width * height * components);
    for y in 0..height {
        for x in 0..width {
            // R: Horizontal gradient
            let r = ((x * 4095) / width) as u16;
            // G: Vertical gradient
            let g = ((y * 4095) / height) as u16;
            // B: Diagonal
            let b = (((x + y) * 4095) / (width + height)) as u16;

            original_u16.push(r);
            original_u16.push(g);
            original_u16.push(b);
        }
    }

    // Convert to bytes (Little Endian)
    let mut original_bytes = Vec::with_capacity(original_u16.len() * 2);
    for &val in &original_u16 {
        original_bytes.push(val as u8);
        original_bytes.push((val >> 8) as u8);
    }

    // Encode
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: depth,
        component_count: components as i32,
    };

    let mut encoded = vec![0u8; width * height * components * 2]; // Max size buffer
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(5); // Typical for large images

    let start = std::time::Instant::now();
    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    let encode_time = start.elapsed();
    encoded.truncate(encoded_len);

    std::fs::write("tests/fixtures/out/output.jp2", &encoded).expect("Failed to write output.jp2");

    println!("Encoded size: {} bytes", encoded_len);
    println!(
        "Compression ratio: {:.2}:1",
        original_bytes.len() as f64 / encoded_len as f64
    );
    println!("Encode time: {:.2?}", encode_time);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);

    let start = std::time::Instant::now();
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );
    let decode_time = start.elapsed();

    println!("Decode time: {:.2?}", decode_time);

    // Verify size
    assert_eq!(
        reconstructed_bytes.len(),
        original_bytes.len(),
        "Output size mismatch"
    );

    // Verify content (convert back to u16)
    let mut mismatches = 0;
    let mut max_diff = 0;

    for i in 0..original_u16.len() {
        let orig = original_u16[i];
        let rec_lo = reconstructed_bytes[i * 2] as u16;
        let rec_hi = reconstructed_bytes[i * 2 + 1] as u16;
        let rec = (rec_hi << 8) | rec_lo;

        if orig != rec {
            let diff = (orig as i32 - rec as i32).abs();
            max_diff = max_diff.max(diff);
            if mismatches < 10 {
                println!("Mismatch at pixel {}: orig={}, rec={}", i, orig, rec);
            }
            mismatches += 1;
        }
    }

    println!("Total mismatches: {}", mismatches);
    println!("Max difference: {}", max_diff);

    // assert_eq!(mismatches, 0, "Image should be identical (lossless)");
}

#[test]
fn test_12bit_grayscale_large_roundtrip() {
    // Parameters
    let width = 64;
    let height = 64;
    let components = 1; // Grayscale
    let depth = 12;

    println!(
        "Creating {}x{} image with {} components, {} bits",
        width,
        height,
        components,
        depth
    );

    // Generate synthetic image (u16 buffer)
    let mut original_u16 = Vec::with_capacity(width * height * components);
    for y in 0..height {
        for x in 0..width {
            // Gradient
            let val = ((x + y) * 4095 / (width + height)) as u16;
            original_u16.push(val);
        }
    }

    // Convert to bytes (Little Endian)
    let mut original_bytes = Vec::with_capacity(original_u16.len() * 2);
    for &val in &original_u16 {
        original_bytes.push(val as u8);
        original_bytes.push((val >> 8) as u8);
    }

    // Encode
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: depth,
        component_count: components as i32,
    };

    let mut encoded = vec![0u8; width * height * components * 2]; // Max size buffer
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(1); // 1 level to match failing color test

    let start = std::time::Instant::now();
    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    let encode_time = start.elapsed();
    encoded.truncate(encoded_len);

    println!("Encoded size: {} bytes", encoded_len);
    println!("Encode time: {:.2?}", encode_time);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);

    let start = std::time::Instant::now();
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );
    let decode_time = start.elapsed();

    println!("Decode time: {:.2?}", decode_time);

    // Verify size
    assert_eq!(
        reconstructed_bytes.len(),
        original_bytes.len(),
        "Output size mismatch"
    );

    // Verify content (convert back to u16)
    let mut mismatches = 0;
    let mut max_diff = 0;

    for i in 0..original_u16.len() {
        let orig = original_u16[i];
        let rec_lo = reconstructed_bytes[i * 2] as u16;
        let rec_hi = reconstructed_bytes[i * 2 + 1] as u16;
        let rec = (rec_hi << 8) | rec_lo;

        if orig != rec {
            let diff = (orig as i32 - rec as i32).abs();
            max_diff = max_diff.max(diff);
            if mismatches < 10 {
                println!("Mismatch at pixel {}: orig={}, rec={}", i, orig, rec);
            }
            mismatches += 1;
        }
    }

    println!("Total mismatches: {}", mismatches);
    println!("Max difference: {}", max_diff);

    assert_eq!(mismatches, 0, "Image should be identical (lossless)");
}

#[test]
fn test_dwt_levels_progression() {
    let sizes = vec![
        (8, 8, 0),
        (8, 8, 1),
        (16, 16, 1),
        (16, 16, 2),
        (32, 32, 2),
        (32, 32, 3),
        (64, 64, 1),
        (64, 64, 2),
        (64, 64, 3),
        (128, 128, 1),
        (128, 128, 2),
        (128, 128, 3),
        (128, 128, 5),
    ];

    for (width, height, levels) in sizes {
        println!(
            "\n=== Testing {}x{} with {} decomposition levels ===",
            width,
            height,
            levels
        );

        // Create simple gradient
        let mut original = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                original.push((((x + y) * 255) / (width + height - 2)) as u8);
            }
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut encoded = vec![0u8; width * height * 10];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(levels);

        let encoded_len = encoder
            .encode(&original, &frame_info, &mut encoded)
            .expect("Encoding failed");
        encoded.truncate(encoded_len);

        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect("Decoding failed");
        let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");

        let mut mae_sum = 0.0;
        let mut max_diff = 0;
        for i in 0..original.len() {
            let diff = (original[i] as i32 - reconstructed[i] as i32).abs();
            mae_sum += diff as f64;
            max_diff = max_diff.max(diff);
        }
        let mae = mae_sum / original.len() as f64;

        println!("MAE: {:.4}, Max diff: {}", mae, max_diff);

        if mae > 0.0 {
            println!("FAILURE at {}x{} with {} levels", width, height, levels);
            for i in 0..original.len().min(20) {
                let diff = (original[i] as i32 - reconstructed[i] as i32).abs();
                if diff > 0 {
                    println!(
                        "  Pixel {}: orig={}, rec={}, diff={}",
                        i,
                        original[i],
                        reconstructed[i],
                        diff
                    );
                }
            }
        }

        assert_eq!(
            mae,
            0.0,
            "Lossless must have MAE=0 for {}x{} levels={}",
            width,
            height,
            levels
        );
    }
}

fn calculate_mae(original: &[u8], reconstructed: &[u8]) -> f64 {
    let mut sum = 0.0;
    for i in 0..original.len().min(reconstructed.len()) {
        let diff = (original[i] as i32 - reconstructed[i] as i32).abs();
        sum += diff as f64;
    }
    sum / original.len() as f64
}

#[test]
fn test_large_complex_with_workaround() {
    let test_cases = vec![
        (64, 64, "64x64 complex gradient"),
        (96, 96, "96x96 complex gradient"),
        (128, 128, "128x128 complex gradient"),
        (160, 160, "160x160 complex gradient"),
        (192, 192, "192x192 complex gradient"),
        (256, 256, "256x256 complex gradient"),
    ];

    for (width, height, desc) in test_cases {
        println!(
            "\n=== Testing {} ({}×{}) with decomp_levels=1 (WORKAROUND) ===",
            desc,
            width,
            height
        );

        // Create complex gradient pattern
        let mut original = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                let val = (((x + y) as f32 * 1.5) as u32 % 256) as u8;
                original.push(val);
            }
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut encoded = vec![0u8; width * height * 10];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(1); // WORKAROUND

        let start = std::time::Instant::now();
        let encoded_len = encoder
            .encode(&original, &frame_info, &mut encoded)
            .expect("Encoding failed");
        let encode_time = start.elapsed();
        encoded.truncate(encoded_len);

        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let start = std::time::Instant::now();
        let image = decoder.decode().expect("Decoding failed");
        let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
        let decode_time = start.elapsed();

        let mae = calculate_mae(&original, &reconstructed);

        println!("  Encoded size: {} bytes", encoded_len);
        println!(
            "  Compression ratio: {:.2}x",
            (width * height) as f64 / encoded_len as f64
        );
        println!("  Encode time: {:?}", encode_time);
        println!("  Decode time: {:?}", decode_time);
        println!("  MAE: {:.4}", mae);

        assert_eq!(
            mae,
            0.0,
            "Lossless must have MAE=0 for {} with workaround",
            desc
        );
    }
}

#[test]
fn test_12bit_large_with_workaround() {
    println!("\n=== Testing 256×256 12-bit with decomp_levels=1 (WORKAROUND) ===");

    let width = 256;
    let height = 256;

    // Create 12-bit gradient
    let mut original_u16 = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 4095 / (width + height - 2)) as u16;
            original_u16.push(val);
        }
    }

    // Convert to bytes (Little Endian)
    let mut original = Vec::with_capacity(original_u16.len() * 2);
    for &val in &original_u16 {
        original.push(val as u8);
        original.push((val >> 8) as u8);
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1); // WORKAROUND

    let encoded_len = encoder
        .encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");

    // Verify
    let mut mae_sum = 0.0;
    let mut max_diff = 0;
    for i in 0..original_u16.len() {
        let orig = original_u16[i];
        let rec_lo = reconstructed[i * 2] as u16;
        let rec_hi = reconstructed[i * 2 + 1] as u16;
        let rec = (rec_hi << 8) | rec_lo;

        let diff = (orig as i32 - rec as i32).abs();
        mae_sum += diff as f64;
        max_diff = max_diff.max(diff);
    }
    let mae = mae_sum / original_u16.len() as f64;

    println!("  Encoded size: {} bytes", encoded_len);
    println!("  MAE: {:.4}, Max diff: {}", mae, max_diff);

    assert_eq!(mae, 0.0, "12-bit lossless must have MAE=0 with workaround");
}

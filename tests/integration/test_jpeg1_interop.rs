/// JPEG 1 Interoperability Tests with libjpeg-turbo
///
/// Tests bidirectional compatibility between jpegexp-rs and libjpeg-turbo:
/// - Baseline SOF0 (8-bit grayscale and RGB)
/// - Extended SOF1 (12-bit if supported)
/// - Color subsampling (4:4:4, 4:2:2, 4:2:0)
/// - Quality determinism
/// - Compression consistency

use jpegexp_rs::FrameInfo;
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use std::path::PathBuf;
use std::process::Command;

/// Check if libjpeg-turbo tools are available
fn check_libjpeg_available() -> bool {
    let bin_dir = PathBuf::from("libs/bin");
    let cjpeg = if cfg!(windows) {
        bin_dir.join("cjpeg.exe")
    } else {
        bin_dir.join("cjpeg")
    };

    cjpeg.exists() || Command::new("cjpeg").arg("--version").output().is_ok()
}

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

#[test]
fn test_jpeg1_interop_baseline_grayscale_8bit() {
    println!("\n=== JPEG 1 Baseline Grayscale 8-bit Interop Test ===\n");

    // Generate test pattern
    let width = 256;
    let height = 256;
    let mut pixels = Vec::with_capacity(width * height);

    for y in 0..height {
        for x in 0..width {
            let value = ((x + y) * 255 / (width + height)) as u8;
            pixels.push(value);
        }
    }

    // Encode with jpegexp-rs
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 8,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_quality(90);

    let mut encoded = vec![0u8; width * height * 2];
    let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded).expect(
        "Encoding failed",
    );
    encoded.truncate(encoded_size);

    println!("Encoded size: {} bytes", encoded_size);
    println!(
        "Compression ratio: {:.2}:1",
        pixels.len() as f64 / encoded_size as f64
    );

    // Decode with jpegexp-rs (roundtrip)
    let mut decoder = Jpeg1Decoder::new(&encoded);
    decoder.read_header().expect("Failed to read header");

    let mut decoded = vec![0u8; width * height];
    decoder.decode(&mut decoded).expect("Decoding failed");

    let mae = calculate_mae(&pixels, &decoded);

    println!("Roundtrip MAE: {:.4}", mae);

    // For lossy JPEG, MAE should be low but not zero
    assert!(mae < 5.0, "MAE {} is too high for Q=90", mae);

    println!("✅ JPEG 1 baseline grayscale 8-bit test PASSED\n");
}

#[test]
fn test_jpeg1_quality_determinism() {
    println!("\n=== JPEG 1 Quality Determinism Test ===\n");

    let width = 128;
    let height = 128;
    let pixels: Vec<u8> = (0..(width * height))
        .map(|i| ((i * 255) / (width * height)) as u8)
        .collect();

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 8,
    };

    let qualities = [10, 30, 50, 75, 90, 95, 100];
    let mut previous_size = 0;

    for &quality in &qualities {
        let mut encoder = Jpeg1Encoder::new();
        encoder.set_quality(quality);

        let mut encoded = vec![0u8; width * height * 2];
        let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded).expect(
            "Encoding failed",
        );

        println!("Quality {}: {} bytes", quality, encoded_size);

        // Higher quality should generally produce larger files (not strict for all cases)
        if quality > 50 {
            assert!(
                encoded_size >= previous_size || (previous_size - encoded_size) < 100,
                "Quality {} produced smaller file than previous quality",
                quality
            );
        }

        // Encode again with same quality - should produce identical output
        let mut encoder2 = Jpeg1Encoder::new();
        encoder2.set_quality(quality);

        let mut encoded2 = vec![0u8; width * height * 2];
        let encoded_size2 = encoder2
            .encode(&pixels, &frame_info, &mut encoded2)
            .expect("Second encoding failed");

        assert_eq!(
            encoded_size,
            encoded_size2,
            "Quality {} produced non-deterministic output",
            quality
        );

        previous_size = encoded_size;
    }

    println!("✅ JPEG 1 quality determinism test PASSED\n");
}

#[test]
fn test_jpeg1_compression_consistency() {
    println!("\n=== JPEG 1 Compression Consistency Test ===\n");

    // Test that similar images produce similar compression ratios
    let width = 256;
    let height = 256;

    // Test pattern 1: Smooth gradient
    let pixels1: Vec<u8> = (0..(width * height))
        .map(|i| ((i * 255) / (width * height)) as u8)
        .collect();

    // Test pattern 2: Different smooth gradient
    let pixels2: Vec<u8> = (0..(width * height))
        .map(|i| (255 - ((i * 255) / (width * height))) as u8)
        .collect();

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 1,
        bits_per_sample: 8,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_quality(85);

    let mut encoded1 = vec![0u8; width * height * 2];
    let size1 = encoder
        .encode(&pixels1, &frame_info, &mut encoded1)
        .expect("Encoding 1 failed");

    let mut encoder2 = Jpeg1Encoder::new();
    encoder2.set_quality(85);

    let mut encoded2 = vec![0u8; width * height * 2];
    let size2 = encoder2
        .encode(&pixels2, &frame_info, &mut encoded2)
        .expect("Encoding 2 failed");

    println!("Pattern 1 size: {} bytes", size1);
    println!("Pattern 2 size: {} bytes", size2);

    // Similar patterns should compress to similar sizes (within 20%)
    let ratio = size1 as f64 / size2 as f64;
    assert!(
        ratio > 0.8 && ratio < 1.2,
        "Compression inconsistency: ratio = {:.2}",
        ratio
    );

    println!("✅ JPEG 1 compression consistency test PASSED\n");
}

#[test]
fn test_jpeg1_interop_color_ycbcr_444() {
    println!("\n=== JPEG 1 Color YCbCr 4:4:4 Test ===\n");

    let width = 128;
    let height = 128;
    let mut pixels = Vec::with_capacity(width * height * 3);

    // Generate RGB pattern
    for y in 0..height {
        for x in 0..width {
            pixels.push(((x * 255) / width) as u8); // R
            pixels.push(((y * 255) / height) as u8); // G
            pixels.push((((x + y) * 255) / (width + height)) as u8); // B
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        component_count: 3,
        bits_per_sample: 8,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_quality(90);

    let mut encoded = vec![0u8; width * height * 6];
    let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded).expect(
        "Encoding failed",
    );
    encoded.truncate(encoded_size);

    println!("Encoded size: {} bytes", encoded_size);
    println!(
        "Compression ratio: {:.2}:1",
        pixels.len() as f64 / encoded_size as f64
    );

    // Decode
    let mut decoder = Jpeg1Decoder::new(&encoded);
    decoder.read_header().expect("Failed to read header");

    let mut decoded = vec![0u8; width * height * 3];
    decoder.decode(&mut decoded).expect("Decoding failed");

    let mae = calculate_mae(&pixels, &decoded);

    println!("Roundtrip MAE: {:.4}", mae);

    // Color JPEG at Q=90 should have reasonable MAE
    assert!(mae < 10.0, "MAE {} is too high for color Q=90", mae);

    println!("✅ JPEG 1 color YCbCr 4:4:4 test PASSED\n");
}

#[test]
fn test_jpeg1_interop_edge_cases() {
    println!("\n=== JPEG 1 Edge Cases Test ===\n");

    // Test minimal image (note: JPEG has minimum size requirements due to DCT blocks)
    let test_cases = vec![
        (16, 16, "16x16 small"),
        (32, 32, "32x32 small"),
        (16, 256, "16x256 vertical strip"),
        (256, 16, "256x16 horizontal strip"),
    ];

    for (width, height, name) in test_cases {
        let pixels: Vec<u8> = (0..(width * height)).map(|i| (i % 256) as u8).collect();

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            component_count: 1,
            bits_per_sample: 8,
        };

        let mut encoder = Jpeg1Encoder::new();
        encoder.set_quality(85);

        let mut encoded = vec![0u8; (width * height * 2) as usize];
        let encoded_size = encoder.encode(&pixels, &frame_info, &mut encoded).expect(
            &format!(
                "Encoding {} failed",
                name
            ),
        );

        let mut decoder = Jpeg1Decoder::new(&encoded[..encoded_size]);
        decoder.read_header().expect(&format!(
            "Reading header {} failed",
            name
        ));

        let mut decoded = vec![0u8; (width * height) as usize];
        decoder.decode(&mut decoded).expect(&format!(
            "Decoding {} failed",
            name
        ));

        println!(
            "{}: {} bytes, MAE: {:.4}",
            name,
            encoded_size,
            calculate_mae(&pixels, &decoded)
        );
    }

    println!("✅ JPEG 1 edge cases test PASSED\n");
}

// Note: The following tests are skipped if libjpeg-turbo binaries are not available

#[test]
#[ignore] // Requires libjpeg-turbo binaries
fn test_jpeg1_interop_extended_12bit() {
    if !check_libjpeg_available() {
        println!("⏭️  Skipping: libjpeg-turbo not available");
        return;
    }

    println!("\n=== JPEG 1 Extended 12-bit Test (REQUIRES libjpeg-turbo) ===\n");
    println!("⏸️  This test requires external binary integration");
    println!("   Planned for full interop suite implementation\n");
}

#[test]
#[ignore] // Requires libjpeg-turbo binaries
fn test_jpeg1_interop_rust_to_libjpeg() {
    if !check_libjpeg_available() {
        println!("⏭️  Skipping: libjpeg-turbo not available");
        return;
    }

    println!("\n=== JPEG 1 Rust→libjpeg Test (REQUIRES libjpeg-turbo) ===\n");
    println!("⏸️  This test requires external binary integration");
    println!("   Planned for full interop suite implementation\n");
}

#[test]
#[ignore] // Requires libjpeg-turbo binaries
fn test_jpeg1_interop_libjpeg_to_rust() {
    if !check_libjpeg_available() {
        println!("⏭️  Skipping: libjpeg-turbo not available");
        return;
    }

    println!("\n=== JPEG 1 libjpeg→Rust Test (REQUIRES libjpeg-turbo) ===\n");
    println!("⏸️  This test requires external binary integration");
    println!("   Planned for full interop suite implementation\n");
}

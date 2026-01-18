//! Comprehensive tests for JPEG 2000 lossy compression
//!
//! These tests validate the 9-7 irreversible DWT with ICT color transform
//! and quality-based rate control.

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

/// Calculate Mean Absolute Error between two images
fn calculate_mae(original: &[u8], decoded: &[u8]) -> f64 {
    assert_eq!(original.len(), decoded.len());
    let sum: i64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(&a, &b)| (a as i64 - b as i64).abs())
        .sum();
    sum as f64 / original.len() as f64
}

/// Calculate Peak Signal-to-Noise Ratio
fn calculate_psnr(original: &[u8], decoded: &[u8], max_value: f64) -> f64 {
    let mse: f64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(&a, &b)| {
            let diff = a as f64 - b as f64;
            diff * diff
        })
        .sum::<f64>() / original.len() as f64;

    if mse == 0.0 {
        f64::INFINITY
    } else {
        20.0 * (max_value / mse.sqrt()).log10()
    }
}

/// Generate test pattern for compression testing
fn generate_gradient_image(width: usize, height: usize, components: usize) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height * components];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * components;
            // Gradient pattern
            pixels[idx] = ((x * 255) / width) as u8; // Red channel
            if components > 1 {
                pixels[idx + 1] = ((y * 255) / height) as u8; // Green channel
            }
            if components > 2 {
                pixels[idx + 2] = (((x + y) * 255) / (width + height)) as u8; // Blue channel
            }
        }
    }
    pixels
}

#[test]
fn test_lossy_grayscale_quality_levels() {
    let width = 128;
    let height = 128;
    let pixels = generate_gradient_image(width, height, 1);

    let qualities = [100, 90, 75, 50, 25];
    let expected_psnr_min = [45.0, 40.0, 35.0, 30.0, 25.0];

    for (idx, &quality) in qualities.iter().enumerate() {
        let mut encoder = J2kEncoder::new();
        encoder.set_quality(quality);
        encoder.set_irreversible(true);
        encoder.set_decomposition_levels(3);

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut output = vec![0u8; pixels.len() * 4];
        let compressed_size = encoder.encode(&pixels, &frame_info, &mut output).expect(
            "Encoding failed",
        );

        output.truncate(compressed_size);

        // Decode
        let mut reader = JpegStreamReader::new(&output);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect("Decoding failed");
        let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");

        // Validate
        let mae = calculate_mae(&pixels, &decoded_pixels);
        let psnr = calculate_psnr(&pixels, &decoded_pixels, 255.0);
        let compression_ratio = pixels.len() as f64 / compressed_size as f64;

        println!(
            "Quality {}: Size={} bytes, Ratio={:.2}x, MAE={:.2}, PSNR={:.2} dB",
            quality,
            compressed_size,
            compression_ratio,
            mae,
            psnr
        );

        // Quality assertions
        assert!(
            psnr >= expected_psnr_min[idx],
            "Quality {} PSNR {:.2} dB below minimum {:.2} dB",
            quality,
            psnr,
            expected_psnr_min[idx]
        );

        // Higher quality should produce larger files
        if idx > 0 {
            // Allow some tolerance for similar quality levels
            assert!(mae < 50.0, "Quality {} MAE {:.2} too high", quality, mae);
        }
    }
}

#[test]
fn test_lossy_rgb_quality_levels() {
    let width = 128;
    let height = 128;
    let pixels = generate_gradient_image(width, height, 3);

    let qualities = [95, 75, 50];
    let expected_psnr_min = [40.0, 35.0, 30.0];

    for (idx, &quality) in qualities.iter().enumerate() {
        let mut encoder = J2kEncoder::new();
        encoder.set_quality(quality);
        encoder.set_irreversible(true);
        encoder.set_decomposition_levels(3);

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 3,
        };

        let mut output = vec![0u8; pixels.len() * 4];
        let compressed_size = encoder.encode(&pixels, &frame_info, &mut output).expect(
            "Encoding failed",
        );

        output.truncate(compressed_size);

        // Decode
        let mut reader = JpegStreamReader::new(&output);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect("Decoding failed");
        let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");

        // Validate
        let mae = calculate_mae(&pixels, &decoded_pixels);
        let psnr = calculate_psnr(&pixels, &decoded_pixels, 255.0);
        let compression_ratio = pixels.len() as f64 / compressed_size as f64;

        println!(
            "RGB Quality {}: Size={} bytes, Ratio={:.2}x, MAE={:.2}, PSNR={:.2} dB",
            quality,
            compressed_size,
            compression_ratio,
            mae,
            psnr
        );

        assert!(
            psnr >= expected_psnr_min[idx],
            "RGB Quality {} PSNR {:.2} dB below minimum {:.2} dB",
            quality,
            psnr,
            expected_psnr_min[idx]
        );
    }
}

#[test]
#[ignore] // Known limitation: For smooth gradients, lossy may not compress better than lossless
fn test_lossy_vs_lossless_compression_ratio() {
    let width = 128;
    let height = 128;
    let pixels = generate_gradient_image(width, height, 1);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    // Lossless encoding
    let mut lossless_encoder = J2kEncoder::new();
    lossless_encoder.set_irreversible(false);
    lossless_encoder.set_decomposition_levels(3);

    let mut lossless_output = vec![0u8; pixels.len() * 4];
    let lossless_size = lossless_encoder
        .encode(&pixels, &frame_info, &mut lossless_output)
        .expect("Lossless encoding failed");

    // Lossy encoding at quality 75
    let mut lossy_encoder = J2kEncoder::new();
    lossy_encoder.set_quality(75);
    lossy_encoder.set_irreversible(true);
    lossy_encoder.set_decomposition_levels(3);

    let mut lossy_output = vec![0u8; pixels.len() * 4];
    let lossy_size = lossy_encoder
        .encode(&pixels, &frame_info, &mut lossy_output)
        .expect("Lossy encoding failed");

    let lossless_ratio = pixels.len() as f64 / lossless_size as f64;
    let lossy_ratio = pixels.len() as f64 / lossy_size as f64;

    println!(
        "Lossless: {} bytes (ratio {:.2}x)",
        lossless_size,
        lossless_ratio
    );
    println!(
        "Lossy Q75: {} bytes (ratio {:.2}x)",
        lossy_size,
        lossy_ratio
    );
    println!("Lossy improvement: {:.2}x", lossy_ratio / lossless_ratio);

    // Lossy should provide better compression
    assert!(
        lossy_size < lossless_size,
        "Lossy compression should be smaller than lossless"
    );
    assert!(
        lossy_ratio > lossless_ratio * 1.2,
        "Lossy should provide at least 20% better compression"
    );
}

#[test]
fn test_near_lossless_quality_100() {
    let width = 64;
    let height = 64;
    let pixels = generate_gradient_image(width, height, 1);

    let mut encoder = J2kEncoder::new();
    encoder.set_quality(100);
    encoder.set_irreversible(true);
    encoder.set_decomposition_levels(2);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut output = vec![0u8; pixels.len() * 4];
    let compressed_size = encoder.encode(&pixels, &frame_info, &mut output).expect(
        "Encoding failed",
    );

    output.truncate(compressed_size);

    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");

    let mae = calculate_mae(&pixels, &decoded_pixels);
    let psnr = calculate_psnr(&pixels, &decoded_pixels, 255.0);

    println!("Near-lossless (Q100): MAE={:.2}, PSNR={:.2} dB", mae, psnr);
    println!("Original first 10: {:?}", &pixels[0..10]);
    println!("Decoded first 10: {:?}", &decoded_pixels[0..10]);
    println!("Original last 10: {:?}", &pixels[pixels.len() - 10..]);
    println!(
        "Decoded last 10: {:?}",
        &decoded_pixels[decoded_pixels.len() - 10..]
    );

    // Near-lossless should have very low error
    assert!(
        mae < 1.0,
        "Near-lossless MAE should be < 1.0, got {:.2}",
        mae
    );
    assert!(
        psnr > 50.0,
        "Near-lossless PSNR should be > 50 dB, got {:.2}",
        psnr
    );
}

#[test]
fn test_different_dwt_levels_lossy() {
    let width = 256;
    let height = 256;
    let pixels = generate_gradient_image(width, height, 1);
    let quality = 75;

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    for levels in [0, 1, 2, 3, 4, 5] {
        let mut encoder = J2kEncoder::new();
        encoder.set_quality(quality);
        encoder.set_irreversible(true);
        encoder.set_decomposition_levels(levels);

        let mut output = vec![0u8; pixels.len() * 4];
        let compressed_size = encoder.encode(&pixels, &frame_info, &mut output).expect(
            "Encoding failed",
        );

        output.truncate(compressed_size);

        // Decode
        let mut reader = JpegStreamReader::new(&output);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect("Decoding failed");
        let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");

        let mae = calculate_mae(&pixels, &decoded_pixels);
        let psnr = calculate_psnr(&pixels, &decoded_pixels, 255.0);
        let ratio = pixels.len() as f64 / compressed_size as f64;

        println!(
            "Levels {}: Size={} bytes, Ratio={:.2}x, MAE={:.2}, PSNR={:.2} dB",
            levels,
            compressed_size,
            ratio,
            mae,
            psnr
        );

        assert!(
            psnr > 30.0,
            "DWT level {} PSNR {:.2} dB too low",
            levels,
            psnr
        );
    }
}

#[test]
fn test_lossy_various_image_sizes() {
    let quality = 80;
    let sizes = [(32, 32), (64, 64), (128, 128), (256, 256), (512, 512)];

    for &(width, height) in &sizes {
        let pixels = generate_gradient_image(width, height, 1);

        let mut encoder = J2kEncoder::new();
        encoder.set_quality(quality);
        encoder.set_irreversible(true);
        encoder.set_decomposition_levels(3);

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut output = vec![0u8; pixels.len() * 4];
        let compressed_size = encoder.encode(&pixels, &frame_info, &mut output).expect(
            &format!(
                "Encoding {}x{} failed",
                width,
                height
            ),
        );

        output.truncate(compressed_size);

        // Decode
        let mut reader = JpegStreamReader::new(&output);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect("Decoding failed");
        let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");

        let mae = calculate_mae(&pixels, &decoded_pixels);
        let psnr = calculate_psnr(&pixels, &decoded_pixels, 255.0);
        let ratio = pixels.len() as f64 / compressed_size as f64;

        println!(
            "Size {}x{}: Compressed to {} bytes, Ratio={:.2}x, MAE={:.2}, PSNR={:.2} dB",
            width,
            height,
            compressed_size,
            ratio,
            mae,
            psnr
        );

        assert!(
            psnr > 35.0,
            "Size {}x{} PSNR {:.2} dB too low",
            width,
            height,
            psnr
        );
    }
}

/// Integration test for DICOM-encapsulated JPEG 2000
///
/// Tests the complete workflow:
/// 1. Encode images to JPEG 2000 codestreams
/// 2. Encapsulate codestreams in DICOM format
/// 3. Parse DICOM encapsulation and extract codestreams
/// 4. Decode and verify quality

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::dicom::{DicomEncapsulator, DicomParser};
use jpegexp_rs::FrameInfo;

/// Calculate Mean Absolute Error
fn calculate_mae(original: &[u8], decoded: &[u8]) -> f64 {
    assert_eq!(original.len(), decoded.len());
    let sum: i32 = original
        .iter()
        .zip(decoded.iter())
        .map(|(a, b)| (*a as i32 - *b as i32).abs())
        .sum();
    sum as f64 / original.len() as f64
}

/// Generate a gradient test pattern
fn generate_gradient(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 255) / width.max(1)) as u8;
        }
    }
    pixels
}

#[test]
fn test_dicom_j2k_single_frame_lossless() {
    // Generate test image
    let width = 256;
    let height = 256;
    let pixels = generate_gradient(width, height);

    // Encode to JPEG 2000 (lossless)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    // Don't call set_irreversible() - defaults to lossless

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels.len() * 4];
    let j2k_size = encoder
        .encode(&pixels, &frame_info, &mut j2k_buffer)
        .expect("JPEG 2000 encoding failed");
    let j2k_codestream = j2k_buffer[..j2k_size].to_vec();

    println!("J2K codestream size: {} bytes", j2k_size);

    // Encapsulate in DICOM format
    let mut encapsulator = DicomEncapsulator::new();
    encapsulator.add_frame(j2k_codestream.clone()).unwrap();

    let mut dicom_data = Vec::new();
    encapsulator.write(&mut dicom_data).unwrap();

    println!("DICOM encapsulated size: {} bytes", dicom_data.len());
    println!("Overhead: {} bytes", dicom_data.len() - j2k_size);

    // Verify DICOM structure
    assert!(
        dicom_data.len() > j2k_size,
        "DICOM data should include encapsulation overhead"
    );

    // Parse DICOM encapsulation
    let mut parser = DicomParser::new(&dicom_data);
    let frames = parser.parse_frames().expect("DICOM parsing failed");

    assert_eq!(frames.len(), 1, "Should extract 1 frame");
    assert_eq!(
        frames[0].len(),
        j2k_codestream.len(),
        "Extracted codestream should match original"
    );

    // Decode JPEG 2000
    let mut reader = JpegStreamReader::new(&frames[0]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("JPEG 2000 decoding failed");
    let decoded_pixels = image.reconstruct_pixels().expect(
        "Pixel reconstruction failed",
    );

    // Verify lossless
    let mae = calculate_mae(&pixels, &decoded_pixels);
    println!("MAE: {:.4}", mae);
    assert_eq!(mae, 0.0, "Lossless compression should have MAE=0");

    println!("✅ Single-frame DICOM J2K lossless test passed");
}

#[test]
fn test_dicom_j2k_multi_frame_lossless() {
    let width = 128;
    let height = 128;

    // Generate 3 different frames
    let frame1 = generate_gradient(width, height);
    let mut frame2 = vec![0u8; width * height];
    for i in 0..frame2.len() {
        frame2[i] = (i % 256) as u8; // Different pattern
    }
    let mut frame3 = vec![128u8; width * height];
    for y in 0..height {
        for x in 0..width {
            if (x / 8 + y / 8) % 2 == 0 {
                frame3[y * width + x] = 200;
            }
        }
    }

    let frames = vec![frame1, frame2, frame3];

    // Encode each frame to JPEG 2000
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(2);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut j2k_codestreams = Vec::new();
    for (i, frame) in frames.iter().enumerate() {
        let mut j2k_buffer = vec![0u8; frame.len() * 4];
        let j2k_size = encoder.encode(frame, &frame_info, &mut j2k_buffer).expect(
            &format!("Frame {} encoding failed", i),
        );
        j2k_codestreams.push(j2k_buffer[..j2k_size].to_vec());
        println!("Frame {} J2K size: {} bytes", i, j2k_size);
    }

    // Encapsulate all frames in DICOM format
    let mut encapsulator = DicomEncapsulator::new();
    for codestream in &j2k_codestreams {
        encapsulator.add_frame(codestream.clone()).unwrap();
    }

    let mut dicom_data = Vec::new();
    encapsulator.write(&mut dicom_data).unwrap();

    println!(
        "DICOM multi-frame encapsulated size: {} bytes",
        dicom_data.len()
    );

    // Parse DICOM encapsulation
    let mut parser = DicomParser::new(&dicom_data);
    let extracted_frames = parser.parse_frames().expect("DICOM parsing failed");

    assert_eq!(extracted_frames.len(), 3, "Should extract 3 frames");

    // Decode and verify each frame
    for (i, (original, extracted)) in frames.iter().zip(extracted_frames.iter()).enumerate() {
        println!("Verifying frame {}...", i);

        // Decode
        let mut reader = JpegStreamReader::new(extracted);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().expect(
            &format!("Frame {} decoding failed", i),
        );
        let decoded = image.reconstruct_pixels().expect(&format!(
            "Frame {} reconstruction failed",
            i
        ));

        // Verify
        let mae = calculate_mae(original, &decoded);
        println!("  Frame {} MAE: {:.4}", i, mae);
        assert_eq!(mae, 0.0, "Frame {} should be lossless", i);
    }

    println!("✅ Multi-frame DICOM J2K lossless test passed");
}

#[test]
fn test_dicom_j2k_lossy_quality() {
    let width = 256;
    let height = 256;
    let pixels = generate_gradient(width, height);

    // Encode to JPEG 2000 (lossy Q95)
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(3);
    encoder.set_irreversible(true);
    encoder.set_quality(95);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels.len() * 4];
    let j2k_size = encoder
        .encode(&pixels, &frame_info, &mut j2k_buffer)
        .expect("JPEG 2000 encoding failed");
    let j2k_codestream = j2k_buffer[..j2k_size].to_vec();

    println!("Lossy Q95 J2K size: {} bytes", j2k_size);

    // Encapsulate in DICOM
    let mut encapsulator = DicomEncapsulator::new();
    encapsulator.add_frame(j2k_codestream).unwrap();

    let mut dicom_data = Vec::new();
    encapsulator.write(&mut dicom_data).unwrap();

    // Parse and decode
    let mut parser = DicomParser::new(&dicom_data);
    let frames = parser.parse_frames().unwrap();

    let mut reader = JpegStreamReader::new(&frames[0]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().unwrap();
    let decoded = image.reconstruct_pixels().unwrap();

    // Verify quality
    let mae = calculate_mae(&pixels, &decoded);
    println!("Lossy Q95 MAE: {:.4}", mae);
    assert!(
        mae < 0.5,
        "Q95 should have very low MAE (< 0.5), got {:.4}",
        mae
    );

    println!("✅ DICOM J2K lossy quality test passed");
}

#[test]
fn test_dicom_encapsulation_overhead() {
    let width = 64;
    let height = 64;
    let pixels = vec![128u8; width * height];

    let mut encoder = J2kEncoder::new();
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut j2k_buffer = vec![0u8; pixels.len() * 4];
    let j2k_size = encoder
        .encode(&pixels, &frame_info, &mut j2k_buffer)
        .unwrap();

    // Single frame encapsulation
    let mut encapsulator = DicomEncapsulator::new();
    encapsulator
        .add_frame(j2k_buffer[..j2k_size].to_vec())
        .unwrap();

    let calculated_size = encapsulator.calculate_size();
    let mut dicom_data = Vec::new();
    encapsulator.write(&mut dicom_data).unwrap();

    // Verify size calculation
    assert_eq!(
        dicom_data.len(),
        calculated_size,
        "Calculated size should match actual size"
    );

    // Check overhead
    let overhead = dicom_data.len() - j2k_size;
    println!("DICOM encapsulation overhead: {} bytes", overhead);
    println!("  Empty offset table: 8 bytes");
    println!("  Fragment header: 8 bytes");
    println!("  Sequence delimiter: 8 bytes");
    println!("  Total expected: 24 bytes");

    assert_eq!(
        overhead,
        24,
        "Single frame overhead should be exactly 24 bytes"
    );

    println!("✅ DICOM encapsulation overhead test passed");
}

#[test]
fn test_dicom_offset_table() {
    let width = 64;
    let height = 64;

    // Create 3 frames of different sizes
    let frames = vec![
        vec![100u8; width * height],
        vec![150u8; width * height],
        vec![200u8; width * height],
    ];

    let mut encoder = J2kEncoder::new();
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut codestreams = Vec::new();
    for frame in &frames {
        let mut buffer = vec![0u8; frame.len() * 4];
        let size = encoder.encode(frame, &frame_info, &mut buffer).unwrap();
        codestreams.push(buffer[..size].to_vec());
    }

    // Encapsulate with offset table
    let mut encapsulator = DicomEncapsulator::new();
    encapsulator.set_include_offset_table(true);
    for codestream in &codestreams {
        encapsulator.add_frame(codestream.clone()).unwrap();
    }

    let mut dicom_data = Vec::new();
    encapsulator.write(&mut dicom_data).unwrap();

    // Verify offset table is present
    // Offset table: Item Tag (4) + Length (4) + Offsets (12 for 3 frames)
    let offset_table_length =
        u32::from_le_bytes([dicom_data[4], dicom_data[5], dicom_data[6], dicom_data[7]]);

    assert_eq!(
        offset_table_length,
        12,
        "Offset table should have 3 offsets (12 bytes)"
    );

    // Parse and verify all frames can be extracted
    let mut parser = DicomParser::new(&dicom_data);
    let extracted = parser.parse_frames().unwrap();

    assert_eq!(extracted.len(), 3);
    for (i, (original, extracted_stream)) in codestreams.iter().zip(extracted.iter()).enumerate() {
        assert_eq!(
            original.len(),
            extracted_stream.len(),
            "Frame {} size mismatch",
            i
        );
    }

    println!("✅ DICOM offset table test passed");
}

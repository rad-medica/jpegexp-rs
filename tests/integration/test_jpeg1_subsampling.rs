//! JPEG 1 Chroma Subsampling Tests
//! 
//! Tests 4:2:0 and 4:2:2 chroma subsampling encoding/decoding.
//! Verifies file size reduction and acceptable quality degradation.

use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::FrameInfo;

fn calculate_mae(original: &[u8], decoded: &[u8]) -> f64 {
    if original.len() != decoded.len() {
        panic!("Buffer size mismatch: {} vs {}", original.len(), decoded.len());
    }
    let sum: i64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(&a, &b)| (a as i64 - b as i64).abs())
        .sum();
    sum as f64 / original.len() as f64
}

#[test]
fn test_420_subsampling_encode_decode() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height * 3];
    
    // Create RGB gradient pattern
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            source[idx] = (x * 4) as u8;           // R
            source[idx + 1] = (y * 4) as u8;       // G
            source[idx + 2] = ((x + y) * 2) as u8; // B
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };

    // Encode with 4:2:0 subsampling
    let mut encoder_420 = Jpeg1Encoder::new();
    encoder_420.set_quality(80);
    encoder_420.set_subsampling_420();
    
    let mut encoded_420 = vec![0u8; 100000];
    let enc_len_420 = encoder_420.encode(&source, &frame_info, &mut encoded_420)
        .expect("4:2:0 encode failed");

    // Encode with 4:4:4 (no subsampling) for comparison
    let mut encoder_444 = Jpeg1Encoder::new();
    encoder_444.set_quality(80);
    encoder_444.set_subsampling_444();
    
    let mut encoded_444 = vec![0u8; 100000];
    let enc_len_444 = encoder_444.encode(&source, &frame_info, &mut encoded_444)
        .expect("4:4:4 encode failed");

    // Verify 4:2:0 produces smaller file
    println!("4:2:0 size: {} bytes, 4:4:4 size: {} bytes", enc_len_420, enc_len_444);
    let size_ratio = enc_len_420 as f64 / enc_len_444 as f64;
    println!("Size ratio (4:2:0 / 4:4:4): {:.2}%", size_ratio * 100.0);
    
    // 4:2:0 should be 50-80% the size of 4:4:4
    assert!(size_ratio < 0.85, "4:2:0 file not significantly smaller: {:.2}%", size_ratio * 100.0);
    assert!(size_ratio > 0.45, "4:2:0 file unexpectedly small: {:.2}%", size_ratio * 100.0);

    // Decode 4:2:0
    let mut decoder_420 = Jpeg1Decoder::new(&encoded_420[..enc_len_420]);
    decoder_420.read_header().expect("Read header failed");

    let decoded_info = decoder_420.frame_info();
    assert_eq!(decoded_info.component_count, 3, "Component count mismatch");

    let mut decoded = vec![0u8; width * height * 3];
    decoder_420.decode(&mut decoded).expect("Decode failed");

    // Calculate MAE
    let mae = calculate_mae(&source, &decoded);
    println!("4:2:0 MAE: {:.2}", mae);
    
    // Chroma subsampling introduces some error, but should be acceptable
    assert!(mae < 20.0, "4:2:0 MAE too high: {:.2}", mae);
}

#[test]
fn test_422_subsampling_encode_decode() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height * 3];
    
    // Create RGB pattern
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            source[idx] = ((x * 3 + y) % 256) as u8;     // R
            source[idx + 1] = ((x + y * 3) % 256) as u8; // G
            source[idx + 2] = ((x * 2 + y * 2) % 256) as u8; // B
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };

    // Encode with 4:2:2 subsampling
    let mut encoder_422 = Jpeg1Encoder::new();
    encoder_422.set_quality(80);
    encoder_422.set_subsampling_422();
    
    let mut encoded_422 = vec![0u8; 100000];
    let enc_len_422 = encoder_422.encode(&source, &frame_info, &mut encoded_422)
        .expect("4:2:2 encode failed");

    // Encode with 4:4:4 for comparison
    let mut encoder_444 = Jpeg1Encoder::new();
    encoder_444.set_quality(80);
    encoder_444.set_subsampling_444();
    
    let mut encoded_444 = vec![0u8; 100000];
    let enc_len_444 = encoder_444.encode(&source, &frame_info, &mut encoded_444)
        .expect("4:4:4 encode failed");

    // Verify 4:2:2 produces smaller file
    println!("4:2:2 size: {} bytes, 4:4:4 size: {} bytes", enc_len_422, enc_len_444);
    let size_ratio = enc_len_422 as f64 / enc_len_444 as f64;
    println!("Size ratio (4:2:2 / 4:4:4): {:.2}%", size_ratio * 100.0);
    
    // 4:2:2 should be 65-95% the size of 4:4:4 (less savings than 4:2:0 since only horizontal subsampling)
    // Note: Actual savings depend on JPEG compression and image content
    assert!(size_ratio < 0.98, "4:2:2 file not smaller: {:.2}%", size_ratio * 100.0);
    assert!(size_ratio > 0.60, "4:2:2 file unexpectedly small: {:.2}%", size_ratio * 100.0);

    // Decode 4:2:2
    let mut decoder_422 = Jpeg1Decoder::new(&encoded_422[..enc_len_422]);
    decoder_422.read_header().expect("Read header failed");

    let mut decoded = vec![0u8; width * height * 3];
    decoder_422.decode(&mut decoded).expect("Decode failed");

    // Calculate MAE
    let mae = calculate_mae(&source, &decoded);
    println!("4:2:2 MAE: {:.2}", mae);
    
    // 4:2:2 should have less error than 4:2:0 since vertical chroma is preserved
    assert!(mae < 18.0, "4:2:2 MAE too high: {:.2}", mae);
}

#[test]
fn test_444_no_subsampling() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height * 3];
    
    // Simple pattern
    for i in 0..source.len() {
        source[i] = (i % 256) as u8;
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };

    // Encode with explicit 4:4:4 (no subsampling)
    let mut encoder = Jpeg1Encoder::new();
    encoder.set_quality(85);
    encoder.set_subsampling_444();
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
        .expect("4:4:4 encode failed");

    // Decode
    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u8; width * height * 3];
    decoder.decode(&mut decoded).expect("Decode failed");

    // Calculate MAE
    let mae = calculate_mae(&source, &decoded);
    println!("4:4:4 MAE: {:.2}", mae);
    
    // No subsampling should have low error at quality 85
    assert!(mae < 10.0, "4:4:4 MAE too high: {:.2}", mae);
}

#[test]
fn test_420_large_image() {
    let width = 256;
    let height = 256;
    let mut source = vec![0u8; width * height * 3];
    
    // Create a complex pattern
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            source[idx] = ((x ^ y) % 256) as u8;
            source[idx + 1] = ((x + y) % 256) as u8;
            source[idx + 2] = ((x * y / 16) % 256) as u8;
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };

    // Encode with 4:2:0
    let mut encoder = Jpeg1Encoder::new();
    encoder.set_quality(75);
    encoder.set_subsampling_420();
    
    let mut encoded = vec![0u8; 500000];
    let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
        .expect("Large 4:2:0 encode failed");

    println!("Large image (256x256) 4:2:0 size: {} bytes", enc_len);

    // Decode
    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let decoded_info = decoder.frame_info();
    assert_eq!(decoded_info.width, width as u32);
    assert_eq!(decoded_info.height, height as u32);

    let mut decoded = vec![0u8; width * height * 3];
    decoder.decode(&mut decoded).expect("Decode failed");

    let mae = calculate_mae(&source, &decoded);
    println!("Large image MAE: {:.2}", mae);
    assert!(mae < 25.0, "Large image MAE too high: {:.2}", mae);
}

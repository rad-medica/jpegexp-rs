//! JPEG 1 10-bit Precision Tests
//! 
//! Tests 10-bit extended sequential encoding/decoding.
//! Target: Low compression error (MAE < 10 for quality=90).

use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::FrameInfo;

fn calculate_mae_u16(original: &[u16], decoded: &[u16]) -> f64 {
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
fn test_10bit_grayscale_encode_decode() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u16; width * height];
    
    // Simple 10-bit pattern
    for i in 0..source.len() {
        source[i] = (i % 1024) as u16;
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 10,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_bits_per_sample(10);
    encoder.set_quality(90);
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode_u16(&source, &frame_info, &mut encoded)
        .expect("10-bit encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let decoded_info = decoder.frame_info();
    assert_eq!(decoded_info.bits_per_sample, 10, "Bit depth mismatch");

    let mut decoded = vec![0u16; width * height];
    decoder.decode_u16(&mut decoded).expect("Decode failed");

    let mae = calculate_mae_u16(&source, &decoded);
    assert!(mae < 10.0, "10-bit MAE too high: {}", mae);
}

#[test]
fn test_10bit_rgb_encode_decode() {
    let width = 32;
    let height = 32;
    let mut source = vec![0u16; width * height * 3];
    
    // 10-bit RGB gradient
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            source[idx] = ((x * 32) % 1024) as u16;       // R
            source[idx + 1] = ((y * 32) % 1024) as u16;   // G
            source[idx + 2] = (((x + y) * 16) % 1024) as u16; // B
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 10,
        component_count: 3,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_bits_per_sample(10);
    encoder.set_quality(85);
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode_u16(&source, &frame_info, &mut encoded)
        .expect("10-bit RGB encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u16; width * height * 3];
    decoder.decode_u16(&mut decoded).expect("Decode failed");

    let mae = calculate_mae_u16(&source, &decoded);
    assert!(mae < 15.0, "10-bit RGB MAE too high: {}", mae);
}

#[test]
fn test_10bit_high_quality() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u16; width * height];
    
    // Smooth gradient
    for y in 0..height {
        for x in 0..width {
            source[y * width + x] = ((y * 16) % 1024) as u16;
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 10,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_bits_per_sample(10);
    encoder.set_quality(95); // Very high quality
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode_u16(&source, &frame_info, &mut encoded)
        .expect("10-bit high quality encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u16; width * height];
    decoder.decode_u16(&mut decoded).expect("Decode failed");

    let mae = calculate_mae_u16(&source, &decoded);
    assert!(mae < 5.0, "10-bit high quality MAE too high: {}", mae);
}

#[test]
fn test_10bit_lossless() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u16; width * height];
    
    for i in 0..source.len() {
        source[i] = (i * 7 % 1024) as u16;
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 10,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_bits_per_sample(10);
    encoder.set_lossless(1); // Lossless mode
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode_u16(&source, &frame_info, &mut encoded)
        .expect("10-bit lossless encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u16; width * height];
    decoder.decode_u16(&mut decoded).expect("Decode failed");

    let mae = calculate_mae_u16(&source, &decoded);
    assert_eq!(mae, 0.0, "10-bit lossless MAE must be 0, got {}", mae);
}

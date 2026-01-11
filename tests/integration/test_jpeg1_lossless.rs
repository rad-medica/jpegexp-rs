//! JPEG 1 Lossless (SOF3) Encoding/Decoding Tests
//! 
//! Tests lossless mode (ISO/IEC 10918-1 Annex H) with all predictor functions.
//! Target: MAE=0 for all tests (perfect reconstruction).

use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::FrameInfo;

/// Calculate Mean Absolute Error between two buffers.
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
fn test_lossless_8bit_grayscale_predictor1() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height];
    
    // Generate gradient pattern
    for y in 0..height {
        for x in 0..width {
            source[y * width + x] = ((x + y) % 256) as u8;
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_lossless(1); // Predictor 1
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
        .expect("Lossless encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u8; width * height];
    decoder.decode(&mut decoded).expect("Decode failed");

    let mae = calculate_mae(&source, &decoded);
    assert_eq!(mae, 0.0, "Lossless predictor 1: MAE must be 0, got {}", mae);
}

#[test]
fn test_lossless_8bit_grayscale_predictor2() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            source[y * width + x] = ((x * y) % 256) as u8;
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_lossless(2); // Predictor 2
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
        .expect("Lossless encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u8; width * height];
    decoder.decode(&mut decoded).expect("Decode failed");

    let mae = calculate_mae(&source, &decoded);
    assert_eq!(mae, 0.0, "Lossless predictor 2: MAE must be 0, got {}", mae);
}

#[test]
fn test_lossless_8bit_grayscale_predictor4() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height];
    
    // Checkerboard pattern
    for y in 0..height {
        for x in 0..width {
            source[y * width + x] = if (x + y) % 2 == 0 { 200 } else { 50 };
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_lossless(4); // Predictor 4 (A + B - C)
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
        .expect("Lossless encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u8; width * height];
    decoder.decode(&mut decoded).expect("Decode failed");

    let mae = calculate_mae(&source, &decoded);
    assert_eq!(mae, 0.0, "Lossless predictor 4: MAE must be 0, got {}", mae);
}

#[test]
fn test_lossless_8bit_grayscale_predictor7() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height];
    
    // Random-like pattern
    for y in 0..height {
        for x in 0..width {
            source[y * width + x] = ((x * 17 + y * 31) % 256) as u8;
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_lossless(7); // Predictor 7 ((A + B) / 2)
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
        .expect("Lossless encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u8; width * height];
    decoder.decode(&mut decoded).expect("Decode failed");

    let mae = calculate_mae(&source, &decoded);
    assert_eq!(mae, 0.0, "Lossless predictor 7: MAE must be 0, got {}", mae);
}

#[test]
fn test_lossless_8bit_rgb() {
    let width = 32;
    let height = 32;
    let mut source = vec![0u8; width * height * 3];
    
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            source[idx] = ((x * 8) % 256) as u8;       // R
            source[idx + 1] = ((y * 8) % 256) as u8;   // G
            source[idx + 2] = ((x + y) % 256) as u8;   // B
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_lossless(1); // Predictor 1
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
        .expect("Lossless RGB encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u8; width * height * 3];
    decoder.decode(&mut decoded).expect("Decode failed");

    let mae = calculate_mae(&source, &decoded);
    // Lossless RGB encodes components directly without color conversion
    assert_eq!(mae, 0.0, "Lossless RGB: MAE must be 0, got {}", mae);
}

#[test]
fn test_lossless_12bit_grayscale() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u16; width * height];
    
    // 12-bit gradient (medical imaging use case)
    for y in 0..height {
        for x in 0..width {
            source[y * width + x] = ((x * 64 + y * 32) % 4096) as u16;
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_bits_per_sample(12);
    encoder.set_lossless(1);
    
    let mut encoded = vec![0u8; 100000];
    let enc_len = encoder.encode_u16(&source, &frame_info, &mut encoded)
        .expect("12-bit lossless encode failed");

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");

    let mut decoded = vec![0u16; width * height];
    decoder.decode_u16(&mut decoded).expect("Decode failed");

    let mae: f64 = source.iter()
        .zip(decoded.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).abs() as f64)
        .sum::<f64>() / source.len() as f64;
    
    assert_eq!(mae, 0.0, "12-bit lossless: MAE must be 0, got {}", mae);
}

#[test]
fn test_lossless_all_predictors() {
    // Test all predictors (1-7) on the same image
    let width = 32;
    let height = 32;
    let mut source = vec![0u8; width * height];
    
    for i in 0..source.len() {
        source[i] = (i * 7 % 256) as u8;
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    for predictor in 1..=7 {
        let mut encoder = Jpeg1Encoder::new();
        encoder.set_lossless(predictor);
        
        let mut encoded = vec![0u8; 100000];
        let enc_len = encoder.encode(&source, &frame_info, &mut encoded)
            .unwrap_or_else(|_| panic!("Predictor {} encode failed", predictor));

        let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
        decoder.read_header()
            .unwrap_or_else(|_| panic!("Predictor {} read header failed", predictor));

        let mut decoded = vec![0u8; width * height];
        decoder.decode(&mut decoded)
            .unwrap_or_else(|_| panic!("Predictor {} decode failed", predictor));

        let mae = calculate_mae(&source, &decoded);
        assert_eq!(mae, 0.0, "Predictor {}: MAE must be 0, got {}", predictor, mae);
    }
}

/// Debug test for 256×256 to identify the specific issue
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_160x160_solid() {
    let width = 160;
    let height = 160;
    let original_bytes = vec![128u8; width * height];

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);

    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );

    let mae = calculate_mae(&original_bytes, &reconstructed_bytes);
    println!("MAE: {:.6}", mae);
    assert_eq!(mae, 0.0, "Solid color should have MAE=0");
}

#[test]
fn test_256x256_solid() {
    let width = 256;
    let height = 256;
    let original_bytes = vec![128u8; width * height];

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);

    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );

    let mae = calculate_mae(&original_bytes, &reconstructed_bytes);
    println!("MAE: {:.6}", mae);
    assert_eq!(mae, 0.0, "Solid color should have MAE=0");
}

fn calculate_mae(original: &[u8], reconstructed: &[u8]) -> f64 {
    assert_eq!(original.len(), reconstructed.len());
    let mut sum_error = 0u64;
    for i in 0..original.len() {
        sum_error += (original[i] as i64 - reconstructed[i] as i64).abs() as u64;
    }
    sum_error as f64 / original.len() as f64
}

#[test]
fn test_256x256_vertical_gradient() {
    // Vertical gradient to ensure LH subband has strong signal
    let width = 256;
    let height = 256;

    let mut original_bytes = Vec::with_capacity(width * height);
    for y in 0..height {
        for _x in 0..width {
            let val = (y * 255 / height) as u8;
            original_bytes.push(val);
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);

    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );

    let mae = calculate_mae(&original_bytes, &reconstructed_bytes);
    println!("MAE: {:.6}", mae);

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0");
}

#[test]
fn test_256x256_horizontal_gradient() {
    // Horizontal gradient to ensure HL subband has strong signal
    let width = 256;
    let height = 256;

    let mut original_bytes = Vec::with_capacity(width * height);
    for _y in 0..height {
        for x in 0..width {
            let val = (x * 255 / width) as u8;
            original_bytes.push(val);
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoded = vec![0u8; width * height * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);

    let encoded_len = encoder
        .encode(&original_bytes, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding failed");
    let reconstructed_bytes = decoded_image.reconstruct_pixels().expect(
        "Reconstruction failed",
    );

    let mae = calculate_mae(&original_bytes, &reconstructed_bytes);
    println!("MAE: {:.6}", mae);

    assert_eq!(mae, 0.0, "Lossless encoding should have MAE=0");
}

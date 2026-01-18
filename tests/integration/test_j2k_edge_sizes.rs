/// Test JPEG 2000 with various edge case sizes to identify DWT or grid issues
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn calculate_mae(original: &[u8], reconstructed: &[u8]) -> f64 {
    assert_eq!(original.len(), reconstructed.len());
    let mut sum_error = 0u64;
    for i in 0..original.len() {
        sum_error += (original[i] as i64 - reconstructed[i] as i64).abs() as u64;
    }
    sum_error as f64 / original.len() as f64
}

fn test_size(width: usize, height: usize) {
    println!("\n=== Testing {}×{} ===", width, height);

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
    let mae = calculate_mae(&original_bytes, &reconstructed_bytes);
    println!("MAE: {:.6}", mae);

    assert_eq!(
        mae,
        0.0,
        "Lossless encoding should have MAE=0 for {}×{}",
        width,
        height
    );
}

#[test]
fn test_power_of_two_sizes() {
    // These should all work
    test_size(64, 64);
    test_size(128, 128);
    test_size(256, 256);
}

#[test]
fn test_non_power_of_two_sizes() {
    // Test sizes that are multiples of 64 but not powers of 2
    test_size(192, 192); // 64 * 3
    test_size(320, 320); // 64 * 5
}

#[test]
fn test_odd_multiples_of_64() {
    // Test 160 (which failed in the original bug report)
    test_size(160, 160); // 64 * 2.5
}

#[test]
fn test_non_multiples_of_64() {
    // Test sizes that are NOT multiples of 64
    test_size(129, 129);
    test_size(130, 130);
    test_size(200, 200);
}

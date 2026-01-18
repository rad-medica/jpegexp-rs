use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_j2k_lossy_quality() {
    let width = 64u32;
    let height = 64u32;
    let components = 1;
    let depth = 8;

    // Create random noise (hard to compress)
    // Use a fixed seed for reproducibility (pseudo-random)
    let mut original = Vec::with_capacity((width * height) as usize);
    let mut seed: u32 = 12345;
    for _ in 0..(width * height) {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let val = (seed >> 24) as u8;
        original.push(val);
    }

    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: depth,
        component_count: components as i32,
    };

    let mut encoded = vec![0u8; 128 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(true); // Enable lossy 9-7 DWT
    encoder.set_quality(90); // High quality
    encoder.set_decomposition_levels(3);

    let encoded_len = encoder
        .encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    println!("Encoded size: {} bytes", encoded_len);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");

    // Calculate PSNR
    let mse: f64 = original
        .iter()
        .zip(reconstructed.iter())
        .map(|(a, b)| {
            let diff = *a as f64 - *b as f64;
            diff * diff
        })
        .sum::<f64>() / (original.len() as f64);

    let psnr = if mse == 0.0 {
        100.0
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    };

    println!("MSE: {:.4}", mse);
    println!("PSNR: {:.2} dB", psnr);

    // Expect > 40 dB for Q90
    assert!(
        psnr > 40.0,
        "PSNR too low: {:.2} dB (expected > 40 dB for Q90)",
        psnr
    );
}

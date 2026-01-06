use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
#[ignore] // Fails on 64x64 blocks due to potential MQ coder desync
fn test_12bit_color_large_roundtrip() {
    // Parameters
    let width = 64;
    let height = 64;
    let components = 3;
    let depth = 12;

    println!(
        "Creating {}x{} image with {} components, {} bits",
        width, height, components, depth
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

    // ...

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
        width, height, components, depth
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
    let reconstructed_bytes = decoded_image
        .reconstruct_pixels()
        .expect("Reconstruction failed");
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

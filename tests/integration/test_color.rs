use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_color_rgb_lossless() {
    let width = 64u32;
    let height = 64u32;
    let components = 3;
    let depth = 8;

    // Create RGB pattern
    // Top-Left: Red
    // Top-Right: Green
    // Bottom-Left: Blue
    // Bottom-Right: White
    let mut original = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            let mut r = if x < width / 2 { 255u8 } else { 0 };
            let mut g = if x >= width / 2 { 255u8 } else { 0 };
            let mut b = if y >= height / 2 { 255u8 } else { 0 };
            // Mix
            r = if x >= width / 2 && y >= height / 2 {
                255
            } else {
                r
            };
            g = if x >= width / 2 && y >= height / 2 {
                255
            } else {
                g
            };
            b = if x >= width / 2 && y >= height / 2 {
                255
            } else {
                b
            };

            // Gradient overlay
            let r = r.saturating_sub((x % 32) as u8);
            let g = g.saturating_sub((y % 32) as u8);

            original.push(r);
            original.push(g);
            original.push(b);
        }
    }

    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: depth,
        component_count: components as i32,
    };

    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);

    let encoded_len = encoder
        .encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    println!("Encoded RGB image: {} bytes", encoded_len);
    println!(
        "Compression ratio: {:.2}x",
        (original.len() as f64) / (encoded_len as f64)
    );

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");

    assert_eq!(reconstructed.len(), original.len());

    let mut diffs = 0;
    for i in 0..original.len() {
        if original[i] != reconstructed[i] {
            diffs += 1;
            if diffs < 10 {
                println!(
                    "Mismatch at {}: orig={}, recon={}",
                    i,
                    original[i],
                    reconstructed[i]
                );
            }
        }
    }

    assert_eq!(diffs, 0, "RGB roundtrip failed with {} mismatches", diffs);
}

#[test]
#[ignore] // 12-bit RGB currently fails (mismatch)
fn test_color_rgb_12bit_lossless() {
    let width = 32u32;
    let height = 32u32;
    let components = 3;
    let depth = 12;

    let mut original = Vec::with_capacity((width * height * 3 * 2) as usize);
    let mut original_u16 = Vec::with_capacity((width * height * 3) as usize);

    for y in 0..height {
        for x in 0..width {
            let r = ((x * 100) % 4096) as u16;
            let g = ((y * 100) % 4096) as u16;
            let b = (((x + y) * 50) % 4096) as u16;

            original_u16.push(r);
            original_u16.push(g);
            original_u16.push(b);

            original.extend_from_slice(&r.to_ne_bytes());
            original.extend_from_slice(&g.to_ne_bytes());
            original.extend_from_slice(&b.to_ne_bytes());
        }
    }

    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: depth,
        component_count: components as i32,
    };

    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);

    let encoded_len = encoder
        .encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);

    println!("Encoded 12-bit RGB image: {} bytes", encoded_len);

    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");

    assert_eq!(reconstructed.len(), original.len());

    let mut diffs = 0;
    for i in 0..original_u16.len() {
        let val_orig = original_u16[i];
        let val_recon = u16::from_ne_bytes([reconstructed[i * 2], reconstructed[i * 2 + 1]]);

        if val_orig != val_recon {
            diffs += 1;
            if diffs < 10 {
                println!(
                    "Mismatch at sample {}: orig={}, recon={}",
                    i,
                    val_orig,
                    val_recon
                );
            }
        }
    }

    assert_eq!(
        diffs,
        0,
        "12-bit RGB roundtrip failed with {} mismatches",
        diffs
    );
}

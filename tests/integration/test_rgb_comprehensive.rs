use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

/// Test RGB encoding at various sizes
#[test]
fn test_rgb_various_sizes() {
    let sizes = vec![64, 128, 256, 512];

    for size in sizes {
        println!("\nTesting {}x{} RGB image...", size, size);

        // Create RGB gradient pattern
        let mut original = Vec::with_capacity((size * size * 3) as usize);
        for y in 0..size {
            for x in 0..size {
                let r = ((x * 255) / size) as u8;
                let g = ((y * 255) / size) as u8;
                let b = (((x + y) * 128) / size) as u8;

                original.push(r);
                original.push(g);
                original.push(b);
            }
        }

        let frame_info = FrameInfo {
            width: size as u32,
            height: size as u32,
            bits_per_sample: 8,
            component_count: 3,
        };

        let mut encoded = vec![0u8; size * size * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(5);

        let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
            .expect(&format!("Encoding failed for {}x{}", size, size));
        encoded.truncate(encoded_len);

        println!("  Encoded: {} bytes (ratio: {:.2}x)",
                 encoded_len,
                 (original.len() as f64) / (encoded_len as f64));

        // Decode
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode()
            .expect(&format!("Decoding failed for {}x{}", size, size));
        let reconstructed = image.reconstruct_pixels()
            .expect(&format!("Reconstruction failed for {}x{}", size, size));

        // Calculate MAE per component
        assert_eq!(reconstructed.len(), original.len(),
                   "Size mismatch for {}x{}", size, size);

        let mut total_error = [0.0; 3];
        let mut max_error = [0; 3];
        let pixel_count = (size * size) as f64;

        for (i, (&orig, &recon)) in original.chunks_exact(3).zip(reconstructed.chunks_exact(3)).enumerate() {
            for c in 0..3 {
                let error = orig[c].abs_diff(recon[c]);
                total_error[c] += error as f64;
                max_error[c] = max_error[c].max(error as i32);
            }
        }

        let mae_r = total_error[0] / pixel_count;
        let mae_g = total_error[1] / pixel_count;
        let mae_b = total_error[2] / pixel_count;
        let mae_avg = (mae_r + mae_g + mae_b) / 3.0;

        println!("  MAE: R={:.6}, G={:.6}, B={:.6}, Avg={:.6}",
                 mae_r, mae_g, mae_b, mae_avg);
        println!("  Max: R={}, G={}, B={}",
                 max_error[0], max_error[1], max_error[2]);

        if mae_avg > 0.01 {
            println!("  ❌ FAIL - MAE too high");

            // Show first few mismatches
            let mut shown = 0;
            for (i, (&orig, &recon)) in original.chunks_exact(3).zip(reconstructed.chunks_exact(3)).enumerate() {
                for c in 0..3 {
                    if orig[c] != recon[c] && shown < 10 {
                        let comp = ["R", "G", "B"][c];
                        println!("    Pixel {} {}: orig={}, recon={}",
                                 i, comp, orig[c], recon[c]);
                        shown += 1;
                    }
                }
            }

            panic!("RGB encoding failed for {}x{} with MAE={:.6}",
                   size, size, mae_avg);
        } else {
            println!("  ✅ PASS");
        }
    }
}

/// Test RGB with different DWT levels
#[test]
fn test_rgb_dwt_levels() {
    let size = 128;

    for dwt_level in 0..=5 {
        println!("\nTesting {}x{} RGB with DWT level {}...", size, size, dwt_level);

        // Create RGB checkerboard
        let mut original = Vec::with_capacity((size * size * 3) as usize);
        for y in 0..size {
            for x in 0..size {
                let is_white = (x / 16 + y / 16) % 2 == 0;
                let val = if is_white { 255u8 } else { 0 };

                original.push(val);          // R
                original.push(if is_white { 0 } else { 255 }); // G (inverted)
                original.push(val);          // B
            }
        }

        let frame_info = FrameInfo {
            width: size as u32,
            height: size as u32,
            bits_per_sample: 8,
            component_count: 3,
        };

        let mut encoded = vec![0u8; size * size * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);

        let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
            .expect(&format!("Encoding failed for DWT level {}", dwt_level));
        encoded.truncate(encoded_len);

        // Decode
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode()
            .expect(&format!("Decoding failed for DWT level {}", dwt_level));
        let reconstructed = image.reconstruct_pixels()
            .expect(&format!("Reconstruction failed for DWT level {}", dwt_level));

        // Calculate MAE
        let mae = original.iter().zip(reconstructed.iter())
            .map(|(&o, &r)| o.abs_diff(r) as f64)
            .sum::<f64>() / original.len() as f64;

        println!("  MAE: {:.6}", mae);

        if mae > 0.01 {
            println!("  ❌ FAIL");
            panic!("RGB DWT level {} failed with MAE={:.6}", dwt_level, mae);
        } else {
            println!("  ✅ PASS");
        }
    }
}

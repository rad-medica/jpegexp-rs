//! JPEG 2000 Lossless Roundtrip Test
//!
//! This test verifies that the encoder and decoder produce a lossless roundtrip.

use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_j2k_lossless_roundtrip_grayscale() {
    // Create a simple 8x8 grayscale test pattern
    let width = 8usize;
    let height = 8usize;
    let components = 1usize;

    let mut original_pixels = Vec::with_capacity(width * height * components);
    for y in 0..height {
        for x in 0..width {
            // Create gradient pattern
            let val = ((x + y) * 16).min(255) as u8;
            original_pixels.push(val);
        }
    }

    println!("Original pixels: {:?}", original_pixels);

    // Encode
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut encoded = vec![0u8; 4096];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Use lossless 5-3 transform
    encoder.set_decomposition_levels(3);

    let encoded_len = encoder
        .encode(&original_pixels, &frame_info, &mut encoded)
        .expect("Encoding should succeed");
    encoded.truncate(encoded_len);

    println!("Encoded {} bytes", encoded_len);
    println!("First 16 bytes: {:02X?}", &encoded[..16.min(encoded_len)]);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding should succeed");

    // DEBUG: Print decoded tile structure
    for (t_idx, tile) in decoded_image.tiles.iter().enumerate() {
        println!("Tile {}:", t_idx);
        for (c_idx, comp) in tile.components.iter().enumerate() {
            println!("  Component {}:", c_idx);
            for (r_idx, res) in comp.resolutions.iter().enumerate() {
                println!("    Resolution {} ({}x{}):", r_idx, res.width, res.height);
                for (s_idx, sb) in res.subbands.iter().enumerate() {
                    let cb_count = sb.codeblocks.len();
                    if cb_count > 0 || sb.width > 0 {
                        println!(
                            "      Subband {} {:?} ({}x{}): {} codeblocks",
                            s_idx, sb.orientation, sb.width, sb.height, cb_count
                        );
                        for cb in &sb.codeblocks {
                            let coeff_sum: i64 = cb.coefficients.iter().map(|&x| x as i64).sum();
                            println!(
                                "        CB[{},{}] {}x{}: {} coeffs, sum={}, passes={}",
                                cb.x,
                                cb.y,
                                cb.width,
                                cb.height,
                                cb.coefficients.len(),
                                coeff_sum,
                                cb.coding_passes
                            );
                        }
                    }
                }
            }
        }
    }

    let reconstructed = decoded_image
        .reconstruct_pixels()
        .expect("Reconstruction should succeed");

    println!("Reconstructed pixels: {:?}", reconstructed);
    println!("Decoded {} pixels", reconstructed.len());

    // Calculate MAE
    let mut total_diff = 0u64;
    let mut max_diff = 0u32;
    let mut pixel_count = 0;

    for (i, (&orig, &recon)) in original_pixels.iter().zip(reconstructed.iter()).enumerate() {
        let orig_val = orig as i32;
        let recon_val = recon as i32;
        let diff = (orig_val - recon_val).abs() as u32;
        if diff > 0 && i < 20 {
            println!(
                "  Pixel {}: orig={}, recon={}, diff={}",
                i, orig_val, recon_val, diff
            );
        }
        total_diff += diff as u64;
        max_diff = max_diff.max(diff);
        pixel_count += 1;
    }

    let mae = total_diff as f64 / pixel_count as f64;

    println!("MAE: {:.4}", mae);
    println!("Max diff: {}", max_diff);

    // For lossless, MAE should be 0
    assert!(
        mae < 1.0,
        "Lossless roundtrip should have MAE < 1, got {}",
        mae
    );
}

#[test]
fn test_j2k_lossless_roundtrip_constant() {
    // Create a constant image - simplest case
    let width = 8usize;
    let height = 8usize;
    let components = 1usize;

    // All pixels are 128
    let original_pixels = vec![128u8; width * height * components];

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut encoded = vec![0u8; 4096];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);

    let encoded_len = encoder
        .encode(&original_pixels, &frame_info, &mut encoded)
        .expect("Encoding should succeed");
    encoded.truncate(encoded_len);

    println!("Constant image encoded to {} bytes", encoded_len);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().expect("Decoding should succeed");
    let reconstructed = decoded_image
        .reconstruct_pixels()
        .expect("Reconstruction should succeed");

    // Calculate MAE
    let mut total_diff = 0u64;
    let mut max_diff = 0u32;

    for (&orig, &recon) in original_pixels.iter().zip(reconstructed.iter()) {
        let diff = (orig as i32 - recon as i32).abs() as u32;
        total_diff += diff as u64;
        max_diff = max_diff.max(diff);
    }

    let mae = total_diff as f64 / original_pixels.len() as f64;

    println!("Constant image MAE: {:.4}", mae);
    println!("Max diff: {}", max_diff);

    assert!(
        mae < 1.0,
        "Constant image should decode correctly, MAE={}",
        mae
    );
}

#[test]
fn test_j2k_mq_roundtrip_simple() {
    // Test MQ coder directly for encoding/decoding of known sequence
    use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

    let mut encoder = MqCoder::new();
    encoder.init_contexts(19);
    encoder.init_encoder(); // ✅ FIXED: Initialize encoder before use

    // Encode a simple sequence: 0, 1, 0, 0, 1, 1
    let symbols = [0u8, 1, 0, 0, 1, 1];
    let context = 0;

    for &sym in &symbols {
        encoder.encode(sym, context);
    }
    encoder.flush();

    let encoded = encoder.get_buffer().to_vec();
    println!(
        "MQ encoded {} symbols to {} bytes",
        symbols.len(),
        encoded.len()
    );

    // Decode
    let mut decoder = MqCoder::new();
    decoder.init_contexts(19);
    decoder.init_decoder(&encoded);

    let mut decoded = Vec::new();
    for _ in 0..symbols.len() {
        let sym = decoder.decode_bit(context);
        decoded.push(sym);
    }

    println!("Original: {:?}", symbols);
    println!("Decoded:  {:?}", decoded);

    // Verify
    for (i, (&orig, &dec)) in symbols.iter().zip(decoded.iter()).enumerate() {
        assert_eq!(orig, dec, "Mismatch at index {}: {} vs {}", i, orig, dec);
    }
}

#[test]
fn test_j2k_bit_plane_coder_simple_single_value() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test with a single non-zero value to understand the encoding
    let data = vec![112i32, 0, 0, 0]; // Just one significant value
    let width = 2u32;
    let height = 2u32;

    println!("Testing single value: 112 = {:08b}", 112);

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {}", max_bp);

    let passes = encoder.encode_codeblock(max_bp, 0);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, 0);

    match result {
        Ok(coefficients) => {
            println!("Original: {:?}", data);
            println!("Decoded:  {:?}", coefficients);
            assert_eq!(data[0], coefficients[0], "First coefficient should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_bit_plane_coder_two_values() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test with two non-zero values
    let data = vec![112i32, 64, 0, 0];
    let width = 2u32;
    let height = 2u32;

    println!("Testing two values: 112={:08b}, 64={:08b}", 112, 64);

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {}", max_bp);

    let passes = encoder.encode_codeblock(max_bp, 0);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, 0);

    match result {
        Ok(coefficients) => {
            println!("Original: {:?}", data);
            println!("Decoded:  {:?}", coefficients);

            for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
                if orig != dec {
                    println!(
                        "  Mismatch at {}: orig={:08b}, dec={:08b}",
                        i,
                        orig.abs(),
                        dec.abs()
                    );
                }
            }

            assert_eq!(data[0], coefficients[0], "First coefficient should match");
            assert_eq!(data[1], coefficients[1], "Second coefficient should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_bit_plane_coder_four_values_with_negatives() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test with four non-zero values including negatives
    let data = vec![112i32, 64, -48, -80];
    let width = 2u32;
    let height = 2u32;

    println!("Testing four values: {:?}", data);
    for (i, &v) in data.iter().enumerate() {
        println!(
            "  [{}] {} = {:08b} (sign={})",
            i,
            v.abs(),
            v.abs(),
            if v < 0 { "neg" } else { "pos" }
        );
    }

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {}", max_bp);

    let passes = encoder.encode_codeblock(max_bp, 0);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, 0);

    match result {
        Ok(coefficients) => {
            println!("Original: {:?}", data);
            println!("Decoded:  {:?}", coefficients);

            let mut mismatches = 0;
            for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
                if orig != dec {
                    println!("  Mismatch at {}: orig={}, dec={}", i, orig, dec);
                    mismatches += 1;
                }
            }

            assert_eq!(mismatches, 0, "All coefficients should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_bit_plane_coder_roundtrip() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test with values that all have bit 6 set
    let data = vec![64i32, 65, 112, 80];
    let width = 2u32;
    let height = 2u32;

    println!("Original coefficients (2x2): {:?}", data);
    for (i, &v) in data.iter().enumerate() {
        println!("  [{}] {} = {:08b}, bit6={}", i, v, v, (v >> 6) & 1);
    }

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {}", max_bp);

    let passes = encoder.encode_codeblock(max_bp, 0);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, 0);

    match result {
        Ok(coefficients) => {
            println!("Decoded coefficients: {:?}", coefficients);

            let mut matches = 0;
            let mut mismatches = 0;
            for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
                if orig == dec {
                    matches += 1;
                } else {
                    mismatches += 1;
                    println!("  Mismatch at {}: orig={}, dec={}", i, orig, dec);
                }
            }
            println!("Matches: {}, Mismatches: {}", matches, mismatches);

            assert_eq!(mismatches, 0, "All coefficients should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_bit_plane_coder_3x4_block() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test a 3x4 block
    // Col 0,1: zeros, Col 2: 16s
    let data: Vec<i32> = vec![0, 0, 16, 0, 0, 16, 0, 0, 16, 0, 0, 16];
    let width = 3u32;
    let height = 4u32;

    println!("Testing 3x4 block:");
    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);

    let orientation = 1u8; // HL
    let passes = encoder.encode_codeblock(max_bp, orientation);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!(
        "Encoded {} passes to {} bytes: {:02X?}",
        passes,
        encoded.len(),
        encoded
    );

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, orientation);
    let coefficients = result.expect("Decode failed");

    let mut mismatches = 0;
    for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
        if orig != dec {
            println!("  Mismatch at {}: orig={}, dec={}", i, orig, dec);
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0);
}

#[test]
fn test_j2k_bit_plane_coder_2x4_block() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test a 2x4 block (2 columns, 4 rows = 1 stripe)
    // Column 0: all zeros, Column 1: all 16s
    let data: Vec<i32> = vec![0, 16, 0, 16, 0, 16, 0, 16];
    let width = 2u32;
    let height = 4u32;

    println!("Testing 2x4 block:");
    for y in 0..4 {
        println!("  Row {}: {:?}", y, &data[y * 2..(y + 1) * 2]);
    }

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {}", max_bp);

    let orientation = 1u8; // HL orientation
    let passes = encoder.encode_codeblock(max_bp, orientation);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());
    println!("Encoded bytes: {:02X?}", encoded);

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, orientation);

    match result {
        Ok(coefficients) => {
            println!("Original: {:?}", data);
            println!("Decoded:  {:?}", coefficients);

            let mut mismatches = 0;
            for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
                if orig != dec {
                    println!("  Mismatch at {}: orig={}, dec={}", i, orig, dec);
                    mismatches += 1;
                }
            }

            assert_eq!(mismatches, 0, "All coefficients should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_bit_plane_coder_4x1_column() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test a 4x1 column (one stripe)
    let data: Vec<i32> = vec![16, 16, 16, 16];
    let width = 1u32;
    let height = 4u32;

    println!("Testing 4x1 column with [16, 16, 16, 16]:");

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {}", max_bp);

    let orientation = 1u8; // HL orientation
    let passes = encoder.encode_codeblock(max_bp, orientation);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());
    println!("Encoded bytes: {:02X?}", encoded);

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, orientation);

    match result {
        Ok(coefficients) => {
            println!("Original: {:?}", data);
            println!("Decoded:  {:?}", coefficients);

            for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
                if orig != dec {
                    println!("  Mismatch at {}: orig={}, dec={}", i, orig, dec);
                }
            }

            assert_eq!(data, coefficients, "Coefficients should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_bit_plane_coder_2x2_simple() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Simpler test: 2x2 with just one non-zero
    let data: Vec<i32> = vec![0, 16, 0, 0];
    let width = 2u32;
    let height = 2u32;

    println!("Testing 2x2 with [0, 16, 0, 0]:");

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {}", max_bp);

    let orientation = 1u8; // HL orientation
    let passes = encoder.encode_codeblock(max_bp, orientation);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());
    println!("Encoded bytes: {:02X?}", encoded);

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, orientation);

    match result {
        Ok(coefficients) => {
            println!("Original: {:?}", data);
            println!("Decoded:  {:?}", coefficients);

            for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
                if orig != dec {
                    println!("  Mismatch at {}: orig={}, dec={}", i, orig, dec);
                }
            }

            assert_eq!(data, coefficients, "Coefficients should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_bit_plane_coder_4x4_hl_pattern() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test with the specific Res 3 HL pattern: [0,0,0,16] repeated for 4 rows
    let data: Vec<i32> = vec![0, 0, 0, 16, 0, 0, 0, 16, 0, 0, 0, 16, 0, 0, 0, 16];
    let width = 4u32;
    let height = 4u32;

    println!("Testing 4x4 HL pattern (expected sum=64):");
    for y in 0..4 {
        println!("  Row {}: {:?}", y, &data[y * 4..(y + 1) * 4]);
    }

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max bit plane: {} (max_val=16, 16=0b10000, bit 4)", max_bp);

    let orientation = 1u8; // HL orientation
    let passes = encoder.encode_codeblock(max_bp, orientation);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} passes to {} bytes", passes, encoded.len());
    println!("Encoded bytes: {:02X?}", encoded);

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, orientation);

    match result {
        Ok(coefficients) => {
            println!("Original: {:?}", data);
            println!("Decoded:  {:?}", coefficients);

            let orig_sum: i32 = data.iter().sum();
            let dec_sum: i32 = coefficients.iter().sum();
            println!("Original sum: {}, Decoded sum: {}", orig_sum, dec_sum);

            let mut mismatches = 0;
            for (i, (&orig, &dec)) in data.iter().zip(coefficients.iter()).enumerate() {
                if orig != dec {
                    println!("  Mismatch at {}: orig={}, dec={}", i, orig, dec);
                    mismatches += 1;
                }
            }

            assert_eq!(mismatches, 0, "All coefficients should match");
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_dwt_roundtrip_2d() {
    use jpegexp_rs::jpeg2000::dwt::Dwt53;

    // Simple 8x8 gradient pattern
    let width = 8usize;
    let height = 8usize;

    let mut original: Vec<i32> = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            original.push(((x + y) * 16).min(255) as i32 - 128); // Level-shifted
        }
    }

    println!("Original coefficients (first 16): {:?}", &original[..16]);

    // Forward DWT (1 level)
    let mut coeffs = original.clone();

    // Apply 1D DWT to rows
    for y in 0..height {
        let row: Vec<i32> = coeffs[y * width..(y + 1) * width].to_vec();
        let l_len = (width + 1) / 2;
        let h_len = width / 2;
        let mut out_l = vec![0i32; l_len];
        let mut out_h = vec![0i32; h_len];

        Dwt53::forward(&row, &mut out_l, &mut out_h);

        for (i, &v) in out_l.iter().enumerate() {
            coeffs[y * width + i] = v;
        }
        for (i, &v) in out_h.iter().enumerate() {
            coeffs[y * width + l_len + i] = v;
        }
    }

    // Apply 1D DWT to columns
    for x in 0..width {
        let col: Vec<i32> = (0..height).map(|y| coeffs[y * width + x]).collect();
        let l_len = (height + 1) / 2;
        let h_len = height / 2;
        let mut out_l = vec![0i32; l_len];
        let mut out_h = vec![0i32; h_len];

        Dwt53::forward(&col, &mut out_l, &mut out_h);

        for (i, &v) in out_l.iter().enumerate() {
            coeffs[i * width + x] = v;
        }
        for (i, &v) in out_h.iter().enumerate() {
            coeffs[(l_len + i) * width + x] = v;
        }
    }

    println!("After forward DWT (first 16): {:?}", &coeffs[..16]);

    // Extract subbands
    let ll_w = (width + 1) / 2;
    let ll_h = (height + 1) / 2;
    let hl_w = width / 2;
    let lh_h = height / 2;

    // LL: top-left
    let mut ll = vec![0i32; ll_w * ll_h];
    for y in 0..ll_h {
        for x in 0..ll_w {
            ll[y * ll_w + x] = coeffs[y * width + x];
        }
    }

    // HL: top-right
    let mut hl = vec![0i32; hl_w * ll_h];
    for y in 0..ll_h {
        for x in 0..hl_w {
            hl[y * hl_w + x] = coeffs[y * width + ll_w + x];
        }
    }

    // LH: bottom-left
    let mut lh = vec![0i32; ll_w * lh_h];
    for y in 0..lh_h {
        for x in 0..ll_w {
            lh[y * ll_w + x] = coeffs[(ll_h + y) * width + x];
        }
    }

    // HH: bottom-right
    let mut hh = vec![0i32; hl_w * lh_h];
    for y in 0..lh_h {
        for x in 0..hl_w {
            hh[y * hl_w + x] = coeffs[(ll_h + y) * width + ll_w + x];
        }
    }

    println!("LL ({}x{}): {:?}", ll_w, ll_h, &ll);
    println!("HL ({}x{}): {:?}", hl_w, ll_h, &hl);
    println!("LH ({}x{}): {:?}", ll_w, lh_h, &lh);
    println!("HH ({}x{}): {:?}", hl_w, lh_h, &hh);

    // Inverse DWT
    let mut reconstructed = vec![0i32; width * height];
    Dwt53::inverse_2d(
        &ll,
        &hl,
        &lh,
        &hh,
        width as u32,
        height as u32,
        &mut reconstructed,
    );

    println!("Reconstructed (first 16): {:?}", &reconstructed[..16]);

    // Compare
    let mut max_diff = 0;
    let mut total_diff = 0i64;
    for (i, (&orig, &rec)) in original.iter().zip(reconstructed.iter()).enumerate() {
        let diff = (orig - rec).abs();
        if diff > 0 && i < 20 {
            println!("  Diff at {}: orig={}, rec={}, diff={}", i, orig, rec, diff);
        }
        max_diff = max_diff.max(diff);
        total_diff += diff as i64;
    }

    let mae = total_diff as f64 / (width * height) as f64;
    println!("DWT roundtrip MAE: {:.4}, max_diff: {}", mae, max_diff);

    assert_eq!(max_diff, 0, "DWT 5-3 should be perfectly reversible");
}

#[test]
fn test_j2k_coefficient_roundtrip() {
    // Test that coefficients survive the encode/decode cycle
    use jpegexp_rs::jpeg2000::dwt::Dwt53;

    // Create a simple 8x8 grayscale test pattern
    let width = 8usize;
    let height = 8usize;

    let mut original_pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 16).min(255) as u8;
            original_pixels.push(val);
        }
    }

    // Manually compute what the DWT coefficients should be
    let mut expected_coeffs: Vec<i32> = original_pixels
        .iter()
        .map(|&p| p as i32 - 128) // Level shift
        .collect();

    // Apply forward DWT manually (3 levels)
    let mut current_w = width;
    let mut current_h = height;

    for level in 0..3 {
        if current_w < 2 || current_h < 2 {
            break;
        }

        // Rows
        for y in 0..current_h {
            let row: Vec<i32> = (0..current_w)
                .map(|x| expected_coeffs[y * width + x])
                .collect();
            let l_len = (current_w + 1) / 2;
            let h_len = current_w / 2;
            let mut out_l = vec![0i32; l_len];
            let mut out_h = vec![0i32; h_len];

            Dwt53::forward(&row, &mut out_l, &mut out_h);

            for (i, &v) in out_l.iter().enumerate() {
                expected_coeffs[y * width + i] = v;
            }
            for (i, &v) in out_h.iter().enumerate() {
                expected_coeffs[y * width + l_len + i] = v;
            }
        }

        // Columns
        for x in 0..current_w {
            let col: Vec<i32> = (0..current_h)
                .map(|y| expected_coeffs[y * width + x])
                .collect();
            let l_len = (current_h + 1) / 2;
            let h_len = current_h / 2;
            let mut out_l = vec![0i32; l_len];
            let mut out_h = vec![0i32; h_len];

            Dwt53::forward(&col, &mut out_l, &mut out_h);

            for (i, &v) in out_l.iter().enumerate() {
                expected_coeffs[i * width + x] = v;
            }
            for (i, &v) in out_h.iter().enumerate() {
                expected_coeffs[(l_len + i) * width + x] = v;
            }
        }

        current_w = (current_w + 1) / 2;
        current_h = (current_h + 1) / 2;
        println!(
            "After level {}: {}x{} LL, first 4: {:?}",
            level + 1,
            current_w,
            current_h,
            &expected_coeffs[..4]
        );
    }

    println!("\nExpected coefficient layout (8x8 storage):");
    for y in 0..height {
        let row: Vec<i32> = (0..width).map(|x| expected_coeffs[y * width + x]).collect();
        println!("  Row {}: {:?}", y, row);
    }

    // Now encode and decode
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoded = vec![0u8; 4096];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(3);

    let encoded_len = encoder
        .encode(&original_pixels, &frame_info, &mut encoded)
        .unwrap();
    encoded.truncate(encoded_len);

    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().unwrap();

    // Check decoded coefficients
    println!("\nDecoded coefficient summary:");
    let tile = &decoded_image.tiles[0];
    let comp = &tile.components[0];

    for (r_idx, res) in comp.resolutions.iter().enumerate() {
        println!("  Resolution {} ({}x{}):", r_idx, res.width, res.height);
        for sb in &res.subbands {
            for cb in &sb.codeblocks {
                println!(
                    "    {:?}: CB[{},{}] {}x{}, coeffs: {:?}, zero_bp={}, passes={}",
                    sb.orientation,
                    cb.x,
                    cb.y,
                    cb.width,
                    cb.height,
                    &cb.coefficients,
                    cb.zero_bit_planes,
                    cb.coding_passes
                );
            }
        }
    }

    // Compare LL coefficient
    let ll_coeff = comp.resolutions[0].subbands[0].codeblocks[0].coefficients[0];
    assert_eq!(
        ll_coeff, expected_coeffs[0],
        "LL coefficient mismatch: expected {}, got {}",
        expected_coeffs[0], ll_coeff
    );
}

#[test]
fn test_j2k_single_coeff_73() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    // Test encoding/decoding the value 73 directly
    let data = vec![73i32];
    let width = 1u32;
    let height = 1u32;

    println!("Testing single value 73:");
    println!("  73 = {:08b}", 73);

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("  max_bit_plane = {}", max_bp);

    let passes = encoder.encode_codeblock(max_bp, 1); // orientation 1 = HL
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();
    println!("  Encoded {} passes to {} bytes", passes, encoded.len());
    println!("  Encoded bytes: {:02X?}", encoded);

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let result = decoder.decode_codeblock(&encoded, max_bp, passes, 1);

    match result {
        Ok(coefficients) => {
            println!("  Decoded: {:?}", coefficients);
            assert_eq!(
                coefficients[0], 73,
                "Should decode to 73, got {}",
                coefficients[0]
            );
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

#[test]
fn test_j2k_trace_encoded_bytes() {
    // This test encodes a simple image and traces the exact bytes for each codeblock
    use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;

    let width = 8usize;
    let height = 8usize;

    let mut original_pixels = Vec::with_capacity(width * height);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 16).min(255) as u8;
            original_pixels.push(val);
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoded = vec![0u8; 4096];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(3);

    let encoded_len = encoder
        .encode(&original_pixels, &frame_info, &mut encoded)
        .unwrap();
    encoded.truncate(encoded_len);

    println!("Full encoded stream ({} bytes):", encoded_len);

    // Find SOD marker (FF 93) to locate start of tile data
    let mut sod_pos = None;
    for i in 0..encoded.len() - 1 {
        if encoded[i] == 0xFF && encoded[i + 1] == 0x93 {
            sod_pos = Some(i + 2);
            break;
        }
    }

    if let Some(pos) = sod_pos {
        println!("SOD marker at {}, tile data starts at {}", pos - 2, pos);
        println!("Tile data bytes: {:02X?}", &encoded[pos..encoded_len - 2]); // Exclude EOC
    }

    // Decode and print what we get
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_image = decoder.decode().unwrap();

    let tile = &decoded_image.tiles[0];
    let comp = &tile.components[0];

    println!("\nDecoded coefficients by resolution:");
    for (r_idx, res) in comp.resolutions.iter().enumerate() {
        for sb in &res.subbands {
            if !sb.codeblocks.is_empty() {
                for cb in &sb.codeblocks {
                    println!(
                        "  Res {} {:?}: coeffs={:?}, zero_bp={}, passes={}",
                        r_idx,
                        sb.orientation,
                        cb.coefficients,
                        cb.zero_bit_planes,
                        cb.coding_passes
                    );
                }
            }
        }
    }
}

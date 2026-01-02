// JPEG-LS decoder validation tests against CharLS-encoded reference images
//
// These tests use images encoded by CharLS (the reference implementation) to validate
// our JPEG-LS decoder. If CharLS can decode an image, we should be able to decode it too.

#[cfg(test)]
mod jpegls_decoder_validation {
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_tiny_8x8_gray_gradient() {
        test_charls_decode("tiny_8x8_gray_gradient", 8, 8, 1, 8);
    }

    #[test]
    fn test_tiny_8x8_gray_noise() {
        test_charls_decode("tiny_8x8_gray_noise", 8, 8, 1, 8);
    }

    #[test]
    fn test_tiny_8x8_gray_checker() {
        test_charls_decode("tiny_8x8_gray_checker", 8, 8, 1, 8);
    }

    #[test]
    fn test_tiny_8x8_gray_solid() {
        test_charls_decode("tiny_8x8_gray_solid", 8, 8, 1, 8);
    }

    #[test]
    fn test_small_16x16_gray_gradient() {
        test_charls_decode("small_16x16_gray_gradient", 16, 16, 1, 8);
    }

    #[test]
    fn test_small_32x32_gray_gradient() {
        test_charls_decode("small_32x32_gray_gradient", 32, 32, 1, 8);
    }

    #[test]
    fn test_medium_64x64_gray_gradient() {
        test_charls_decode("medium_64x64_gray_gradient", 64, 64, 1, 8);
    }

    #[test]
    fn test_medium_128x128_gray_gradient() {
        test_charls_decode("medium_128x128_gray_gradient", 128, 128, 1, 8);
    }

    #[test]
    fn test_large_256x256_gray_gradient() {
        test_charls_decode("large_256x256_gray_gradient", 256, 256, 1, 8);
    }

    #[test]
    fn test_rect_16x32_gray_gradient() {
        test_charls_decode("rect_16x32_gray_gradient", 16, 32, 1, 8);
    }

    #[test]
    fn test_rect_32x16_gray_gradient() {
        test_charls_decode("rect_32x16_gray_gradient", 32, 16, 1, 8);
    }

    #[test]
    #[ignore] // RGB not yet supported
    fn test_tiny_8x8_rgb_gradient() {
        test_charls_decode("tiny_8x8_rgb_gradient", 8, 8, 3, 8);
    }

    #[test]
    #[ignore] // RGB not yet supported
    fn test_small_16x16_rgb_gradient() {
        test_charls_decode("small_16x16_rgb_gradient", 16, 16, 3, 8);
    }

    #[test]
    #[ignore] // RGB not yet supported
    fn test_small_32x32_rgb_gradient() {
        test_charls_decode("small_32x32_rgb_gradient", 32, 32, 3, 8);
    }

    #[test]
    #[ignore] // RGB not yet supported
    fn test_medium_64x64_rgb_gradient() {
        test_charls_decode("medium_64x64_rgb_gradient", 64, 64, 3, 8);
    }

    #[test]
    #[ignore] // RGB not yet supported
    fn test_small_16x16_rgb_noise() {
        test_charls_decode("small_16x16_rgb_noise", 16, 16, 3, 8);
    }

    #[test]
    #[ignore] // RGB not yet supported
    fn test_small_16x16_rgb_checker() {
        test_charls_decode("small_16x16_rgb_checker", 16, 16, 3, 8);
    }

    #[test]
    #[ignore] // 16-bit not yet tested
    fn test_small_16x16_gray16_gradient() {
        test_charls_decode("small_16x16_gray16_gradient", 16, 16, 1, 16);
    }

    #[test]
    #[ignore] // 16-bit not yet tested
    fn test_small_32x32_gray16_gradient() {
        test_charls_decode("small_32x32_gray16_gradient", 32, 32, 1, 16);
    }

    #[test]
    #[ignore] // Edge case - may fail
    fn test_edge_1x1_gray() {
        test_charls_decode("edge_1x1_gray", 1, 1, 1, 8);
    }

    #[test]
    #[ignore] // Edge case - may fail
    fn test_edge_1x8_gray() {
        test_charls_decode("edge_1x8_gray", 1, 8, 1, 8);
    }

    #[test]
    #[ignore] // Edge case - may fail
    fn test_edge_8x1_gray() {
        test_charls_decode("edge_8x1_gray", 8, 1, 1, 8);
    }

    /// Test decoding a CharLS-encoded image
    fn test_charls_decode(name: &str, width: u32, height: u32, components: u8, bit_depth: u8) {
        // Load the CharLS-encoded JPEG-LS file
        let jls_path = format!("tests/jpegls_test_images/{}.jls", name);
        let jls_data = fs::read(&jls_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", jls_path, e));

        // Load the expected raw data
        let raw_path = format!("tests/jpegls_test_images/{}.raw", name);
        let expected_data = fs::read(&raw_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", raw_path, e));

        // Decode with our decoder
        let mut decoder = jpegexp_rs::jpegls::JpeglsDecoder::new(&jls_data);
        
        // Read header
        decoder.read_header().unwrap_or_else(|e| {
            panic!("Failed to read header for {}: {}", name, e)
        });

        let frame_info = decoder.frame_info();
        
        // Validate frame info
        assert_eq!(frame_info.width, width, "Width mismatch for {}", name);
        assert_eq!(frame_info.height, height, "Height mismatch for {}", name);
        assert_eq!(frame_info.component_count, components as i32, "Component count mismatch for {}", name);
        assert_eq!(frame_info.bits_per_sample, bit_depth as i32, "Bit depth mismatch for {}", name);

        // Decode the image
        let bytes_per_sample = if bit_depth <= 8 { 1 } else { 2 };
        let buffer_size = (width * height * components as u32 * bytes_per_sample as u32) as usize;
        let mut decoded_data = vec![0u8; buffer_size];
        
        match decoder.decode(&mut decoded_data) {
            Ok(_) => {
                // Compare with expected data
                if decoded_data == expected_data {
                    println!("✓ {}: PASS - Perfect pixel match", name);
                } else {
                    // Calculate statistics
                    let max_diff = decoded_data.iter()
                        .zip(expected_data.iter())
                        .map(|(a, b)| (*a as i32 - *b as i32).abs())
                        .max()
                        .unwrap_or(0);
                    
                    let total_diff: i64 = decoded_data.iter()
                        .zip(expected_data.iter())
                        .map(|(a, b)| (*a as i32 - *b as i32).abs() as i64)
                        .sum();
                    
                    let mae = total_diff as f64 / decoded_data.len() as f64;
                    
                    // Find first mismatch
                    let first_mismatch = decoded_data.iter()
                        .zip(expected_data.iter())
                        .enumerate()
                        .find(|(_, (a, b))| a != b);
                    
                    if let Some((idx, (got, expected))) = first_mismatch {
                        eprintln!("✗ {}: FAIL - Pixel mismatch", name);
                        eprintln!("  MAE: {:.2}", mae);
                        eprintln!("  Max diff: {}", max_diff);
                        eprintln!("  First mismatch at byte {}: got {}, expected {}", idx, got, expected);
                        eprintln!("  Decoded {} bytes, expected {} bytes", decoded_data.len(), expected_data.len());
                    }
                    
                    panic!("Pixel data mismatch for {}", name);
                }
            }
            Err(e) => {
                eprintln!("✗ {}: FAIL - Decode error: {}", name, e);
                eprintln!("  Frame info: {}x{}, {} components, {} bits", width, height, components, bit_depth);
                eprintln!("  JLS file size: {} bytes", jls_data.len());
                panic!("Failed to decode {}: {}", name, e);
            }
        }
    }

    #[test]
    fn test_images_exist() {
        // Verify test images were generated
        let test_dir = Path::new("tests/jpegls_test_images");
        assert!(test_dir.exists(), "Test images directory not found. Run: python3 tests/generate_jpegls_test_images.py");
        
        let readme = test_dir.join("README.md");
        assert!(readme.exists(), "README.md not found in test images directory");
        
        // Check for at least one test image
        let tiny_gray = test_dir.join("tiny_8x8_gray_gradient.jls");
        assert!(tiny_gray.exists(), "Test images not generated. Run: python3 tests/generate_jpegls_test_images.py");
    }
}

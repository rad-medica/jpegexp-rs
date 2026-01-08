// Quick batch test for various sizes without requiring OpenJPEG
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn calculate_mae(original: &[u8], decoded: &[u8]) -> f64 {
    if original.len() != decoded.len() {
        return f64::MAX;
    }
    original.iter()
        .zip(decoded.iter())
        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
        .sum::<f64>() / original.len() as f64
}

fn test_encode_decode(name: &str, pixels: &[u8], width: u32, height: u32, dwt_levels: u8) -> f64 {
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(dwt_levels);
    
    let info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; (width * height * 10) as usize];
    let len = encoder.encode(pixels, &info, &mut output)
        .expect(&format!("Failed to encode {}", name));
    output.truncate(len);
    
    let bpp = (len * 8) as f64 / (width * height) as f64;
    
    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect(&format!("Failed to decode {}", name));
    let decoded = image.reconstruct_pixels().expect("Failed to reconstruct");
    
    let mae = calculate_mae(pixels, &decoded);
    
    println!("{:<30} {}x{} L{}: MAE={:.6}, size={} bytes ({:.2} bpp) {}",
             name, width, height, dwt_levels, mae, len, bpp,
             if mae < 0.01 { "✅" } else { "❌" });
    
    mae
}

fn generate_gradient(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            pixels[idx] = ((x * 255) / width.max(1)) as u8;
        }
    }
    pixels
}

fn generate_checkerboard(width: u32, height: u32, square_size: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            pixels[idx] = if ((x / square_size) + (y / square_size)) % 2 == 0 { 0 } else { 255 };
        }
    }
    pixels
}

#[test]
fn test_various_sizes_self_roundtrip() {
    println!("\n{}", "=".repeat(100));
    println!("JPEG2000 Self-Roundtrip Test Suite (Various Sizes & DWT Levels)");
    println!("{}", "=".repeat(100));
    
    let mut all_passed = true;
    
    // Test configurations: (width, height, dwt_levels, pattern_name)
    let tests = vec![
        // Small images with no DWT
        (64, 64, 0, "Gradient"),
        (64, 64, 0, "Checkerboard"),
        (64, 64, 0, "Solid Black"),
        
        // Small images with DWT
        (64, 64, 2, "Gradient"),
        (64, 64, 2, "Checkerboard"),
        
        // Medium images
        (128, 128, 0, "Gradient"),
        (128, 128, 3, "Gradient"),
        (128, 128, 0, "Checkerboard"),
        (128, 128, 3, "Checkerboard"),
        
        // Larger images
        (256, 256, 0, "Gradient"),
        (256, 256, 4, "Gradient"),
        (256, 256, 0, "Checkerboard"),
        (256, 256, 4, "Checkerboard"),
        
        // Large images
        (512, 512, 0, "Gradient"),
        (512, 512, 5, "Gradient"),
        (512, 512, 0, "Checkerboard"),
        (512, 512, 5, "Checkerboard"),
        
        // Very large (if time permits)
        (1024, 1024, 0, "Gradient"),
        (1024, 1024, 5, "Gradient"),
    ];
    
    for (width, height, dwt_levels, pattern) in tests {
        let pixels = match pattern {
            "Gradient" => generate_gradient(width, height),
            "Checkerboard" => generate_checkerboard(width, height, 8),
            "Solid Black" => vec![0u8; (width * height) as usize],
            _ => continue,
        };
        
        let mae = test_encode_decode(pattern, &pixels, width, height, dwt_levels);
        if mae >= 0.01 {
            all_passed = false;
        }
    }
    
    println!("{}", "=".repeat(100));
    if all_passed {
        println!("✅ ALL TESTS PASSED - Perfect self-roundtrip for all sizes!");
    } else {
        println!("❌ SOME TESTS FAILED");
    }
    println!("{}", "=".repeat(100));
    
    assert!(all_passed, "Some self-roundtrip tests failed");
}

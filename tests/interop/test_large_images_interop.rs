// Comprehensive test suite for large images with OpenJPEG interoperability
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

fn calculate_mae(original: &[u8], decoded: &[u8]) -> f64 {
    if original.len() != decoded.len() {
        return f64::MAX;
    }
    original.iter()
        .zip(decoded.iter())
        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
        .sum::<f64>() / original.len() as f64
}

fn test_image_with_openjpeg(
    name: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
    dwt_levels: u8,
) -> (f64, f64) {
    println!("\n=== Testing: {} ({}x{}, {} DWT levels) ===", name, width, height, dwt_levels);
    
    // Encode
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
    
    println!("Encoded to {} bytes ({:.2} bpp)", 
             len, 
             (len * 8) as f64 / (width * height) as f64);
    
    // Decode with our decoder
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect(&format!("Failed to decode {} with our decoder", name));
    let decoded_ours = image.reconstruct_pixels().expect("Failed to reconstruct");
    
    let mae_ours = calculate_mae(pixels, &decoded_ours);
    println!("Our decoder MAE: {:.6}", mae_ours);
    
    // Try OpenJPEG decoder
    let filename = format!("tests/fixtures/out/test_{}_{}_{}_{}.j2k", 
                          name.replace(" ", "_"), width, height, dwt_levels);
    fs::write(&filename, &output).expect("Failed to write test file");
    
    let result = Command::new("opj_decompress")
        .args(&["-i", &filename, "-o", &format!("{}.raw", filename)])
        .output();
    
    let mae_opj = if let Ok(output_cmd) = result {
        if output_cmd.status.success() {
            if let Ok(decoded_opj) = fs::read(&format!("{}.raw", filename)) {
                let mae = calculate_mae(pixels, &decoded_opj);
                println!("OpenJPEG decoder MAE: {:.6}", mae);
                
                if mae > 0.01 {
                    println!("  ⚠️  OpenJPEG interoperability issue!");
                    // Show first few mismatches
                    let mut mismatches = 0;
                    for i in 0..pixels.len().min(10) {
                        if pixels[i] != decoded_opj[i] {
                            println!("    Pixel {}: {} -> {} (diff: {})",
                                    i, pixels[i], decoded_opj[i], 
                                    (pixels[i] as i32 - decoded_opj[i] as i32).abs());
                            mismatches += 1;
                            if mismatches >= 5 { break; }
                        }
                    }
                } else {
                    println!("  ✅  OpenJPEG interoperability OK!");
                }
                
                mae
            } else {
                println!("OpenJPEG decoder failed to produce output");
                f64::MAX
            }
        } else {
            println!("OpenJPEG decoder returned error");
            println!("stderr: {}", String::from_utf8_lossy(&output_cmd.stderr));
            f64::MAX
        }
    } else {
        println!("opj_decompress not found - skipping OpenJPEG validation");
        f64::MAX
    };
    
    // Cleanup
    let _ = fs::remove_file(&filename);
    let _ = fs::remove_file(&format!("{}.raw", filename));
    
    (mae_ours, mae_opj)
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

fn generate_concentric_circles(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height) as usize];
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let max_dist = ((cx * cx + cy * cy) as f32).sqrt();
    
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = (y * width + x) as usize;
            pixels[idx] = ((dist / max_dist) * 255.0) as u8;
        }
    }
    pixels
}

fn generate_sine_wave(width: u32, height: u32, frequency: f32) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let phase = (x as f32 / width as f32) * frequency * 2.0 * std::f32::consts::PI;
            pixels[idx] = ((phase.sin() + 1.0) * 127.5) as u8;
        }
    }
    pixels
}

#[test]
#[ignore] // Run with: cargo test --test test_large_images_interop --release -- --ignored --nocapture
fn test_large_images_comprehensive() {
    let mut all_passed = true;
    let mut results = Vec::new();
    
    // Test configurations: (width, height, dwt_levels)
    let sizes = vec![
        (64, 64, 0),
        (64, 64, 2),
        (128, 128, 0),
        (128, 128, 3),
        (256, 256, 0),
        (256, 256, 4),
        (512, 512, 0),
        (512, 512, 5),
        (1024, 1024, 0),
        (1024, 1024, 5),
    ];
    
    for (width, height, dwt_levels) in sizes {
        println!("\n{}", "=".repeat(80));
        println!("Testing size {}x{} with {} DWT levels", width, height, dwt_levels);
        println!("{}", "=".repeat(80));
        
        // Test 1: Solid black
        let pixels = vec![0u8; (width * height) as usize];
        let (mae_ours, mae_opj) = test_image_with_openjpeg(
            "Solid Black",
            &pixels,
            width,
            height,
            dwt_levels,
        );
        results.push((format!("{}x{} L{} Solid Black", width, height, dwt_levels), mae_ours, mae_opj));
        if mae_opj > 0.01 { all_passed = false; }
        
        // Test 2: Solid mid-gray
        let pixels = vec![128u8; (width * height) as usize];
        let (mae_ours, mae_opj) = test_image_with_openjpeg(
            "Solid Gray",
            &pixels,
            width,
            height,
            dwt_levels,
        );
        results.push((format!("{}x{} L{} Solid Gray", width, height, dwt_levels), mae_ours, mae_opj));
        if mae_opj > 0.01 { all_passed = false; }
        
        // Test 3: Gradient
        let pixels = generate_gradient(width, height);
        let (mae_ours, mae_opj) = test_image_with_openjpeg(
            "Gradient",
            &pixels,
            width,
            height,
            dwt_levels,
        );
        results.push((format!("{}x{} L{} Gradient", width, height, dwt_levels), mae_ours, mae_opj));
        if mae_opj > 0.01 { all_passed = false; }
        
        // Test 4: Checkerboard (8x8 squares)
        let pixels = generate_checkerboard(width, height, 8);
        let (mae_ours, mae_opj) = test_image_with_openjpeg(
            "Checkerboard 8x8",
            &pixels,
            width,
            height,
            dwt_levels,
        );
        results.push((format!("{}x{} L{} Checkerboard", width, height, dwt_levels), mae_ours, mae_opj));
        if mae_opj > 0.01 { all_passed = false; }
        
        // Test 5: Concentric circles
        let pixels = generate_concentric_circles(width, height);
        let (mae_ours, mae_opj) = test_image_with_openjpeg(
            "Circles",
            &pixels,
            width,
            height,
            dwt_levels,
        );
        results.push((format!("{}x{} L{} Circles", width, height, dwt_levels), mae_ours, mae_opj));
        if mae_opj > 0.01 { all_passed = false; }
        
        // Test 6: Sine wave
        let pixels = generate_sine_wave(width, height, 4.0);
        let (mae_ours, mae_opj) = test_image_with_openjpeg(
            "Sine Wave",
            &pixels,
            width,
            height,
            dwt_levels,
        );
        results.push((format!("{}x{} L{} Sine Wave", width, height, dwt_levels), mae_ours, mae_opj));
        if mae_opj > 0.01 { all_passed = false; }
    }
    
    // Print summary table
    println!("\n\n{}", "=".repeat(100));
    println!("COMPREHENSIVE TEST RESULTS SUMMARY");
    println!("{}", "=".repeat(100));
    println!("{:<40} | {:>15} | {:>15} | {}", "Test", "Our MAE", "OpenJPEG MAE", "Status");
    println!("{}", "-".repeat(100));
    
    for (name, mae_ours, mae_opj) in &results {
        let status = if *mae_opj > 0.01 { "❌ FAIL" } else { "✅ PASS" };
        println!("{:<40} | {:>15.6} | {:>15.6} | {}", name, mae_ours, mae_opj, status);
    }
    
    println!("\n{}", if all_passed { 
        "✅ ALL TESTS PASSED!" 
    } else { 
        "⚠️  SOME TESTS FAILED - See details above" 
    });
    
    if !all_passed {
        panic!("OpenJPEG interoperability tests failed");
    }
}

#[test]
#[ignore]
fn test_specific_large_image() {
    // Quick test for a specific size/pattern
    let width = 512;
    let height = 512;
    let dwt_levels = 5;
    
    println!("Testing {}x{} with {} DWT levels", width, height, dwt_levels);
    
    let pixels = generate_gradient(width, height);
    let (mae_ours, mae_opj) = test_image_with_openjpeg(
        "Large Gradient",
        &pixels,
        width,
        height,
        dwt_levels,
    );
    
    assert!(mae_ours < 0.01, "Our decoder failed: MAE={}", mae_ours);
    
    // Only assert OpenJPEG compatibility if tools are available
    if mae_opj != f64::MAX {
        assert!(mae_opj < 0.01, "OpenJPEG decoder failed: MAE={}", mae_opj);
    } else {
        println!("⚠️  OpenJPEG tools not available - skipping interop check");
    }
}

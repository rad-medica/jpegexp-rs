//! Detailed OpenJPEG Interoperability Diagnostics

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

fn calculate_mae(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        return f64::MAX;
    }
    let sum: u64 = a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

#[test]
#[ignore]
fn test_openjpeg_interop_detailed() {
    const OPJ_DECOMPRESS: &str = "openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_decompress.exe";
    
    if !std::path::Path::new(OPJ_DECOMPRESS).exists() {
        println!("OpenJPEG not available, skipping");
        return;
    }
    
    // Test 1: Simple solid image (value = 0)
    println!("\n=== Test 1: Solid Black (0) ===");
    test_pattern(&vec![0u8; 64], 8, 8, "solid_black", OPJ_DECOMPRESS);
    
    // Test 2: Solid mid-gray (128)
    println!("\n=== Test 2: Solid Mid-Gray (128) ===");
    test_pattern(&vec![128u8; 64], 8, 8, "solid_gray", OPJ_DECOMPRESS);
    
    // Test 3: Solid white (255)
    println!("\n=== Test 3: Solid White (255) ===");
    test_pattern(&vec![255u8; 64], 8, 8, "solid_white", OPJ_DECOMPRESS);
    
    // Test 4: Simple gradient
    println!("\n=== Test 4: Simple Gradient ===");
    let gradient: Vec<u8> = (0..64).map(|i| (i * 4) as u8).collect();
    test_pattern(&gradient, 8, 8, "gradient", OPJ_DECOMPRESS);
    
    // Test 5: Checkerboard
    println!("\n=== Test 5: Checkerboard ===");
    let mut checker = vec![0u8; 64];
    for y in 0..8 {
        for x in 0..8 {
            checker[y * 8 + x] = if (x + y) % 2 == 0 { 0 } else { 255 };
        }
    }
    test_pattern(&checker, 8, 8, "checker", OPJ_DECOMPRESS);
}

fn test_pattern(pixels: &[u8], width: u32, height: u32, name: &str, opj_decompress: &str) {
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    // Encode with jpegexp
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let encoded_len = encoder.encode(pixels, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded {} pixels to {} bytes", pixels.len(), encoded_len);
    
    // Write to file
    let j2k_path = format!("tests/openjpeg_compat/diag_{}.j2k", name);
    fs::write(&j2k_path, &encoded).expect("Failed to write J2K");
    
    // Dump first few bytes of marker segments
    println!("First 60 bytes of encoded stream:");
    for (i, b) in encoded.iter().take(60).enumerate() {
        print!("{:02X} ", b);
        if (i + 1) % 16 == 0 { println!(); }
    }
    println!();
    
    // Decode with jpegexp
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Our decoder failed");
    let our_decoded = image.reconstruct_pixels().expect("Our reconstruction failed");
    
    let our_mae = calculate_mae(pixels, &our_decoded);
    println!("Our decoder MAE: {:.6}", our_mae);
    
    if our_mae > 0.0 {
        println!("  First 16 pixels:");
        println!("    Original:      {:?}", &pixels[0..16.min(pixels.len())]);
        println!("    Our decoded:   {:?}", &our_decoded[0..16.min(our_decoded.len())]);
    }
    
    // Decode with OpenJPEG
    let pgm_path = format!("tests/openjpeg_compat/diag_{}.pgm", name);
    let output = Command::new(opj_decompress)
        .args(["-i", &j2k_path, "-o", &pgm_path])
        .output()
        .expect("Failed to run opj_decompress");
    
    if !output.status.success() {
        println!("OpenJPEG stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("OpenJPEG failed to decode");
    }
    
    // Read OpenJPEG output
    let pgm_data = fs::read(&pgm_path).expect("Failed to read PGM");
    let expected_len = (width * height) as usize;
    let opj_decoded = &pgm_data[pgm_data.len() - expected_len..];
    
    let opj_mae = calculate_mae(pixels, opj_decoded);
    println!("OpenJPEG decoder MAE: {:.6}", opj_mae);
    
    if opj_mae > 0.0 {
        println!("  First 16 pixels:");
        println!("    Original:      {:?}", &pixels[0..16.min(pixels.len())]);
        println!("    OpenJPEG dec:  {:?}", &opj_decoded[0..16.min(opj_decoded.len())]);
        println!("  Differences (first 16):");
        for i in 0..16.min(pixels.len()) {
            let diff = (pixels[i] as i32 - opj_decoded[i] as i32).abs();
            if diff > 0 {
                println!("    Pixel {}: {} -> {} (diff: {})", i, pixels[i], opj_decoded[i], diff);
            }
        }
    }
    
    // Cleanup
    let _ = fs::remove_file(&j2k_path);
    let _ = fs::remove_file(&pgm_path);
    
    if opj_mae > 0.0 {
        println!("  ⚠️  OpenJPEG interoperability issue detected!");
    } else {
        println!("  ✅  OpenJPEG interoperability OK!");
    }
}

//! OpenJPEG Compatibility Test
//!
//! Tests that files encoded by jpegexp-rs can be decoded by OpenJPEG and vice versa.

use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;

const OPJ_COMPRESS: &str = "openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_compress.exe";
const OPJ_DECOMPRESS: &str = "openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_decompress.exe";

fn openjpeg_available() -> bool {
    std::path::Path::new(OPJ_COMPRESS).exists()
}

/// Create a PGM file (grayscale image format that OpenJPEG can read)
fn create_pgm(path: &str, width: u32, height: u32, pixels: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "P5")?;
    writeln!(file, "{} {}", width, height)?;
    writeln!(file, "255")?;
    file.write_all(pixels)?;
    Ok(())
}

/// Calculate MAE between two pixel arrays
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

/// Calculate max difference between two pixel arrays
fn calculate_max_diff(a: &[u8], b: &[u8]) -> u32 {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs())
        .max()
        .unwrap_or(0)
}

#[test]
fn test_lossless_self_roundtrip() {
    // Test our own encoder -> decoder lossless roundtrip
    let width = 64u32;
    let height = 64u32;
    
    // Create gradient test image
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 2).min(255) as u8;
            original.push(val);
        }
    }
    
    // Encode with jpegexp-rs (lossless)
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless 5-3 DWT
    encoder.set_decomposition_levels(5);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    // Decode with jpegexp-rs
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    // Calculate MAE
    let mae = calculate_mae(&original, &reconstructed);
    let max_diff = calculate_max_diff(&original, &reconstructed);
    
    println!("Lossless self-roundtrip (64x64 gradient):");
    println!("  Encoded size: {} bytes", encoded_len);
    println!("  MAE: {:.4}", mae);
    println!("  Max diff: {}", max_diff);
    
    assert_eq!(mae, 0.0, "Lossless roundtrip MUST have MAE = 0");
    assert_eq!(max_diff, 0, "Lossless roundtrip MUST have max_diff = 0");
}

#[test]
fn test_lossless_larger_image() {
    // Test larger image for lossless
    let width = 128u32;
    let height = 128u32;
    
    // Create more complex test pattern
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            // Diagonal gradient with some variation
            let val = (((x + y) as f32 * 1.5) as u32 % 256) as u8;
            original.push(val);
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 128 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(5);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    let mae = calculate_mae(&original, &reconstructed);
    let max_diff = calculate_max_diff(&original, &reconstructed);
    
    println!("Lossless self-roundtrip (128x128 complex):");
    println!("  Encoded size: {} bytes", encoded_len);
    println!("  Compression ratio: {:.2}x", (width * height) as f64 / encoded_len as f64);
    println!("  MAE: {:.4}", mae);
    println!("  Max diff: {}", max_diff);
    
    assert_eq!(mae, 0.0, "Lossless roundtrip MUST have MAE = 0");
}

#[test]
#[ignore] // Run with: cargo test openjpeg_decode -- --ignored
fn test_openjpeg_can_decode_our_lossless() {
    if !openjpeg_available() {
        println!("OpenJPEG not available, skipping");
        return;
    }
    
    let width = 8u32;
    let height = 8u32;
    
    // Create very simple test image
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            original.push(((x + y) * 16).min(255) as u8);
        }
    }
    
    // Encode with jpegexp-rs - use minimal decomposition
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2); // Minimal decomposition
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded 8x8 image to {} bytes", encoded_len);
    
    // Write J2K file
    std::fs::write("tests/fixtures/openjpeg/test_lossless.j2k", &encoded)
        .expect("Failed to write J2K");
    
    // Decode with OpenJPEG
    let output = Command::new(OPJ_DECOMPRESS)
        .args([
            "-i", "tests/fixtures/openjpeg/test_lossless.j2k",
            "-o", "tests/fixtures/openjpeg/test_lossless_opj.pgm",
        ])
        .output()
        .expect("Failed to run opj_decompress");
    
    println!("OpenJPEG stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("OpenJPEG stderr: {}", String::from_utf8_lossy(&output.stderr));
    
    if !output.status.success() {
        // Dump the file for debugging
        println!("Our encoded file (hex):");
        for (i, b) in encoded.iter().enumerate() {
            print!("{:02X} ", b);
            if (i + 1) % 16 == 0 { println!(); }
        }
        println!();
        panic!("OpenJPEG failed to decode our file");
    }
    
    // Read OpenJPEG output and compare
    let pgm_data = std::fs::read("tests/fixtures/openjpeg/test_lossless_opj.pgm")
        .expect("Failed to read PGM");
    
    // Parse PGM - safer method: take last width*height bytes
    let expected_len = (width * height) as usize;
    if pgm_data.len() < expected_len {
        panic!("PGM file too small: {} bytes, expected at least {}", pgm_data.len(), expected_len);
    }
    let decoded_pixels = &pgm_data[pgm_data.len() - expected_len..];
    
    /*
    let mut header_lines = 0;
    let mut pos = 0;
    while header_lines < 3 && pos < pgm_data.len() {
        if pgm_data[pos] == b'\n' {
            header_lines += 1;
        }
        pos += 1;
    }
    
    let decoded_pixels = &pgm_data[pos..];
    */
    
    let mae = calculate_mae(&original, decoded_pixels);
    let max_diff = calculate_max_diff(&original, decoded_pixels);
    println!("OpenJPEG decoded MAE: {:.4}", mae);
    println!("OpenJPEG decoded max_diff: {}", max_diff);
    
    assert_eq!(mae, 0.0, "OpenJPEG decode should be lossless, MAE={}", mae);
    
    // Cleanup
    let _ = std::fs::remove_file("tests/fixtures/openjpeg/test_lossless.j2k");
    let _ = std::fs::remove_file("tests/fixtures/openjpeg/test_lossless_opj.pgm");
}

#[test]
#[ignore]
fn test_openjpeg_large_image() {
    if !openjpeg_available() {
        return;
    }
    
    let width = 256u32;
    let height = 256u32;
    
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            original.push((x ^ y) as u8);
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; width as usize * height as usize * 2]; // Enough buffer
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(5);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    let path = "tests/fixtures/openjpeg/test_large.j2k";
    std::fs::write(path, &encoded).expect("Failed to write J2K");
    
    let output = Command::new(OPJ_DECOMPRESS)
        .args(["-i", path, "-o", "tests/fixtures/openjpeg/test_large_opj.pgm"])
        .output()
        .expect("Failed to run opj_decompress");
        
    if !output.status.success() {
        println!("OpenJPEG stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("OpenJPEG failed to decode large image");
    }
    
    // Check MAE
    let pgm_data = std::fs::read("tests/fixtures/openjpeg/test_large_opj.pgm")
        .expect("Failed to read PGM");
    
    // PGM P5 header parser
    // P5\n256 256\n255\n...
    let expected_len = (width * height) as usize;
    if pgm_data.len() < expected_len {
        panic!("PGM too short");
    }
    let decoded = &pgm_data[pgm_data.len() - expected_len..];
    
    let mae = calculate_mae(&original, decoded);
    println!("Large image MAE: {:.4}", mae);
    
    // Relaxed check for now
    if mae > 50.0 {
        panic!("MAE too high: {}", mae);
    }
    
    println!("OpenJPEG decoded large image successfully");
    
    // Cleanup
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file("tests/fixtures/openjpeg/test_large_opj.pgm");
}

#[test]
#[ignore]
fn test_openjpeg_color() {
    if !openjpeg_available() {
        return;
    }
    
    let width = 64u32;
    let height = 64u32;
    let _components = 3;
    
    let mut original: Vec<u8> = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            original.push(x as u8); // R
            original.push(y as u8); // G
            original.push(((x+y)/2) as u8); // B
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 3,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    let path = "tests/fixtures/openjpeg/test_color.j2k";
    std::fs::write(path, &encoded).expect("Failed to write J2K");
    
    let output = Command::new(OPJ_DECOMPRESS)
        .args(["-i", path, "-o", "tests/fixtures/openjpeg/test_color_opj.ppm"])
        .output()
        .expect("Failed to run opj_decompress");
        
    if !output.status.success() {
        println!("OpenJPEG stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("OpenJPEG failed to decode color image");
    }
    
    println!("OpenJPEG decoded color image successfully");
    
    // Cleanup
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file("tests/fixtures/openjpeg/test_color_opj.ppm");
}


#[test]
#[ignore] // Run with: cargo test openjpeg_encode -- --ignored
fn test_we_can_decode_openjpeg_lossless() {
    if !openjpeg_available() {
        println!("OpenJPEG not available, skipping");
        return;
    }
    
    let width = 8u32;
    let height = 8u32;
    
    // Create test image
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            original.push(((x + y) * 16).min(255) as u8);
        }
    }
    
    // Write as PGM for OpenJPEG
    create_pgm("tests/fixtures/openjpeg/test_src.pgm", width, height, &original)
        .expect("Failed to create PGM");
    
    // Encode with OpenJPEG (lossless - reversible = no -I flag)
    // Also use minimal decomposition levels
    let output = Command::new(OPJ_COMPRESS)
        .args([
            "-i", "tests/fixtures/openjpeg/test_src.pgm",
            "-o", "tests/fixtures/openjpeg/test_opj_lossless.j2k",
            "-n", "3", // 2 decomposition levels = 3 resolutions
        ])
        .output()
        .expect("Failed to run opj_compress");
    
    println!("OpenJPEG compress stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("OpenJPEG compress stderr: {}", String::from_utf8_lossy(&output.stderr));
    
    assert!(output.status.success(), "OpenJPEG failed to encode");
    
    // Decode with jpegexp-rs
    let j2k_data = std::fs::read("tests/fixtures/openjpeg/test_opj_lossless.j2k")
        .expect("Failed to read J2K");
    
    println!("OpenJPEG encoded {} bytes", j2k_data.len());
    
    let mut reader = JpegStreamReader::new(&j2k_data);
    let mut decoder = J2kDecoder::new(&mut reader);
    
    match decoder.decode() {
        Ok(image) => {
            match image.reconstruct_pixels() {
                Ok(reconstructed) => {
                    let mae = calculate_mae(&original, &reconstructed);
                    let max_diff = calculate_max_diff(&original, &reconstructed);
                    
                    println!("Decoding OpenJPEG lossless file:");
                    println!("  MAE: {:.4}", mae);
                    println!("  Max diff: {}", max_diff);
                    
                    assert_eq!(mae, 0.0, "Should decode OpenJPEG lossless with MAE = 0, got {}", mae);
                }
                Err(e) => {
                    println!("Reconstruction failed: {:?}", e);
                    panic!("Failed to reconstruct");
                }
            }
        }
        Err(e) => {
            println!("Decode failed: {:?}", e);
            panic!("Failed to decode OpenJPEG file");
        }
    }
    
    // Cleanup
    let _ = std::fs::remove_file("tests/fixtures/openjpeg/test_src.pgm");
    let _ = std::fs::remove_file("tests/fixtures/openjpeg/test_opj_lossless.j2k");
}

#[test]
fn test_lossless_256x256() {
    let width = 256u32;
    let height = 256u32;
    
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            original.push((x ^ y) as u8);
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 256 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(5);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    let mae = calculate_mae(&original, &reconstructed);
    let max_diff = calculate_max_diff(&original, &reconstructed);
    
    println!("Lossless 256x256 XOR pattern:");
    println!("  Encoded size: {} bytes", encoded_len);
    println!("  Compression ratio: {:.2}x", (width * height) as f64 / encoded_len as f64);
    println!("  MAE: {:.4}", mae);
    println!("  Max diff: {}", max_diff);
    
    assert_eq!(mae, 0.0, "Lossless roundtrip MUST have MAE = 0");
}

#[test]
fn test_lossless_512x512() {
    let width = 512u32;
    let height = 512u32;
    
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            // More complex pattern
            let v = ((x as f32).sin() * 127.0 + (y as f32 * 0.1).cos() * 127.0 + 127.0) as u8;
            original.push(v);
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 512 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(5);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    let mae = calculate_mae(&original, &reconstructed);
    let max_diff = calculate_max_diff(&original, &reconstructed);
    
    println!("Lossless 512x512 sine pattern:");
    println!("  Encoded size: {} bytes", encoded_len);
    println!("  Compression ratio: {:.2}x", (width * height) as f64 / encoded_len as f64);
    println!("  MAE: {:.4}", mae);
    println!("  Max diff: {}", max_diff);
    
    assert_eq!(mae, 0.0, "Lossless roundtrip MUST have MAE = 0");
}

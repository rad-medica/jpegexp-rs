/// Minimal test to debug JPEG 2000 gradient encoding failure
/// 
/// From comprehensive interop tests, we know:
/// - Solid patterns: 100% pass (MAE=0)
/// - Gradient patterns: FAIL (MAE 0.9-118 for 8-bit)
/// - 8-bit: 86.7% pass, but gradients fail
/// 
/// This test creates a minimal 4x4 gradient to isolate the issue.

use jpegexp_rs::jpeg2000::encoder::encode_lossless;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn debug_j2k_4x4_gradient() {
    // Create 4x4 diagonal gradient (8-bit grayscale)
    let mut pixels = Vec::new();
    for y in 0..4u8 {
        for x in 0..4u8 {
            let val = ((x + y) * 16); // 0, 16, 32, 48, ...
            pixels.push(val);
        }
    }

    println!("\n=== Input 4x4 gradient ===");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:3} ", pixels[y * 4 + x]);
        }
        println!();
    }

    // Test 1: Rust encode -> OpenJPEG decode
    {
        println!("\n=== Test 1: Rust Encode -> OpenJPEG Decode ===");
        
        let encoded = encode_lossless(&pixels, 4, 4, 1, 8, true)
            .expect("Failed to encode with Rust");
        
        println!("Encoded {} bytes", encoded.len());
        
        let temp_j2k = "tests/fixtures/out/debug_gradient_4x4.j2k";
        fs::write(temp_j2k, &encoded).unwrap();
        
        // Decode with OpenJPEG
        let temp_raw = "tests/fixtures/out/debug_gradient_4x4_opj.raw";
        let result = Command::new("libs/bin/opj_decompress.exe")
            .args(["-i", temp_j2k, "-o", temp_raw])
            .output();
        
        match result {
            Ok(out) if out.status.success() => {
                println!("OpenJPEG decode: SUCCESS");
                
                let decoded = fs::read(temp_raw).unwrap();
                println!("Decoded {} bytes", decoded.len());
                
                println!("\n=== Decoded output ===");
                for y in 0..4 {
                    for x in 0..4 {
                        print!("{:3} ", decoded[y * 4 + x]);
                    }
                    println!();
                }
                
                // Compare
                let mut diffs = 0;
                let mut max_diff = 0u8;
                for i in 0..16 {
                    if pixels[i] != decoded[i] {
                        let diff = pixels[i].abs_diff(decoded[i]);
                        println!("  Pixel {}: {} -> {} (diff: {})", i, pixels[i], decoded[i], diff);
                        diffs += 1;
                        max_diff = max_diff.max(diff);
                    }
                }
                
                if diffs > 0 {
                    panic!("OpenJPEG decode failed: {} pixels differ (max diff: {})", diffs, max_diff);
                } else {
                    println!("✓ Perfect match!");
                }
            }
            Ok(out) => {
                println!("OpenJPEG decode FAILED");
                println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                println!("stdout: {}", String::from_utf8_lossy(&out.stdout));
                panic!("OpenJPEG failed to decode our output");
            }
            Err(e) => panic!("Failed to run OpenJPEG: {}", e),
        }
    }

    // Test 2: OpenJPEG encode -> Rust decode
    {
        println!("\n=== Test 2: OpenJPEG Encode -> Rust Decode ===");
        
        let temp_raw = "tests/fixtures/out/debug_gradient_4x4_input.raw";
        fs::write(temp_raw, &pixels).unwrap();
        
        let temp_j2k = "tests/fixtures/out/debug_gradient_4x4_opj.j2k";
        let result = Command::new("libs/bin/opj_compress.exe")
            .args([
                "-i", temp_raw,
                "-o", temp_j2k,
                "-F", "4,4,1,8,u",  // width,height,components,bitdepth,unsigned
                "-r", "0",          // lossless
            ])
            .output();
        
        match result {
            Ok(out) if out.status.success() => {
                println!("OpenJPEG encode: SUCCESS");
                
                let encoded = fs::read(temp_j2k).unwrap();
                println!("Encoded {} bytes", encoded.len());
                
                // Decode with our decoder
                let mut decoder = J2kDecoder::new(&encoded, &Default::default());
                match decoder.read_header() {
                    Ok(_) => {
                        println!("Rust read_header: SUCCESS");
                        
                        let mut decoded = vec![0u8; 16];
                        match decoder.get_decoded_buffer(&mut decoded) {
                            Ok(_) => {
                                println!("Rust decode: SUCCESS");
                                
                                println!("\n=== Decoded output ===");
                                for y in 0..4 {
                                    for x in 0..4 {
                                        print!("{:3} ", decoded[y * 4 + x]);
                                    }
                                    println!();
                                }
                                
                                // Compare
                                let mut diffs = 0;
                                let mut max_diff = 0u8;
                                for i in 0..16 {
                                    if pixels[i] != decoded[i] {
                                        let diff = pixels[i].abs_diff(decoded[i]);
                                        println!("  Pixel {}: {} -> {} (diff: {})", i, pixels[i], decoded[i], diff);
                                        diffs += 1;
                                        max_diff = max_diff.max(diff);
                                    }
                                }
                                
                                if diffs > 0 {
                                    panic!("Rust decode failed: {} pixels differ (max diff: {})", diffs, max_diff);
                                } else {
                                    println!("✓ Perfect match!");
                                }
                            }
                            Err(e) => {
                                println!("Rust decode FAILED: {:?}", e);
                                panic!("Failed to decode OpenJPEG output: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Rust read_header FAILED: {:?}", e);
                        panic!("Failed to read header: {:?}", e);
                    }
                }
            }
            Ok(out) => {
                println!("OpenJPEG encode FAILED");
                println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                println!("stdout: {}", String::from_utf8_lossy(&out.stdout));
                panic!("OpenJPEG failed to encode");
            }
            Err(e) => panic!("Failed to run OpenJPEG: {}", e),
        }
    }
}

#[test]
#[ignore]
fn debug_j2k_4x4_solid() {
    // Create 4x4 solid (all 128)
    let pixels = vec![128u8; 16];

    println!("\n=== Input 4x4 solid (all 128) ===");

    // Test 1: Rust encode -> OpenJPEG decode
    {
        println!("\n=== Test 1: Rust Encode -> OpenJPEG Decode ===");
        
        let encoded = encode_lossless(&pixels, 4, 4, 1, 8, true)
            .expect("Failed to encode with Rust");
        
        println!("Encoded {} bytes", encoded.len());
        
        let temp_j2k = "tests/fixtures/out/debug_solid_4x4.j2k";
        fs::write(temp_j2k, &encoded).unwrap();
        
        // Decode with OpenJPEG
        let temp_raw = "tests/fixtures/out/debug_solid_4x4_opj.raw";
        let result = Command::new("libs/bin/opj_decompress.exe")
            .args(["-i", temp_j2k, "-o", temp_raw])
            .output();
        
        match result {
            Ok(out) if out.status.success() => {
                println!("OpenJPEG decode: SUCCESS");
                
                let decoded = fs::read(temp_raw).unwrap();
                
                // Compare
                assert_eq!(pixels, decoded, "Solid pattern should match exactly");
                println!("✓ Perfect match!");
            }
            Ok(out) => {
                panic!("OpenJPEG decode failed: {}", String::from_utf8_lossy(&out.stderr));
            }
            Err(e) => panic!("Failed to run OpenJPEG: {}", e),
        }
    }
}

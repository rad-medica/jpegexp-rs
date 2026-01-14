// Diagnostic test to investigate 40x40 diagonal gradient failure
// This test creates detailed dumps to compare with OpenJPEG

use std::process::Command;
use std::fs;

#[test]
#[ignore]
fn diagnostic_40x40_detailed() {
    let size = 40;
    let levels = 2;
    
    // Create diagonal gradient pattern (same as failing test)
    let mut image = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let val = ((x * 4 + y * 4) % 256) as u8;
            image.push(val);
        }
    }
    
    // Write raw image
    let input_path = "diagnostic_40x40_input.raw";
    fs::write(input_path, &image).expect("Failed to write input");
    
    // Encode with our encoder
    let ours_path = "diagnostic_40x40_ours.j2k";
    let mut encoder = jpegexp_rs::jpeg2000::encoder::J2kEncoder::new();
    encoder.set_decomposition_levels(levels);
    encoder.set_irreversible(false);
    
    let frame_info = jpegexp_rs::FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; size * size * 4];
    let len = encoder.encode(&image, &frame_info, &mut output)
        .expect("Encoding failed");
    output.truncate(len);
    
    fs::write(ours_path, &output).expect("Failed to write our output");
    println!("Our encoder: {} bytes", len);
    
    // Encode with OpenJPEG
    let opj_path = "diagnostic_40x40_opj.j2k";
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", input_path,
            "-o", opj_path,
            "-F", &format!("{},{},{},8,u", size, size, 1),
            "-n", &levels.to_string(),
            "-I", // Lossless
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    assert!(status.success(), "OpenJPEG encoding failed");
    
    let opj_bytes = fs::read(opj_path).expect("Failed to read OpenJPEG output");
    println!("OpenJPEG encoder: {} bytes", opj_bytes.len());
    println!("Size difference: {} bytes (ours - opj)", len as i32 - opj_bytes.len() as i32);
    
    // Decode with OpenJPEG
    let decoded_path = "diagnostic_40x40_decoded.pgm";
    let status = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", ours_path,
            "-o", decoded_path,
        ])
        .output()
        .expect("Failed to run opj_decompress");
    
    if !status.status.success() {
        eprintln!("OpenJPEG decode stderr: {}", String::from_utf8_lossy(&status.stderr));
    }
    
    // Read decoded image
    if let Ok(decoded_data) = fs::read(decoded_path) {
        // Parse PGM format (skip header)
        let decoded_pixels = parse_pgm(&decoded_data);
        
        if decoded_pixels.len() == image.len() {
            // Calculate error metrics
            let mut max_error = 0u32;
            let mut sum_error = 0u64;
            let mut error_count = 0usize;
            let mut first_errors = Vec::new();
            
            for (i, (&expected, &actual)) in image.iter().zip(decoded_pixels.iter()).enumerate() {
                let error = (expected as i32 - actual as i32).abs() as u32;
                if error > 0 {
                    error_count += 1;
                    sum_error += error as u64;
                    max_error = max_error.max(error);
                    
                    if first_errors.len() < 10 {
                        let y = i / size;
                        let x = i % size;
                        first_errors.push((x, y, expected, actual, error));
                    }
                }
            }
            
            let mae = if error_count > 0 {
                sum_error as f64 / error_count as f64
            } else {
                0.0
            };
            
            println!("\n=== Decode Results ===");
            println!("MAE: {:.4}", mae);
            println!("Max Error: {}", max_error);
            println!("Error Count: {}/{}", error_count, image.len());
            
            if !first_errors.is_empty() {
                println!("\n=== First 10 Errors ===");
                for (x, y, expected, actual, err) in first_errors {
                    println!("  ({:2}, {:2}): expected={:3}, actual={:3}, error={}", 
                             x, y, expected, actual, err);
                }
            }
            
            // Now dump subband information regardless of decode success
            dump_subband_analysis(size, levels);
            
        } else {
            println!("Decoded pixel count mismatch: {} vs {}", decoded_pixels.len(), image.len());
            
            // Still dump subband analysis
            dump_subband_analysis(size, levels);
        }
    } else {
        println!("Failed to read decoded image");
        
        // Still dump subband analysis
        dump_subband_analysis(size, levels);
    }
}

fn parse_pgm(data: &[u8]) -> Vec<u8> {
    // Simple PGM parser
    let mut iter = data.split(|&b| b == b'\n');
    
    // Skip P5 header
    let _ = iter.next();
    
    // Skip comments and find width/height
    while let Some(line) = iter.next() {
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        // Skip width/height line
        break;
    }
    
    // Skip maxval
    let _ = iter.next();
    
    // Rest is pixel data
    let header_len = data.iter().position(|&b| b == b'\n')
        .and_then(|p1| data[p1 + 1..].iter().position(|&b| b == b'\n').map(|p2| p1 + 1 + p2))
        .and_then(|p2| data[p2 + 1..].iter().position(|&b| b == b'\n').map(|p3| p2 + 1 + p3))
        .map(|p3| p3 + 1);
    
    if let Some(start) = header_len {
        data[start..].to_vec()
    } else {
        Vec::new()
    }
}

fn dump_subband_analysis(img_size: usize, levels: u8) {
    println!("\n=== Subband Dimensions Analysis ===");
    println!("Image size: {}x{}", img_size, img_size);
    println!("Decomposition levels: {}", levels);
    
    let mut w = img_size;
    let mut h = img_size;
    
    for level in 1..=levels {
        let ll_w = (w + 1) / 2;
        let ll_h = (h + 1) / 2;
        let hl_w = w - ll_w;
        let hl_h = ll_h;
        let lh_w = ll_w;
        let lh_h = h - ll_h;
        let hh_w = w - ll_w;
        let hh_h = h - ll_h;
        
        println!("\nLevel {}:", level);
        println!("  LL: {}x{}", ll_w, ll_h);
        println!("  HL: {}x{} {}", hl_w, hl_h, if hl_h % 4 == 0 { "✓" } else { "⚠ NOT multiple of 4!" });
        println!("  LH: {}x{} {}", lh_w, lh_h, if lh_h % 4 == 0 { "✓" } else { "⚠ NOT multiple of 4!" });
        println!("  HH: {}x{} {}", hh_w, hh_h, if hh_h % 4 == 0 { "✓" } else { "⚠ NOT multiple of 4!" });
        
        w = ll_w;
        h = ll_h;
    }
}

/// Test 16x16 diagonal gradient at level 2

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_16x16_diagonal_level2() {
    let width = 16;
    let height = 16;
    let mut pixels = vec![0u8; width * height];
    
    // Diagonal pattern that fails at level 2
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Testing 16x16 Diagonal at Level 2 ===");
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);  // Level 2!
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    
    fs::write("test_16x16_l2_ours.j2k", &our_output[..our_size]).unwrap();
    fs::write("test_16x16_l2_input.raw", &pixels).unwrap();
    
    // Encode with OpenJPEG
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "test_16x16_l2_input.raw",
            "-o", "test_16x16_l2_opj.j2k",
            "-n", "3",  // Level 2 = 3 decompositions
            "-r", "1",
            "-F", "16,16,1,8,u",
        ])
        .output();
    
    // Decode with OpenJPEG
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_16x16_l2_ours.j2k", "-o", "test_16x16_l2_decoded.pnm"])
        .output()
        .expect("Failed to decode");
    
    if output.status.success() {
        fn parse_pnm(data: &[u8]) -> Vec<u8> {
            let mut offset = 0;
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
            while offset < data.len() && (data[offset] == b'#' || data[offset] == b'\n') {
                while offset < data.len() && data[offset] != b'\n' {
                    offset += 1;
                }
                offset += 1;
            }
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
            data[offset..].to_vec()
        }
        
        let decoded_data = fs::read("test_16x16_l2_decoded.pnm").unwrap();
        let decoded_pixels = parse_pnm(&decoded_data);
        
        let mut errors = 0;
        let mut sum_error = 0.0;
        let mut max_error = 0;
        for i in 0..pixels.len().min(decoded_pixels.len()) {
            let error = (pixels[i] as i32 - decoded_pixels[i] as i32).abs();
            if error > 0 {
                if errors < 10 {
                    let y = i / width;
                    let x = i % width;
                    println!("  Error at ({},{}): {} -> {}, diff={}", 
                             x, y, pixels[i], decoded_pixels[i], error);
                }
                errors += 1;
                sum_error += error as f64;
                max_error = max_error.max(error);
            }
        }
        
        let mae = sum_error / pixels.len() as f64;
        
        if mae == 0.0 {
            println!("\n✅ 16x16 Level 2 test PASSED");
        } else {
            println!("\n❌ 16x16 Level 2 test FAILED");
            println!("   MAE={:.4}, Max={}, Errors={}/{}", mae, max_error, errors, pixels.len());
        }
        
        let opj_size = fs::metadata("test_16x16_l2_opj.j2k").unwrap().len();
        println!("   File sizes: Ours={}B, OpenJPEG={}B (diff={}B)", 
                 our_size, opj_size, our_size as i64 - opj_size as i64);
    } else {
        println!("\n❌ OpenJPEG decode failed");
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
#[ignore]
fn test_32x32_diagonal_level2() {
    let width = 32;
    let height = 32;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Testing 32x32 Diagonal at Level 2 ===");
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    
    fs::write("test_32x32_l2_ours.j2k", &our_output[..our_size]).unwrap();
    fs::write("test_32x32_l2_input.raw", &pixels).unwrap();
    
    // Encode with OpenJPEG
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "test_32x32_l2_input.raw",
            "-o", "test_32x32_l2_opj.j2k",
            "-n", "3",
            "-r", "1",
            "-F", "32,32,1,8,u",
        ])
        .output();
    
    // Decode with OpenJPEG
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_32x32_l2_ours.j2k", "-o", "test_32x32_l2_decoded.pnm"])
        .output()
        .expect("Failed to decode");
    
    if output.status.success() {
        fn parse_pnm(data: &[u8]) -> Vec<u8> {
            let mut offset = 0;
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
            while offset < data.len() && (data[offset] == b'#' || data[offset] == b'\n') {
                while offset < data.len() && data[offset] != b'\n' {
                    offset += 1;
                }
                offset += 1;
            }
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
            data[offset..].to_vec()
        }
        
        let decoded_data = fs::read("test_32x32_l2_decoded.pnm").unwrap();
        let decoded_pixels = parse_pnm(&decoded_data);
        
        let mut errors = 0;
        let mut sum_error = 0.0;
        let mut max_error = 0;
        for i in 0..pixels.len().min(decoded_pixels.len()) {
            let error = (pixels[i] as i32 - decoded_pixels[i] as i32).abs();
            if error > 0 {
                if errors < 10 {
                    let y = i / width;
                    let x = i % width;
                    println!("  Error at ({},{}): {} -> {}, diff={}", 
                             x, y, pixels[i], decoded_pixels[i], error);
                }
                errors += 1;
                sum_error += error as f64;
                max_error = max_error.max(error);
            }
        }
        
        let mae = sum_error / pixels.len() as f64;
        
        if mae == 0.0 {
            println!("\n✅ 32x32 Level 2 test PASSED");
        } else {
            println!("\n❌ 32x32 Level 2 test FAILED");
            println!("   MAE={:.4}, Max={}, Errors={}/{}", mae, max_error, errors, pixels.len());
        }
        
        let opj_size = fs::metadata("test_32x32_l2_opj.j2k").unwrap().len();
        println!("   File sizes: Ours={}B, OpenJPEG={}B (diff={}B)", 
                 our_size, opj_size, our_size as i64 - opj_size as i64);
    } else {
        println!("\n❌ OpenJPEG decode failed");
    }
}

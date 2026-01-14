/// Compare HH subband encoding between our encoder and OpenJPEG
/// This will help us find the exact divergence point

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;

#[path = "../common/mod.rs"]
mod common;
use common::file_io::get_test_output_path;

#[test]
#[ignore]
fn extract_and_compare_hh_subbands() {
    // Create the failing diagonal gradient pattern
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Extracting HH Subband Data for Comparison ===");
    println!("Pattern: Diagonal gradient (x*4 + y*4)");
    println!("This pattern FAILS with OpenJPEG at level 2");
    
    // Enable detailed tracing for HH subbands only
    std::env::set_var("BPC_TRACE_HH", "1");
    
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
    let our_bytes = &our_output[..our_size];
    
    let j2k_path = get_test_output_path("compare_hh_ours.j2k");
    let raw_path = get_test_output_path("compare_hh_input.raw");

    fs::write(&j2k_path, our_bytes).unwrap();
    fs::write(&raw_path, &pixels).unwrap();
    
    println!("\n=== Files Created ===");
    println!("  compare_hh_ours.j2k - Our encoding");
    println!("  compare_hh_input.raw - Input data");
    println!("\nTo compare with OpenJPEG:");
    println!("  1. Encode with OpenJPEG:");
    println!("     libs/bin/opj_compress.exe -i {} -o {} -n 3 -r 1 -F 64,64,1,8,u", 
        raw_path.to_str().unwrap(),
        get_test_output_path("compare_hh_opj.j2k").to_str().unwrap());
    println!("  2. Compare packet structures with Python script");
}

#[test]
#[ignore]
fn trace_hh_encoding_detailed() {
    // Minimal test case: 16x16 image that will create a small HH subband
    let width = 16;
    let height = 16;
    let mut pixels = vec![0u8; width * height];
    
    // Simple diagonal pattern
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Tracing 16x16 Diagonal Gradient ===");
    println!("Input pattern:");
    for y in 0..8 {
        print!("  ");
        for x in 0..8 {
            print!("{:3} ", pixels[y * width + x]);
        }
        println!();
    }
    
    // Enable all tracing
    std::env::set_var("BPC_TRACE", "1");
    std::env::set_var("MQ_SYMBOL_TRACE", "1");
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);  // Just 1 level for simplicity
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    
    println!("\n=== Encoding Complete ===");
    println!("Output size: {} bytes", our_size);
    
    let j2k_path = get_test_output_path("trace_16x16_diag.j2k");
    let raw_path = get_test_output_path("trace_16x16_diag.raw");
    let opj_path = get_test_output_path("trace_16x16_diag_opj.j2k");
    let pnm_path = get_test_output_path("trace_16x16_diag_decoded.pnm");

    fs::write(&j2k_path, &our_output[..our_size]).unwrap();
    fs::write(&raw_path, &pixels).unwrap();
    
    // Test with OpenJPEG
    use std::process::Command;
    
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", raw_path.to_str().unwrap(),
            "-o", opj_path.to_str().unwrap(),
            "-n", "2",
            "-r", "1",
            "-F", "16,16,1,8,u",
        ])
        .output();
    
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", j2k_path.to_str().unwrap(),
            "-o", pnm_path.to_str().unwrap()
        ])
        .output()
        .expect("Failed to decode");
    
    if output.status.success() {
        // Parse and compare
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
        
        let decoded_data = fs::read(&pnm_path).unwrap();
        let decoded_pixels = parse_pnm(&decoded_data);
        
        let mut errors = 0;
        let mut sum_error = 0.0;
        for i in 0..pixels.len().min(decoded_pixels.len()) {
            let error = (pixels[i] as i32 - decoded_pixels[i] as i32).abs();
            if error > 0 {
                errors += 1;
                sum_error += error as f64;
            }
        }
        
        let mae = sum_error / pixels.len() as f64;
        
        if mae == 0.0 {
            println!("\n✅ 16x16 test PASSED with OpenJPEG decoder");
        } else {
            println!("\n❌ 16x16 test FAILED: MAE={:.4}, {} errors", mae, errors);
        }
    } else {
        println!("\n❌ OpenJPEG decode failed");
    }
}

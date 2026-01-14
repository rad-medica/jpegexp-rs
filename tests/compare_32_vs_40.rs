/// Compare 32x32 (working) vs 40x40 (failing) with detailed logging

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

fn test_with_logging(size: usize, name: &str) {
    println!("\n============================================================");
    println!("Testing {}x{} - {}", size, size, name);
    println!("============================================================\n");
    
    let mut pixels = vec![0u8; size * size];
    
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    // Enable ALL debugging
    std::env::set_var("J2K_CBLK_DETAIL", "1");
    std::env::set_var("J2K_PKT_DEBUG", "1");
    std::env::set_var("J2K_EXTRACT_DEBUG", "1");
    
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
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
    
    let our_file = format!("compare_{}_{}.j2k", size, name);
    fs::write(&our_file, &our_output[..our_size]).unwrap();
    
    // Decode with OpenJPEG
    let decoded_file = format!("compare_{}_{}_decoded.pnm", size, name);
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", &our_file, "-o", &decoded_file])
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
        
        let decoded_data = fs::read(&decoded_file).unwrap();
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
            println!("\n✅ PASSED: MAE=0.0000");
        } else {
            println!("\n❌ FAILED: MAE={:.4}, {} errors", mae, errors);
        }
    } else {
        println!("\n❌ Decode failed");
    }
}

#[test]
#[ignore]
fn compare_32_vs_40_detailed() {
    test_with_logging(32, "working");
    test_with_logging(40, "failing");
}

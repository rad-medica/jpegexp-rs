/// Test different image sizes to find where the issue starts

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

fn test_size(size: usize) {
    let mut pixels = vec![0u8; size * size];
    
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
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
    
    let our_file = format!("test_{}x{}_ours.j2k", size, size);
    let raw_file = format!("test_{}x{}_input.raw", size, size);
    let opj_file = format!("test_{}x{}_opj.j2k", size, size);
    let decoded_file = format!("test_{}x{}_decoded.pnm", size, size);
    
    fs::write(&our_file, &our_output[..our_size]).unwrap();
    fs::write(&raw_file, &pixels).unwrap();
    
    // Encode with OpenJPEG
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", &raw_file,
            "-o", &opj_file,
            "-n", "3",
            "-r", "1",
            "-F", &format!("{},{},1,8,u", size, size),
        ])
        .output();
    
    // Decode with OpenJPEG
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
        let mut max_error = 0;
        for i in 0..pixels.len().min(decoded_pixels.len()) {
            let error = (pixels[i] as i32 - decoded_pixels[i] as i32).abs();
            if error > 0 {
                errors += 1;
                sum_error += error as f64;
                max_error = max_error.max(error);
            }
        }
        
        let mae = sum_error / pixels.len() as f64;
        let opj_size = fs::metadata(&opj_file).unwrap().len();
        
        let status = if mae == 0.0 { "✅" } else { "❌" };
        println!("{} {}x{}: MAE={:.4}, Max={}, Errors={}/{}, Size={}B vs {}B", 
                 status, size, size, mae, max_error, errors, pixels.len(), our_size, opj_size);
    } else {
        println!("❌ {}x{}: Decode failed", size, size);
    }
}

#[test]
#[ignore]
fn test_size_sweep_level2() {
    println!("\n=== Size Sweep at Level 2 (Diagonal Gradient) ===\n");
    
    // Test powers of 2 and some intermediate sizes
    for &size in &[8, 12, 16, 20, 24, 28, 32, 40, 48, 56, 64, 80, 96, 128] {
        test_size(size);
    }
}

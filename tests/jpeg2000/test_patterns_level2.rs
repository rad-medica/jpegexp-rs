/// Test various patterns at level 2 to isolate the issue

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

fn test_pattern(name: &str, pixels: &[u8], width: usize, height: usize) {
    println!("\n=== Testing {} Pattern ({}x{}) at Level 2 ===", name, width, height);
    
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
    
    let our_file = format!("test_{}_ours.j2k", name);
    let opj_file = format!("test_{}_opj.j2k", name);
    let raw_file = format!("test_{}_input.raw", name);
    let decoded_file = format!("test_{}_ours_decoded.pnm", name);
    
    fs::write(&our_file, our_bytes).unwrap();
    fs::write(&raw_file, &pixels).unwrap();
    
    // Encode with OpenJPEG
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", &raw_file,
            "-o", &opj_file,
            "-n", "3",
            "-r", "1",
            "-F", &format!("{},{},1,8,u", width, height),
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    if !status.success() {
        println!("❌ opj_compress failed");
        return;
    }
    
    // Decode our file with OpenJPEG
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", &our_file, "-o", &decoded_file])
        .output()
        .expect("Failed to decode");
    
    if !output.status.success() {
        println!("❌ Decode failed");
        return;
    }
    
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
    
    let status = if mae == 0.0 { "✅" } else { "❌" };
    println!("{} {} test: MAE={:.4}, Max={}, Errors={}/{}", 
             status, name, mae, max_error, errors, pixels.len());
    
    let opj_size = fs::metadata(&opj_file).unwrap().len();
    println!("   File sizes: Ours={}B, OpenJPEG={}B (diff={}B)", 
             our_size, opj_size, our_size as i64 - opj_size as i64);
}

#[test]
#[ignore]
fn test_all_patterns_level2() {
    let width = 64;
    let height = 64;
    
    // 1. Solid color (already tested, but include for completeness)
    let solid = vec![128u8; width * height];
    test_pattern("solid", &solid, width, height);
    
    // 2. Checkerboard (high frequency)
    let mut checkerboard = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            checkerboard[y * width + x] = if (x + y) % 2 == 0 { 0 } else { 255 };
        }
    }
    test_pattern("checkerboard", &checkerboard, width, height);
    
    // 3. Horizontal gradient (like our failing test)
    let mut h_gradient = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            h_gradient[y * width + x] = ((x * 4) % 256) as u8;
        }
    }
    test_pattern("h_gradient", &h_gradient, width, height);
    
    // 4. Vertical gradient
    let mut v_gradient = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            v_gradient[y * width + x] = ((y * 4) % 256) as u8;
        }
    }
    test_pattern("v_gradient", &v_gradient, width, height);
    
    // 5. Diagonal gradient (like debug_level_sweep)
    let mut d_gradient = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            d_gradient[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    test_pattern("d_gradient", &d_gradient, width, height);
    
    // 6. Ramp (0 to 255 repeated)
    let mut ramp = vec![0u8; width * height];
    for i in 0..width * height {
        ramp[i] = (i % 256) as u8;
    }
    test_pattern("ramp", &ramp, width, height);
    
    // 7. Two-tone (half black, half white)
    let mut two_tone = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            two_tone[y * width + x] = if x < width / 2 { 0 } else { 255 };
        }
    }
    test_pattern("two_tone", &two_tone, width, height);
}

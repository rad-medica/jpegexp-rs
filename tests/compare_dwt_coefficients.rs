/// Test solid color encoding to isolate the issue

#[test]
#[ignore]
fn test_solid_color_level2() {
    // Test with a solid color to see if the issue persists
    let width = 64;
    let height = 64;
    let pixels = vec![128u8; width * height];
    
    println!("\n=== Testing Solid Color (128) at Level 2 ===");
    
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    use std::fs;
    use std::process::Command;
    
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
    
    fs::write("test_solid_ours.j2k", our_bytes).unwrap();
    fs::write("test_solid_input.raw", &pixels).unwrap();
    
    // Encode with OpenJPEG
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "test_solid_input.raw",
            "-o", "test_solid_opj.j2k",
            "-n", "3",
            "-r", "1",
            "-F", "64,64,1,8,u",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    if !status.success() {
        println!("❌ opj_compress failed");
        return;
    }
    
    // Decode our file with OpenJPEG
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_solid_ours.j2k", "-o", "test_solid_ours_decoded.pnm"])
        .output()
        .expect("Failed to decode");
    
    if !output.status.success() {
        println!("❌ Decode failed");
        println!("{}", String::from_utf8_lossy(&output.stderr));
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
    
    let decoded_data = fs::read("test_solid_ours_decoded.pnm").unwrap();
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
        println!("✅ Solid color test PASSED: MAE=0.0000");
    } else {
        println!("❌ Solid color test FAILED: MAE={:.4}, {} errors", mae, errors);
    }
    
    println!("File sizes: Ours={}B, OpenJPEG={}B", 
             our_size, 
             fs::metadata("test_solid_opj.j2k").unwrap().len());
}

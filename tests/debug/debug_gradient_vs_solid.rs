use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn debug_gradient_vs_solid_file_comparison() {
    let width = 8;
    let height = 8;
    
    println!("\n=== SOLID PATTERN (128) ===");
    test_pattern(vec![128u8; width * height], "solid_128", width, height);
    
    println!("\n=== GRADIENT PATTERN ===");
    let mut gradient = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            gradient[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    test_pattern(gradient, "gradient", width, height);
}

fn test_pattern(pixels: Vec<u8>, name: &str, width: usize, height: usize) {
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    let our_bytes = &our_output[..our_size];
    
    let our_filename = format!("debug_ours_{}.j2k", name);
    let opj_filename = format!("debug_opj_{}.j2k", name);
    let raw_filename = format!("debug_input_{}.raw", name);
    
    fs::write(&our_filename, our_bytes).unwrap();
    fs::write(&raw_filename, &pixels).unwrap();
    
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", &raw_filename,
            "-o", &opj_filename,
            "-n", "2",
            "-r", "1",
            "-F", &format!("{},{},1,8,u", width, height),
            "-I",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    assert!(status.success(), "opj_compress failed");
    
    let opj_bytes = fs::read(&opj_filename).unwrap();
    
    println!("Ours:     {} bytes", our_size);
    println!("OpenJPEG: {} bytes", opj_bytes.len());
    println!("Size diff: {}", (our_size as i32) - (opj_bytes.len() as i32));
    
    let our_decoded_pnm = format!("debug_ours_decoded_{}.pnm", name);
    let opj_decoded_pnm = format!("debug_opj_decoded_{}.pnm", name);
    
    let status1 = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", &our_filename, "-o", &our_decoded_pnm])
        .output()
        .expect("Failed to decode our file");
    
    let status2 = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", &opj_filename, "-o", &opj_decoded_pnm])
        .output()
        .expect("Failed to decode OpenJPEG file");
    
    if !status1.status.success() {
        println!("❌ OpenJPEG failed to decode OUR file!");
        println!("stderr: {}", String::from_utf8_lossy(&status1.stderr));
        return;
    }
    
    if !status2.status.success() {
        println!("❌ Failed to decode OpenJPEG reference file!");
        return;
    }
    
    let our_decoded_data = fs::read(&our_decoded_pnm).unwrap();
    let opj_decoded_data = fs::read(&opj_decoded_pnm).unwrap();
    
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
    
    let our_pixels_decoded = parse_pnm(&our_decoded_data);
    let opj_pixels_decoded = parse_pnm(&opj_decoded_data);
    
    let mut sum_error = 0.0;
    let mut max_error = 0;
    let mut error_count = 0;
    
    let pixel_count = (width * height).min(our_pixels_decoded.len()).min(opj_pixels_decoded.len());
    
    for i in 0..pixel_count {
        let original = pixels[i] as i32;
        let our_decoded = our_pixels_decoded[i] as i32;
        
        let error = (original - our_decoded).abs();
        
        sum_error += error as f64;
        max_error = max_error.max(error);
        
        if error > 0 {
            error_count += 1;
            if error_count <= 10 {
                println!("  Pixel [{}] ({},{}): orig={}, ours={}, err={}",
                         i, i % width, i / width, original, our_decoded, error);
            }
        }
    }
    
    let mae = sum_error / pixel_count as f64;
    
    println!("MAE:         {:.4}", mae);
    println!("Max Error:   {}", max_error);
    println!("Error count: {}/{}", error_count, pixel_count);
    
    if mae == 0.0 {
        println!("✅ PERFECT");
    } else {
        println!("❌ HAS ERRORS");
    }
}

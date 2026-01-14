use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn debug_64x64_gradient_5_levels() {
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    println!("\n=== Testing 64x64 gradient with 5 decomposition levels ===\n");
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(5);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    let our_bytes = &our_output[..our_size];
    
    fs::write("debug_ours_64x64.j2k", our_bytes).unwrap();
    
    fs::write("debug_input_64x64.raw", &pixels).unwrap();
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "debug_input_64x64.raw",
            "-o", "debug_opj_64x64.j2k",
            "-n", "6",
            "-r", "1",
            "-F", "64,64,1,8,u",
            "-I",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    assert!(status.success(), "opj_compress failed");
    
    let opj_bytes = fs::read("debug_opj_64x64.j2k").unwrap();
    
    println!("--- File Sizes ---");
    println!("Ours:     {} bytes", our_size);
    println!("OpenJPEG: {} bytes", opj_bytes.len());
    
    let our_decoded_pnm = "debug_ours_decoded_64x64.pnm";
    let opj_decoded_pnm = "debug_opj_decoded_64x64.pnm";
    
    let status1 = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "debug_ours_64x64.j2k", "-o", our_decoded_pnm])
        .output()
        .expect("Failed to decode our file");
    
    let status2 = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "debug_opj_64x64.j2k", "-o", opj_decoded_pnm])
        .output()
        .expect("Failed to decode OpenJPEG file");
    
    if !status1.status.success() {
        println!("❌ OpenJPEG failed to decode OUR file!");
        println!("stderr: {}", String::from_utf8_lossy(&status1.stderr));
        panic!("Decode failed");
    }
    
    if !status2.status.success() {
        println!("❌ Failed to decode OpenJPEG reference file!");
        println!("stderr: {}", String::from_utf8_lossy(&status2.stderr));
        panic!("Decode failed");
    }
    
    let our_decoded_data = fs::read(our_decoded_pnm).unwrap();
    let opj_decoded_data = fs::read(opj_decoded_pnm).unwrap();
    
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
    
    let mut sum_our_error = 0.0;
    let mut sum_opj_error = 0.0;
    let mut max_our_error = 0;
    let mut max_opj_error = 0;
    let mut error_count = 0;
    
    let pixel_count = (width * height).min(our_pixels_decoded.len()).min(opj_pixels_decoded.len());
    
    for i in 0..pixel_count {
        let original = pixels[i] as i32;
        let our_decoded = our_pixels_decoded[i] as i32;
        let opj_decoded = opj_pixels_decoded[i] as i32;
        
        let our_error = (original - our_decoded).abs();
        let opj_error = (original - opj_decoded).abs();
        
        sum_our_error += our_error as f64;
        sum_opj_error += opj_error as f64;
        max_our_error = max_our_error.max(our_error);
        max_opj_error = max_opj_error.max(opj_error);
        
        if our_error > 0 || opj_error > 0 {
            error_count += 1;
            if error_count <= 20 {
                println!("Pixel [{}] ({},{}): orig={}, ours={} (err={}), opj={} (err={})",
                         i, i % width, i / width, original, our_decoded, our_error, opj_decoded, opj_error);
            }
        }
    }
    
    let mae_ours = sum_our_error / pixel_count as f64;
    let mae_opj = sum_opj_error / pixel_count as f64;
    
    println!("\n--- Metrics ---");
    println!("MAE (ours vs original):     {:.4}", mae_ours);
    println!("MAE (OpenJPEG vs original): {:.4}", mae_opj);
    println!("Max Error (ours):           {}", max_our_error);
    println!("Max Error (OpenJPEG):       {}", max_opj_error);
    println!("Pixels with errors:         {}/{}", error_count, pixel_count);
    
    if mae_ours == 0.0 {
        println!("\n✅ OUR ENCODER IS PERFECT: MAE = 0");
    } else {
        println!("\n❌ OUR ENCODER HAS ERRORS: MAE = {}", mae_ours);
    }
    
    if mae_opj > 0.0 {
        println!("⚠️  OpenJPEG encoder also has errors: MAE = {}", mae_opj);
    }
}

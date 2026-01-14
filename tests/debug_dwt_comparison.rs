use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn debug_dwt_simple_gradient() {
    // Create a simple 8x8 gradient
    let width = 8;
    let height = 8;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = (x * 16 + y * 16) as u8;
        }
    }
    
    println!("\n=== Input 8x8 Gradient ===");
    for y in 0..height {
        print!("  ");
        for x in 0..width {
            print!("{:3} ", pixels[y * width + x]);
        }
        println!();
    }
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    // Test with 1 decomposition level first (simpler to debug)
    for decomp_levels in [1, 2, 5] {
        println!("\n============================================================");
        println!("Testing with {} decomposition levels", decomp_levels);
        println!("============================================================");
        
        // Encode with our encoder
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(decomp_levels);
        
        let mut our_output = vec![0u8; 1024 * 1024];
        let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
        let our_bytes = &our_output[..our_size];
        
        fs::write(format!("debug_ours_8x8_dwt{}.j2k", decomp_levels), our_bytes).unwrap();
        
        // Encode with OpenJPEG
        fs::write("debug_input_8x8.raw", &pixels).unwrap();
        let status = Command::new("libs/bin/opj_compress.exe")
            .args(&[
                "-i", "debug_input_8x8.raw",
                "-o", &format!("debug_opj_8x8_dwt{}.j2k", decomp_levels),
                "-n", &format!("{}", decomp_levels + 1), // resolutions = levels + 1
                "-r", "1", // lossless
                "-F", "8,8,1,8,u",
                "-I", // irreversible=false (use 5-3)
            ])
            .status()
            .expect("Failed to run opj_compress");
        
        assert!(status.success(), "opj_compress failed");
        
        let opj_bytes = fs::read(format!("debug_opj_8x8_dwt{}.j2k", decomp_levels)).unwrap();
        
        println!("\n--- File Sizes ---");
        println!("Ours:     {} bytes", our_size);
        println!("OpenJPEG: {} bytes", opj_bytes.len());
        
        // Decode both with OpenJPEG
        let our_decoded_pnm = format!("debug_ours_decoded_{}.pnm", decomp_levels);
        let opj_decoded_pnm = format!("debug_opj_decoded_{}.pnm", decomp_levels);
        
        let status1 = Command::new("libs/bin/opj_decompress.exe")
            .args(&["-i", &format!("debug_ours_8x8_dwt{}.j2k", decomp_levels), "-o", &our_decoded_pnm])
            .status()
            .expect("Failed to decode our file");
        
        let status2 = Command::new("libs/bin/opj_decompress.exe")
            .args(&["-i", &format!("debug_opj_8x8_dwt{}.j2k", decomp_levels), "-o", &opj_decoded_pnm])
            .status()
            .expect("Failed to decode OpenJPEG file");
        
        if !status1.success() {
            println!("❌ OpenJPEG failed to decode OUR file!");
            continue;
        }
        
        if !status2.success() {
            println!("❌ Failed to decode OpenJPEG reference file!");
            continue;
        }
        
        // Read decoded pixels
        let our_decoded_data = fs::read(&our_decoded_pnm).unwrap();
        let opj_decoded_data = fs::read(&opj_decoded_pnm).unwrap();
        
        // Parse PNM format (skip header)
        fn parse_pnm(data: &[u8]) -> Vec<u8> {
            let mut lines = data.split(|&b| b == b'\n');
            lines.next(); // P5 or P6
            
            // Skip comments
            loop {
                let line = lines.next().unwrap();
                if !line.is_empty() && line[0] != b'#' {
                    break;
                }
            }
            
            // Skip maxval line
            lines.next();
            
            // Rest is pixel data
            lines.flatten().copied().collect()
        }
        
        let our_pixels_decoded = parse_pnm(&our_decoded_data);
        let opj_pixels_decoded = parse_pnm(&opj_decoded_data);
        
        println!("\n--- Decoded Pixel Comparison ---");
        println!("Our decoded:");
        for y in 0..height {
            print!("  ");
            for x in 0..width {
                if y * width + x < our_pixels_decoded.len() {
                    print!("{:3} ", our_pixels_decoded[y * width + x]);
                } else {
                    print!("??? ");
                }
            }
            println!();
        }
        
        println!("\nOpenJPEG decoded:");
        for y in 0..height {
            print!("  ");
            for x in 0..width {
                if y * width + x < opj_pixels_decoded.len() {
                    print!("{:3} ", opj_pixels_decoded[y * width + x]);
                } else {
                    print!("??? ");
                }
            }
            println!();
        }
        
        // Calculate MAE
        let mut sum_error = 0.0;
        let mut max_error = 0;
        let pixel_count = (width * height).min(our_pixels_decoded.len()).min(opj_pixels_decoded.len());
        
        for i in 0..pixel_count {
            let original = pixels[i] as i32;
            let our_decoded = our_pixels_decoded[i] as i32;
            let opj_decoded = opj_pixels_decoded[i] as i32;
            
            let our_error = (original - our_decoded).abs();
            let opj_error = (original - opj_decoded).abs();
            
            sum_error += our_error as f64;
            max_error = max_error.max(our_error);
            
            if our_error > 0 || opj_error > 0 {
                println!("Pixel [{}]: orig={}, ours={} (err={}), opj={} (err={})",
                         i, original, our_decoded, our_error, opj_decoded, opj_error);
            }
        }
        
        let mae = sum_error / pixel_count as f64;
        println!("\n--- Metrics ---");
        println!("MAE (ours):  {:.4}", mae);
        println!("Max Error:   {}", max_error);
        
        if mae > 0.0 {
            println!("❌ LOSSLESS FAILED: MAE > 0");
        } else {
            println!("✅ LOSSLESS PERFECT: MAE = 0");
        }
    }
}

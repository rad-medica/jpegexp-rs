// Minimal test case to debug J2K gradient interoperability issue
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

#[test]
#[ignore]
fn debug_8x8_gradient() {
    // Create a minimal 8x8 gradient pattern
    let width = 8;
    let height = 8;
    let mut pixels: Vec<u8> = vec![0; (width * height) as usize];
    
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height - 2)) as u8;
            pixels[(y * width + x) as usize] = val;
        }
    }
    
    println!("Original gradient 8x8:");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", pixels[(y * width + x) as usize]);
        }
        println!();
    }
    
    // Encode with our encoder
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    
    let mut encoded = vec![0u8; (width * height * 4) as usize];
    let encoded_len = encoder.encode(&pixels, &frame_info, &mut encoded).unwrap();
    encoded.truncate(encoded_len);
    
    println!("\nEncoded to {} bytes", encoded_len);
    
    // Write to file for OpenJPEG
    fs::write("test_gradient_8x8_rust.j2k", &encoded).unwrap();
    
    // Decode with OpenJPEG
    let status = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_gradient_8x8_rust.j2k", "-o", "test_gradient_8x8_rust_opj.pgm"])
        .output()
        .expect("Failed to run opj_decompress");
    
    if !status.status.success() {
        println!("OpenJPEG decode failed: {}", String::from_utf8_lossy(&status.stderr));
        panic!("OpenJPEG failed to decode");
    }
    
    // Read back the PGM
    let pgm_data = fs::read("test_gradient_8x8_rust_opj.pgm").unwrap();
    
    // Parse PGM (skip P5 header)
    let mut lines = pgm_data.split(|&b| b == b'\n');
    let _magic = lines.next().unwrap();
    let mut dims_line = lines.next().unwrap();
    while dims_line.starts_with(b"#") {
        dims_line = lines.next().unwrap();
    }
    let _maxval = lines.next().unwrap();
    let decoded_pixels: Vec<u8> = lines.flatten().copied().collect();
    
    println!("\nDecoded by OpenJPEG:");
    for y in 0..height {
        for x in 0..width {
            if (y * width + x) < decoded_pixels.len() as u32 {
                print!("{:3} ", decoded_pixels[(y * width + x) as usize]);
            }
        }
        println!();
    }
    
    // Compare
    let mut total_error = 0.0;
    let mut max_error = 0i32;
    for i in 0..(width * height).min(decoded_pixels.len() as u32) {
        let orig = pixels[i as usize] as i32;
        let dec = decoded_pixels[i as usize] as i32;
        let error = (orig - dec).abs();
        total_error += error as f64;
        max_error = max_error.max(error);
        if error > 0 {
            println!("Pixel [{}]: {} -> {} (error = {})", i, orig, dec, error);
        }
    }
    
    let mae = total_error / (width * height) as f64;
    println!("\nMAE: {:.4}, Max Error: {}", mae, max_error);
    
    // Clean up
    let _ = fs::remove_file("test_gradient_8x8_rust.j2k");
    let _ = fs::remove_file("test_gradient_8x8_rust_opj.pgm");
    
    if mae > 0.0 {
        panic!("OpenJPEG failed to decode our output with MAE={:.4}", mae);
    }
}

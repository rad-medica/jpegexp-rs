use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn minimal_8x8_level2() {
    let width = 8;
    let height = 8;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = (x * 32 + y * 32) as u8;
        }
    }
    
    println!("\n=== Minimal 8x8 Level 2 Test ===");
    println!("Input pattern (diagonal gradient):");
    for y in 0..height {
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
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    let our_bytes = &our_output[..our_size];
    
    fs::write("minimal_ours.j2k", our_bytes).unwrap();
    fs::write("minimal_input.raw", &pixels).unwrap();
    
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "minimal_input.raw",
            "-o", "minimal_opj.j2k",
            "-n", "3",
            "-r", "1",
            "-F", "8,8,1,8,u",
            "-I",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    assert!(status.success());
    
    let _ = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "minimal_ours.j2k", "-o", "minimal_ours_decoded.pgm"])
        .output();
    
    let opj_bytes = fs::read("minimal_opj.j2k").unwrap();
    
    println!("\nFile sizes:");
    println!("  Ours:     {} bytes", our_size);
    println!("  OpenJPEG: {} bytes", opj_bytes.len());
    
    if let Ok(decoded_data) = fs::read("minimal_ours_decoded.pgm") {
        let idx = decoded_data.windows(2).position(|w| w == b"\n\n" || w == b"\n2").unwrap_or(0) + 2;
        let idx = decoded_data[idx..].iter().position(|&b| b == b'\n').unwrap() + idx + 1;
        let decoded_pixels = &decoded_data[idx..idx+64];
        
        let mut errors = 0;
        let mut max_err = 0;
        println!("\nDecoded output:");
        for y in 0..height {
            for x in 0..width {
                let orig = pixels[y * width + x];
                let dec = decoded_pixels[y * width + x];
                print!("{:3} ", dec);
                if orig != dec {
                    errors += 1;
                    max_err = max_err.max((orig as i32 - dec as i32).abs());
                }
            }
            println!();
        }
        
        if errors == 0 {
            println!("\n✅ PERFECT! Lossless encoding");
        } else {
            println!("\n❌ Errors: {}/64, Max error: {}", errors, max_err);
        }
    }
}

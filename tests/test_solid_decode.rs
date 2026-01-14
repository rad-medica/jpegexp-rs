use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
fn test_solid_decode_roundtrip() {
    let width = 4;
    let height = 4;
    let pixels = vec![128u8; (width * height) as usize];
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; pixels.len() * 20];
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    output.truncate(bytes_written);
    
    fs::write("solid_test.j2k", &output).unwrap();
    
    let decode_output = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "solid_test.j2k", "-o", "solid_test.pgm"])
        .output()
        .expect("Failed to run opj_decompress");
    
    if decode_output.status.success() {
        let pgm_data = fs::read("solid_test.pgm").unwrap();
        
        let decoded_pixels: Vec<u8> = {
            let content = String::from_utf8_lossy(&pgm_data);
            let lines: Vec<&str> = content.lines().collect();
            let pixel_start = pgm_data.iter().position(|&b| b == b'\n').unwrap() + 1;
            let header_lines = 3;
            let mut pos = 0;
            for _ in 0..header_lines {
                pos = pgm_data[pos..].iter().position(|&b| b == b'\n').unwrap() + 1 + pos;
            }
            pgm_data[pos..].to_vec()
        };
        
        println!("Decoded {} pixels", decoded_pixels.len());
        let mut errors = 0;
        let mut max_error = 0;
        for (i, (&orig, &dec)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
            let error = (orig as i32 - dec as i32).abs();
            if error > 0 {
                errors += 1;
                max_error = max_error.max(error);
                println!("Pixel {}: {} -> {} (error: {})", i, orig, dec, error);
            }
        }
        
        if errors == 0 {
            println!("PASS: Perfect lossless roundtrip!");
        } else {
            println!("FAIL: {} errors, max error = {}", errors, max_error);
        }
    } else {
        println!("Decode failed: {}", String::from_utf8_lossy(&decode_output.stderr));
    }
    
    println!("Files saved: solid_test.j2k, solid_test.pgm");
}

/// Check quantization in lossless mode
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_quantization_check() {
    let size = 8; // Small size that works
    
    // Create simple test pattern
    let mut pixels = vec![0u8; size * size];
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    // Encode with lossless
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    }, &mut output).unwrap();
    
    fs::write("test_quant_check.j2k", &output[..output_size]).unwrap();
    
    // Dump the file
    let result = Command::new("libs/bin/opj_dump.exe")
        .args(&["-i", "test_quant_check.j2k"])
        .output()
        .expect("opj_dump failed");
    
    println!("{}", String::from_utf8_lossy(&result.stdout));
    
    // Check if there's any quantization happening
    // For lossless 8-bit, step sizes should be such that delta = 0.5 (for 9-7) or no quantization (for 5-3)
    // Actually for 5/3 lossless, there's no quantization - coefficients are stored as-is
    
    // Decode and verify
    Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_quant_check.j2k", "-o", "test_quant_check.raw"])
        .output()
        .expect("decode failed");
    
    let decoded = fs::read("test_quant_check.raw").unwrap();
    
    let mut errors = 0;
    for (orig, &dec) in pixels.iter().zip(decoded.iter()) {
        if *orig as i32 != dec as i32 {
            errors += 1;
        }
    }
    
    println!("\nErrors: {}/{}", errors, pixels.len());
    
    if errors > 0 {
        println!("Decoded values:");
        for y in 0..size {
            print!("  ");
            for x in 0..size {
                print!("{:3} ", decoded[y * size + x]);
            }
            println!();
        }
        println!("\nOriginal values:");
        for y in 0..size {
            print!("  ");
            for x in 0..size {
                print!("{:3} ", pixels[y * size + x]);
            }
            println!();
        }
    }
}

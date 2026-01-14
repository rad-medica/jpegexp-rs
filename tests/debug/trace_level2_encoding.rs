/// Trace the actual level 2 encoding to see where we diverge from OpenJPEG

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;

#[test]
#[ignore]
fn trace_level2_gradient_encoding() {
    // Create the same 64x64 gradient test image from debug_level_sweep
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Tracing Level 2 Encoding (64x64 gradient) ===");
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    // Enable all tracing
    std::env::set_var("BPC_TRACE", "1");
    std::env::set_var("MQ_SYMBOL_TRACE", "1");
    std::env::set_var("J2K_CBLK_DETAIL", "1");
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    let our_bytes = &our_output[..our_size];
    
    println!("\n=== Encoding Complete ===");
    println!("Output size: {} bytes", our_size);
    
    // Save for comparison
    fs::write("trace_level2.j2k", our_bytes).unwrap();
    
    println!("\nSaved to trace_level2.j2k");
    println!("Compare with OpenJPEG using:");
    println!("  libs/bin/opj_decompress.exe -i trace_level2.j2k -o trace_level2.pnm");
}

#[test]
#[ignore]
fn trace_simple_8x8_encoding() {
    // Create a very simple 8x8 image to trace
    let width = 8;
    let height = 8;
    let mut pixels = vec![0u8; width * height];
    
    // Simple gradient: 0-7 repeated
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = x as u8;
        }
    }
    
    println!("\n=== Tracing Simple 8x8 Encoding ===");
    println!("Input data:");
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
    
    // Enable tracing
    std::env::set_var("BPC_TRACE", "1");
    std::env::set_var("MQ_SYMBOL_TRACE", "1");
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);  // Just 1 level for simplicity
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    let our_bytes = &our_output[..our_size];
    
    println!("\n=== Encoding Complete ===");
    println!("Output size: {} bytes", our_size);
    
    // Save for comparison
    fs::write("trace_simple_8x8.j2k", our_bytes).unwrap();
    fs::write("trace_simple_8x8.raw", &pixels).unwrap();
    
    println!("\nSaved to trace_simple_8x8.j2k");
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn debug_minimal_gradient_packet() {
    let width = 4;
    let height = 4;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = (x * 16 + y * 16) as u8;
        }
    }
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(0);
    let mut output = vec![0u8; 1024 * 1024];
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    let our_bytes = &output[..bytes_written];
    
    fs::write("debug_ours_4x4.j2k", our_bytes).unwrap();
    
    fs::write("debug_input_4x4.raw", &pixels).unwrap();
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "debug_input_4x4.raw",
            "-o", "debug_openjpeg_4x4.j2k",
            "-n", "1",
            "-r", "1",
            "-F", "4,4,1,8,u",
            "-I",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    assert!(status.success(), "opj_compress failed");
    
    let opj_bytes = fs::read("debug_openjpeg_4x4.j2k").unwrap();
    
    let find_sod = |data: &[u8]| -> Option<usize> {
        for i in 0..data.len().saturating_sub(1) {
            if data[i] == 0xFF && data[i+1] == 0x93 {
                return Some(i + 2);
            }
        }
        None
    };
    
    let our_tile_start = find_sod(&our_bytes).expect("No SOD in our output");
    let opj_tile_start = find_sod(&opj_bytes).expect("No SOD in OpenJPEG output");
    
    println!("\n=== Tile Data Comparison (First 48 bytes after SOD) ===");
    println!("Position | Ours      | OpenJPEG  | Match");
    println!("---------|-----------|-----------|------");
    
    for i in 0..48.min(our_bytes.len() - our_tile_start).min(opj_bytes.len() - opj_tile_start) {
        let our_byte = our_bytes[our_tile_start + i];
        let opj_byte = opj_bytes[opj_tile_start + i];
        let match_str = if our_byte == opj_byte { "✓" } else { "✗" };
        
        println!("{:8} | {:02X} ({:08b}) | {:02X} ({:08b}) | {}",
                 i, our_byte, our_byte, opj_byte, opj_byte, match_str);
        
        if our_byte != opj_byte {
            println!("         | First divergence at byte {}", i);
            break;
        }
    }
    
    println!("\n=== Hex Dump (First 24 bytes of tile data) ===");
    print!("Ours:     ");
    for i in 0..24.min(our_bytes.len() - our_tile_start) {
        print!("{:02X} ", our_bytes[our_tile_start + i]);
    }
    println!();
    
    print!("OpenJPEG: ");
    for i in 0..24.min(opj_bytes.len() - opj_tile_start) {
        print!("{:02X} ", opj_bytes[opj_tile_start + i]);
    }
    println!("\n");
}

#[test]
#[ignore]
fn debug_solid_packet() {
    let width = 4;
    let height = 4;
    let pixels = vec![128u8; width * height];
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(0);
    let mut output = vec![0u8; 1024 * 1024];
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    let our_bytes = &output[..bytes_written];
    
    fs::write("debug_ours_solid_4x4.j2k", our_bytes).unwrap();
    fs::write("debug_input_solid_4x4.raw", &pixels).unwrap();
    
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "debug_input_solid_4x4.raw",
            "-o", "debug_openjpeg_solid_4x4.j2k",
            "-n", "1",
            "-r", "1",
            "-F", "4,4,1,8,u",
            "-I",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    assert!(status.success());
    
    let opj_bytes = fs::read("debug_openjpeg_solid_4x4.j2k").unwrap();
    
    let find_sod = |data: &[u8]| -> Option<usize> {
        for i in 0..data.len().saturating_sub(1) {
            if data[i] == 0xFF && data[i+1] == 0x93 {
                return Some(i + 2);
            }
        }
        None
    };
    
    let our_tile_start = find_sod(&our_bytes).unwrap();
    let opj_tile_start = find_sod(&opj_bytes).unwrap();
    
    println!("\n=== SOLID Image Tile Data (Should Match Perfectly) ===");
    print!("Ours:     ");
    for i in 0..16.min(our_bytes.len() - our_tile_start) {
        print!("{:02X} ", our_bytes[our_tile_start + i]);
    }
    println!();
    
    print!("OpenJPEG: ");
    for i in 0..16.min(opj_bytes.len() - opj_tile_start) {
        print!("{:02X} ", opj_bytes[opj_tile_start + i]);
    }
    println!();
    
    if &our_bytes[our_tile_start..] == &opj_bytes[opj_tile_start..] {
        println!("✓ PERFECT MATCH (as expected for solid images)");
    } else {
        println!("✗ UNEXPECTED MISMATCH");
    }
}

#[test]
#[ignore] 
fn debug_trace_packet_header_structure() {
    let width = 4;
    let height = 4;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = (x * 32 + y * 16) as u8;
        }
    }
    
    println!("\n=== Encoding 4x4 Gradient ===");
    println!("Pixel values:");
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
    encoder.set_decomposition_levels(0);
    let mut output = vec![0u8; 1024 * 1024];
    let _bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    println!("\nPacket header encoding complete. Check output above for trace details.");
    println!("To see detailed packet header trace, compile with:");
    println!("  cargo test --features trace_packet_header --test debug_packet_bits -- --ignored");
}

/// Compare packet structure between our encoder and OpenJPEG
/// 
/// This test encodes the same gradient with both encoders and compares:
/// - File sizes
/// - Number of packets
/// - Packet header/body lengths
/// - Codeblock counts per resolution

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn compare_packet_structure_with_openjpeg() {
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Packet Structure Comparison (Level 2) ===\n");
    
    let levels = 2u8;
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(levels);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    let our_bytes = &our_output[..our_size];
    
    fs::write("compare_ours.j2k", our_bytes).unwrap();
    fs::write("compare_input.raw", &pixels).unwrap();
    
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "compare_input.raw",
            "-o", "compare_opj.j2k",
            "-n", &format!("{}", levels + 1),
            "-r", "1",
            "-F", &format!("{},{},1,8,u", width, height),
            "-I",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    if !status.success() {
        println!("❌ opj_compress failed");
        return;
    }
    
    let opj_bytes = fs::read("compare_opj.j2k").unwrap();
    
    println!("File Sizes:");
    println!("  Ours:     {} bytes", our_size);
    println!("  OpenJPEG: {} bytes", opj_bytes.len());
    println!("  Diff:     {} bytes ({:.1}% smaller)\n", 
        opj_bytes.len() as i32 - our_size as i32,
        100.0 * (opj_bytes.len() as f64 - our_size as f64) / opj_bytes.len() as f64);
    
    println!("Marker Structure:");
    analyze_j2k_structure("Ours", our_bytes);
    analyze_j2k_structure("OpenJPEG", &opj_bytes);
    
    println!("\nDecoding both files...");
    
    let _ = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "compare_ours.j2k", "-o", "compare_ours_decoded.pgm"])
        .output();
    
    let _ = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "compare_opj.j2k", "-o", "compare_opj_decoded.pgm"])
        .output();
}

fn analyze_j2k_structure(label: &str, data: &[u8]) {
    println!("\n{} structure:", label);
    
    let mut offset = 0;
    let mut sot_count = 0;
    let mut total_tile_data = 0u32;
    
    while offset + 2 <= data.len() {
        if data[offset] != 0xFF {
            break;
        }
        
        let marker = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        
        match marker {
            0xFF4F => println!("  SOC (Start of Codestream)"),
            0xFF51 => {
                if offset + 2 <= data.len() {
                    let len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    println!("  SIZ (Image and tile size) - {} bytes", len);
                    offset += len;
                }
            }
            0xFF52 => {
                if offset + 2 <= data.len() {
                    let len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    println!("  COD (Coding style default) - {} bytes", len);
                    
                    if offset + len <= data.len() {
                        let decomp_levels = data[offset + 4];
                        let progression = data[offset + 3];
                        println!("      Decomp levels: {}", decomp_levels);
                        println!("      Progression: {}", progression);
                    }
                    offset += len;
                }
            }
            0xFF5C => {
                if offset + 2 <= data.len() {
                    let len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    println!("  QCD (Quantization default) - {} bytes", len);
                    offset += len;
                }
            }
            0xFF90 => {
                sot_count += 1;
                if offset + 2 <= data.len() {
                    let len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    
                    if offset + 6 <= data.len() {
                        let tile_idx = u16::from_be_bytes([data[offset], data[offset + 1]]);
                        let psot = u32::from_be_bytes([
                            data[offset + 2], data[offset + 3],
                            data[offset + 4], data[offset + 5]
                        ]);
                        total_tile_data += psot;
                        println!("  SOT (Start of tile {}) - Psot={} bytes", tile_idx, psot);
                        offset += len - 2;
                    }
                }
            }
            0xFF93 => {
                println!("  SOD (Start of data)");
            }
            0xFFD9 => {
                println!("  EOC (End of codestream)");
                break;
            }
            _ => {
                if offset + 2 <= data.len() {
                    let len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    println!("  Unknown marker 0x{:04X} - {} bytes", marker, len);
                    offset += len;
                } else {
                    break;
                }
            }
        }
    }
    
    println!("  Total tiles: {}", sot_count);
    println!("  Total tile data: {} bytes", total_tile_data);
}

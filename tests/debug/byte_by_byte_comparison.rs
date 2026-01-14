use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[path = "../common/mod.rs"]
mod common;
use common::file_io::get_test_output_path;

#[test]
#[ignore]
fn byte_by_byte_comparison_level2() {
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Byte-by-Byte Comparison (Level 2) ===\n");
    
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
    
    let j2k_path = get_test_output_path("bytecmp_ours.j2k");
    let raw_path = get_test_output_path("bytecmp_input.raw");
    let opj_path = get_test_output_path("bytecmp_opj.j2k");

    fs::write(&j2k_path, our_bytes).unwrap();
    fs::write(&raw_path, &pixels).unwrap();
    
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", raw_path.to_str().unwrap(),
            "-o", opj_path.to_str().unwrap(),
            "-n", "3",
            "-r", "1",
            "-F", "64,64,1,8,u",
            "-I",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    assert!(status.success());
    
    let opj_bytes = fs::read(&opj_path).unwrap();
    
    println!("Our size:     {} bytes", our_size);
    println!("OpenJPEG size: {} bytes", opj_bytes.len());
    println!("Difference:    {} bytes\n", opj_bytes.len() as i32 - our_size as i32);
    
    let min_len = our_size.min(opj_bytes.len());
    let mut offset = 0;
    let mut marker_start = 0;
    
    while offset < min_len {
        if our_bytes[offset] == 0xFF && offset + 1 < min_len {
            if our_bytes[offset + 1] != 0xFF {
                marker_start = offset;
                let marker = u16::from_be_bytes([our_bytes[offset], our_bytes[offset + 1]]);
                
                let our_marker = marker;
                let opj_marker = if offset + 1 < opj_bytes.len() {
                    u16::from_be_bytes([opj_bytes[offset], opj_bytes[offset + 1]])
                } else {
                    0
                };
                
                if our_marker != opj_marker {
                    println!("MARKER MISMATCH at offset {}: Ours=0x{:04X}, OpenJPEG=0x{:04X}", 
                        offset, our_marker, opj_marker);
                    break;
                }
                
                match marker {
                    0xFF4F => println!("[{}] SOC", offset),
                    0xFF51 => {
                        if offset + 2 <= min_len {
                            let our_len = u16::from_be_bytes([our_bytes[offset + 2], our_bytes[offset + 3]]);
                            let opj_len = u16::from_be_bytes([opj_bytes[offset + 2], opj_bytes[offset + 3]]);
                            println!("[{}] SIZ - Ours: {} bytes, OpenJPEG: {} bytes", offset, our_len, opj_len);
                            
                            if our_len != opj_len {
                                println!("  LENGTH MISMATCH!");
                            }
                            offset += 2 + our_len as usize;
                            continue;
                        }
                    }
                    0xFF52 => {
                        if offset + 2 <= min_len {
                            let our_len = u16::from_be_bytes([our_bytes[offset + 2], our_bytes[offset + 3]]);
                            let opj_len = u16::from_be_bytes([opj_bytes[offset + 2], opj_bytes[offset + 3]]);
                            println!("[{}] COD - Ours: {} bytes, OpenJPEG: {} bytes", offset, our_len, opj_len);
                            
                            if our_len != opj_len {
                                println!("  LENGTH MISMATCH!");
                                compare_bytes(&our_bytes[offset..offset+2+our_len as usize], 
                                            &opj_bytes[offset..offset+2+opj_len as usize], offset);
                            }
                            offset += 2 + our_len as usize;
                            continue;
                        }
                    }
                    0xFF5C => {
                        if offset + 2 <= min_len {
                            let our_len = u16::from_be_bytes([our_bytes[offset + 2], our_bytes[offset + 3]]);
                            let opj_len = u16::from_be_bytes([opj_bytes[offset + 2], opj_bytes[offset + 3]]);
                            println!("[{}] QCD - Ours: {} bytes, OpenJPEG: {} bytes", offset, our_len, opj_len);
                            
                            if our_len != opj_len {
                                println!("  LENGTH MISMATCH!");
                                println!("  Comparing QCD content:");
                                let our_end = (offset + 2 + our_len as usize).min(our_bytes.len());
                                let opj_end = (offset + 2 + opj_len as usize).min(opj_bytes.len());
                                compare_bytes(&our_bytes[offset..our_end], 
                                            &opj_bytes[offset..opj_end], offset);
                            }
                            offset += 2 + our_len as usize;
                            continue;
                        }
                    }
                    0xFF90 => {
                        if offset + 2 <= min_len {
                            let our_len = u16::from_be_bytes([our_bytes[offset + 2], our_bytes[offset + 3]]);
                            let opj_len = u16::from_be_bytes([opj_bytes[offset + 2], opj_bytes[offset + 3]]);
                            
                            let our_psot = if offset + 6 <= min_len {
                                u32::from_be_bytes([our_bytes[offset+6], our_bytes[offset+7], 
                                                   our_bytes[offset+8], our_bytes[offset+9]])
                            } else { 0 };
                            let opj_psot = if offset + 6 <= opj_bytes.len() {
                                u32::from_be_bytes([opj_bytes[offset+6], opj_bytes[offset+7], 
                                                   opj_bytes[offset+8], opj_bytes[offset+9]])
                            } else { 0 };
                            
                            println!("[{}] SOT - Ours: Lsot={}, Psot={}, OpenJPEG: Lsot={}, Psot={}", 
                                offset, our_len, our_psot, opj_len, opj_psot);
                            
                            offset += 2 + our_len as usize;
                            continue;
                        }
                    }
                    0xFF93 => {
                        println!("[{}] SOD - Tile data starts here", offset);
                        println!("\nTile data comparison:");
                        println!("  Our remaining:     {} bytes", our_bytes.len() - offset - 2);
                        println!("  OpenJPEG remaining: {} bytes", opj_bytes.len() - offset - 2);
                        
                        let our_tile_data = &our_bytes[offset + 2..];
                        let opj_tile_data = &opj_bytes[offset + 2..];
                        
                        println!("\nFirst 64 bytes of tile data:");
                        println!("Ours:");
                        print_hex(&our_tile_data[..64.min(our_tile_data.len())]);
                        println!("\nOpenJPEG:");
                        print_hex(&opj_tile_data[..64.min(opj_tile_data.len())]);
                        
                        break;
                    }
                    _ => {
                        if offset + 2 <= min_len {
                            let our_len = u16::from_be_bytes([our_bytes[offset + 2], our_bytes[offset + 3]]);
                            println!("[{}] 0x{:04X} - {} bytes", offset, marker, our_len);
                            offset += 2 + our_len as usize;
                            continue;
                        }
                    }
                }
                offset += 2;
            } else {
                offset += 1;
            }
        } else {
            offset += 1;
        }
    }
}

fn compare_bytes(ours: &[u8], opj: &[u8], base_offset: usize) {
    let min_len = ours.len().min(opj.len());
    for i in 0..min_len {
        if ours[i] != opj[i] {
            println!("  [{}+{}] Ours: 0x{:02X}, OpenJPEG: 0x{:02X} DIFF", 
                base_offset, i, ours[i], opj[i]);
        }
    }
    if ours.len() != opj.len() {
        println!("  Length difference: Ours={}, OpenJPEG={}", ours.len(), opj.len());
    }
}

fn print_hex(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("  {:04X}: ", i * 16);
        for byte in chunk {
            print!("{:02X} ", byte);
        }
        println!();
    }
}

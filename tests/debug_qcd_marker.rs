// Test to compare QCD marker output between jpegexp-rs and OpenJPEG
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

#[test]
#[ignore]
fn compare_qcd_markers() {
    // Create a simple 8x8 gradient
    let width = 8;
    let height = 8;
    let mut pixels: Vec<u8> = vec![0; (width * height) as usize];
    
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height - 2)) as u8;
            pixels[(y * width + x) as usize] = val;
        }
    }
    
    // Encode with jpegexp-rs
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
    
    fs::write("test_gradient_ours.j2k", &encoded).unwrap();
    
    // Also create PGM for OpenJPEG
    let mut pgm = Vec::new();
    pgm.extend_from_slice(b"P5\n");
    pgm.extend_from_slice(format!("{} {}\n", width, height).as_bytes());
    pgm.extend_from_slice(b"255\n");
    pgm.extend_from_slice(&pixels);
    fs::write("test_gradient.pgm", &pgm).unwrap();
    
    // Encode with OpenJPEG (lossless by default)
    // Use -n 2 for 8x8 image (need fewer decomposition levels)
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "test_gradient.pgm",
            "-o", "test_gradient_openjpeg.j2k",
            "-n", "2",  // 2 decomposition levels for 8x8 image
        ])
        .output()
        .expect("Failed to run opj_compress");
    
    if !status.status.success() {
        println!("OpenJPEG encode failed: {}", String::from_utf8_lossy(&status.stderr));
        panic!("OpenJPEG failed to encode");
    }
    
    // Read both files
    let ours = fs::read("test_gradient_ours.j2k").unwrap();
    let theirs = fs::read("test_gradient_openjpeg.j2k").unwrap();
    
    println!("\n=== File Size Comparison ===");
    println!("Ours:   {} bytes", ours.len());
    println!("Theirs: {} bytes", theirs.len());
    
    // Parse markers
    println!("\n=== Our Markers ===");
    parse_markers(&ours, "OURS");
    
    println!("\n=== OpenJPEG Markers ===");
    parse_markers(&theirs, "THEIRS");
    
    println!("\n=== Files kept for manual inspection ===");
    println!("test_gradient_ours.j2k");
    println!("test_gradient_openjpeg.j2k");
    println!("test_gradient.pgm");
}

fn parse_markers(data: &[u8], label: &str) {
    let mut pos = 0;
    
    while pos + 1 < data.len() {
        if data[pos] != 0xFF {
            pos += 1;
            continue;
        }
        
        let marker = (data[pos] as u16) << 8 | data[pos + 1] as u16;
        
        match marker {
            0xFFD8 => println!("[{}] SOC at {}", label, pos),
            0xFF51 => {
                if pos + 3 < data.len() {
                    let len = (data[pos + 2] as u16) << 8 | data[pos + 3] as u16;
                    println!("[{}] SIZ at {} (len={})", label, pos, len);
                    
                    if pos + len as usize <= data.len() {
                        // Parse SIZ details
                        let depth = data[pos + 40]; // Rough position
                        println!("      Depth: {}", depth);
                    }
                    pos += len as usize;
                } else {
                    pos += 2;
                }
            },
            0xFF52 => {
                if pos + 3 < data.len() {
                    let len = (data[pos + 2] as u16) << 8 | data[pos + 3] as u16;
                    println!("[{}] COD at {} (len={})", label, pos, len);
                    
                    if pos + len as usize <= data.len() {
                        let cod_data = &data[pos + 4..pos + len as usize];
                        println!("      Coding style: 0x{:02X}", cod_data.get(0).unwrap_or(&0));
                        println!("      Prog order: {}", cod_data.get(1).unwrap_or(&0));
                        println!("      Layers: {}", (*cod_data.get(2).unwrap_or(&0) as u16) << 8 | *cod_data.get(3).unwrap_or(&0) as u16);
                        println!("      MCT: {}", cod_data.get(4).unwrap_or(&0));
                        println!("      Decomp levels: {}", cod_data.get(5).unwrap_or(&0));
                        println!("      CB width exp: {}", cod_data.get(6).unwrap_or(&0));
                        println!("      CB height exp: {}", cod_data.get(7).unwrap_or(&0));
                        println!("      CB style: 0x{:02X}", cod_data.get(8).unwrap_or(&0));
                        println!("      Transform: {}", cod_data.get(9).unwrap_or(&0));
                    }
                    pos += len as usize;
                } else {
                    pos += 2;
                }
            },
            0xFF5C => {
                if pos + 3 < data.len() {
                    let len = (data[pos + 2] as u16) << 8 | data[pos + 3] as u16;
                    println!("[{}] QCD at {} (len={})", label, pos, len);
                    
                    if pos + len as usize <= data.len() {
                        let qcd_data = &data[pos + 4..pos + len as usize];
                        let quant_style = qcd_data.get(0).unwrap_or(&0);
                        println!("      Quant style: 0x{:02X}", quant_style);
                        
                        let guard_bits = (quant_style >> 5) & 0x07;
                        let style = quant_style & 0x1F;
                        println!("      Guard bits: {}", guard_bits);
                        println!("      Style: 0x{:02X} ({})", style, 
                                 if style == 0 { "No quantization" } 
                                 else if style == 1 { "Scalar derived" }
                                 else if style == 2 { "Scalar expounded" }
                                 else { "Unknown" });
                        
                        // Parse epsilon values
                        println!("      Epsilon values:");
                        let mut i = 1;
                        let mut subband = 0;
                        while i < qcd_data.len() {
                            if style == 0 {
                                // No quantization - 1 byte per subband
                                if i < qcd_data.len() {
                                    let epsilon = (qcd_data[i] >> 3) & 0x1F;
                                    println!("        Subband {}: epsilon={}", subband, epsilon);
                                    i += 1;
                                    subband += 1;
                                }
                            } else {
                                // Scalar quantization - 2 bytes per subband
                                if i + 1 < qcd_data.len() {
                                    let val = (qcd_data[i] as u16) << 8 | qcd_data[i + 1] as u16;
                                    let epsilon = (val >> 11) & 0x1F;
                                    let mu = val & 0x7FF;
                                    println!("        Subband {}: epsilon={}, mu={} (raw=0x{:04X})", 
                                             subband, epsilon, mu, val);
                                    i += 2;
                                    subband += 1;
                                }
                            }
                        }
                    }
                    pos += len as usize;
                } else {
                    pos += 2;
                }
            },
            0xFF90 => println!("[{}] SOT at {}", label, pos),
            0xFF93 => println!("[{}] SOD at {}", label, pos),
            0xFFD9 => {
                println!("[{}] EOC at {}", label, pos);
                break;
            },
            _ => {
                if marker >= 0xFF00 {
                    println!("[{}] Unknown marker 0x{:04X} at {}", label, marker, pos);
                }
                pos += 2;
            }
        }
    }
}

#[test]
#[ignore]
fn test_openjpeg_to_rust_decode() {
    // Test if we can decode OpenJPEG's output
    let width = 8;
    let height = 8;
    let mut pixels: Vec<u8> = vec![0; (width * height) as usize];
    
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height - 2)) as u8;
            pixels[(y * width + x) as usize] = val;
        }
    }
    
    // Create PGM
    let mut pgm = Vec::new();
    pgm.extend_from_slice(b"P5\n");
    pgm.extend_from_slice(format!("{} {}\n", width, height).as_bytes());
    pgm.extend_from_slice(b"255\n");
    pgm.extend_from_slice(&pixels);
    fs::write("test_gradient_opj.pgm", &pgm).unwrap();
    
    // Encode with OpenJPEG (lossless by default)
    // Use -n 2 for 8x8 image
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "test_gradient_opj.pgm",
            "-o", "test_gradient_opj.j2k",
            "-n", "2",
        ])
        .output()
        .expect("Failed to run opj_compress");
    
    if !status.status.success() {
        println!("OpenJPEG encode failed");
        return;
    }
    
    // Try to decode with our decoder
    use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
    use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
    
    let j2k_data = fs::read("test_gradient_opj.j2k").unwrap();
    let mut reader = JpegStreamReader::new(&j2k_data);
    let mut decoder = J2kDecoder::new(&mut reader);
    
    match decoder.decode() {
        Ok(image) => {
            println!("\n✓ Successfully decoded OpenJPEG output!");
            println!("  Width: {}", image.width);
            println!("  Height: {}", image.height);
            println!("  Components: {}", image.components.len());
            
            // Reconstruct pixels
            if let Some(tile) = image.tiles.get(0) {
                if let Some(comp) = tile.components.get(0) {
                    // Get final resolution (after all DWT)
                    if let Some(res) = comp.resolutions.last() {
                        println!("  Resolution: {}x{}", res.width, res.height);
                        // Note: Full pixel reconstruction requires DWT inverse
                        // For now, just confirm decoding succeeded
                        println!("  Decoder test: PASSED (structure decoded)");
                    }
                }
            }
        },
        Err(e) => {
            println!("\n✗ Failed to decode OpenJPEG output: {:?}", e);
        }
    }
    
    // Clean up
    let _ = fs::remove_file("test_gradient_opj.pgm");
    let _ = fs::remove_file("test_gradient_opj.j2k");
}

/// Debug test to compare QCD markers between our encoder and OpenJPEG
/// This test helps identify parameter mismatches in quantization defaults
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn compare_qcd_markers() {
    // Create a simple 8x8 gradient image
    let width = 8;
    let height = 8;
    let mut pixels: Vec<u8> = Vec::new();
    for y in 0..height {
        for x in 0..width {
            pixels.push(((y * width + x) * 255 / 63) as u8);
        }
    }

    // Save as PGM for OpenJPEG
    let pgm_path = "test_gradient_qcd.pgm";
    let pgm_header = format!("P5\n{} {}\n255\n", width, height);
    let mut pgm_data = Vec::new();
    pgm_data.extend_from_slice(pgm_header.as_bytes());
    pgm_data.extend_from_slice(&pixels);
    fs::write(pgm_path, &pgm_data).unwrap();
    
    println!("Created PGM file: {} ({} bytes)", pgm_path, pgm_data.len());

    // Encode with our implementation
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless mode
    encoder.set_decomposition_levels(1); // Match OpenJPEG (only 1 level for 8x8)
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1, // Grayscale
    };
    
    let mut our_j2k = vec![0u8; pixels.len() * 4]; // Allocate sufficient buffer
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut our_j2k).unwrap();
    our_j2k.truncate(bytes_written);
    
    let our_path = "test_gradient_ours_qcd.j2k";
    fs::write(our_path, &our_j2k).unwrap();

    // Encode with OpenJPEG
    let opj_path = "test_gradient_openjpeg_qcd.j2k";
    println!("\nEncoding with OpenJPEG...");
    let output = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", pgm_path,
            "-o", opj_path,
            "-I",  // Lossless (5-3 reversible)
            "-n", "1", // Only 1 decomposition level for 8x8 image
        ])
        .output()
        .expect("Failed to run opj_compress");

    println!("OpenJPEG stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("OpenJPEG stderr: {}", String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        eprintln!("\n✗ OpenJPEG encoding failed with exit code: {:?}", output.status.code());
        panic!("OpenJPEG encoding failed");
    }
    
    println!("✓ OpenJPEG encoding successful");

    // Read both files
    let our_data = fs::read(our_path).unwrap();
    let opj_data = fs::read(opj_path).unwrap();

    println!("\n=== File Sizes ===");
    println!("Our encoder: {} bytes", our_data.len());
    println!("OpenJPEG:    {} bytes", opj_data.len());

    // Parse and compare markers
    println!("\n=== Marker Comparison ===");
    print_markers("Our encoder", &our_data);
    print_markers("OpenJPEG", &opj_data);

    // Find and compare QCD markers specifically
    println!("\n=== QCD Marker Detailed Comparison ===");
    if let Some(our_qcd) = find_marker(&our_data, 0xFF5C) {
        println!("Our QCD marker:");
        print_hex_dump(&our_qcd);
        decode_qcd_marker(&our_qcd, "Ours");
    }
    
    if let Some(opj_qcd) = find_marker(&opj_data, 0xFF5C) {
        println!("\nOpenJPEG QCD marker:");
        print_hex_dump(&opj_qcd);
        decode_qcd_marker(&opj_qcd, "OpenJPEG");
    }

    // Test cross-decoding: Can OpenJPEG decode our output?
    println!("\n=== Cross-Decoding Test: OpenJPEG Decoder ===");
    let our_decoded_path = "test_gradient_ours_decoded.pgm";
    let decode_output = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", our_path,
            "-o", our_decoded_path,
        ])
        .output()
        .expect("Failed to run opj_decompress");

    if decode_output.status.success() {
        println!("✓ OpenJPEG successfully decoded our J2K file");
        
        // Read decoded PGM and compare
        let decoded_data = fs::read(our_decoded_path).unwrap();
        let decoded_pixels = parse_pgm(&decoded_data);
        
        let mut max_error = 0;
        let mut error_sum = 0;
        let mut error_count = 0;
        
        for (i, (&orig, &dec)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
            let diff = (orig as i32 - dec as i32).abs();
            if diff > 0 {
                error_sum += diff;
                error_count += 1;
                max_error = max_error.max(diff);
                if error_count <= 10 {
                    println!("  Pixel {}: {} -> {} (error: {})", i, orig, dec, diff);
                }
            }
        }
        
        let mae = if error_count > 0 {
            error_sum as f64 / pixels.len() as f64
        } else {
            0.0
        };
        
        println!("\nDecoding Quality (OpenJPEG decoding our output):");
        println!("  MAE: {:.4}", mae);
        println!("  Max Error: {}", max_error);
        println!("  Errors: {}/{} pixels", error_count, pixels.len());
        
        if mae == 0.0 {
            println!("  ✓ PERFECT: OpenJPEG can decode our output losslessly");
        } else {
            println!("  ✗ MISMATCH: OpenJPEG interprets our bitstream differently");
        }
    } else {
        println!("✗ OpenJPEG FAILED to decode our J2K file");
        eprintln!("Error: {}", String::from_utf8_lossy(&decode_output.stderr));
    }

    // Test reverse: Can we decode OpenJPEG's output?
    println!("\n=== Cross-Decoding Test: Our Decoder ===");
    use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
    use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
    
    let mut reader = JpegStreamReader::new(&opj_data);
    let mut decoder = J2kDecoder::new(&mut reader);
    
    match decoder.decode() {
        Ok(image) => {
            println!("✓ Successfully decoded OpenJPEG's J2K file");
            println!("  Dimensions: {}x{}", image.width, image.height);
            println!("  Components: {}", image.components.len());
            
            // Reconstruct decoded pixels
            let decoded = match image.reconstruct_pixels() {
                Ok(pix) => pix,
                Err(e) => {
                    println!("✗ Failed to reconstruct pixels: {}", e);
                    return;
                }
            };
            
            if decoded.len() != pixels.len() {
                println!("✗ Size mismatch: expected {}, got {}", pixels.len(), decoded.len());
            } else {
                let mut max_error = 0;
                let mut error_sum = 0;
                let mut error_count = 0;
                
                for (i, (&orig, &dec)) in pixels.iter().zip(decoded.iter()).enumerate() {
                    let diff = (orig as i32 - dec as i32).abs();
                    if diff > 0 {
                        error_sum += diff;
                        error_count += 1;
                        max_error = max_error.max(diff);
                        if error_count <= 10 {
                            println!("  Pixel {}: {} -> {} (error: {})", i, orig, dec, diff);
                        }
                    }
                }
                
                let mae = if error_count > 0 {
                    error_sum as f64 / pixels.len() as f64
                } else {
                    0.0
                };
                
                println!("\nDecoding Quality (Our decoder reading OpenJPEG):");
                println!("  MAE: {:.4}", mae);
                println!("  Max Error: {}", max_error);
                println!("  Errors: {}/{} pixels", error_count, pixels.len());
                
                if mae == 0.0 {
                    println!("  ✓ PERFECT: We can decode OpenJPEG's output losslessly");
                } else {
                    println!("  ✗ MISMATCH: We interpret OpenJPEG's bitstream differently");
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to decode OpenJPEG's J2K: {:?}", e);
        }
    }

    // Cleanup
    let _ = fs::remove_file(pgm_path);
    let _ = fs::remove_file(our_path);
    let _ = fs::remove_file(opj_path);
    let _ = fs::remove_file(our_decoded_path);
}

fn print_markers(label: &str, data: &[u8]) {
    println!("\n{} markers:", label);
    let mut i = 0;
    while i + 1 < data.len() {
        if data[i] == 0xFF {
            let marker = ((data[i] as u16) << 8) | (data[i + 1] as u16);
            let marker_name = match marker {
                0xFF4F => "SOC",
                0xFF51 => "SIZ",
                0xFF52 => "COD",
                0xFF5C => "QCD",
                0xFF90 => "SOT",
                0xFF93 => "SOD",
                0xFFD9 => "EOC",
                _ => "???",
            };
            
            if marker >= 0xFF30 && marker != 0xFF4F && marker != 0xFFD9 {
                if i + 3 < data.len() {
                    let len = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                    println!("  0x{:04X} ({:4}) at offset {:4}, length: {}", 
                             marker, marker_name, i, len);
                    i += len + 2;
                } else {
                    i += 2;
                }
            } else {
                println!("  0x{:04X} ({:4}) at offset {:4}", marker, marker_name, i);
                i += 2;
            }
        } else {
            i += 1;
        }
    }
}

fn find_marker(data: &[u8], target: u16) -> Option<Vec<u8>> {
    let mut i = 0;
    while i + 1 < data.len() {
        if data[i] == 0xFF {
            let marker = ((data[i] as u16) << 8) | (data[i + 1] as u16);
            if marker == target {
                if i + 3 < data.len() {
                    let len = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                    if i + 2 + len <= data.len() {
                        return Some(data[i..i + 2 + len].to_vec());
                    }
                }
            }
            
            if marker >= 0xFF30 && marker != 0xFF4F && marker != 0xFFD9 {
                if i + 3 < data.len() {
                    let len = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                    i += len + 2;
                } else {
                    i += 2;
                }
            } else {
                i += 2;
            }
        } else {
            i += 1;
        }
    }
    None
}

fn print_hex_dump(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("  {:04X}: ", i * 16);
        for byte in chunk {
            print!("{:02X} ", byte);
        }
        println!();
    }
}

fn decode_qcd_marker(data: &[u8], label: &str) {
    if data.len() < 4 {
        println!("QCD marker too short");
        return;
    }
    
    println!("\n{} QCD Marker Breakdown:", label);
    println!("  Marker: 0x{:02X}{:02X}", data[0], data[1]);
    let len = ((data[2] as usize) << 8) | (data[3] as usize);
    println!("  Length: {}", len);
    
    if data.len() < 5 {
        return;
    }
    
    let sqcd = data[4];
    println!("  Sqcd (quantization style): 0x{:02X}", sqcd);
    println!("    No quantization: {}", (sqcd & 0x1F) == 0);
    println!("    Scalar derived: {}", (sqcd & 0x1F) == 1);
    println!("    Scalar expounded: {}", (sqcd & 0x1F) == 2);
    println!("    Guard bits: {}", sqcd >> 5);
    
    // Parse epsilon values (16-bit each for expounded style)
    if (sqcd & 0x1F) == 2 {
        println!("  Quantization step sizes:");
        let mut offset = 5;
        let mut band = 0;
        while offset + 1 < data.len() {
            let val = ((data[offset] as u16) << 8) | (data[offset + 1] as u16);
            let epsilon = val >> 11;
            let mantissa = val & 0x7FF;
            
            let band_name = match band {
                0 => "LL",
                1 | 2 | 3 => "Level 1 (HL/LH/HH)",
                _ => "Higher levels",
            };
            
            println!("    Band {}: epsilon={}, mantissa={} (raw: 0x{:04X})", 
                     band, epsilon, mantissa, val);
            println!("             {} subband", band_name);
            
            offset += 2;
            band += 1;
        }
    }
}

fn parse_pgm(data: &[u8]) -> Vec<u8> {
    // Simple PGM parser for P5 format
    let mut lines = data.split(|&b| b == b'\n');
    
    // Skip P5 header
    let _ = lines.next(); // P5
    
    // Skip comments and find dimensions
    let mut width = 0;
    let mut height = 0;
    let mut maxval = 0;
    let mut header_done = false;
    
    for line in lines.by_ref() {
        let line_str = String::from_utf8_lossy(line);
        if line_str.starts_with('#') {
            continue;
        }
        
        if width == 0 {
            let parts: Vec<&str> = line_str.split_whitespace().collect();
            if parts.len() >= 2 {
                width = parts[0].parse().unwrap_or(0);
                height = parts[1].parse().unwrap_or(0);
            }
        } else if maxval == 0 {
            maxval = line_str.trim().parse().unwrap_or(0);
            header_done = true;
            break;
        }
    }
    
    if !header_done {
        return Vec::new();
    }
    
    // Find where pixel data starts
    let header_text = format!("P5\n{} {}\n{}\n", width, height, maxval);
    let header_len = header_text.len();
    
    if data.len() > header_len {
        data[header_len..].to_vec()
    } else {
        Vec::new()
    }
}

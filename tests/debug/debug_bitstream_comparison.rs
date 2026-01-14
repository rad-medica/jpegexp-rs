/// Deep bitstream comparison between our encoder and OpenJPEG
/// This test creates identical input, encodes with both, and compares byte-by-byte
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn compare_bitstream_detailed() {
    // Start with the simplest possible test case: 4x4 solid image
    // This should have minimal entropy and be easiest to debug
    let width = 4;
    let height = 4;
    let pixels = vec![128u8; (width * height) as usize]; // Solid gray
    
    println!("\n=== Testing {}x{} Solid Image (value=128) ===", width, height);
    encode_and_compare(&pixels, width, height, 1);
    
    // If solid works, try a simple gradient
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            pixels.push(((y * width + x) * 255 / 15) as u8);
        }
    }
    println!("\n=== Testing {}x{} Simple Gradient ===", width, height);
    encode_and_compare(&pixels, width, height, 1);
}

fn encode_and_compare(pixels: &[u8], width: u32, height: u32, decomp_levels: u8) {
    // Save as PGM for OpenJPEG
    let pgm_path = "test_bitstream.pgm";
    let pgm_header = format!("P5\n{} {}\n255\n", width, height);
    let mut pgm_data = Vec::new();
    pgm_data.extend_from_slice(pgm_header.as_bytes());
    pgm_data.extend_from_slice(pixels);
    fs::write(pgm_path, &pgm_data).unwrap();
    
    println!("Input pixels: {:?}", pixels);
    
    // Encode with our implementation
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(decomp_levels);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut our_j2k = vec![0u8; pixels.len() * 20];
    let bytes_written = encoder.encode(pixels, &frame_info, &mut our_j2k).unwrap();
    our_j2k.truncate(bytes_written);
    
    let our_path = "test_bitstream_ours.j2k";
    fs::write(our_path, &our_j2k).unwrap();
    
    let opj_path = "test_bitstream_openjpeg.j2k";
    let num_resolutions = decomp_levels + 1;
    let output = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", pgm_path,
            "-o", opj_path,
            "-n", &num_resolutions.to_string(),
        ])
        .output()
        .expect("Failed to run opj_compress");
    
    if !output.status.success() {
        eprintln!("OpenJPEG failed: {}", String::from_utf8_lossy(&output.stderr));
        panic!("OpenJPEG encoding failed");
    }
    
    let opj_data = fs::read(opj_path).unwrap();
    
    println!("\nFile sizes:");
    println!("  Ours:     {} bytes", our_j2k.len());
    println!("  OpenJPEG: {} bytes", opj_data.len());
    
    // Find SOD markers and compare tile data
    let our_sod_pos = find_marker_position(&our_j2k, 0xFF93);
    let opj_sod_pos = find_marker_position(&opj_data, 0xFF93);
    
    if let (Some(our_pos), Some(opj_pos)) = (our_sod_pos, opj_sod_pos) {
        println!("\nSOD marker positions:");
        println!("  Ours:     offset {}", our_pos);
        println!("  OpenJPEG: offset {}", opj_pos);
        
        // SOD marker is 2 bytes, data starts after it
        let our_tile_data = &our_j2k[our_pos + 2..];
        let opj_tile_data = &opj_data[opj_pos + 2..];
        
        println!("\nTile data sizes:");
        println!("  Ours:     {} bytes", our_tile_data.len());
        println!("  OpenJPEG: {} bytes", opj_tile_data.len());
        
        println!("\nTile data comparison (first 64 bytes):");
        println!("  Ours:     {}", hex_string(&our_tile_data[..our_tile_data.len().min(64)]));
        println!("  OpenJPEG: {}", hex_string(&opj_tile_data[..opj_tile_data.len().min(64)]));
        
        // Find first difference
        let min_len = our_tile_data.len().min(opj_tile_data.len());
        let mut first_diff = None;
        for i in 0..min_len {
            if our_tile_data[i] != opj_tile_data[i] {
                first_diff = Some(i);
                break;
            }
        }
        
        if let Some(pos) = first_diff {
            println!("\n✗ First difference at byte {}", pos);
            println!("  Context (ours):     {}", hex_string(&our_tile_data[pos.saturating_sub(4)..pos.saturating_add(8).min(our_tile_data.len())]));
            println!("  Context (OpenJPEG): {}", hex_string(&opj_tile_data[pos.saturating_sub(4)..pos.saturating_add(8).min(opj_tile_data.len())]));
            println!("  Ours byte:     0x{:02X} (0b{:08b})", our_tile_data[pos], our_tile_data[pos]);
            println!("  OpenJPEG byte: 0x{:02X} (0b{:08b})", opj_tile_data[pos], opj_tile_data[pos]);
        } else if our_tile_data.len() == opj_tile_data.len() {
            println!("\n✓ Tile data matches perfectly!");
        } else {
            println!("\n✗ Length mismatch (data matches up to {} bytes)", min_len);
        }
    }
    
    // Test if OpenJPEG can decode our output
    println!("\n=== Cross-validation ===");
    let our_decoded_path = "test_bitstream_ours_decoded.pgm";
    let decode_output = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", our_path,
            "-o", our_decoded_path,
        ])
        .output()
        .expect("Failed to run opj_decompress");
    
    if decode_output.status.success() {
        println!("✓ OpenJPEG can decode our file");
        let decoded_data = fs::read(our_decoded_path).unwrap();
        let decoded_pixels = parse_pgm(&decoded_data);
        
        let mut errors = 0;
        for (i, (&orig, &dec)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
            if orig != dec {
                println!("  Pixel {}: {} -> {} (error: {})", i, orig, dec, (orig as i32 - dec as i32).abs());
                errors += 1;
            }
        }
        
        if errors == 0 {
            println!("  ✓ Perfect lossless roundtrip");
        } else {
            println!("  ✗ {} errors", errors);
        }
    } else {
        println!("✗ OpenJPEG FAILED to decode our file");
        eprintln!("  Error: {}", String::from_utf8_lossy(&decode_output.stderr));
    }
    
    // Cleanup
    let _ = fs::remove_file(pgm_path);
    let _ = fs::remove_file(our_path);
    let _ = fs::remove_file(opj_path);
    let _ = fs::remove_file(our_decoded_path);
}

fn find_marker_position(data: &[u8], marker: u16) -> Option<usize> {
    let mut i = 0;
    while i + 1 < data.len() {
        if data[i] == 0xFF {
            let m = ((data[i] as u16) << 8) | (data[i + 1] as u16);
            if m == marker {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn hex_string(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_pgm(data: &[u8]) -> Vec<u8> {
    // Simple PGM parser
    let mut lines = data.split(|&b| b == b'\n');
    let _ = lines.next(); // P5
    
    // Find dimensions and maxval, skipping comments
    let mut width = 0;
    let mut height = 0;
    let mut maxval = 0;
    
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
            break;
        }
    }
    
    // Find pixel data start
    let header_text = format!("P5\n{} {}\n{}\n", width, height, maxval);
    let header_len = header_text.len();
    
    if data.len() > header_len {
        data[header_len..].to_vec()
    } else {
        Vec::new()
    }
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
fn trace_packet_header_bits() {
    // Create a 4x4 gradient image
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let frame_info = FrameInfo {
        width: 4,
        height: 4,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; 500];
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    output.truncate(bytes_written);
    
    fs::write("packet_bits_test.j2k", &output).unwrap();
    
    // Create OpenJPEG reference
    let pgm_header = "P5\n4 4\n255\n";
    let mut pgm_data = pgm_header.as_bytes().to_vec();
    pgm_data.extend_from_slice(&pixels);
    fs::write("packet_bits_test.pgm", &pgm_data).unwrap();
    
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&["-i", "packet_bits_test.pgm", "-o", "packet_bits_test_opj.j2k", "-n", "2"])
        .output();
    
    // Find SOD marker and extract tile data
    let our_sod = find_marker(&output, 0xFF93).unwrap();
    let our_tile = &output[our_sod + 2..];
    
    let opj_data = fs::read("packet_bits_test_opj.j2k").unwrap();
    let opj_sod = find_marker(&opj_data, 0xFF93).unwrap();
    let opj_tile = &opj_data[opj_sod + 2..];
    
    println!("=== Packet Structure Analysis ===\n");
    
    // Analyze packet structure
    println!("Expected structure:");
    println!("  Bytes 0-2: Resolution 0 packet header");
    println!("  Bytes 3-8: LL subband codeblock data (6 bytes)");
    println!("  Bytes 9-14: Resolution 1 packet header");
    println!("  Bytes 15+: HL, LH codeblock data");
    println!();
    
    println!("Our tile data ({} bytes):", our_tile.len());
    print_hex_annotated(our_tile);
    println!();
    
    println!("OpenJPEG tile data ({} bytes):", opj_tile.len());
    print_hex_annotated(opj_tile);
    println!();
    
    // Byte-by-byte comparison
    println!("=== Byte-by-byte comparison ===");
    let min_len = our_tile.len().min(opj_tile.len());
    let mut first_diff = None;
    
    for i in 0..min_len {
        let ours = our_tile[i];
        let opj = opj_tile[i];
        
        if ours != opj && first_diff.is_none() {
            first_diff = Some(i);
        }
        
        let status = if ours == opj { "✓" } else { "✗" };
        let annotation = match i {
            0..=2 => "Res0 header",
            3..=8 => "LL data",
            9..=14 => "Res1 header",
            _ => "HL/LH data",
        };
        
        println!("Byte {:2} [{}]: {:02X} vs {:02X} {} - {}", 
                 i, status, ours, opj, 
                 if ours == opj { "MATCH" } else { "DIFF" },
                 annotation);
        
        if ours != opj {
            println!("         Binary: {:08b} vs {:08b}", ours, opj);
            let xor = ours ^ opj;
            println!("         XOR:    {:08b} ({} bits differ)", xor, xor.count_ones());
        }
    }
    
    if let Some(diff_pos) = first_diff {
        println!("\n=== First divergence at byte {} ===", diff_pos);
        
        if diff_pos >= 9 && diff_pos <= 14 {
            println!("Divergence is in Resolution 1 packet header!");
            println!("This packet header encodes info for HL, LH, HH subbands.");
            println!();
            println!("Resolution 1 has 1x1 codeblock grid per subband (3 subbands).");
            println!("For each codeblock, the header writes:");
            println!("  1. Inclusion tag tree encoding");
            println!("  2. Zero bitplanes tag tree encoding");
            println!("  3. Number of coding passes");
            println!("  4. Lblock increment (comma code)");
            println!("  5. Data length (lblock + log2(passes) bits)");
        }
    }
    
    // Cleanup
    let _ = fs::remove_file("packet_bits_test.j2k");
    let _ = fs::remove_file("packet_bits_test.pgm");
    let _ = fs::remove_file("packet_bits_test_opj.j2k");
}

fn find_marker(data: &[u8], marker: u16) -> Option<usize> {
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == (marker >> 8) as u8 && data[i + 1] == (marker & 0xFF) as u8 {
            return Some(i);
        }
    }
    None
}

fn print_hex_annotated(data: &[u8]) {
    for (i, &byte) in data.iter().enumerate() {
        if i % 16 == 0 {
            if i > 0 { println!(); }
            print!("{:04X}: ", i);
        }
        print!("{:02X} ", byte);
    }
    println!();
}

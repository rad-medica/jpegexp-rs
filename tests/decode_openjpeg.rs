use std::fs;
use std::process::Command;

#[test]
fn decode_openjpeg_zero_bp() {
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    
    let pgm_header = "P5\n4 4\n255\n";
    let mut pgm_data = pgm_header.as_bytes().to_vec();
    pgm_data.extend_from_slice(&pixels);
    fs::write("decode_zero_bp.pgm", &pgm_data).unwrap();
    
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&["-i", "decode_zero_bp.pgm", "-o", "decode_zero_bp.j2k", "-n", "2"])
        .output();
    
    let opj_data = fs::read("decode_zero_bp.j2k").unwrap();
    
    let sod_pos = find_marker(&opj_data, 0xFF93).unwrap();
    let tile_data = &opj_data[sod_pos + 2..];
    
    println!("=== Decoding OpenJPEG packet headers ===\n");
    println!("Tile data ({} bytes total):", tile_data.len());
    print_hex(&tile_data[..16.min(tile_data.len())]);
    println!();
    
    println!("Expected structure:");
    println!("  - Resolution 0 packet:");
    println!("    - Header: encodes LL subband (1x1 CB grid)");
    println!("    - Body: 6 bytes of LL codeblock data");
    println!("  - Resolution 1 packet:");
    println!("    - Header: encodes HL, LH, HH subbands (each 1x1 CB grid)");
    println!("    - Body: HL (4 bytes) + LH (4 bytes) + HH (excluded)");
    println!();
    
    println!("Manual bit-by-bit decoding:");
    println!();
    
    println!("Byte 0: {:08b} = 0x{:02X}", tile_data[0], tile_data[0]);
    println!("  Bit 0 (MSB): {} - Packet not empty", tile_data[0] >> 7);
    println!("  Bits 1-7: Inclusion/zero_bp/num_passes/lblock/data_len for LL");
    println!();
    
    println!("For LL subband:");
    println!("  - From trace: max_bp=7, 22 passes, 6 bytes");
    println!("  - Expected zero_bp: ? (need to decode from tag tree)");
    println!("  - num_passes encoding for 22:");
    println!("    22 is in range 6..=36, so: 11 11 NNNNN where NNNNN = 22-6 = 16 = 10000");
    println!("    Full encoding: 11 11 10000 (9 bits)");
    println!();
    
    println!("Let's check what data_len encoding would be:");
    println!("  For 22 passes: log2(22) = floor(4.46) = 4");
    println!("  For 6 bytes: bits_needed = floor(log2(6)) + 1 = floor(2.58) + 1 = 3");
    println!("  increment = max(0, 3 - 3 - 4) = max(0, -4) = 0");
    println!("  lblock = 3 + 0 = 3");
    println!("  lbits = 3 + 4 = 7 bits");
    println!("  Comma code for 0: single bit '0'");
    println!("  Data length (6) in 7 bits: {:07b}", 6);
    println!();
    
    println!("Similarly for HL/LH (13 passes, 4 bytes):");
    println!("  log2(13) = 3");
    println!("  bits_needed = floor(log2(4)) + 1 = 3");
    println!("  increment = max(0, 3 - 3 - 3) = 0");
    println!("  lblock = 3");
    println!("  lbits = 6 bits");
    println!("  Data length (4) in 6 bits: {:06b}", 4);
    
    let _ = fs::remove_file("decode_zero_bp.pgm");
    let _ = fs::remove_file("decode_zero_bp.j2k");
}

fn find_marker(data: &[u8], marker: u16) -> Option<usize> {
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == (marker >> 8) as u8 && data[i + 1] == (marker & 0xFF) as u8 {
            return Some(i);
        }
    }
    None
}

fn print_hex(data: &[u8]) {
    for (i, &byte) in data.iter().enumerate() {
        if i % 16 == 0 && i > 0 {
            println!();
        }
        print!("{:02X} ", byte);
    }
    println!();
}

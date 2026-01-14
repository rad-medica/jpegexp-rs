use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
fn trace_packet_structure() {
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
    
    fs::write("packet_test.j2k", &output).unwrap();
    
    let pgm_header = "P5\n4 4\n255\n";
    let mut pgm_data = pgm_header.as_bytes().to_vec();
    pgm_data.extend_from_slice(&pixels);
    fs::write("packet_test.pgm", &pgm_data).unwrap();
    
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&["-i", "packet_test.pgm", "-o", "packet_test_opj.j2k", "-n", "2"])
        .output();
    
    println!("=== Full file comparison ===");
    
    let opj_data = fs::read("packet_test_opj.j2k").unwrap_or_default();
    
    let our_sod = find_marker(&output, 0xFF93);
    let opj_sod = find_marker(&opj_data, 0xFF93);
    
    if let (Some(our_pos), Some(opj_pos)) = (our_sod, opj_sod) {
        let our_tile = &output[our_pos + 2..];
        let opj_tile = &opj_data[opj_pos + 2..];
        
        println!("\nOur tile data ({} bytes, SOD at {}):", our_tile.len(), our_pos);
        print_hex(our_tile);
        
        println!("\nOpenJPEG tile data ({} bytes, SOD at {}):", opj_tile.len(), opj_pos);
        print_hex(opj_tile);
        
        println!("\nBit-by-bit comparison of first 20 bytes:");
        for i in 0..20.min(our_tile.len()).min(opj_tile.len()) {
            let ours = our_tile[i];
            let opj = opj_tile[i];
            if ours == opj {
                println!("Byte {:2}: {:02X} = {:02X} OK", i, ours, opj);
            } else {
                println!("Byte {:2}: {:02X} != {:02X} DIFF", i, ours, opj);
                println!("        {:08b} vs {:08b}", ours, opj);
                let diff = ours ^ opj;
                println!("        XOR: {:08b} (bits {} differ)", diff, diff.count_ones());
            }
        }
    }
    
    let _ = fs::remove_file("packet_test.j2k");
    let _ = fs::remove_file("packet_test.pgm");
    let _ = fs::remove_file("packet_test_opj.j2k");
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
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:04X}: ", i * 16);
        for b in chunk {
            print!("{:02X} ", b);
        }
        println!();
    }
}

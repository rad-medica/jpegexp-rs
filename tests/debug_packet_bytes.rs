use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use std::process::Command;

#[test]
fn compare_packet_bytes() {
    let gradient_4x4: Vec<u8> = (0..16).collect();
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(1);
    
    let our_j2k = encoder.encode(&gradient_4x4, 4, 4, 8, 1).unwrap();
    
    std::fs::write("debug_ours.j2k", &our_j2k).unwrap();
    
    std::fs::write("debug_gradient.pgm", format!("P5\n4 4\n255\n").as_bytes()).unwrap();
    std::fs::write("debug_gradient.pgm", &gradient_4x4).unwrap();
    
    let _ = Command::new("opj_compress")
        .args(&[
            "-i", "debug_gradient.pgm",
            "-o", "debug_openjpeg.j2k",
            "-p", "RLCP",
            "-t", "4,4",
        ])
        .output();
    
    if let Ok(opj_data) = std::fs::read("debug_openjpeg.j2k") {
        println!("\n=== TILE DATA COMPARISON ===");
        
        let our_sod_pos = our_j2k.windows(2).position(|w| w == [0xFF, 0x93]).map(|p| p + 2);
        let opj_sod_pos = opj_data.windows(2).position(|w| w == [0xFF, 0x93]).map(|p| p + 2);
        
        if let (Some(our_pos), Some(opj_pos)) = (our_sod_pos, opj_sod_pos) {
            let our_tile = &our_j2k[our_pos..our_pos.saturating_add(30).min(our_j2k.len())];
            let opj_tile = &opj_data[opj_pos..opj_pos.saturating_add(30).min(opj_data.len())];
            
            println!("Ours:     {:02X?}", our_tile);
            println!("OpenJPEG: {:02X?}", opj_tile);
            
            println!("\n=== BYTE-BY-BYTE COMPARISON ===");
            for i in 0..our_tile.len().min(opj_tile.len()) {
                let match_str = if our_tile[i] == opj_tile[i] { "✓" } else { "✗" };
                println!("Byte {}: {:08b} vs {:08b} {}", 
                    i, our_tile[i], opj_tile[i], match_str);
            }
        }
    }
}

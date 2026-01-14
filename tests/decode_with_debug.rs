use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use std::fs;
use std::process::Command;
use std::env;

#[test]
fn decode_openjpeg_with_debug() {
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    
    let pgm_header = "P5\n4 4\n255\n";
    let mut pgm_data = pgm_header.as_bytes().to_vec();
    pgm_data.extend_from_slice(&pixels);
    fs::write("debug_decode.pgm", &pgm_data).unwrap();
    
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&["-i", "debug_decode.pgm", "-o", "debug_decode.j2k", "-n", "2"])
        .output();
    
    let j2k_data = fs::read("debug_decode.j2k").unwrap();
    
    println!("=== Decoding OpenJPEG file with J2K_DEBUG=1 ===\n");
    
    env::set_var("J2K_DEBUG", "1");
    
    let mut decoder = J2kDecoder::new();
    let frame_info = decoder.read_header(&j2k_data).unwrap();
    
    println!("\nFrame info: {:?}\n", frame_info);
    println!("Now decoding (watch for tag tree values)...\n");
    
    let mut output = vec![0u8; frame_info.width as usize * frame_info.height as usize];
    let _ = decoder.decode(&j2k_data, &mut output);
    
    env::remove_var("J2K_DEBUG");
    
    let _ = fs::remove_file("debug_decode.pgm");
    let _ = fs::remove_file("debug_decode.j2k");
}

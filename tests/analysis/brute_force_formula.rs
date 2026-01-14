use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[path = "../common/mod.rs"]
mod common;
use common::file_io::get_test_output_path;

#[test]
fn brute_force_zero_bp_formula() {
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    
    let pgm_header = "P5\n4 4\n255\n";
    let mut pgm_data = pgm_header.as_bytes().to_vec();
    pgm_data.extend_from_slice(&pixels);
    
    let pgm_path = get_test_output_path("brute_test.pgm");
    let j2k_path = get_test_output_path("brute_test_opj.j2k");

    fs::write(&pgm_path, &pgm_data).unwrap();
    
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", pgm_path.to_str().unwrap(),
            "-o", j2k_path.to_str().unwrap(),
            "-n", "2"
        ])
        .output();
    
    let opj_data = fs::read(&j2k_path).unwrap();
    let opj_sod = find_marker(&opj_data, 0xFF93).unwrap();
    let opj_tile = &opj_data[opj_sod + 2..];
    
    println!("OpenJPEG reference (first 15 bytes):");
    for i in 0..15 {
        print!("{:02X} ", opj_tile[i]);
    }
    println!("\n");
    
    println!("Known values for this test:");
    println!("  LL: max_bp=7, epsilon=8, guard=2");
    println!("  HL: max_bp=4, epsilon=9, guard=2");
    println!("  LH: max_bp=6, epsilon=9, guard=2");
    println!();
    
    println!("Testing different zero_bp formulas:");
    println!();
    
    let formulas: Vec<(&str, Box<dyn Fn(i32, i32, i32) -> i32>)> = vec![
        ("epsilon + guard - max_bp - 2", Box::new(|eps, g, mb| eps + g - mb - 2)),
        ("epsilon + guard - max_bp - 1", Box::new(|eps, g, mb| eps + g - mb - 1)),
        ("epsilon + guard - max_bp", Box::new(|eps, g, mb| eps + g - mb)),
        ("epsilon + guard - max_bp + 1", Box::new(|eps, g, mb| eps + g - mb + 1)),
    ];
    
    for (name, formula) in formulas {
        let zero_bp_ll = formula(8, 2, 7);
        let zero_bp_hl = formula(9, 2, 4);
        let zero_bp_lh = formula(9, 2, 6);
        
        println!("Formula: {}", name);
        println!("  LL zero_bp = {}", zero_bp_ll);
        println!("  HL zero_bp = {}", zero_bp_hl);
        println!("  LH zero_bp = {}", zero_bp_lh);
        println!();
    }
    
    // let _ = fs::remove_file("brute_test.pgm");
    // let _ = fs::remove_file("brute_test_opj.j2k");
}

fn find_marker(data: &[u8], marker: u16) -> Option<usize> {
    for i in 0..data.len().saturating_sub(1) {
        if data[i] == (marker >> 8) as u8 && data[i + 1] == (marker & 0xFF) as u8 {
            return Some(i);
        }
    }
    None
}

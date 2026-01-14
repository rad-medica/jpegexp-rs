/// Compare our 40x40 encoding byte-by-byte with OpenJPEG to find where they diverge
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_40x40_byte_by_byte() {
    let size = 40;
    
    // Create simple gradient pattern
    let mut pixels = vec![0u8; size * size];
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = (x + y) as u8; // Simple x+y pattern
        }
    }
    
    // Create raw file for OpenJPEG
    fs::write("test_40x40_simple.raw", &pixels).unwrap();
    
    // Encode with OpenJPEG
    Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "test_40x40_simple.raw",
            "-o", "test_40x40_simple_opj.j2k",
            "-n", "3",
            "-r", "1",
            "-F", &format!("{},{},1,8,u", size, size),
        ])
        .output()
        .expect("OpenJPEG failed");
    
    // Encode with ours
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    }, &mut output).unwrap();
    
    fs::write("test_40x40_simple_ours.j2k", &output[..output_size]).unwrap();
    
    // Read both files
    let opj_data = fs::read("test_40x40_simple_opj.j2k").unwrap();
    let ours_data = fs::read("test_40x40_simple_ours.j2k").unwrap();
    
    println!("OpenJPEG size: {} bytes", opj_data.len());
    println!("Ours size: {} bytes", ours_data.len());
    
    // Find where they first differ
    let mut first_diff = None;
    for i in 0..std::cmp::min(opj_data.len(), ours_data.len()) {
        if opj_data[i] != ours_data[i] {
            first_diff = Some(i);
            break;
        }
    }
    
    if let Some(pos) = first_diff {
        println!("\nFirst difference at byte {} (0x{:04X})", pos, pos);
        println!("OpenJPEG: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                 opj_data.get(pos).unwrap_or(&0),
                 opj_data.get(pos+1).unwrap_or(&0),
                 opj_data.get(pos+2).unwrap_or(&0),
                 opj_data.get(pos+3).unwrap_or(&0),
                 opj_data.get(pos+4).unwrap_or(&0),
                 opj_data.get(pos+5).unwrap_or(&0),
                 opj_data.get(pos+6).unwrap_or(&0),
                 opj_data.get(pos+7).unwrap_or(&0));
        println!("Ours:     {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                 ours_data.get(pos).unwrap_or(&0),
                 ours_data.get(pos+1).unwrap_or(&0),
                 ours_data.get(pos+2).unwrap_or(&0),
                 ours_data.get(pos+3).unwrap_or(&0),
                 ours_data.get(pos+4).unwrap_or(&0),
                 ours_data.get(pos+5).unwrap_or(&0),
                 ours_data.get(pos+6).unwrap_or(&0),
                 ours_data.get(pos+7).unwrap_or(&0));
        
        // Check if this is a marker byte (0xFF)
        if opj_data[pos] == 0xFF {
            let marker = ((opj_data[pos] as u16) << 8) | *opj_data.get(pos+1).unwrap_or(&0) as u16;
            println!("OpenJPEG marker: 0x{:04X}", marker);
        }
    }
    
    // Try decoding both
    Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_40x40_simple_opj.j2k", "-o", "test_40x40_simple_opj_decoded.raw"])
        .output()
        .expect("OpenJPEG decode failed");
    
    Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_40x40_simple_ours.j2k", "-o", "test_40x40_simple_ours_decoded.raw"])
        .output()
        .expect("OpenJPEG decode ours failed");
    
    // Compare decoded data
    let opj_decoded = fs::read("test_40x40_simple_opj_decoded.raw").unwrap();
    let ours_decoded = fs::read("test_40x40_simple_ours_decoded.raw").unwrap();
    
    let mut errors = 0;
    let mut total_diff = 0i32;
    for (_, (a, b)) in opj_decoded.iter().zip(ours_decoded.iter()).enumerate() {
        if a != b {
            errors += 1;
            total_diff += (*a as i32 - *b as i32).abs() as i32;
        }
    }
    
    println!("\nDecoded differences: {}/{}", errors, opj_decoded.len());
    println!("MAE: {:.4}", total_diff as f64 / opj_decoded.len() as f64);
    
    // Now compare original with decoded
    let mut orig_errors = 0;
    let mut orig_total_diff = 0i32;
    for (_, (orig, &dec)) in pixels.iter().zip(ours_decoded.iter()).enumerate() {
        if *orig as i32 != dec as i32 {
            orig_errors += 1;
            orig_total_diff += (*orig as i32 - dec as i32).abs() as i32;
        }
    }
    
    println!("\nOriginal vs decoded errors: {}/{}", orig_errors, pixels.len());
    println!("MAE vs original: {:.4}", orig_total_diff as f64 / pixels.len() as f64);
}

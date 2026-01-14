// Test to trace which orientations are being used during encoding

use std::process::Command;
use std::fs;

#[test]
#[ignore]
fn trace_orientation_usage() {
    std::env::set_var("J2K_ORIENT_DEBUG", "1");
    std::env::set_var("J2K_CBLK_DETAIL", "1");
    
    let size = 40;
    let levels = 2;
    
    // Create diagonal gradient
    let mut image = Vec::with_capacity(size * size);
    for y in 0..size {
        for x in 0..size {
            let val = ((x * 4 + y * 4) % 256) as u8;
            image.push(val);
        }
    }
    
    let frame_info = jpegexp_rs::FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = jpegexp_rs::jpeg2000::encoder::J2kEncoder::new();
    encoder.set_decomposition_levels(levels);
    encoder.set_irreversible(false);
    
    let mut output = vec![0u8; size * size * 4];
    
    eprintln!("\n=== ENCODING 40x40 WITH LEVEL {} ===\n", levels);
    
    let len = encoder.encode(&image, &frame_info, &mut output)
        .expect("Encoding failed");
    
    eprintln!("\n=== ENCODING COMPLETE: {} bytes ===\n", len);
    
    output.truncate(len);
    fs::write("trace_40x40_ours.j2k", &output).unwrap();
    
    // Decode with OpenJPEG
    let _status = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "trace_40x40_ours.j2k", "-o", "trace_40x40_decoded.pgm"])
        .output();
}

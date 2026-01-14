/// Test 10x10 image with 1 decomposition level
/// This gives level 0 = 5x5

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_10x10_level1() {
    let size = 10;
    let mut pixels = vec![128u8; size * size];
    
    // Simple gradient
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x + y) * 25) as u8;
        }
    }
    
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);  // Only 1 level
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    
    let our_file = "test_10x10_level1.j2k";
    fs::write(our_file, &our_output[..our_size]).unwrap();
    
    println!("Wrote {} bytes to {}", our_size, our_file);
    
    // Decode with OpenJPEG
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", our_file, "-o", "test_10x10_level1_decoded.pnm"])
        .output()
        .expect("Failed to run opj_decompress");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("OpenJPEG stdout: {}", stdout);
    println!("OpenJPEG stderr: {}", stderr);
    
    assert!(output.status.success(), "OpenJPEG decode failed for 10x10");
}

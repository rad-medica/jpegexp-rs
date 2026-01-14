/// Test 10x10 with 8-bit depth
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_10x10_8bit() {
    let size = 10;
    let mut pixels = vec![0u8; size * size];
    
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x * 4 + y * 4) % 256) as u8;
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
    encoder.set_decomposition_levels(2);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    let filename = "test_10x10_8bit.j2k";
    fs::write(filename, &output[..output_size]).unwrap();
    
    println!("Created {} ({} bytes)", filename, output_size);
    
    // Decode with OpenJPEG
    let result = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", filename, "-o", "test_10x10_8bit_decoded.pnm"])
        .output()
        .expect("Failed to run opj_decompress");
    
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    
    println!("OpenJPEG stdout: {}", stdout);
    println!("OpenJPEG stderr: {}", stderr);
    
    if result.status.success() {
        println!("✅ 8-bit 10x10 PASSED!");
    } else {
        println!("❌ 8-bit 10x10 FAILED!");
    }
}

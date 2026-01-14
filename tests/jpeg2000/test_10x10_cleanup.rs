// Test to debug 10x10 codeblock cleanup pass issue
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;

/// Helper to encode/decode and check MAE
fn test_size(size: u32) -> f64 {
    println!("\n========== Testing {}x{} ==========", size, size);
    
    // Create diagonal gradient pattern (as u8)
    let mut pixels = vec![0u8; (size * size * 2) as usize]; // 2 bytes per 16-bit sample
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 2) as usize;
            let val = ((x * 4 + y * 4) % 256) as u16;
            pixels[idx] = (val & 0xFF) as u8;
            pixels[idx + 1] = (val >> 8) as u8;
        }
    }

    // Encode with our encoder
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Reversible/lossless
    encoder.set_decomposition_levels(2);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let frame_info = FrameInfo {
        width: size,
        height: size,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut j2k_data = vec![0u8; 1024 * 1024];
    let j2k_size = encoder
        .encode(&pixels, &frame_info, &mut j2k_data)
        .expect("Encode failed");
    
    let output_path = format!("test_cleanup_{}x{}.j2k", size, size);
    std::fs::write(&output_path, &j2k_data[..j2k_size]).expect("Write failed");
    println!("Wrote {} bytes to {}", j2k_size, output_path);

    // Decode with OpenJPEG
    let raw_path = format!("test_cleanup_{}x{}.raw", size, size);
    let result = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", &output_path,
            "-o", &raw_path,
            "-r", "0",
        ])
        .output()
        .expect("OpenJPEG failed");

    if !result.status.success() {
        eprintln!("OpenJPEG stdout: {}", String::from_utf8_lossy(&result.stdout));
        eprintln!("OpenJPEG stderr: {}", String::from_utf8_lossy(&result.stderr));
        panic!("OpenJPEG decode failed for {}x{}", size, size);
    }

    // Read decoded data (16-bit little endian)
    let decoded_bytes = std::fs::read(&raw_path).expect("Read decoded failed");
    let decoded: Vec<u16> = decoded_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    
    // Extract original values as u16
    let original: Vec<u16> = pixels
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    // Calculate MAE
    let mae: f64 = original
        .iter()
        .zip(decoded.iter())
        .map(|(&orig, &dec)| (orig as i32 - dec as i32).abs() as f64)
        .sum::<f64>()
        / original.len() as f64;

    println!("MAE: {:.4}", mae);
    
    // Cleanup
    std::fs::remove_file(&output_path).ok();
    std::fs::remove_file(&raw_path).ok();
    
    mae
}

#[test]
#[ignore]
fn test_8x8_vs_10x10() {
    // Set trace environment
    std::env::set_var("BPC_CLEANUP_TRACE", "1");
    std::env::set_var("J2K_ORIENT_DEBUG", "1");
    std::env::set_var("J2K_CBLK_DETAIL", "1");
    
    println!("\n=== Testing 8x8 (should pass) ===");
    let mae_8 = test_size(8);
    assert!(mae_8 < 0.01, "8x8 failed with MAE={}", mae_8);
    
    println!("\n\n=== Testing 10x10 (currently fails) ===");
    let mae_10 = test_size(10);
    
    // This will currently fail, but we want to see the logs
    if mae_10 > 0.01 {
        panic!("10x10 failed with MAE={} (expected < 0.01)", mae_10);
    }
}

#[test]
#[ignore]
fn test_just_10x10() {
    //std::env::set_var("BPC_CLEANUP_TRACE", "1");
    std::env::set_var("J2K_ORIENT_DEBUG", "1");
    std::env::set_var("J2K_PKT_DEBUG", "1");
    std::env::set_var("J2K_PKT_TRACE", "1");
    std::env::set_var("J2K_CBLK_DETAIL", "1");
    std::env::set_var("J2K_RES_DEBUG", "1");
    
    let mae = test_size(10);
    println!("\nFinal MAE for 10x10: {:.4}", mae);
}

#[test]
#[ignore]
fn test_level2_subband_sizes() {
    // At level 2, a 10x10 image produces these subband sizes:
    // Level 0 (LL0): 10x10 → 5x5 (LL1) after level 1
    // Level 1 from 10x10: HL1=5x5, LH1=5x5, HH1=5x5
    // Level 2 from 5x5 LL1: LL2=3x3, HL2=2x2, LH2=2x2, HH2=2x2
    
    // Wait, let me recalculate...
    // 10 → (ceil(10/2), floor(10/2)) = (5, 5)
    // 5 → (ceil(5/2), floor(5/2)) = (3, 2)
    
    // So level 2 subbands from 10x10 image:
    // LL2: 3x3
    // HL2, LH2, HH2: 2x2
    
    // Hmm, this doesn't match our earlier understanding of 10x10 subbands!
    // Let me check what the actual subband sizes are...
    
    println!("This test just documents the expected subband sizes");
    println!("10x10 at level 2:");
    println!("  LL2: 3x3");
    println!("  HL2, LH2, HH2: 2x2");
    println!("");
    println!("40x40 at level 2:");
    println!("  40 → 20 → 10");
    println!("  LL2: 10x10");
    println!("  HL2, LH2, HH2: 10x10");
}

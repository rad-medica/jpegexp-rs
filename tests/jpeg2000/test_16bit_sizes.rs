/// Test 16-bit encoding at various sizes
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[path = "../common/mod.rs"]
mod common;
use common::file_io::get_test_output_path;

fn test_16bit_size(size: usize) -> f64 {
    let mut pixels = vec![0u8; size * size * 2];
    
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 2;
            let val = ((x * 4 + y * 4) % 256) as u16;
            pixels[idx] = (val & 0xFF) as u8;
            pixels[idx + 1] = (val >> 8) as u8;
        }
    }
    
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    let filename = format!("test_16bit_{}x{}.j2k", size, size);
    let output_path = get_test_output_path(&filename);
    
    fs::write(&output_path, &output[..output_size]).unwrap();
    
    // Decode with OpenJPEG
    let raw_file = format!("test_16bit_{}x{}.raw", size, size);
    let raw_path = get_test_output_path(&raw_file);

    let result = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", output_path.to_str().unwrap(),
            "-o", raw_path.to_str().unwrap(),
            "-r", "0"
        ])
        .output()
        .expect("OpenJPEG failed");
    
    if !result.status.success() {
        eprintln!("OpenJPEG failed for {}x{}", size, size);
        eprintln!("stdout: {}", String::from_utf8_lossy(&result.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&result.stderr));
        return -1.0;
    }
    
    // Read decoded data
    let decoded_bytes = fs::read(&raw_path).expect("Read decoded failed");
    let decoded: Vec<u16> = decoded_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    
    // Extract original values
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
    
    mae
}

#[test]
#[ignore]
fn test_16bit_size_sweep() {
    println!("\n=== 16-bit Size Sweep ===\n");
    
    let sizes = [8, 10, 12, 16, 20, 24, 32, 40, 48, 64];
    
    for size in sizes {
        let mae = test_16bit_size(size);
        if mae < 0.0 {
            println!("FAIL {}x{}: OpenJPEG decode failed", size, size);
        } else if mae == 0.0 {
            println!("PASS {}x{}: MAE={:.4}", size, size, mae);
        } else {
            println!("FAIL {}x{}: MAE={:.4}", size, size, mae);
        }
    }
}

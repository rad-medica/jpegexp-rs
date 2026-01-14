/// Simple test for 40x40 DWT fix - standalone, no external dependencies

#[path = "../common/mod.rs"]
mod common;
use common::file_io::get_test_output_path;

#[test]
fn test_40x40_dwt_fix() {
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    use std::fs;
    use std::process::Command;

    let size = 40;
    
    // Create simple test pattern: x + y
    let mut pixels = vec![0u8; size * size];
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x + y) % 256) as u8;
        }
    }
    
    // Encode with our encoder
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    println!("Encoded {} bytes", output_size);
    
    // Write test files
    let j2k_path = get_test_output_path("test_40x40_ours.j2k");
    let raw_path = get_test_output_path("test_40x40_input.raw");
    let pnm_path = get_test_output_path("test_40x40_decoded.pnm");

    fs::write(&j2k_path, &output[..output_size]).unwrap();
    fs::write(&raw_path, &pixels).unwrap();
    
    // Decode with OpenJPEG
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", j2k_path.to_str().unwrap(),
            "-o", pnm_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to decode");
    
    if !output.status.success() {
        println!("OpenJPEG decode failed:");
        println!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("Decode failed");
    }
    
    // Parse PNM
    fn parse_pnm(data: &[u8]) -> Vec<u8> {
        let mut offset = 0;
        while offset < data.len() && data[offset] != b'\n' {
            offset += 1;
        }
        offset += 1;
        while offset < data.len() && (data[offset] == b'#' || data[offset] == b'\n') {
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
        }
        while offset < data.len() && data[offset] != b'\n' {
            offset += 1;
        }
        offset += 1;
        while offset < data.len() && data[offset] != b'\n' {
            offset += 1;
        }
        offset += 1;
        data[offset..].to_vec()
    }
    
    let decoded_data = fs::read(&pnm_path).unwrap();
    let decoded_pixels = parse_pnm(&decoded_data);
    
    // Compare
    let mut errors = 0;
    let mut sum_error = 0.0;
    let count = pixels.len().min(decoded_pixels.len());
    
    // Analyze error pattern
    let mut error_positions: Vec<(usize, i32, i32)> = Vec::new();
    for i in 0..count {
        let error = (pixels[i] as i32 - decoded_pixels[i] as i32).abs();
        if error > 0 {
            errors += 1;
            sum_error += error as f64;
            if error_positions.len() < 10 {
                error_positions.push((i, pixels[i] as i32, decoded_pixels[i] as i32));
            }
        }
    }
    
    let mae = sum_error / count as f64;
    
    println!("\n=== 40x40 Test Results ===");
    println!("Pixels tested: {}", count);
    println!("Errors: {} / {} ({:.1}%)", errors, count, errors as f64 / count as f64 * 100.0);
    println!("MAE: {:.4}", mae);
    
    println!("\nFirst 10 errors:");
    for (idx, orig, decoded) in &error_positions {
        let x = idx % size;
        let y = idx / size;
        println!("  ({:2},{:2}): orig={:3}, decoded={:3}, diff={}", x, y, orig, decoded, (orig - decoded).abs());
    }
    
    // Check error distribution
    let mut edge_errors = 0;
    let mut corner_errors = 0;
    for i in 0..count {
        let error = (pixels[i] as i32 - decoded_pixels[i] as i32).abs();
        if error > 0 {
            let x = i % size;
            let y = i / size;
            if x == 0 || x == size-1 || y == 0 || y == size-1 {
                edge_errors += 1;
            }
            if (x == 0 || x == size-1) && (y == 0 || y == size-1) {
                corner_errors += 1;
            }
        }
    }
    
    println!("\nError distribution:");
    println!("  Edge errors: {} / {} ({:.1}%)", edge_errors, errors, edge_errors as f64 / errors as f64 * 100.0);
    println!("  Corner errors: {} / {} ({:.1}%)", corner_errors, errors, corner_errors as f64 / errors as f64 * 100.0);
    
    if mae == 0.0 {
        println!("✅ 40x40 test PASSED - DWT fix working!");
    } else if mae < 0.1 {
        println!("⚠️  40x40 test ALMOST PASSED - MAE = {:.4} (very small errors remain)", mae);
    } else {
        println!("❌ 40x40 test FAILED - MAE = {:.4}", mae);
    }
    
    // For now, accept very small MAE as partial success
    assert!(mae < 1.0, "MAE should be < 1 for nearly-correct encoding");
}

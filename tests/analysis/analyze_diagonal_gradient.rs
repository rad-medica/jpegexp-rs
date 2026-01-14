/// Analyze what's special about the diagonal gradient
mod common;

#[test]
#[ignore]
fn analyze_diagonal_vs_horizontal() {
    println!("\n=== Analyzing Diagonal vs Horizontal Gradient ===\n");

    // Diagonal: x*4 + y*4
    println!("Diagonal gradient (x*4 + y*4) - first 8x8:");
    for y in 0..8 {
        print!("  ");
        for x in 0..8 {
            let val = ((x * 4 + y * 4) % 256) as u8;
            print!("{:4} ", val);
        }
        println!();
    }

    println!("\nHorizontal gradient (x*4) - first 8x8:");
    for y in 0..8 {
        print!("  ");
        for x in 0..8 {
            let val = ((x * 4) % 256) as u8;
            print!("{:4} ", val);
        }
        println!();
    }

    println!("\nKey difference:");
    println!("  - Horizontal: Changes only in X direction (constant in Y)");
    println!("  - Diagonal: Changes in BOTH X and Y directions");
    println!("  - This creates energy in the HH (diagonal) subband!");

    // Let's test a simpler diagonal pattern
    println!("\n=== Testing Simpler Diagonal Patterns ===");

    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    use std::fs;
    use std::process::Command;

    // Test 1: x + y (simpler diagonal)
    let width = 64;
    let height = 64;
    let mut simple_diag = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            simple_diag[y * width + x] = ((x + y) % 256) as u8;
        }
    }

    test_pattern("simple_diag", &simple_diag, width, height);

    // Test 2: x*2 + y*2 (half the slope)
    let mut half_slope = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            half_slope[y * width + x] = ((x * 2 + y * 2) % 256) as u8;
        }
    }

    test_pattern("half_slope", &half_slope, width, height);

    // Test 3: x*8 + y*8 (double the slope)
    let mut double_slope = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            double_slope[y * width + x] = ((x * 8 + y * 8) % 256) as u8;
        }
    }

    test_pattern("double_slope", &double_slope, width, height);
}

fn test_pattern(name: &str, pixels: &[u8], width: usize, height: usize) {
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    use std::fs;
    use std::process::Command;
    use crate::common::file_io::get_test_output_path;

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);

    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder
        .encode(&pixels, &frame_info, &mut our_output)
        .unwrap();
    let our_bytes = &our_output[..our_size];

    let our_file_name = format!("test_{}_ours.j2k", name);
    let our_file_path = get_test_output_path(&our_file_name);
    let decoded_file_name = format!("test_{}_ours_decoded.pnm", name);
    let decoded_file_path = get_test_output_path(&decoded_file_name);

    fs::write(&our_file_path, our_bytes).unwrap();

    // Decode our file with OpenJPEG
    let output = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i",
            our_file_path.to_str().unwrap(),
            "-o",
            decoded_file_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to decode");

    if !output.status.success() {
        println!("❌ {} decode failed", name);
        return;
    }

    // Parse and compare
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

    let decoded_data = fs::read(&decoded_file_path).unwrap();
    let decoded_pixels = parse_pnm(&decoded_data);

    let mut errors = 0;
    let mut sum_error = 0.0;
    let mut max_error = 0;
    for i in 0..pixels.len().min(decoded_pixels.len()) {
        let error = (pixels[i] as i32 - decoded_pixels[i] as i32).abs();
        if error > 0 {
            errors += 1;
            sum_error += error as f64;
            max_error = max_error.max(error);
        }
    }

    let mae = sum_error / pixels.len() as f64;

    let status = if mae == 0.0 { "✅" } else { "❌" };
    println!(
        "{} {}: MAE={:.4}, Max={}, Errors={}/{}, Size={}B",
        status,
        name,
        mae,
        max_error,
        errors,
        pixels.len(),
        our_size
    );
}

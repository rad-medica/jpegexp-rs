// Compare our encoder output with OpenJPEG encoder output
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

use std::path::Path;

#[path = "../common/mod.rs"]
mod common;
use common::file_io::get_test_output_path;

/// Find a binary in libs/bin or PATH
fn find_binary(name: &str) -> Option<String> {
    let bin_dir = "libs/bin";
    let bin_path = format!("{}/{}.exe", bin_dir, name);
    if Path::new(&bin_path).exists() {
        return Some(bin_path);
    }

    let bin_path_unix = format!("{}/{}", bin_dir, name);
    if Path::new(&bin_path_unix).exists() {
        return Some(bin_path_unix);
    }

    if Command::new(name).arg("--help").output().is_ok() {
        return Some(name.to_string());
    }

    None
}

#[test]
#[ignore]
fn compare_gradient_encoding() {
    // Create a simple gradient
    let width = 8;
    let height = 8;
    let mut pixels = vec![0u8; (width * height) as usize];
    for i in 0..64 {
        pixels[i] = (i * 4) as u8;
    }

    // Encode with our encoder
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(1); // 1 level DWT

    let info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut output_buffer = vec![0u8; 10000]; // Allocate sufficient buffer
    let bytes_written = encoder.encode(&pixels, &info, &mut output_buffer).expect(
        "Encoding failed",
    );
    let our_output = &output_buffer[..bytes_written];

    let our_j2k_path = get_test_output_path("test_our_gradient.j2k");
    fs::write(&our_j2k_path, our_output).expect("Failed to write our output");
    println!("Our encoder output: {} bytes", our_output.len());

    // Save raw pixels for OpenJPEG
    let grad_raw_path = get_test_output_path("test_gradient.raw");
    fs::write(&grad_raw_path, &pixels).expect("Failed to write raw");

    // Define cleanup paths upfront
    let our_dec_raw_path = get_test_output_path("test_our_decoded.raw");
    let opj_dec_raw_path = get_test_output_path("test_opj_decoded.raw");
    let opj_j2k_path = get_test_output_path("test_opj_gradient.j2k");

    // Encode with OpenJPEG
    let compress_bin = find_binary("opj_compress").unwrap_or_else(|| "opj_compress".to_string());
    let opj_result = Command::new(compress_bin)
        .args(
            &[
            "-i", grad_raw_path.to_str().unwrap(),
            "-o", opj_j2k_path.to_str().unwrap(),
            "-F", "8,8,8,1,u@1x1",  // 8x8, 8bpp, 1 component, unsigned
            "-n", "1",  // 1 resolution level (same as 1 decomposition)
            "-I",  // Lossless
        ],
        )
        .output();

    if let Ok(output) = opj_result {
        if output.status.success() {
            let opj_output = fs::read(&opj_j2k_path).expect("Failed to read OpenJPEG output");
            println!("OpenJPEG encoder output: {} bytes", opj_output.len());

            // Compare byte by byte
            println!("\n=== First 100 bytes comparison ===");
            let min_len = our_output.len().min(opj_output.len()).min(100);
            for i in 0..min_len {
                if our_output[i] != opj_output[i] {
                    println!(
                        "Byte {}: Ours=0x{:02X}, OpenJPEG=0x{:02X} ❌",
                        i,
                        our_output[i],
                        opj_output[i]
                    );
                } else {
                    if i < 20 {
                        println!("Byte {}: 0x{:02X} ✓", i, our_output[i]);
                    }
                }
            }

            // Test decoding both with OpenJPEG
            let decompress_bin =
                find_binary("opj_decompress").unwrap_or_else(|| "opj_decompress".to_string());
            let decode_ours = Command::new(&decompress_bin)
                .args(&[
                    "-i", our_j2k_path.to_str().unwrap(), 
                    "-o", our_dec_raw_path.to_str().unwrap()
                ])
                .output();

            let decode_opj = Command::new(&decompress_bin)
                .args(&[
                    "-i", opj_j2k_path.to_str().unwrap(), 
                    "-o", opj_dec_raw_path.to_str().unwrap()
                ])
                .output();

            if decode_ours.is_ok() && decode_opj.is_ok() {
                let our_decoded = fs::read(&our_dec_raw_path).ok();
                let opj_decoded = fs::read(&opj_dec_raw_path).ok();

                if let (Some(our_dec), Some(opj_dec)) = (our_decoded, opj_decoded) {
                    println!("\n=== Decoded pixel comparison ===");
                    println!("Original: {:?}", &pixels[0..16]);
                    println!("OpenJPEG decoded their encoding: {:?}", &opj_dec[0..16]);
                    println!("OpenJPEG decoded our encoding: {:?}", &our_dec[0..16]);

                    // Calculate MAE
                    let mae_our: f64 = pixels
                        .iter()
                        .zip(our_dec.iter())
                        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
                        .sum::<f64>() / pixels.len() as f64;

                    let mae_opj: f64 = pixels
                        .iter()
                        .zip(opj_dec.iter())
                        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
                        .sum::<f64>() / pixels.len() as f64;

                    println!("MAE (OpenJPEG decoding our encoding): {}", mae_our);
                    println!("MAE (OpenJPEG decoding their encoding): {}", mae_opj);
                }
            }
        } else {
            println!(
                "OpenJPEG encoder failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    } else {
        println!("opj_compress not found - skipping OpenJPEG comparison");
    }

    // Cleanup
    // let _ = fs::remove_file(grad_raw_path);
    // let _ = fs::remove_file(our_j2k_path);
    // let _ = fs::remove_file(opj_j2k_path);
    // let _ = fs::remove_file(our_dec_raw_path);
    // let _ = fs::remove_file(opj_dec_raw_path);
}

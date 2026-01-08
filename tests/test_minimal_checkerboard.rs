// Minimal test to debug checkerboard encoding
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

#[test]
#[ignore]
fn debug_checkerboard_minimal() {
    // Smallest possible checkerboard
    let width = 4;
    let height = 4;
    let mut pixels = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            pixels[idx] = if (x + y) % 2 == 0 { 0 } else { 255 };
        }
    }
    
    println!("=== 4x4 Checkerboard ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", pixels[(y * width + x) as usize]);
        }
        println!();
    }
    
    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(0); // NO DWT - test raw encoding
    
    let info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; 10000];
    let len = encoder.encode(&pixels, &info, &mut output).expect("Encode failed");
    output.truncate(len);
    
    println!("\n=== Encoded {} bytes ===", len);
    println!("First 100 bytes:");
    for (i, chunk) in output.chunks(16).enumerate().take(7) {
        print!("{:04X}: ", i * 16);
        for b in chunk {
            print!("{:02X} ", b);
        }
        println!();
    }
    
    // Decode with our decoder
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decode failed");
    let decoded_ours = image.reconstruct_pixels().expect("Reconstruct failed");
    
    println!("\n=== Our decoder result ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", decoded_ours[(y * width + x) as usize]);
        }
        println!();
    }
    
    let mae_ours: f64 = pixels.iter().zip(decoded_ours.iter())
        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
        .sum::<f64>() / pixels.len() as f64;
    println!("Our decoder MAE: {}", mae_ours);
    
    // Try OpenJPEG decoder
    fs::write("test_checker_4x4.j2k", &output).expect("Write failed");
    
    let result = Command::new("opj_decompress")
        .args(&["-i", "test_checker_4x4.j2k", "-o", "test_checker_4x4_opj.raw"])
        .output();
    
    if let Ok(output_cmd) = result {
        if output_cmd.status.success() {
            if let Ok(decoded_opj) = fs::read("test_checker_4x4_opj.raw") {
                println!("\n=== OpenJPEG decoder result ===");
                for y in 0..height {
                    for x in 0..width {
                        if (y * width + x) < decoded_opj.len() as u32 {
                            print!("{:3} ", decoded_opj[(y * width + x) as usize]);
                        }
                    }
                    println!();
                }
                
                let mae_opj: f64 = pixels.iter().zip(decoded_opj.iter())
                    .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
                    .sum::<f64>() / pixels.len() as f64;
                println!("OpenJPEG decoder MAE: {}", mae_opj);
                
                // Show pixel-by-pixel comparison
                println!("\n=== Pixel comparison ===");
                println!("Idx | Orig | Ours | OpenJPEG | Diff");
                for i in 0..pixels.len() {
                    println!("{:3} | {:4} | {:4} | {:8} | {:4}",
                             i, pixels[i], decoded_ours[i], 
                             decoded_opj.get(i).copied().unwrap_or(0),
                             (pixels[i] as i32 - decoded_opj.get(i).copied().unwrap_or(0) as i32).abs());
                }
            } else {
                println!("OpenJPEG decoder failed to produce output");
                println!("stderr: {}", String::from_utf8_lossy(&output_cmd.stderr));
            }
        } else {
            println!("OpenJPEG decoder returned error");
            println!("stderr: {}", String::from_utf8_lossy(&output_cmd.stderr));
        }
    } else {
        println!("opj_decompress not found");
    }
    
    // Cleanup
    let _ = fs::remove_file("test_checker_4x4.j2k");
    let _ = fs::remove_file("test_checker_4x4_opj.raw");
}

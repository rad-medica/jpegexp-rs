// Compare our encoder output with OpenJPEG encoder output
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;

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
    let bytes_written = encoder.encode(&pixels, &info, &mut output_buffer).expect("Encoding failed");
    let our_output = &output_buffer[..bytes_written];
    
    fs::write("test_our_gradient.j2k", our_output).expect("Failed to write our output");
    println!("Our encoder output: {} bytes", our_output.len());
    
    // Save raw pixels for OpenJPEG
    fs::write("test_gradient.raw", &pixels).expect("Failed to write raw");
    
    // Encode with OpenJPEG
    let opj_result = Command::new("opj_compress")
        .args(&[
            "-i", "test_gradient.raw",
            "-o", "test_opj_gradient.j2k",
            "-F", "8,8,8,1,u@1x1",  // 8x8, 8bpp, 1 component, unsigned
            "-n", "1",  // 1 resolution level (same as 1 decomposition)
            "-I",  // Lossless
        ])
        .output();
    
    if let Ok(output) = opj_result {
        if output.status.success() {
            let opj_output = fs::read("test_opj_gradient.j2k").expect("Failed to read OpenJPEG output");
            println!("OpenJPEG encoder output: {} bytes", opj_output.len());
            
            // Compare byte by byte
            println!("\n=== First 100 bytes comparison ===");
            let min_len = our_output.len().min(opj_output.len()).min(100);
            for i in 0..min_len {
                if our_output[i] != opj_output[i] {
                    println!("Byte {}: Ours=0x{:02X}, OpenJPEG=0x{:02X} ❌", i, our_output[i], opj_output[i]);
                } else {
                    if i < 20 {
                        println!("Byte {}: 0x{:02X} ✓", i, our_output[i]);
                    }
                }
            }
            
            // Test decoding both with OpenJPEG
            let decode_ours = Command::new("opj_decompress")
                .args(&["-i", "test_our_gradient.j2k", "-o", "test_our_decoded.raw"])
                .output();
            
            let decode_opj = Command::new("opj_decompress")
                .args(&["-i", "test_opj_gradient.j2k", "-o", "test_opj_decoded.raw"])
                .output();
            
            if decode_ours.is_ok() && decode_opj.is_ok() {
                let our_decoded = fs::read("test_our_decoded.raw").ok();
                let opj_decoded = fs::read("test_opj_decoded.raw").ok();
                
                if let (Some(our_dec), Some(opj_dec)) = (our_decoded, opj_decoded) {
                    println!("\n=== Decoded pixel comparison ===");
                    println!("Original: {:?}", &pixels[0..16]);
                    println!("OpenJPEG decoded their encoding: {:?}", &opj_dec[0..16]);
                    println!("OpenJPEG decoded our encoding: {:?}", &our_dec[0..16]);
                    
                    // Calculate MAE
                    let mae_our: f64 = pixels.iter().zip(our_dec.iter())
                        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
                        .sum::<f64>() / pixels.len() as f64;
                    
                    let mae_opj: f64 = pixels.iter().zip(opj_dec.iter())
                        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
                        .sum::<f64>() / pixels.len() as f64;
                    
                    println!("MAE (OpenJPEG decoding our encoding): {}", mae_our);
                    println!("MAE (OpenJPEG decoding their encoding): {}", mae_opj);
                }
            }
        } else {
            println!("OpenJPEG encoder failed: {}", String::from_utf8_lossy(&output.stderr));
        }
    } else {
        println!("opj_compress not found - skipping OpenJPEG comparison");
    }
    
    // Cleanup
    let _ = fs::remove_file("test_gradient.raw");
    let _ = fs::remove_file("test_our_gradient.j2k");
    let _ = fs::remove_file("test_opj_gradient.j2k");
    let _ = fs::remove_file("test_our_decoded.raw");
    let _ = fs::remove_file("test_opj_decoded.raw");
}

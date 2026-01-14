/// Debug 40x40 encoding issue
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_40x40_debug() {
    let size = 40;
    
    // Create simple diagonal gradient (16-bit)
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
    
    fs::write("test_40x40_ours.j2k", &output[..output_size]).unwrap();
    println!("Wrote {} bytes to test_40x40_ours.j2k", output_size);
    
    // Create reference with OpenJPEG
    let raw_file = "test_40x40_input.raw";
    fs::write(raw_file, &pixels).unwrap();
    
    let result = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", raw_file,
            "-o", "test_40x40_openjpeg.j2k",
            "-n", "3", // 2 decomposition levels
            "-r", "1", // lossless
            "-F", "40,40,1,16,u",
        ])
        .output()
        .expect("OpenJPEG compress failed");
    
    println!("\nOpenJPEG encode:");
    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
    
    // Decode both with OpenJPEG
    println!("\n=== Decoding OpenJPEG reference ===");
    let result = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_40x40_openjpeg.j2k", "-o", "test_40x40_openjpeg_decoded.raw", "-r", "0"])
        .output()
        .expect("OpenJPEG decompress failed");
    
    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
    
    println!("\n=== Decoding our output ===");
    let result = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_40x40_ours.j2k", "-o", "test_40x40_ours_decoded.raw", "-r", "0"])
        .output()
        .expect("OpenJPEG decompress failed");
    
    println!("stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&result.stderr));
    
    if !result.status.success() {
        println!("\n❌ OpenJPEG failed to decode our output!");
        return;
    }
    
    // Compare decoded outputs
    let opj_decoded = fs::read("test_40x40_openjpeg_decoded.raw").unwrap();
    let our_decoded = fs::read("test_40x40_ours_decoded.raw").unwrap();
    
    let opj_pixels: Vec<u16> = opj_decoded
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    
    let our_pixels: Vec<u16> = our_decoded
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    
    let original: Vec<u16> = pixels
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    
    // Calculate MAE for both
    let opj_mae: f64 = original.iter().zip(opj_pixels.iter())
        .map(|(&o, &d)| (o as i32 - d as i32).abs() as f64)
        .sum::<f64>() / original.len() as f64;
    
    let our_mae: f64 = original.iter().zip(our_pixels.iter())
        .map(|(&o, &d)| (o as i32 - d as i32).abs() as f64)
        .sum::<f64>() / original.len() as f64;
    
    println!("\nOpenJPEG reference MAE: {:.4}", opj_mae);
    println!("Our output MAE: {:.4}", our_mae);
    
    // Find first difference
    for (i, ((&orig, &opj), &ours)) in original.iter().zip(opj_pixels.iter()).zip(our_pixels.iter()).enumerate() {
        if orig != ours {
            let x = i % size;
            let y = i / size;
            println!("\nFirst error at pixel ({}, {}): orig={}, opj={}, ours={}", 
                     x, y, orig, opj, ours);
            
            // Show surrounding pixels
            println!("\nSurrounding pixels (3x3):");
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = (x as i32 + dx).max(0).min(size as i32 - 1) as usize;
                    let ny = (y as i32 + dy).max(0).min(size as i32 - 1) as usize;
                    let idx = ny * size + nx;
                    print!("({},{}):{}/{} ", nx, ny, original[idx], our_pixels[idx]);
                }
                println!();
            }
            break;
        }
    }
}

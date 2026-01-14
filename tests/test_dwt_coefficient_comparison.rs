/// Detailed comparison of DWT coefficients for 40x40 image
use jpegexp_rs::jpeg2000::dwt::{dwt_5x3_forward, DWTLevel};
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_dwt_coefficient_comparison() {
    let size = 40;
    
    // Create diagonal gradient pattern (8-bit for simplicity)
    let mut pixels = vec![0u16; size * size];
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x * 4 + y * 4) % 256) as u16;
        }
    }
    
    // Compute DWT with our implementation
    let mut coeffs_ours = pixels.clone();
    let levels = dwt_5x3_forward(&mut coeffs_ours, size, size, 2);
    
    println!("Our DWT levels: {:?}", levels);
    println!("\nOur LL subband (first 10): {:?}", &coeffs_ours[..10]);
    
    // Create reference with OpenJPEG
    let raw_file = "test_dwt_comparison.raw";
    let pixels_bytes: Vec<u8> = pixels.iter()
        .flat_map(|&p| p.to_le_bytes())
        .collect();
    fs::write(raw_file, &pixels_bytes).unwrap();
    
    let result = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", raw_file,
            "-o", "test_dwt_comparison_opj.j2k",
            "-n", "3", // 2 decomposition levels
            "-r", "1",
            "-F", &format!("{},{},1,8,u", size, size),
        ])
        .output()
        .expect("OpenJPEG failed");
    
    println!("\nOpenJPEG stdout: {}", String::from_utf8_lossy(&result.stdout));
    
    // Decode and extract coefficients
    let result = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", "test_dwt_comparison_opj.j2k",
            "-o", "test_dwt_comparison_decoded.raw",
        ])
        .output()
        .expect("OpenJPEG decode failed");
    
    // Read decoded
    let decoded_bytes = fs::read("test_dwt_comparison_decoded.raw").unwrap();
    let decoded: Vec<u16> = decoded_bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    
    // Compare first few values
    println!("\nOriginal first 10: {:?}", &pixels[..10]);
    println!("Decoded first 10: {:?}", &decoded[..10]);
    
    // Find first difference
    let mut first_diff = None;
    for (i, (orig, &dec)) in pixels.iter().zip(decoded.iter()).enumerate() {
        if *orig as i32 != dec as i32 {
            first_diff = Some((i, *orig, dec));
            break;
        }
    }
    
    if let Some((idx, orig, dec)) = first_diff {
        let x = idx % size;
        let y = idx / size;
        println!("\nFirst diff at ({},{}): orig={}, decoded={}", x, y, orig, dec);
    } else {
        println!("\n✅ Perfect decode!");
    }
    
    // Now let's check if the issue is in DWT or in EBCOT
    // Compare our encoded file with OpenJPEG's
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels_bytes, &FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    }, &mut output).unwrap();
    
    fs::write("test_dwt_comparison_ours.j2k", &output[..output_size]).unwrap();
    println!("\nOur file size: {} bytes", output_size);
    println!("OpenJPEG file size: {} bytes", fs::metadata("test_dwt_comparison_opj.j2k").unwrap().len());
    
    // Decode our file
    let result = Command::new("libs/bin/opj_decompress.exe")
        .args(&[
            "-i", "test_dwt_comparison_ours.j2k",
            "-o", "test_dwt_comparison_ours_decoded.raw",
        ])
        .output()
        .expect("OpenJPEG decode our file failed");
    
    println!("OpenJPEG decode our file: {}", String::from_utf8_lossy(&result.stderr));
    
    let ours_decoded_bytes = fs::read("test_dwt_comparison_ours_decoded.raw").unwrap();
    let ours_decoded: Vec<u16> = ours_decoded_bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    
    // Compare
    let mut errors = 0;
    let mut total_diff = 0i32;
    let mut max_diff = 0i32;
    for (i, (orig, &dec)) in pixels.iter().zip(ours_decoded.iter()).enumerate() {
        let diff = (*orig as i32 - dec as i32).abs();
        if diff > 0 {
            errors += 1;
            total_diff += diff;
            max_diff = max_diff.max(diff);
        }
    }
    
    println!("\nOur file decode errors: {}/{}", errors, pixels.len());
    println!("Total diff: {}, Max diff: {}", total_diff, max_diff);
    println!("MAE: {:.4}", total_diff as f64 / pixels.len() as f64);
}

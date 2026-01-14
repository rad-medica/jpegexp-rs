/// Extract and compare DWT coefficients from our encoder
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_extract_dwt_coefficients() {
    let size = 40;
    
    // Create simple test pattern
    let mut pixels = vec![0u8; size * size];
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    // We need to modify the encoder to expose DWT coefficients
    // For now, let's check the QCD marker
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    }, &mut output).unwrap();
    
    fs::write("test_dwt.j2k", &output[..output_size]).unwrap();
    
    // Dump the file to see markers
    let result = Command::new("libs/bin/opj_dump.exe")
        .args(&["-i", "test_dwt.j2k"])
        .output()
        .expect("opj_dump failed");
    
    println!("{}", String::from_utf8_lossy(&result.stdout));
    
    // Let me check the actual encoded data around byte 72
    println!("\n\n=== Packet data analysis ===");
    println!("File size: {} bytes", output_size);
    
    // SOD marker is at position 71 (0xFF93)
    if output_size > 72 && output[71] == 0xFF && output[72] == 0x93 {
        println!("SOD marker found at position 71");
        let packet_data_start = 73;
        println!("Packet data starts at byte {}", packet_data_start);
        
        // Show first few bytes of packet data
        let show_len = std::cmp::min(16, output_size - packet_data_start);
        println!("First {} bytes of packet data:", show_len);
        for i in 0..show_len {
            print!("{:02X} ", output[packet_data_start + i]);
            if (i + 1) % 8 == 0 {
                println!();
            }
        }
        println!();
    }
    
    // Now let's check if the issue is in the cleanup pass or somewhere else
    // by creating a simpler test with only 1 decomposition level
    let mut encoder1 = J2kEncoder::new();
    encoder1.set_irreversible(false);
    encoder1.set_decomposition_levels(1);
    
    let mut output1 = vec![0u8; 1024 * 1024];
    let output_size1 = encoder1.encode(&pixels, &FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    }, &mut output1).unwrap();
    
    fs::write("test_dwt_1level.j2k", &output1[..output_size1]).unwrap();
    
    // Decode and check
    Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_dwt_1level.j2k", "-o", "test_dwt_1level_decoded.raw"])
        .output()
        .expect("decode failed");
    
    let decoded1 = fs::read("test_dwt_1level_decoded.raw").unwrap();
    
    let mut errors1 = 0;
    let mut total_diff1 = 0i32;
    for (_, (orig, &dec)) in pixels.iter().zip(decoded1.iter()).enumerate() {
        if *orig as i32 != dec as i32 {
            errors1 += 1;
            total_diff1 += (*orig as i32 - dec as i32).abs() as i32;
        }
    }
    
    println!("\n=== 1 decomposition level ===");
    println!("File size: {} bytes", output_size1);
    println!("Errors: {}/{}", errors1, pixels.len());
    println!("MAE: {:.4}", total_diff1 as f64 / pixels.len() as f64);
    
    // Now decode the 2-level case (we already did this above)
    Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_dwt.j2k", "-o", "test_dwt_2level_decoded.raw"])
        .output()
        .expect("decode failed");
    
    let decoded2 = fs::read("test_dwt_2level_decoded.raw").unwrap();
    
    let mut errors2 = 0;
    let mut total_diff2 = 0i32;
    for (_, (orig, &dec)) in pixels.iter().zip(decoded2.iter()).enumerate() {
        if *orig as i32 != dec as i32 {
            errors2 += 1;
            total_diff2 += (*orig as i32 - dec as i32).abs() as i32;
        }
    }
    
    println!("\n=== 2 decomposition levels ===");
    println!("File size: {} bytes", output_size);
    println!("Errors: {}/{}", errors2, pixels.len());
    println!("MAE: {:.4}", total_diff2 as f64 / pixels.len() as f64);
}

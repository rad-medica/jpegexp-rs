// Debug test to understand CharLS RGB decoding failure
//
// This test loads a CharLS-encoded RGB image and traces through the decoding process
// to identify exactly where and why the "Invalid data" error occurs.

use std::fs;

#[test]
#[ignore = "RGB decoding debug test - known issue with bit over-consumption"]
fn debug_charls_rgb_decode() {
    // Use the smallest available RGB test image
    let jls_path = "tests/fixtures/jpegls/small_16x16_rgb_checker.jls";
    let raw_path = "tests/fixtures/jpegls/small_16x16_rgb_checker.raw";

    let jls_data =
        fs::read(jls_path).unwrap_or_else(|e| panic!("Failed to read {}: {}", jls_path, e));
    let expected_data =
        fs::read(raw_path).unwrap_or_else(|e| panic!("Failed to read {}: {}", raw_path, e));

    println!("\n=== CharLS RGB Image Debug ===");
    println!("JLS file size: {} bytes", jls_data.len());
    println!(
        "RAW file size: {} bytes (expect 16x16x3 = 768 bytes)",
        expected_data.len()
    );

    // Dump first 100 bytes of JLS file to see header
    println!("\nJLS header (first 100 bytes):");
    for (i, chunk) in jls_data.iter().take(100).enumerate() {
        if i % 16 == 0 {
            print!("\n{:04x}: ", i);
        }
        print!("{:02x} ", chunk);
    }
    println!("\n");

    // Try to decode
    let mut decoder = jpegexp_rs::jpegls::JpeglsDecoder::new(&jls_data);

    // Read header
    match decoder.read_header() {
        Ok(_) => {
            let frame_info = decoder.frame_info();
            println!("✓ Header read successfully:");
            println!("  Width: {}", frame_info.width);
            println!("  Height: {}", frame_info.height);
            println!("  Components: {}", frame_info.component_count);
            println!("  Bits per sample: {}", frame_info.bits_per_sample);

            // Try to decode
            let buffer_size =
                (frame_info.width * frame_info.height * frame_info.component_count as u32) as usize;
            let mut decoded_data = vec![0u8; buffer_size];

            println!("\nAttempting decode of {} bytes...", buffer_size);

            match decoder.decode(&mut decoded_data) {
                Ok(_) => {
                    println!("✓ Decode successful!");

                    // Compare with expected
                    let mismatches: Vec<_> = decoded_data
                        .iter()
                        .zip(expected_data.iter())
                        .enumerate()
                        .filter(|(_, (a, b))| a != b)
                        .take(10)
                        .collect();

                    if mismatches.is_empty() {
                        println!("✓✓ Perfect match with expected data!");
                    } else {
                        println!("✗ Pixel mismatches found:");
                        for (idx, (got, expected)) in mismatches {
                            println!("  Byte {}: got {}, expected {}", idx, got, expected);
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Decode failed: {}", e);
                    println!("  This is the error we're investigating");
                    panic!("Decode failed");
                }
            }
        }
        Err(e) => {
            println!("✗ Header read failed: {:?}", e);
            println!("  Cannot proceed with decode");
        }
    }
}

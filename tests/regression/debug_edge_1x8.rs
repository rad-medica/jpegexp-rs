// Debug test for 1x8 edge case
use jpegexp_rs::jpegls::JpeglsDecoder;
use std::fs;

#[test]
fn debug_edge_1x8() {
    // Load the CharLS-encoded file
    let jls_data = fs::read("tests/fixtures/jpegls/edge_1x8_gray.jls").expect(
        "Failed to read edge_1x8_gray.jls",
    );

    // Load expected data
    let expected = fs::read("tests/fixtures/jpegls/edge_1x8_gray.raw").expect(
        "Failed to read edge_1x8_gray.raw",
    );

    println!("Expected data: {:?}", expected);

    // Enable debug logging
    std::env::set_var("JPEGLS_DEBUG", "1");

    let mut decoder = JpeglsDecoder::new(&jls_data);
    decoder.read_header().expect("Failed to read header");

    let info = decoder.frame_info();
    println!(
        "Frame info: {}x{}, {} components, {} bits",
        info.width,
        info.height,
        info.component_count,
        info.bits_per_sample
    );

    let mut decoded = vec![0u8; expected.len()];
    match decoder.decode(&mut decoded) {
        Ok(_) => {
            println!("Decoded data: {:?}", decoded);
            println!("Differences:");
            for (i, (exp, got)) in expected.iter().zip(decoded.iter()).enumerate() {
                if exp != got {
                    println!(
                        "  Byte {}: expected {}, got {} (diff={})",
                        i,
                        exp,
                        got,
                        (*got as i32 - *exp as i32)
                    );
                }
            }
        }
        Err(e) => {
            panic!("Decode failed: {:?}", e);
        }
    }
}

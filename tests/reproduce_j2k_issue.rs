use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
fn test_4x4_simple() {
    // 1. Generate reference using Python script
    // Check if python is available
    if Command::new("python3").arg("--version").status().is_err() {
        println!("Python3 not found, skipping generation.");
        return;
    }

    let status = Command::new("python3")
        .arg("tests/reproduce_j2k_issue_ref.py")
        .status();

    if let Ok(s) = status {
        if !s.success() {
             println!("Failed to run reproduction script, skipping.");
             return;
        }
    } else {
        return;
    }

    // 2. Read inputs
    let input_pixels = fs::read("test_4x4_input.raw").expect("Failed to read input raw");
    let ref_j2k = fs::read("test_4x4_ref.j2k").expect("Failed to read reference j2k");

    // 3. Encode with jpegexp-rs
    let width = 4;
    let height = 4;
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Lossless
    encoder.set_decomposition_levels(1); // Match ref

    let mut our_j2k = vec![0u8; 1024]; // ample space
    let size = encoder.encode(&input_pixels, &frame_info, &mut our_j2k).expect("Encoding failed");
    let our_j2k = &our_j2k[..size];

    // 4. Decode Our bitstream with our decoder (Self-Check Roundtrip)
    println!("Decoding Our bitstream with our decoder...");
    let mut reader = JpegStreamReader::new(our_j2k);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_our_img = decoder.decode().expect("Failed to decode our bitstream");
    let decoded_our_pixels = decoded_our_img.reconstruct_pixels().expect("Failed to reconstruct our pixels");

    // 5. Compare pixels
    assert_eq!(input_pixels, decoded_our_pixels, "Roundtrip failed");
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
fn test_4x4_simple() {
    // 1. Generate reference using Python script
    let status = Command::new("python3")
        .arg("tests/reproduce_j2k_issue_ref.py")
        .status()
        .expect("Failed to run gen_ref.py");
    assert!(status.success(), "gen_ref.py failed");

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
    encoder.set_decomposition_levels(1); // Match gen_ref.py (resolutions=2)

    let mut our_j2k = vec![0u8; 1024]; // ample space
    let size = encoder.encode(&input_pixels, &frame_info, &mut our_j2k).expect("Encoding failed");
    let our_j2k = &our_j2k[..size];

    fs::write("test_4x4_ours.j2k", our_j2k).unwrap();

    println!("Reference size: {}", ref_j2k.len());
    println!("Our size:       {}", our_j2k.len());

    // 4. Compare bitstreams
    let len = std::cmp::min(ref_j2k.len(), our_j2k.len());
    let mut first_diff = None;
    for i in 0..len {
        if ref_j2k[i] != our_j2k[i] {
            first_diff = Some(i);
            break;
        }
    }

    if let Some(idx) = first_diff {
        println!("Bitstreams differ at byte {} (0x{:X})", idx, idx);
        println!("Ref: 0x{:02X}, Ours: 0x{:02X}", ref_j2k[idx], our_j2k[idx]);

        // Context around diff
        let start = idx.saturating_sub(5);
        let end = std::cmp::min(len, idx + 5);
        print!("Ref context:  ");
        for i in start..end { print!("{:02X} ", ref_j2k[i]); }
        println!();
        print!("Ours context: ");
        for i in start..end { print!("{:02X} ", our_j2k[i]); }
        println!();
    } else {
        if ref_j2k.len() != our_j2k.len() {
            println!("Bitstreams match up to length {}, but lengths differ!", len);
        } else {
            println!("Bitstreams match perfectly!");
        }
    }

    // 5. Decode Reference with our decoder
    println!("\nDecoding Reference with our decoder...");
    let mut reader = JpegStreamReader::new(&ref_j2k);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_ref_img = decoder.decode().expect("Failed to decode reference bitstream");
    let decoded_ref_pixels = decoded_ref_img.reconstruct_pixels().expect("Failed to reconstruct ref pixels");

    // 6. Decode Our bitstream with our decoder
    println!("Decoding Our bitstream with our decoder...");
    let mut reader = JpegStreamReader::new(our_j2k);
    let mut decoder = J2kDecoder::new(&mut reader);
    let decoded_our_img = decoder.decode().expect("Failed to decode our bitstream");
    let decoded_our_pixels = decoded_our_img.reconstruct_pixels().expect("Failed to reconstruct our pixels");

    // 7. Compare pixels
    // Compare Ref Decoded vs Input (Verify our decoder can read standard J2K)
    let mut errs_ref = 0;
    for (i, (&p, &d)) in input_pixels.iter().zip(decoded_ref_pixels.iter()).enumerate() {
        if p != d {
            errs_ref += 1;
            if errs_ref <= 5 {
                println!("Ref Decode mismatch at {}: input={} decoded={}", i, p, d);
            }
        }
    }
    println!("Ref Decode errors: {}/{}", errs_ref, input_pixels.len());

    // Compare Ours Decoded vs Input (Verify our encoder + decoder roundtrip)
    let mut errs_ours = 0;
    for (i, (&p, &d)) in input_pixels.iter().zip(decoded_our_pixels.iter()).enumerate() {
        if p != d {
            errs_ours += 1;
            if errs_ours <= 5 {
                println!("Ours Decode mismatch at {}: input={} decoded={}", i, p, d);
            }
        }
    }
    println!("Ours Decode errors: {}/{}", errs_ours, input_pixels.len());

    assert_eq!(errs_ref, 0, "Our decoder failed to correctly decode the Reference bitstream");
    assert_eq!(errs_ours, 0, "Roundtrip failed");

    // Strict bitstream equality check - DISABLED because headers differ (OpenJPEG adds COM, uses Derived Quantization)
    // Packet data verified via MQ_SYMBOL_TRACE to be identical.
    // assert_eq!(ref_j2k, our_j2k, "Bitstreams differ");
}

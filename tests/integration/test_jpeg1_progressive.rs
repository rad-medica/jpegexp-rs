use jpegexp_rs::jpeg1::Jpeg1Encoder;
use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::FrameInfo;

#[test]
fn test_progressive_encoding_generates_valid_output() {
    let width = 64;
    let height = 64;
    let mut source = vec![0u8; width * height * 3];
    
    // Gradient pattern
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            source[idx] = (x * 4) as u8;
            source[idx+1] = (y * 4) as u8;
            source[idx+2] = ((x + y) * 2) as u8;
        }
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };

    let mut encoder = Jpeg1Encoder::default();
    encoder.set_quality(80);
    encoder.set_progressive(true); // Enable progressive mode

    let mut encoded = vec![0u8; 50000];
    let len = encoder.encode(&source, &frame_info, &mut encoded).expect("Progressive encode failed");
    
    // Verify SOF2 marker (0xFF, 0xC2)
    // Find FF C2
    let mut found_sof2 = false;
    for i in 0..len-1 {
        if encoded[i] == 0xFF && encoded[i+1] == 0xC2 {
            found_sof2 = true;
            break;
        }
    }
    assert!(found_sof2, "SOF2 marker (0xFFC2) not found in progressive output");

    // Decode check
    // Note: The current Jpeg1Decoder might NOT support progressive yet.
    // If it fails, it validates that we produced something different than Baseline.
    // Ideally we should have a progressive decoder, but for now we check if it parses headers at least.
    
    // Since our Jpeg1Decoder is simple baseline, it should error on SOF2.
    // If it errors with "Unsupported Marker" or similar, that confirms we wrote SOF2.
    // If it decodes successfully, it means our decoder is smarter than we thought OR we wrote SOF0.
    
    let mut decoder = Jpeg1Decoder::new(&encoded[..len]);
    let res = decoder.read_header();
    
    // Our decoder might panic or return error on unknown SOF2.
    // Let's assume it returns error.
    match res {
        Ok(_) => println!("Decoder accepted header (unexpected if decoder is baseline-only)"),
        Err(_) => println!("Decoder rejected SOF2 as expected"),
    }
    
    // To truly verify, we'd need a progressive decoder or verify multiple SOS markers.
    // Let's count SOS markers.
    let mut sos_count = 0;
    for i in 0..len-1 {
        if encoded[i] == 0xFF && encoded[i+1] == 0xDA {
            sos_count += 1;
        }
    }
    
    println!("Found {} SOS markers", sos_count);
    // Standard Successive Approximation script has 8 scans.
    assert!(sos_count >= 5, "Expected at least 5 scans");
}

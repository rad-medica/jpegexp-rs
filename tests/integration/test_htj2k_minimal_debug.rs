use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::fs;

#[test]
fn test_htj2k_2x2_solid() {
    println!("\n=== Testing 2x2 solid (all 128) ===");
    
    // Create 2x2 image, all pixels = 128
    let pixels = vec![128u8; 4];
    
    let frame_info = FrameInfo {
        width: 2,
        height: 2,
        bits_per_sample: 8,
        component_count: 1,
    };

    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_htj2k(true);
    encoder.set_irreversible(false); // Lossless

    let mut output = vec![0u8; 4096];
    let len = encoder.encode(&pixels, &frame_info, &mut output).expect("Encode failed");
    output.truncate(len);
    
    println!("Encoded {} bytes", len);
    
    // Save for inspection
    fs::write("test_htj2k_2x2_solid.j2c", &output).ok();
    println!("Saved to test_htj2k_2x2_solid.j2c");

    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decode failed");

    // Reconstruct pixels
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruct failed");
    
    println!("\nOriginal pixels: {:?}", pixels);
    println!("Decoded pixels:  {:?}", decoded_pixels);
    
    // Verify exact match
    for (i, (&orig, &dec)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
        if orig != dec {
            println!("MISMATCH at pixel {}: expected {}, got {}", i, orig, dec);
        }
    }
    
    assert_eq!(decoded_pixels, pixels, "Pixel mismatch");
}

#[test]
fn test_htj2k_2x2_gradient() {
    println!("\n=== Testing 2x2 gradient ===");
    
    // Create 2x2 gradient: [0, 85, 170, 255]
    let pixels = vec![0u8, 85, 170, 255];
    
    let frame_info = FrameInfo {
        width: 2,
        height: 2,
        bits_per_sample: 8,
        component_count: 1,
    };

    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_htj2k(true);
    encoder.set_irreversible(false); // Lossless

    let mut output = vec![0u8; 4096];
    let len = encoder.encode(&pixels, &frame_info, &mut output).expect("Encode failed");
    output.truncate(len);
    
    println!("Encoded {} bytes", len);
    
    // Save for inspection
    fs::write("test_htj2k_2x2_gradient.j2c", &output).ok();
    println!("Saved to test_htj2k_2x2_gradient.j2c");

    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decode failed");

    // Reconstruct pixels
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruct failed");
    
    println!("\nOriginal pixels: {:?}", pixels);
    println!("Decoded pixels:  {:?}", decoded_pixels);
    
    // Verify exact match
    for (i, (&orig, &dec)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
        if orig != dec {
            println!("MISMATCH at pixel {}: expected {}, got {}", i, orig, dec);
        }
    }
    
    assert_eq!(decoded_pixels, pixels, "Pixel mismatch");
}

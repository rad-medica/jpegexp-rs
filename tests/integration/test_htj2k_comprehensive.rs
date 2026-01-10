use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn roundtrip_test(width: u32, height: u32, depth: u8, components: u8) {
    println!("Testing {}x{} depth={} comps={}", width, height, depth, components);
    
    // Generate data
    let mut pixels = vec![0u8; (width * height * components as u32 * if depth > 8 { 2 } else { 1 }) as usize];
    for i in 0..pixels.len() {
        pixels[i] = (i % 255) as u8; 
        if depth > 8 && i % 2 == 1 { pixels[i] = (i % 16) as u8; } // High byte
    }

    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: depth as i32,
        component_count: components as i32,
    };

    // Encode
    let mut encoder = J2kEncoder::new();
    encoder.set_htj2k(true);
    encoder.set_irreversible(false); // Lossless

    let mut output = vec![0u8; pixels.len() * 2 + 2048];
    let len = encoder.encode(&pixels, &frame_info, &mut output).expect("Encode failed");
    output.truncate(len);
    
    // Save for inspection
    let output_path = std::path::Path::new("C:/Users/aroja/CODE/jpegexp-rs/test_htj2k_2x2_gradient_ours.j2k");
    if let Err(e) = std::fs::write(output_path, &output) {
        eprintln!("Failed to write output file: {}", e);
    } else {
        println!("Saved output to {}", output_path.display());
    }
    
    println!("Encoded {} bytes", len);

    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decode failed");

    // Reconstruct pixels
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruct failed");
    
    println!("\nOriginal pixels: {:?}", pixels);
    println!("Decoded pixels:  {:?}", decoded_pixels);
    
    // Verify exact match
    let mut mismatches = 0;
    for (i, (&orig, &dec)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
        if orig != dec {
            println!("MISMATCH at pixel {}: expected {}, got {}", i, orig, dec);
            mismatches += 1;
        }
    }
    
    assert_eq!(mismatches, 0, "Found {} pixel mismatches", mismatches);
}

#[test]

fn test_htj2k_2x2_gradient() {
    println!("\n=== Testing HTJ2K 2x2 gradient ===");
    
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
    encoder.set_decomposition_levels(0); // NO DWT

    let mut output = vec![0u8; 4096];
    let len = encoder.encode(&pixels, &frame_info, &mut output).expect("Encode failed");
    output.truncate(len);
    
    // Save for inspection
    let output_path = std::path::Path::new("C:/Users/aroja/CODE/jpegexp-rs/test_htj2k_2x2_gradient_ours.j2k");
    if let Err(e) = std::fs::write(output_path, &output) {
        eprintln!("Failed to write output file: {}", e);
    } else {
        println!("Saved output to {}", output_path.display());
    }
    
    println!("Encoded {} bytes", len);

    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decode failed");

    // Reconstruct pixels
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruct failed");
    
    println!("\nOriginal pixels: {:?}", pixels);
    println!("Decoded pixels:  {:?}", decoded_pixels);
    
    // Verify exact match
    let mut mismatches = 0;
    for (i, (&orig, &dec)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
        if orig != dec {
            println!("MISMATCH at pixel {}: expected {}, got {}", i, orig, dec);
            mismatches += 1;
        }
    }
    
    assert_eq!(mismatches, 0, "Found {} pixel mismatches", mismatches);
}

#[test]

fn test_htj2k_12bit_gray() {
    roundtrip_test(64, 64, 12, 1);
}

#[test]

fn test_htj2k_16bit_gray() {
    roundtrip_test(64, 64, 16, 1);
}

#[test]

fn test_htj2k_8bit_rgb() {
    roundtrip_test(64, 64, 8, 3);
}

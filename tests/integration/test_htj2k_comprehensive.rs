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

    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decode failed");

    // Verify Metadata
    assert_eq!(image.width, width);
    assert_eq!(image.height, height);
    assert_eq!(image.component_count, components as u32);
    
    // Verify HTJ2K Signaling
    assert!(image.cap.is_some(), "CAP marker missing");
    if let Some(cap) = &image.cap {
        let ht_bit = 0x00020000;
        assert_eq!(cap.pcap & ht_bit, ht_bit, "HTJ2K bit not set in CAP");
        assert_eq!(cap.ccap.len(), components as usize, "Ccap length mismatch");
    }

    // Verify Pixels
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruct failed");
    assert_eq!(decoded_pixels.len(), pixels.len());
    
    // Check exact match (Lossless)
    let mismatch = pixels.iter().zip(decoded_pixels.iter()).filter(|(a, b)| a != b).count();
    
    if mismatch > 0 {
        println!("Mismatch count: {}", mismatch);
        let mut printed = 0;
        for (i, (a, b)) in pixels.iter().zip(decoded_pixels.iter()).enumerate() {
            if a != b {
                println!("  idx {}: expected {}, got {}", i, a, b);
                printed += 1;
                if printed > 10 { break; }
            }
        }
    }

    assert_eq!(mismatch, 0, "Pixel mismatch count: {}", mismatch);
}

#[test]
fn test_htj2k_8bit_gray() {
    roundtrip_test(64, 64, 8, 1);
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

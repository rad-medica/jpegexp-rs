use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_htj2k_encode_basic() {
    let width = 64u32;
    let height = 64u32;
    let components = 1;
    let depth = 8;
    
    // Gradient image
    let mut original = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 2) as u8;
            original.push(val);
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: depth,
        component_count: components as i32,
    };
    
    // 1. Encode with Standard J2K
    let mut encoded_std = vec![0u8; 64 * 1024];
    let mut encoder_std = J2kEncoder::new();
    encoder_std.set_irreversible(false);
    encoder_std.set_htj2k(false);
    let len_std = encoder_std.encode(&original, &frame_info, &mut encoded_std).expect("Std encode failed");
    
    // 2. Encode with HTJ2K
    let mut encoded_ht = vec![0u8; 64 * 1024];
    let mut encoder_ht = J2kEncoder::new();
    encoder_ht.set_irreversible(false);
    encoder_ht.set_htj2k(true);
    let len_ht = encoder_ht.encode(&original, &frame_info, &mut encoded_ht).expect("HT encode failed");
    
    println!("Standard J2K size: {}", len_std);
    println!("HTJ2K size: {}", len_ht);
    
    // HTJ2K should be different
    assert_ne!(len_std, len_ht, "HTJ2K should produce different output");
    
    // Check for CAP marker (0xFF50) in HTJ2K stream
    // SOC (FF4F) + SIZ (FF51) ... CAP (FF50)
    // CAP usually appears early.
    let has_cap = encoded_ht[..len_ht].windows(2).any(|w| w == [0xFF, 0x50]);
    assert!(has_cap, "HTJ2K stream must have CAP marker");
    
    // Attempt decode? (If our decoder supports it)
    // Note: Our J2kDecoder might not support HTJ2K fully yet.
    // But let's try.
    let mut reader = JpegStreamReader::new(&encoded_ht[..len_ht]);
    let mut decoder = J2kDecoder::new(&mut reader);
    
    match decoder.decode() {
        Ok(image) => {
            println!("Decoder successfully parsed HTJ2K headers");
            // Reconstruction might fail if block decoding isn't implemented
            // in the decoder for HT blocks.
            // Check if we can reconstruct
            match image.reconstruct_pixels() {
                Ok(recon) => {
                    println!("Reconstruction successful");
                    // Check MAE
                    let mae: f64 = original.iter().zip(recon.iter())
                        .map(|(a, b)| (*a as i32 - *b as i32).abs() as f64)
                        .sum::<f64>() / original.len() as f64;
                    println!("HTJ2K MAE: {:.4}", mae);
                },
                Err(e) => println!("Reconstruction failed (expected if decoder incomplete): {}", e),
            }
        },
        Err(e) => println!("Decoder failed to parse headers: {}", e),
    }
}

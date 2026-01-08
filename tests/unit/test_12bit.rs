use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_12bit_grayscale_lossless() {
    let width = 64u32;
    let height = 64u32;
    let depth = 12;
    
    // Create 12-bit gradient
    // Store as u16 (2 bytes per pixel)
    let mut original = Vec::with_capacity((width * height * 2) as usize);
    let mut original_u16 = Vec::with_capacity((width * height) as usize);
    
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 32).min(4095) as u16;
            original_u16.push(val);
            // Little endian or Big endian?
            // Encoder expects native slice of bytes?
            // "If bits_per_sample > 8, pixels are treated as u16 slice cast to u8"
            // Actually, we need to check how encoder handles input.
            // For now assume standard native endian u16 slice cast to u8 slice.
            let bytes = val.to_ne_bytes();
            original.push(bytes[0]);
            original.push(bytes[1]);
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: depth,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded 12-bit image: {} bytes", encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    // Compare
    assert_eq!(reconstructed.len(), original.len());
    
    // Reconstruct u16
    let mut diffs = 0;
    for i in 0..original_u16.len() {
        let val_orig = original_u16[i];
        let val_recon = u16::from_ne_bytes([reconstructed[i*2], reconstructed[i*2+1]]);
        
        if val_orig != val_recon {
            diffs += 1;
            if diffs < 10 {
                println!("Mismatch at {}: orig={}, recon={}", i, val_orig, val_recon);
            }
        }
    }
    
    assert_eq!(diffs, 0, "12-bit roundtrip failed with {} mismatches", diffs);
}

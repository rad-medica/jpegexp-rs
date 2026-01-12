use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn repro_j2k_2x2_gradient_coeffs() {
    // 2x2 Gradient (0..3)
    // 0 1
    // 2 3
    let width = 2u32;
    let height = 2u32;
    let mut pixels = vec![0u8; 4];
    pixels[0] = 0; pixels[1] = 1;
    pixels[2] = 2; pixels[3] = 3;
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    
    let frame_info = FrameInfo { width, height, bits_per_sample: 8, component_count: 1 };
    let mut encoded_buffer = vec![0u8; 1024];
    encoder.encode(&pixels, &frame_info, &mut encoded_buffer).unwrap();
    
    // Check output visually in stdout
}

#[test]
fn repro_j2k_gradient_mae_8bit() {
    // 1. Create 64x64 diagonal gradient (8-bit grayscale)
    let width = 64u32;
    let height = 64u32;
    let mut pixels = vec![0u8; (width * height) as usize];
    
    for y in 0..height {
        for x in 0..width {
            // Gradient d: (x + y) scaled to 0..255
            let val = ((x + y) as f32 / ((width + height) as f32) * 255.0) as u8;
            pixels[(y * width + x) as usize] = val;
        }
    }
    
    // 2. Encode Lossless (Rust)
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false); // Use 5-3 Reversible transform (Lossless)
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded_buffer = vec![0u8; (width * height * 4) as usize]; // Adequate buffer
    let compressed_size = encoder.encode(&pixels, &frame_info, &mut encoded_buffer).unwrap();
    let encoded_data = &encoded_buffer[0..compressed_size];
    
    println!("Compressed size: {} bytes", compressed_size);
    
    // 3. Decode Lossless (Rust) - Self-test first
    let mut reader = JpegStreamReader::new(encoded_data);
    let mut decoder = J2kDecoder::new(&mut reader);
    let j2k_image = decoder.decode().unwrap();
    let decoded = j2k_image.reconstruct_pixels().unwrap();
    
    // 4. Calculate MAE
    let mut total_error = 0.0;
    let mut max_error = 0;
    
    assert_eq!(decoded.len(), pixels.len(), "Decoded size mismatch");
    
    for i in 0..pixels.len() {
        let diff = (pixels[i] as i32 - decoded[i] as i32).abs();
        total_error += diff as f32;
        if diff > max_error {
            max_error = diff;
        }
    }
    
    let mae = total_error / pixels.len() as f32;
    println!("Self-Roundtrip MAE: {}, Max Error: {}", mae, max_error);
    
    // If self-roundtrip fails, the bug is definitely in our DWT/Entropy logic
    assert_eq!(mae, 0.0, "MAE should be 0.0 for lossless");
}

#[test]
fn repro_j2k_gradient_mae_16bit() {
    // 1. Create 64x64 diagonal gradient (16-bit grayscale)
    let width = 64u32;
    let height = 64u32;
    let mut pixels = vec![0u8; (width * height * 2) as usize]; // 2 bytes per pixel
    
    for y in 0..height {
        for x in 0..width {
            // Gradient d: (x + y) scaled to 0..65535
            let val = ((x + y) as f32 / ((width + height) as f32) * 65535.0) as u16;
            let idx = (y * width + x) as usize * 2;
            // Little Endian
            pixels[idx] = (val & 0xFF) as u8;
            pixels[idx+1] = (val >> 8) as u8;
        }
    }
    
    // 2. Encode Lossless (Rust)
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut encoded_buffer = vec![0u8; (width * height * 4) as usize];
    let compressed_size = encoder.encode(&pixels, &frame_info, &mut encoded_buffer).unwrap();
    let encoded_data = &encoded_buffer[0..compressed_size];
    
    println!("Compressed 16-bit size: {} bytes", compressed_size);
    
    // 3. Decode Lossless (Rust)
    let mut reader = JpegStreamReader::new(encoded_data);
    let mut decoder = J2kDecoder::new(&mut reader);
    let j2k_image = decoder.decode().unwrap();
    let decoded = j2k_image.reconstruct_pixels().unwrap();
    
    // 4. Calculate MAE
    let mut total_error = 0.0;
    let mut max_error = 0;
    
    assert_eq!(decoded.len(), pixels.len(), "Decoded size mismatch");
    
    for i in 0..(pixels.len()/2) {
        let val_orig = (pixels[i*2] as u16) | ((pixels[i*2+1] as u16) << 8);
        let val_dec = (decoded[i*2] as u16) | ((decoded[i*2+1] as u16) << 8);
        
        let diff = (val_orig as i32 - val_dec as i32).abs();
        total_error += diff as f32;
        if diff > max_error {
            max_error = diff;
        }
    }
    
    let mae = total_error / (pixels.len()/2) as f32;
    println!("Self-Roundtrip 16-bit MAE: {}, Max Error: {}", mae, max_error);
    
    assert_eq!(mae, 0.0, "MAE should be 0.0 for 16-bit lossless");
}

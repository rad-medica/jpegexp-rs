use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_rgb_dwt4_debug() {
    std::env::set_var("J2K_DEBUG", "1");
    
    let size = 128;
    let dwt_level = 4;
    
    println!("\nTesting {}x{} RGB with DWT level {} (DEBUG)...", size, size, dwt_level);
    
    // Create simple gradient (easier to debug than checkerboard)
    let mut original = Vec::with_capacity((size * size * 3) as usize);
    for y in 0..size {
        for x in 0..size {
            let r = ((x * 255) / size) as u8;
            let g = ((y * 255) / size) as u8;
            let b = 128u8; // Constant blue
            
            original.push(r);
            original.push(g);
            original.push(b);
        }
    }
    
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 3,
    };
    
    let mut encoded = vec![0u8; size * size * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(dwt_level);
    
    println!("Encoding...");
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded {} bytes", encoded_len);
    
    // Decode
    println!("\nDecoding...");
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    // Calculate MAE per component
    let mut total_error = [0.0; 3];
    let mut max_error = [0; 3];
    let mut error_locations = Vec::new();
    let pixel_count = (size * size) as f64;
    
    for i in 0..(size * size) as usize {
        for c in 0..3 {
            let idx = i * 3 + c;
            let error = (original[idx] as i32 - reconstructed[idx] as i32).abs();
            total_error[c] += error as f64;
            max_error[c] = max_error[c].max(error);
            
            if error > 0 && error_locations.len() < 20 {
                let x = i % size as usize;
                let y = i / size as usize;
                error_locations.push((x, y, c, original[idx], reconstructed[idx], error));
            }
        }
    }
    
    let mae_r = total_error[0] / pixel_count;
    let mae_g = total_error[1] / pixel_count;
    let mae_b = total_error[2] / pixel_count;
    let mae_avg = (mae_r + mae_g + mae_b) / 3.0;
    
    println!("\nResults:");
    println!("  MAE: R={:.6}, G={:.6}, B={:.6}, Avg={:.6}", 
             mae_r, mae_g, mae_b, mae_avg);
    println!("  Max: R={}, G={}, B={}", 
             max_error[0], max_error[1], max_error[2]);
    
    if !error_locations.is_empty() {
        println!("\nFirst {} errors:", error_locations.len().min(10));
        for (x, y, c, orig, recon, err) in error_locations.iter().take(10) {
            let comp = ["R", "G", "B"][*c];
            println!("  ({}, {}) {}: orig={}, recon={}, err={}", 
                     x, y, comp, orig, recon, err);
        }
    }
    
    if mae_avg > 0.01 {
        panic!("Test failed with MAE={:.6}", mae_avg);
    }
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_rgb_checkerboard_dwt_progression() {
    println!("\n================================================================================");
    println!("Testing RGB Checkerboard with Different DWT Levels");
    println!("================================================================================\n");
    
    let size = 128;
    
    // Create RGB checkerboard with larger blocks (easier for DWT)
    let block_size = 8; // 8x8 blocks instead of 16x16
    
    let mut original = Vec::with_capacity((size * size * 3) as usize);
    for y in 0..size {
        for x in 0..size {
            let is_white = (x / block_size + y / block_size) % 2 == 0;
            let val = if is_white { 255u8 } else { 0 };
            
            original.push(val);          // R
            original.push(if is_white { 0 } else { 255 }); // G (inverted)
            original.push(val);          // B
        }
    }
    
    for dwt_level in 0..=5 {
        println!("Testing DWT level {}...", dwt_level);
        
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
        
        let encoded_len = match encoder.encode(&original, &frame_info, &mut encoded) {
            Ok(len) => len,
            Err(e) => {
                println!("  ❌ Encoding failed: {:?}", e);
                continue;
            }
        };
        encoded.truncate(encoded_len);
        
        // Decode
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = match decoder.decode() {
            Ok(img) => img,
            Err(e) => {
                println!("  ❌ Decoding failed: {:?}", e);
                continue;
            }
        };
        let reconstructed = match image.reconstruct_pixels() {
            Ok(data) => data,
            Err(e) => {
                println!("  ❌ Reconstruction failed: {:?}", e);
                continue;
            }
        };
        
        // Calculate MAE per component
        let mut total_error = [0.0; 3];
        let mut error_count = [0usize; 3];
        let pixel_count = (size * size) as f64;
        
        for i in 0..(size * size) as usize {
            for c in 0..3 {
                let idx = i * 3 + c;
                let error = (original[idx] as i32 - reconstructed[idx] as i32).abs();
                total_error[c] += error as f64;
                if error > 0 {
                    error_count[c] += 1;
                }
            }
        }
        
        let mae_r = total_error[0] / pixel_count;
        let mae_g = total_error[1] / pixel_count;
        let mae_b = total_error[2] / pixel_count;
        let mae_avg = (mae_r + mae_g + mae_b) / 3.0;
        
        let status = if mae_avg > 0.01 { "❌ FAIL" } else { "✅ PASS" };
        println!("  Size: {} bytes, MAE: R={:.4} G={:.4} B={:.4} Avg={:.4} {}", 
                 encoded_len, mae_r, mae_g, mae_b, mae_avg, status);
        
        if mae_avg > 0.01 {
            println!("  Error pixels: R={}/{} G={}/{} B={}/{}", 
                     error_count[0], size*size,
                     error_count[1], size*size,
                     error_count[2], size*size);
        }
    }
}

#[test]
fn test_single_component_vs_rgb_dwt4() {
    println!("\n================================================================================");
    println!("Comparing Single-Component vs RGB at DWT Level 4");
    println!("================================================================================\n");
    
    let size = 128;
    let dwt_level = 4;
    let block_size = 8;
    
    // Test single grayscale first
    println!("Testing single-component grayscale checkerboard...");
    let mut gray_data = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let is_white = (x / block_size + y / block_size) % 2 == 0;
            gray_data.push(if is_white { 255u8 } else { 0 });
        }
    }
    
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; size * size * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(dwt_level);
    
    let encoded_len = encoder.encode(&gray_data, &frame_info, &mut encoded)
        .expect("Gray encoding failed");
    encoded.truncate(encoded_len);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Gray decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Gray reconstruction failed");
    
    let mut gray_mae = 0.0;
    for i in 0..gray_data.len() {
        gray_mae += (gray_data[i] as i32 - reconstructed[i] as i32).abs() as f64;
    }
    gray_mae /= gray_data.len() as f64;
    
    println!("  Grayscale MAE: {:.6}", gray_mae);
    
    // Now test RGB with same pattern
    println!("\nTesting RGB checkerboard...");
    let mut rgb_data = Vec::with_capacity((size * size * 3) as usize);
    for y in 0..size {
        for x in 0..size {
            let is_white = (x / block_size + y / block_size) % 2 == 0;
            let val = if is_white { 255u8 } else { 0 };
            rgb_data.push(val);
            rgb_data.push(if is_white { 0 } else { 255 });
            rgb_data.push(val);
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
    
    let encoded_len = encoder.encode(&rgb_data, &frame_info, &mut encoded)
        .expect("RGB encoding failed");
    encoded.truncate(encoded_len);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("RGB decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("RGB reconstruction failed");
    
    let mut rgb_mae = [0.0; 3];
    for i in 0..(size * size) as usize {
        for c in 0..3 {
            let idx = i * 3 + c;
            rgb_mae[c] += (rgb_data[idx] as i32 - reconstructed[idx] as i32).abs() as f64;
        }
    }
    for c in 0..3 {
        rgb_mae[c] /= (size * size) as f64;
    }
    
    println!("  RGB MAE: R={:.6} G={:.6} B={:.6}", rgb_mae[0], rgb_mae[1], rgb_mae[2]);
    
    println!("\nComparison:");
    println!("  Grayscale: {}", if gray_mae < 0.01 { "✅ PASS" } else { "❌ FAIL" });
    println!("  RGB R:     {}", if rgb_mae[0] < 0.01 { "✅ PASS" } else { "❌ FAIL" });
    println!("  RGB G:     {}", if rgb_mae[1] < 0.01 { "✅ PASS" } else { "❌ FAIL" });
    println!("  RGB B:     {}", if rgb_mae[2] < 0.01 { "✅ PASS" } else { "❌ FAIL" });
}

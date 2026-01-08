use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_rgb_blocksize_dwt_interaction() {
    println!("\n================================================================================");
    println!("Testing RGB Checkerboard: Block Size vs DWT Level");
    println!("================================================================================\n");
    
    let size = 128;
    let block_sizes = vec![4, 8, 16, 32];
    let dwt_levels = vec![2, 3, 4, 5];
    
    println!("Size: {}x{}", size, size);
    println!();
    println!("Block\\DWT |   2   |   3   |   4   |   5   |");
    println!("----------|-------|-------|-------|-------|");
    
    for block_size in &block_sizes {
        print!("{:4}x{:<4} |", block_size, block_size);
        
        for &dwt_level in &dwt_levels {
            // Create checkerboard
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
            
            match encoder.encode(&rgb_data, &frame_info, &mut encoded) {
                Err(_) => {
                    print!(" ENC! |");
                    continue;
                }
                Ok(len) => {
                    encoded.truncate(len);
                }
            }
            
            // Decode
            let mut reader = JpegStreamReader::new(&encoded);
            let mut decoder = J2kDecoder::new(&mut reader);
            let reconstructed = match decoder.decode() {
                Err(_) => {
                    print!(" DEC! |");
                    continue;
                }
                Ok(image) => match image.reconstruct_pixels() {
                    Err(_) => {
                        print!(" REC! |");
                        continue;
                    }
                    Ok(data) => data,
                }
            };
            
            // Calculate MAE
            let mut total_error = 0.0;
            for i in 0..rgb_data.len() {
                total_error += (rgb_data[i] as i32 - reconstructed[i] as i32).abs() as f64;
            }
            let mae = total_error / rgb_data.len() as f64;
            
            if mae < 0.01 {
                print!("  ✅  |");
            } else {
                print!(" {:.1} |", mae);
            }
        }
        println!();
    }
    
    println!();
    println!("Legend: ✅ = MAE < 0.01 (PASS), X.X = MAE value (FAIL)");
}

#[test]
fn test_dwt3_failure_investigation() {
    std::env::set_var("J2K_DEBUG", "1");
    
    let size = 128;
    let block_size = 8;
    let dwt_level = 3;
    
    println!("\n================================================================================");
    println!("Investigating DWT Level 3 Failure with 8x8 Blocks");
    println!("================================================================================\n");
    
    // Create checkerboard
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
    
    println!("Encoding {}x{} RGB checkerboard ({}x{} blocks) with DWT level {}...", 
             size, size, block_size, block_size, dwt_level);
    
    let encoded_len = encoder.encode(&rgb_data, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded {} bytes", encoded_len);
    
    // Decode
    println!("\nDecoding...");
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    // Analyze errors
    let mut total_error = [0.0; 3];
    let mut error_locations = Vec::new();
    
    for i in 0..(size * size) as usize {
        for c in 0..3 {
            let idx = i * 3 + c;
            let error = (rgb_data[idx] as i32 - reconstructed[idx] as i32).abs();
            total_error[c] += error as f64;
            
            if error > 0 && error_locations.len() < 20 {
                let x = i % size as usize;
                let y = i / size as usize;
                error_locations.push((x, y, c, rgb_data[idx], reconstructed[idx], error));
            }
        }
    }
    
    let pixel_count = (size * size) as f64;
    let mae = [
        total_error[0] / pixel_count,
        total_error[1] / pixel_count,
        total_error[2] / pixel_count,
    ];
    
    println!("\nResults:");
    println!("  MAE: R={:.6} G={:.6} B={:.6}", mae[0], mae[1], mae[2]);
    
    if !error_locations.is_empty() {
        println!("\nFirst {} errors:", error_locations.len().min(10));
        for (x, y, c, orig, recon, err) in error_locations.iter().take(10) {
            let comp = ["R", "G", "B"][*c];
            println!("  ({:3},{:3}) {}: orig={:3}, recon={:3}, err={}", 
                     x, y, comp, orig, recon, err);
        }
    }
}

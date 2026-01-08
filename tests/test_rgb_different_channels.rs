/// Test RGB with DIFFERENT patterns in each channel

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn mae(a: &[u8], b: &[u8]) -> f64 {
    let sum: u64 = a.iter().zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

#[test]
fn test_rgb_different_patterns_per_channel() {
    println!("\n================================================================================");
    println!("Testing RGB with DIFFERENT Patterns in Each Channel");
    println!("================================================================================\n");

    let width = 128;
    let height = 128;
    let block_size = 8;
    let dwt_level = 3;

    println!("Test Configuration:");
    println!("  Image: {}x{}", width, height);
    println!("  Block size: {}x{}", block_size, block_size);
    println!("  DWT level: {}\n", dwt_level);

    // Test 1: All channels same (R=G=B checkerboard) - SHOULD PASS
    println!("Test 1: R=G=B (identical checkerboard in all channels)");
    {
        let mut rgb_same = Vec::with_capacity(width * height * 3);
        for y in 0..height {
            for x in 0..width {
                let is_white = (x / block_size + y / block_size) % 2 == 0;
                let val = if is_white { 255u8 } else { 0 };
                rgb_same.push(val); // R
                rgb_same.push(val); // G
                rgb_same.push(val); // B
            }
        }
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 3,
        };
        
        let mut encoded = vec![0u8; width * height * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&rgb_same, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&rgb_same, &decoded);
        println!("  MAE: {:.6} {}\n", error, if error < 0.01 { "✅ PASS" } else { "❌ FAIL" });
    }

    // Test 2: G channel inverted (R=B checkerboard, G inverted) - THIS MIGHT FAIL
    println!("Test 2: R=B (checkerboard), G inverted");
    {
        let mut rgb_g_inverted = Vec::with_capacity(width * height * 3);
        for y in 0..height {
            for x in 0..width {
                let is_white = (x / block_size + y / block_size) % 2 == 0;
                let val = if is_white { 255u8 } else { 0 };
                rgb_g_inverted.push(val);          // R
                rgb_g_inverted.push(if is_white { 0 } else { 255 }); // G inverted
                rgb_g_inverted.push(val);          // B
            }
        }
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 3,
        };
        
        let mut encoded = vec![0u8; width * height * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&rgb_g_inverted, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&rgb_g_inverted, &decoded);
        println!("  MAE: {:.6} {}", error, if error < 0.01 { "✅ PASS" } else { "❌ FAIL" });
        
        if error > 0.01 {
            // Calculate per-component errors
            let mut r_total = 0.0;
            let mut g_total = 0.0;
            let mut b_total = 0.0;
            let pixel_count = width * height;
            
            for i in 0..pixel_count {
                let idx = i * 3;
                r_total += (rgb_g_inverted[idx] as i32 - decoded[idx] as i32).abs() as f64;
                g_total += (rgb_g_inverted[idx+1] as i32 - decoded[idx+1] as i32).abs() as f64;
                b_total += (rgb_g_inverted[idx+2] as i32 - decoded[idx+2] as i32).abs() as f64;
            }
            
            println!("  Per-component MAE:");
            println!("    R: {:.6}", r_total / pixel_count as f64);
            println!("    G: {:.6}", g_total / pixel_count as f64);
            println!("    B: {:.6}", b_total / pixel_count as f64);
        }
        println!();
    }

    // Test 3: All channels different patterns
    println!("Test 3: R (checkerboard 8x8), G (checkerboard 16x16), B (solid)");
    {
        let mut rgb_all_different = Vec::with_capacity(width * height * 3);
        for y in 0..height {
            for x in 0..width {
                // R: 8x8 checkerboard
                let r_white = (x / 8 + y / 8) % 2 == 0;
                let r = if r_white { 255u8 } else { 0 };
                
                // G: 16x16 checkerboard
                let g_white = (x / 16 + y / 16) % 2 == 0;
                let g = if g_white { 255u8 } else { 0 };
                
                // B: solid 128
                let b = 128u8;
                
                rgb_all_different.push(r);
                rgb_all_different.push(g);
                rgb_all_different.push(b);
            }
        }
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 3,
        };
        
        let mut encoded = vec![0u8; width * height * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&rgb_all_different, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&rgb_all_different, &decoded);
        println!("  MAE: {:.6} {}", error, if error < 0.01 { "✅ PASS" } else { "❌ FAIL" });
        
        if error > 0.01 {
            // Calculate per-component errors
            let mut r_total = 0.0;
            let mut g_total = 0.0;
            let mut b_total = 0.0;
            let pixel_count = width * height;
            
            for i in 0..pixel_count {
                let idx = i * 3;
                r_total += (rgb_all_different[idx] as i32 - decoded[idx] as i32).abs() as f64;
                g_total += (rgb_all_different[idx+1] as i32 - decoded[idx+1] as i32).abs() as f64;
                b_total += (rgb_all_different[idx+2] as i32 - decoded[idx+2] as i32).abs() as f64;
            }
            
            println!("  Per-component MAE:");
            println!("    R: {:.6}", r_total / pixel_count as f64);
            println!("    G: {:.6}", g_total / pixel_count as f64);
            println!("    B: {:.6}", b_total / pixel_count as f64);
        }
        println!();
    }

    println!("================================================================================");
    println!("Expected: Test 1 passes, Test 2 might fail (G channel inverted causes issue)");
    println!("================================================================================");
}

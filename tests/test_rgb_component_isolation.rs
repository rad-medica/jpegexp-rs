/// Test individual RGB components as grayscale to isolate the issue

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn create_checkerboard(width: usize, height: usize, block_size: usize) -> Vec<u8> {
    let mut data = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            let block_x = x / block_size;
            let block_y = y / block_size;
            let is_white = (block_x + block_y) % 2 == 0;
            data[y * width + x] = if is_white { 255 } else { 0 };
        }
    }
    data
}

fn create_checkerboard_rgb(width: usize, height: usize, block_size: usize) -> Vec<u8> {
    let mut data = vec![0u8; width * height * 3];
    for y in 0..height {
        for x in 0..width {
            let block_x = x / block_size;
            let block_y = y / block_size;
            let is_white = (block_x + block_y) % 2 == 0;
            let val = if is_white { 255 } else { 0 };
            let idx = (y * width + x) * 3;
            data[idx] = val;     // R
            data[idx + 1] = val; // G
            data[idx + 2] = val; // B
        }
    }
    data
}

fn mae(a: &[u8], b: &[u8]) -> f64 {
    let sum: u64 = a.iter().zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

#[test]
fn test_rgb_components_separately() {
    println!("\n================================================================================");
    println!("Testing RGB Checkerboard: Each Component Encoded Separately as Grayscale");
    println!("================================================================================\n");

    let width = 128;
    let height = 128;
    let block_size = 8;
    let dwt_level = 3; // This is the failing level for 8x8 blocks in RGB

    println!("Image: {}x{}, Block: {}x{}, DWT Level: {}\n", width, height, block_size, block_size, dwt_level);

    let checkerboard_gray = create_checkerboard(width, height, block_size);
    let checkerboard_rgb = create_checkerboard_rgb(width, height, block_size);

    // Test 1: Grayscale checkerboard (should pass)
    println!("Test 1: Grayscale checkerboard");
    {
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };
        
        let mut encoded = vec![0u8; width * height * 2];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&checkerboard_gray, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&checkerboard_gray, &decoded);
        println!("  MAE: {:.6} {}", error, if error < 0.01 { "✅" } else { "❌" });
    }

    // Test 2: R channel only as grayscale
    println!("\nTest 2: R channel only (as grayscale)");
    {
        let r_channel: Vec<u8> = (0..width*height).map(|i| checkerboard_rgb[i*3]).collect();
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };
        
        let mut encoded = vec![0u8; width * height * 2];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&r_channel, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&r_channel, &decoded);
        println!("  MAE: {:.6} {}", error, if error < 0.01 { "✅" } else { "❌" });
    }

    // Test 3: G channel only as grayscale
    println!("\nTest 3: G channel only (as grayscale)");
    {
        let g_channel: Vec<u8> = (0..width*height).map(|i| checkerboard_rgb[i*3+1]).collect();
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };
        
        let mut encoded = vec![0u8; width * height * 2];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&g_channel, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&g_channel, &decoded);
        println!("  MAE: {:.6} {}", error, if error < 0.01 { "✅" } else { "❌" });
    }

    // Test 4: B channel only as grayscale
    println!("\nTest 4: B channel only (as grayscale)");
    {
        let b_channel: Vec<u8> = (0..width*height).map(|i| checkerboard_rgb[i*3+2]).collect();
        
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };
        
        let mut encoded = vec![0u8; width * height * 2];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&b_channel, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&b_channel, &decoded);
        println!("  MAE: {:.6} {}", error, if error < 0.01 { "✅" } else { "❌" });
    }

    // Test 5: Full RGB checkerboard (should fail)
    println!("\nTest 5: Full RGB checkerboard (3 components)");
    {
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
        
        let encoded_len = encoder.encode(&checkerboard_rgb, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let error = mae(&checkerboard_rgb, &decoded);
        println!("  MAE: {:.6} {}", error, if error < 0.01 { "✅" } else { "❌ EXPECTED FAILURE" });
        
        if error > 0.01 {
            // Calculate per-component errors
            let mut r_errors = vec![];
            let mut g_errors = vec![];
            let mut b_errors = vec![];
            
            for i in 0..width*height {
                let idx = i * 3;
                r_errors.push((checkerboard_rgb[idx] as i32 - decoded[idx] as i32).abs());
                g_errors.push((checkerboard_rgb[idx+1] as i32 - decoded[idx+1] as i32).abs());
                b_errors.push((checkerboard_rgb[idx+2] as i32 - decoded[idx+2] as i32).abs());
            }
            
            let r_mae: f64 = r_errors.iter().map(|&e| e as f64).sum::<f64>() / r_errors.len() as f64;
            let g_mae: f64 = g_errors.iter().map(|&e| e as f64).sum::<f64>() / g_errors.len() as f64;
            let b_mae: f64 = b_errors.iter().map(|&e| e as f64).sum::<f64>() / b_errors.len() as f64;
            
            println!("\n  Per-component MAE:");
            println!("    R: {:.6}", r_mae);
            println!("    G: {:.6}", g_mae);
            println!("    B: {:.6}", b_mae);
            
            let r_nonzero = r_errors.iter().filter(|&&e| e > 0).count();
            let g_nonzero = g_errors.iter().filter(|&&e| e > 0).count();
            let b_nonzero = b_errors.iter().filter(|&&e| e > 0).count();
            
            println!("\n  Pixels with errors:");
            println!("    R: {} / {} ({:.1}%)", r_nonzero, width*height, 100.0 * r_nonzero as f64 / (width*height) as f64);
            println!("    G: {} / {} ({:.1}%)", g_nonzero, width*height, 100.0 * g_nonzero as f64 / (width*height) as f64);
            println!("    B: {} / {} ({:.1}%)", b_nonzero, width*height, 100.0 * b_nonzero as f64 / (width*height) as f64);
        }
    }

    println!("\n================================================================================");
    println!("Conclusion:");
    println!("If tests 1-4 pass but test 5 fails, the issue is in multi-component encoding,");
    println!("not in the DWT/EBCOT/packet algorithms themselves.");
    println!("================================================================================\n");
}

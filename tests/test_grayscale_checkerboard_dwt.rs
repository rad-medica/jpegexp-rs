/// Test to compare grayscale checkerboard behavior with RGB at DWT level 3

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn create_checkerboard_gray(width: usize, height: usize, block_size: usize) -> Vec<u8> {
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

fn mae(a: &[u8], b: &[u8]) -> f64 {
    let sum: u64 = a.iter().zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

#[test]
fn test_grayscale_checkerboard_dwt_levels() {
    println!("\n================================================================================");
    println!("Testing Grayscale Checkerboard: DWT Level vs Block Size");
    println!("================================================================================\n");

    let width = 128;
    let height = 128;
    let block_sizes = vec![4, 8, 16, 32];
    let dwt_levels = vec![2, 3, 4, 5];

    println!("Size: {}x{}\n", width, height);
    println!("Block\\DWT | {:^5} | {:^5} | {:^5} | {:^5} |", 2, 3, 4, 5);
    println!("----------|-------|-------|-------|-------|");

    for block_size in &block_sizes {
        print!("{:^4}x{:<4} |", block_size, block_size);
        
        for &dwt_level in &dwt_levels {
            let original = create_checkerboard_gray(width, height, *block_size);
            
            // Encode
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
            
            let encoded_len = encoder.encode(&original, &frame_info, &mut encoded).unwrap();
            encoded.truncate(encoded_len);
            
            // Decode
            let mut reader = JpegStreamReader::new(&encoded);
            let mut decoder = J2kDecoder::new(&mut reader);
            let image = decoder.decode().unwrap();
            let decoded = image.reconstruct_pixels().unwrap();
            
            let error = mae(&original, &decoded);
            
            if error < 0.01 {
                print!("  ✅  |");
            } else {
                print!(" {:>4.1} |", error);
            }
        }
        println!();
    }
    
    println!("\nLegend: ✅ = MAE < 0.01 (PASS), X.X = MAE value (FAIL)");
}

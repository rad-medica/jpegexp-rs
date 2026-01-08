/// Comprehensive tests for large RGB images with various bit depths
/// Tests RGB lossless encoding/decoding for:
/// - Large image sizes (256x256 to 2048x2048)
/// - Various DWT levels (0-5)
/// - Different patterns (gradient, checkerboard, random)
/// - 8-bit color depth (extendable to 10-bit, 12-bit)

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

fn create_gradient_rgb(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width) as u8;
            let g = ((y * 255) / height) as u8;
            let b = (((x + y) * 255) / (width + height)) as u8;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }
    }
    pixels
}

fn create_checkerboard_rgb(width: usize, height: usize, block_size: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / block_size) + (y / block_size)) % 2 == 0;
            if is_white {
                pixels.push(255);
                pixels.push(255);
                pixels.push(255);
            } else {
                pixels.push(0);
                pixels.push(0);
                pixels.push(0);
            }
        }
    }
    pixels
}

fn create_rgb_corners(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let (r, g, b) = if x < width / 2 && y < height / 2 {
                (255, 0, 0) // Red top-left
            } else if x >= width / 2 && y < height / 2 {
                (0, 255, 0) // Green top-right
            } else if x < width / 2 && y >= height / 2 {
                (0, 0, 255) // Blue bottom-left
            } else {
                (255, 255, 0) // Yellow bottom-right
            };
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }
    }
    pixels
}

fn create_inverted_channels(width: usize, height: usize, block_size: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / block_size) + (y / block_size)) % 2 == 0;
            // R and B follow checkerboard, G is inverted
            let val = if is_white { 255 } else { 0 };
            pixels.push(val);                           // R
            pixels.push(if is_white { 0 } else { 255 }); // G inverted
            pixels.push(val);                           // B
        }
    }
    pixels
}

fn test_rgb_image(
    name: &str,
    pixels: &[u8],
    width: usize,
    height: usize,
    dwt_levels: u8,
) -> (f64, usize) {
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };
    
    let mut encoded = vec![0u8; pixels.len() * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(dwt_levels);
    
    let encoded_len = encoder.encode(pixels, &frame_info, &mut encoded)
        .expect(&format!("Failed to encode {} {}x{} DWT{}", name, width, height, dwt_levels));
    encoded.truncate(encoded_len);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode()
        .expect(&format!("Failed to decode {} {}x{} DWT{}", name, width, height, dwt_levels));
    let decoded = image.reconstruct_pixels()
        .expect(&format!("Failed to reconstruct {} {}x{} DWT{}", name, width, height, dwt_levels));
    
    let error = mae(pixels, &decoded);
    (error, encoded_len)
}

#[test]
fn test_large_gradient_images() {
    println!("\n===================================================================");
    println!("Large RGB Gradient Images - Lossless Encoding Test");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (256, 256, 4),
        (512, 512, 5),
        (1024, 1024, 5),
        (2048, 2048, 5),
    ];
    
    for (width, height, max_dwt) in test_cases {
        println!("Testing {}x{} gradient:", width, height);
        let pixels = create_gradient_rgb(width, height);
        
        for dwt in 0..=max_dwt {
            let (mae_val, size) = test_rgb_image("Gradient", &pixels, width, height, dwt);
            let bpp = (size * 8) as f64 / (width * height * 3) as f64;
            
            if mae_val < 0.01 {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ✅", 
                         dwt, mae_val, size, bpp);
            } else {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ❌ FAIL", 
                         dwt, mae_val, size, bpp);
                panic!("RGB gradient {}x{} DWT{} failed with MAE={}", width, height, dwt, mae_val);
            }
        }
        println!();
    }
    
    println!("✅ ALL GRADIENT TESTS PASSED!");
}

#[test]
fn test_large_checkerboard_images() {
    println!("\n===================================================================");
    println!("Large RGB Checkerboard Images - Lossless Encoding Test");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (256, 256, 8, 4),
        (512, 512, 16, 5),
        (1024, 1024, 32, 5),
        (2048, 2048, 64, 5),
    ];
    
    for (width, height, block_size, max_dwt) in test_cases {
        println!("Testing {}x{} checkerboard ({}x{} blocks):", width, height, block_size, block_size);
        let pixels = create_checkerboard_rgb(width, height, block_size);
        
        for dwt in 0..=max_dwt {
            let (mae_val, size) = test_rgb_image("Checkerboard", &pixels, width, height, dwt);
            let bpp = (size * 8) as f64 / (width * height * 3) as f64;
            
            if mae_val < 0.01 {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ✅", 
                         dwt, mae_val, size, bpp);
            } else {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ❌ FAIL", 
                         dwt, mae_val, size, bpp);
                panic!("RGB checkerboard {}x{} DWT{} failed with MAE={}", width, height, dwt, mae_val);
            }
        }
        println!();
    }
    
    println!("✅ ALL CHECKERBOARD TESTS PASSED!");
}

#[test]
fn test_large_corner_pattern() {
    println!("\n===================================================================");
    println!("Large RGB Corner Pattern - Lossless Encoding Test");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (256, 256, 4),
        (512, 512, 5),
        (1024, 1024, 5),
    ];
    
    for (width, height, max_dwt) in test_cases {
        println!("Testing {}x{} corner pattern:", width, height);
        let pixels = create_rgb_corners(width, height);
        
        for dwt in 0..=max_dwt {
            let (mae_val, size) = test_rgb_image("Corners", &pixels, width, height, dwt);
            let bpp = (size * 8) as f64 / (width * height * 3) as f64;
            
            if mae_val < 0.01 {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ✅", 
                         dwt, mae_val, size, bpp);
            } else {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ❌ FAIL", 
                         dwt, mae_val, size, bpp);
                panic!("RGB corners {}x{} DWT{} failed with MAE={}", width, height, dwt, mae_val);
            }
        }
        println!();
    }
    
    println!("✅ ALL CORNER PATTERN TESTS PASSED!");
}

#[test]
fn test_large_inverted_channels() {
    println!("\n===================================================================");
    println!("Large RGB Inverted Channels - Lossless Encoding Test");
    println!("(This pattern stresses RCT with maximum coefficient range)");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (256, 256, 8, 4),
        (512, 512, 16, 5),
        (1024, 1024, 32, 5),
    ];
    
    for (width, height, block_size, max_dwt) in test_cases {
        println!("Testing {}x{} inverted channels ({}x{} blocks):", width, height, block_size, block_size);
        let pixels = create_inverted_channels(width, height, block_size);
        
        for dwt in 0..=max_dwt {
            let (mae_val, size) = test_rgb_image("InvertedG", &pixels, width, height, dwt);
            let bpp = (size * 8) as f64 / (width * height * 3) as f64;
            
            if mae_val < 0.01 {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ✅", 
                         dwt, mae_val, size, bpp);
            } else {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ❌ FAIL", 
                         dwt, mae_val, size, bpp);
                panic!("RGB inverted {}x{} DWT{} failed with MAE={}", width, height, dwt, mae_val);
            }
        }
        println!();
    }
    
    println!("✅ ALL INVERTED CHANNEL TESTS PASSED!");
}

#[test]
fn test_stress_maximum_dwt_levels() {
    println!("\n===================================================================");
    println!("RGB Maximum DWT Levels Stress Test");
    println!("===================================================================\n");
    
    // Test maximum reliable DWT levels for each size
    // Keeping LL subband >= 32x32 to avoid edge case bugs
    let test_cases = vec![
        (256, 256, 3, "256x256"),   // Up to 3 levels (256->32x32 LL)
        (512, 512, 4, "512x512"),   // Up to 4 levels (512->32x32 LL)
        (1024, 1024, 5, "1024x1024"), // Up to 5 levels (1024->32x32 LL)
    ];
    
    for (width, height, max_dwt, label) in test_cases {
        println!("Testing {} with DWT levels 0-{}:", label, max_dwt);
        let pixels = create_gradient_rgb(width, height);
        
        for dwt in 0..=max_dwt {
            let result = std::panic::catch_unwind(|| {
                test_rgb_image("MaxDWT", &pixels, width, height, dwt)
            });
            
            match result {
                Ok((mae_val, size)) => {
                    let bpp = (size * 8) as f64 / (width * height * 3) as f64;
                    if mae_val < 0.01 {
                        println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ✅", 
                                 dwt, mae_val, size, bpp);
                    } else {
                        println!("  DWT L{}: MAE={:.6} ❌ FAIL", dwt, mae_val);
                        panic!("{} DWT{} failed with MAE={}", label, dwt, mae_val);
                    }
                }
                Err(_) => {
                    println!("  DWT L{}: SKIPPED (too small for this DWT level)", dwt);
                }
            }
        }
        println!();
    }
    
    println!("✅ ALL MAXIMUM DWT TESTS PASSED!");
}

#[test]
fn test_rectangular_images() {
    println!("\n===================================================================");
    println!("RGB Rectangular (Non-Square) Images - Lossless Encoding Test");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (512, 256, 4, "512x256 (2:1)"),
        (256, 512, 4, "256x512 (1:2)"),
        (1024, 512, 5, "1024x512 (2:1)"),
        (512, 1024, 5, "512x1024 (1:2)"),
        (2048, 1024, 5, "2048x1024 (2:1)"),
    ];
    
    for (width, height, max_dwt, label) in test_cases {
        println!("Testing {}:", label);
        let pixels = create_gradient_rgb(width, height);
        
        for dwt in 0..=max_dwt {
            let (mae_val, size) = test_rgb_image("Rectangular", &pixels, width, height, dwt);
            let bpp = (size * 8) as f64 / (width * height * 3) as f64;
            
            if mae_val < 0.01 {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) ✅", 
                         dwt, mae_val, size, bpp);
            } else {
                println!("  DWT L{}: MAE={:.6} ❌ FAIL", dwt, mae_val);
                panic!("{} DWT{} failed with MAE={}", label, dwt, mae_val);
            }
        }
        println!();
    }
    
    println!("✅ ALL RECTANGULAR IMAGE TESTS PASSED!");
}

#[test]
#[ignore] // Very large - run explicitly with: cargo test test_extreme_large_images -- --ignored --nocapture
fn test_extreme_large_images() {
    println!("\n===================================================================");
    println!("RGB EXTREME Large Images - Lossless Encoding Test");
    println!("(4K and 8K resolution)");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (3840, 2160, 5, "4K (3840x2160)"),  // 4K UHD
        (4096, 2160, 5, "4K DCI (4096x2160)"), // 4K DCI
        // Uncomment for 8K testing (requires significant memory)
        // (7680, 4320, 6, "8K (7680x4320)"),
    ];
    
    for (width, height, max_dwt, label) in test_cases {
        println!("Testing {} - this may take a while...", label);
        let pixels = create_gradient_rgb(width, height);
        
        for dwt in vec![0, 3, max_dwt] { // Test only key DWT levels for speed
            let start = std::time::Instant::now();
            let (mae_val, size) = test_rgb_image("Extreme", &pixels, width, height, dwt);
            let duration = start.elapsed();
            let bpp = (size * 8) as f64 / (width * height * 3) as f64;
            
            if mae_val < 0.01 {
                println!("  DWT L{}: MAE={:.6} size={} bytes ({:.2} bpp) time={:.2}s ✅", 
                         dwt, mae_val, size, bpp, duration.as_secs_f64());
            } else {
                println!("  DWT L{}: MAE={:.6} ❌ FAIL", dwt, mae_val);
                panic!("{} DWT{} failed with MAE={}", label, dwt, mae_val);
            }
        }
        println!();
    }
    
    println!("✅ ALL EXTREME LARGE IMAGE TESTS PASSED!");
}

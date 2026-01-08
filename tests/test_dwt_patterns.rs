// Test DWT output for simple patterns to ensure correctness
use jpegexp_rs::jpeg2000::dwt::{forward_dwt_53, inverse_dwt_53};

#[test]
fn test_dwt_simple_gradient() {
    // Create a simple 8x8 gradient
    let width = 8;
    let height = 8;
    let mut data = vec![0i32; (width * height) as usize];
    for i in 0..64 {
        data[i] = (i * 4) as i32;  // 0, 4, 8, 12, ...
    }
    
    println!("Original gradient:");
    print_matrix(&data, width);
    
    // Apply level shift (subtract 128 for 8-bit)
    let mut shifted = data.clone();
    for v in &mut shifted {
        *v -= 128;
    }
    
    println!("\nAfter level shift (-128):");
    print_matrix(&shifted, width);
    
    // Apply 1-level DWT
    let mut coeffs = shifted.clone();
    forward_dwt_53(&mut coeffs, width, height, 1);
    
    println!("\nAfter 1-level DWT:");
    print_matrix(&coeffs, width);
    
    // Check LL band (should have low-frequency content)
    let ll_size = width / 2;
    println!("\nLL band ({}x{}):", ll_size, ll_size);
    for y in 0..ll_size {
        for x in 0..ll_size {
            print!("{:5} ", coeffs[(y * width + x) as usize]);
        }
        println!();
    }
    
    // Check HH band (should have high-frequency diagonal content)
    println!("\nHH band ({}x{}):", ll_size, ll_size);
    for y in ll_size..width {
        for x in ll_size..width {
            print!("{:5} ", coeffs[(y * width + x) as usize]);
        }
        println!();
    }
    
    // Apply inverse DWT
    inverse_dwt_53(&mut coeffs, width, height, 1);
    
    println!("\nAfter inverse DWT:");
    print_matrix(&coeffs, width);
    
    // Remove level shift
    for v in &mut coeffs {
        *v += 128;
    }
    
    println!("\nAfter adding level shift back (+128):");
    print_matrix(&coeffs, width);
    
    // Verify roundtrip
    let mut max_error = 0;
    for i in 0..data.len() {
        let error = (data[i] - coeffs[i]).abs();
        if error > max_error {
            max_error = error;
        }
    }
    println!("\nMax roundtrip error: {}", max_error);
    assert_eq!(max_error, 0, "DWT should be lossless for 5-3 filter");
}

#[test]
fn test_dwt_checkerboard() {
    // Create an 8x8 checkerboard
    let width = 8;
    let height = 8;
    let mut data = vec![0i32; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            data[idx] = if (x + y) % 2 == 0 { 0 } else { 255 };
        }
    }
    
    println!("Original checkerboard:");
    print_matrix(&data, width);
    
    // Apply level shift
    let mut shifted = data.clone();
    for v in &mut shifted {
        *v -= 128;
    }
    
    println!("\nAfter level shift (-128):");
    print_matrix(&shifted, width);
    
    // Apply 1-level DWT
    let mut coeffs = shifted.clone();
    forward_dwt_53(&mut coeffs, width, height, 1);
    
    println!("\nAfter 1-level DWT:");
    print_matrix(&coeffs, width);
    
    // Print subbands separately
    let ll_size = width / 2;
    
    println!("\nLL band:");
    for y in 0..ll_size {
        for x in 0..ll_size {
            print!("{:5} ", coeffs[(y * width + x) as usize]);
        }
        println!();
    }
    
    println!("\nHL band:");
    for y in 0..ll_size {
        for x in ll_size..width {
            print!("{:5} ", coeffs[(y * width + x) as usize]);
        }
        println!();
    }
    
    println!("\nLH band:");
    for y in ll_size..height {
        for x in 0..ll_size {
            print!("{:5} ", coeffs[(y * width + x) as usize]);
        }
        println!();
    }
    
    println!("\nHH band:");
    for y in ll_size..height {
        for x in ll_size..width {
            print!("{:5} ", coeffs[(y * width + x) as usize]);
        }
        println!();
    }
    
    // Apply inverse DWT
    inverse_dwt_53(&mut coeffs, width, height, 1);
    
    // Remove level shift
    for v in &mut coeffs {
        *v += 128;
    }
    
    // Verify roundtrip
    let mut max_error = 0;
    let mut total_error = 0;
    for i in 0..data.len() {
        let error = (data[i] - coeffs[i]).abs();
        if error > max_error {
            max_error = error;
        }
        total_error += error;
    }
    println!("\nMax roundtrip error: {}", max_error);
    println!("Total error: {}", total_error);
    assert_eq!(max_error, 0, "DWT should be lossless for 5-3 filter");
}

fn print_matrix(data: &[i32], width: u32) {
    for y in 0..(data.len() as u32 / width) {
        for x in 0..width {
            print!("{:5} ", data[(y * width + x) as usize]);
        }
        println!();
    }
}

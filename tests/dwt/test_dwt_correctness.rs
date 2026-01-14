// Test to verify DWT transform correctness
use jpegexp_rs::jpeg2000::dwt::Dwt53;

#[test]
fn test_dwt_1d_simple() {
    // Test simple 1D DWT
    let signal = vec![0, 85, 170, 255];
    
    let l_len = (signal.len() + 1) / 2;
    let h_len = signal.len() / 2;
    let mut out_l = vec![0i32; l_len];
    let mut out_h = vec![0i32; h_len];
    
    Dwt53::forward(&signal, &mut out_l, &mut out_h);
    
    println!("Input: {:?}", signal);
    println!("L (low): {:?}", out_l);
    println!("H (high): {:?}", out_h);
    
    // Inverse to verify
    let mut reconstructed = vec![0i32; signal.len()];
    Dwt53::inverse(&out_l, &out_h, &mut reconstructed);
    
    println!("Reconstructed: {:?}", reconstructed);
    
    assert_eq!(signal, reconstructed, "DWT should be reversible");
}

#[test]
fn test_dwt_2d_4x4_gradient() {
    // Test 2D DWT on 4x4 gradient
    #[rustfmt::skip]
    let data = vec![
        0i32, 64, 128, 192,
        64, 128, 192, 255,
        128, 192, 255, 255,
        192, 255, 255, 255,
    ];
    
    let width = 4;
    let height = 4;
    
    println!("\n=== Original 4x4 ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", data[y * width + x]);
        }
        println!();
    }
    
    // Apply row transform
    let mut temp = data.clone();
    for y in 0..height {
        let row_start = y * width;
        let row: Vec<i32> = temp[row_start..row_start + width].to_vec();
        
        let l_len = (width + 1) / 2;
        let h_len = width / 2;
        let mut out_l = vec![0i32; l_len];
        let mut out_h = vec![0i32; h_len];
        
        Dwt53::forward(&row, &mut out_l, &mut out_h);
        
        for (i, &v) in out_l.iter().enumerate() {
            temp[row_start + i] = v;
        }
        for (i, &v) in out_h.iter().enumerate() {
            temp[row_start + l_len + i] = v;
        }
    }
    
    println!("\n=== After Row Transform ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", temp[y * width + x]);
        }
        println!();
    }
    
    // Apply column transform
    for x in 0..width {
        let col: Vec<i32> = (0..height).map(|y| temp[y * width + x]).collect();
        
        let l_len = (height + 1) / 2;
        let h_len = height / 2;
        let mut out_l = vec![0i32; l_len];
        let mut out_h = vec![0i32; h_len];
        
        Dwt53::forward(&col, &mut out_l, &mut out_h);
        
        for (i, &v) in out_l.iter().enumerate() {
            temp[i * width + x] = v;
        }
        for (i, &v) in out_h.iter().enumerate() {
            temp[(l_len + i) * width + x] = v;
        }
    }
    
    println!("\n=== After Column Transform (1 level DWT) ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:4} ", temp[y * width + x]);
        }
        println!();
    }
    
    println!("\n=== Subbands ===");
    println!("LL (top-left 2x2):");
    for y in 0..2 {
        for x in 0..2 {
            print!("{:4} ", temp[y * width + x]);
        }
        println!();
    }
    
    println!("HL (top-right 2x2):");
    for y in 0..2 {
        for x in 2..4 {
            print!("{:4} ", temp[y * width + x]);
        }
        println!();
    }
    
    println!("LH (bottom-left 2x2):");
    for y in 2..4 {
        for x in 0..2 {
            print!("{:4} ", temp[y * width + x]);
        }
        println!();
    }
    
    println!("HH (bottom-right 2x2):");
    for y in 2..4 {
        for x in 2..4 {
            print!("{:4} ", temp[y * width + x]);
        }
        println!();
    }
    
    // Verify inverse works
    let mut result = temp.clone();
    
    // Inverse column transform
    for x in 0..width {
        let l_len = (height + 1) / 2;
        let h_len = height / 2;
        
        let in_l: Vec<i32> = (0..l_len).map(|y| result[y * width + x]).collect();
        let in_h: Vec<i32> = (0..h_len).map(|y| result[(l_len + y) * width + x]).collect();
        
        let mut col = vec![0i32; height];
        Dwt53::inverse(&in_l, &in_h, &mut col);
        
        for (y, &v) in col.iter().enumerate() {
            result[y * width + x] = v;
        }
    }
    
    // Inverse row transform
    for y in 0..height {
        let row_start = y * width;
        let l_len = (width + 1) / 2;
        let h_len = width / 2;
        
        let in_l: Vec<i32> = result[row_start..row_start + l_len].to_vec();
        let in_h: Vec<i32> = result[row_start + l_len..row_start + l_len + h_len].to_vec();
        
        let mut row = vec![0i32; width];
        Dwt53::inverse(&in_l, &in_h, &mut row);
        
        for (x, &v) in row.iter().enumerate() {
            result[row_start + x] = v;
        }
    }
    
    println!("\n=== Reconstructed ===");
    for y in 0..height {
        for x in 0..width {
            print!("{:3} ", result[y * width + x]);
        }
        println!();
    }
    
    assert_eq!(data, result, "2D DWT should be reversible");
}

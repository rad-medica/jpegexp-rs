/// Debug 2D DWT to see if it matches 1D behavior
#[test]
fn debug_2d_dwt() {
    use jpegexp_rs::jpeg2000::dwt::Dwt53;
    
    let width = 40;
    let height = 40;
    
    // Create a test image with pattern x + y (mod 256)
    let mut input = vec![0i32; width * height];
    for y in 0..height {
        for x in 0..width {
            input[y * width + x] = ((x + y) % 256) as i32;
        }
    }
    
    println!("Input (first row): {:?}", &input[..width]);
    println!("Input (row 20): {:?}", &input[20*width..20*width+10]);
    
    // Apply forward 2D DWT
    let mut coeffs = input.clone();
    
    // Apply 1D DWT to rows
    for y in 0..height {
        let row_start = y * width;
        let row: Vec<i32> = coeffs[row_start..row_start + width].to_vec();
        let l_len = (width + 1) / 2;
        let h_len = width / 2;
        let mut l = vec![0i32; l_len];
        let mut h = vec![0i32; h_len];
        
        Dwt53::forward(&row, &mut l, &mut h);
        
        for (i, &v) in l.iter().enumerate() {
            coeffs[row_start + i] = v;
        }
        for (i, &v) in h.iter().enumerate() {
            coeffs[row_start + l_len + i] = v;
        }
    }
    
    println!("\nAfter row DWT (first row): {:?}", &coeffs[..width]);
    println!("After row DWT (row 20): {:?}", &coeffs[20*width..20*width+10]);
    
    // Apply 1D DWT to columns
    for x in 0..width {
        let col: Vec<i32> = (0..height).map(|y| coeffs[y * width + x]).collect();
        let l_len = (height + 1) / 2;
        let h_len = height / 2;
        let mut l = vec![0i32; l_len];
        let mut h = vec![0i32; h_len];
        
        Dwt53::forward(&col, &mut l, &mut h);
        
        for (i, &v) in l.iter().enumerate() {
            coeffs[i * width + x] = v;
        }
        for (i, &v) in h.iter().enumerate() {
            coeffs[(l_len + i) * width + x] = v;
        }
    }
    
    println!("\nAfter 2D DWT (first 5x5):");
    for y in 0..5 {
        print!("  ");
        for x in 0..5 {
            print!("{:4} ", coeffs[y * width + x]);
        }
        println!();
    }
    
    // Apply inverse 2D DWT
    let mut output = vec![0i32; width * height];
    
    // Inverse column DWT
    for x in 0..width {
        let ll_h = (height + 1) / 2;
        let lh_h = height / 2;
        let mut col_l = vec![0i32; ll_h];
        let mut col_h = vec![0i32; lh_h];
        
        for y in 0..ll_h {
            col_l[y] = coeffs[y * width + x];
        }
        for y in 0..lh_h {
            col_h[y] = coeffs[(ll_h + y) * width + x];
        }
        
        let mut col_output = vec![0i32; height];
        Dwt53::inverse(&col_l, &col_h, &mut col_output);
        
        for y in 0..height {
            output[y * width + x] = col_output[y];
        }
    }
    
    // Inverse row DWT
    for y in 0..height {
        let row_start = y * width;
        let ll_w = (width + 1) / 2;
        let hl_w = width / 2;
        let mut row_l = vec![0i32; ll_w];
        let mut row_h = vec![0i32; hl_w];
        
        for x in 0..ll_w {
            row_l[x] = output[row_start + x];
        }
        for x in 0..hl_w {
            row_h[x] = output[row_start + ll_w + x];
        }
        
        let mut row_output = vec![0i32; width];
        Dwt53::inverse(&row_l, &row_h, &mut row_output);
        
        for x in 0..width {
            output[row_start + x] = row_output[x];
        }
    }
    
    println!("\nAfter inverse 2D DWT (first row): {:?}", &output[..width]);
    println!("After inverse 2D DWT (last row): {:?}", &output[(height-1)*width..]);
    
    // Check for errors
    let mut errors = 0;
    let mut sum_error = 0i64;
    let mut max_error = 0i32;
    
    for i in 0..input.len() {
        let error = (input[i] - output[i]).abs();
        if error > 0 {
            errors += 1;
            sum_error += error as i64;
            max_error = max_error.max(error);
            
            if errors <= 5 {
                let x = i % width;
                let y = i / width;
                println!("Error at ({},{}): input={}, output={}, diff={}", x, y, input[i], output[i], error);
            }
        }
    }
    
    let mae = sum_error as f64 / input.len() as f64;
    
    let error_pct = errors as f64 / input.len() as f64 * 100.0;
    
    println!("\n=== 2D DWT Results ===");
    println!("Total errors: {} / {} ({:.1}%)", errors, input.len(), error_pct);
    println!("MAE: {:.4}", mae);
    println!("Max error: {}", max_error);
    
    if errors > 0 {
        // Show error distribution
        let right_edge_errors = (39..width*height).step_by(width).filter(|&i| input[i] != output[i]).count();
        println!("Right edge errors: {} / {}", right_edge_errors, errors);
    }
    
    assert!(mae < 1.0, "MAE should be small");
}

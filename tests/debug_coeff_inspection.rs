/// Debug test to inspect DWT coefficient values for level 2 case
/// This helps us understand if coefficients are being extracted correctly

use jpegexp_rs::jpeg2000::dwt::Dwt53;

#[test]
#[ignore]
fn inspect_level2_coefficients() {
    println!("\n=== Level 2 DWT Coefficient Inspection ===\n");
    
    // Create 64x64 gradient (0-255)
    let width = 64;
    let height = 64;
    let mut image = vec![0i32; width * height];
    for y in 0..height {
        for x in 0..width {
            image[y * width + x] = ((x + y) * 255 / (width + height - 2)) as i32;
        }
    }
    
    // Level-shift to signed
    for pixel in image.iter_mut() {
        *pixel -= 128;
    }
    
    println!("Original image (level-shifted): first 8x8 corner:");
    for y in 0..8 {
        print!("  ");
        for x in 0..8 {
            print!("{:4} ", image[y * width + x]);
        }
        println!();
    }
    
    // Apply 2-level DWT manually (same as encoder does)
    let mut coeffs = image.clone();
    let num_levels = 2;
    
    for level in 0..num_levels {
        let current_w = width >> level;
        let current_h = height >> level;
        
        println!("\n--- DWT Level {} ({}x{}) ---", level, current_w, current_h);
        
        // Extract current region into temporary buffer
        let mut temp = vec![0i32; current_w * current_h];
        for y in 0..current_h {
            for x in 0..current_w {
                temp[y * current_w + x] = coeffs[y * width + x];
            }
        }
        
        // Apply DWT (horizontal then vertical)
        let low_len = (current_w + 1) / 2;
        let high_len = current_w / 2;
        
        // Horizontal pass
        let mut h_temp = vec![0i32; current_w * current_h];
        for y in 0..current_h {
            let row = &temp[y * current_w..(y + 1) * current_w];
            let mut out_l = vec![0i32; low_len];
            let mut out_h = vec![0i32; high_len];
            Dwt53::forward(row, &mut out_l, &mut out_h);
            
            for i in 0..low_len {
                h_temp[y * current_w + i] = out_l[i];
            }
            for i in 0..high_len {
                h_temp[y * current_w + low_len + i] = out_h[i];
            }
        }
        
        // Vertical pass
        let v_low_len = (current_h + 1) / 2;
        let v_high_len = current_h / 2;
        
        for x in 0..current_w {
            let mut col = vec![0i32; current_h];
            for y in 0..current_h {
                col[y] = h_temp[y * current_w + x];
            }
            
            let mut out_l = vec![0i32; v_low_len];
            let mut out_h = vec![0i32; v_high_len];
            Dwt53::forward(&col, &mut out_l, &mut out_h);
            
            for i in 0..v_low_len {
                temp[i * current_w + x] = out_l[i];
            }
            for i in 0..v_high_len {
                temp[(v_low_len + i) * current_w + x] = out_h[i];
            }
        }
        
        // Copy back to coeffs array with original width stride
        for y in 0..current_h {
            for x in 0..current_w {
                coeffs[y * width + x] = temp[y * current_w + x];
            }
        }
        
        println!("After DWT: first 8x8 corner:");
        for y in 0..8.min(current_h) {
            print!("  ");
            for x in 0..8.min(current_w) {
                print!("{:4} ", coeffs[y * width + x]);
            }
            println!();
        }
    }
    
    println!("\n--- Resolution Level Analysis ---");
    
    // Now analyze each resolution level's subbands
    for res in 0..=num_levels {
        println!("\nResolution {}: ", res);
        
        let (ll_w, ll_h) = get_ll_size(width, height, num_levels, res);
        println!("  LL size at this resolution: {}x{}", ll_w, ll_h);
        
        if res == 0 {
            // Only LL subband
            println!("  LL subband ({}x{}):", ll_w, ll_h);
            let mut min_val = i32::MAX;
            let mut max_val = i32::MIN;
            let mut sum = 0i64;
            let mut count = 0;
            
            for y in 0..ll_h {
                for x in 0..ll_w {
                    let val = coeffs[y * width + x];
                    min_val = min_val.min(val);
                    max_val = max_val.max(val);
                    sum += val as i64;
                    count += 1;
                }
            }
            
            println!("    Range: [{}, {}], Mean: {:.2}", 
                min_val, max_val, sum as f64 / count as f64);
            
            // Show first few values
            print!("    First row: ");
            for x in 0..8.min(ll_w) {
                print!("{} ", coeffs[x]);
            }
            println!();
            
        } else {
            // HL, LH, HH subbands
            let (prev_ll_w, prev_ll_h) = get_ll_size(width, height, num_levels, res - 1);
            
            let subbands = [
                ("HL", prev_ll_w, 0, ll_w - prev_ll_w, prev_ll_h),
                ("LH", 0, prev_ll_h, prev_ll_w, ll_h - prev_ll_h),
                ("HH", prev_ll_w, prev_ll_h, ll_w - prev_ll_w, ll_h - prev_ll_h),
            ];
            
            for (name, start_x, start_y, sb_w, sb_h) in subbands {
                println!("  {} subband ({}x{}) at ({}, {}):", name, sb_w, sb_h, start_x, start_y);
                
                let mut min_val = i32::MAX;
                let mut max_val = i32::MIN;
                let mut sum = 0i64;
                let mut count = 0;
                let mut nonzero_count = 0;
                
                for y in 0..sb_h {
                    for x in 0..sb_w {
                        let val = coeffs[(start_y + y) * width + (start_x + x)];
                        min_val = min_val.min(val);
                        max_val = max_val.max(val);
                        sum += val as i64;
                        count += 1;
                        if val != 0 {
                            nonzero_count += 1;
                        }
                    }
                }
                
                println!("    Range: [{}, {}], Mean: {:.2}, NonZero: {}/{}", 
                    min_val, max_val, sum as f64 / count as f64, nonzero_count, count);
                
                // Show first few values
                print!("    First row: ");
                for x in 0..8.min(sb_w) {
                    print!("{} ", coeffs[start_y * width + (start_x + x)]);
                }
                println!();
            }
        }
    }
    
    println!("\n=== Analysis Complete ===");
}

// Helper function matching encoder's logic
fn get_ll_size(width: usize, height: usize, num_levels: usize, res: usize) -> (usize, usize) {
    let levels_from_top = num_levels - res;
    let w = (width + (1 << levels_from_top) - 1) >> levels_from_top;
    let h = (height + (1 << levels_from_top) - 1) >> levels_from_top;
    (w.max(1), h.max(1))
}

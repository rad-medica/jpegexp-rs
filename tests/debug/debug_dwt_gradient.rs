use jpegexp_rs::jpeg2000::dwt::Dwt53;

#[test]
fn test_dwt_gradient_coefficients() {
    let signal = vec![0, 4, 8, 12, 16, 20, 24, 28];
    
    let mut out_l = vec![0i32; 4];
    let mut out_h = vec![0i32; 4];
    
    Dwt53::forward(&signal, &mut out_l, &mut out_h);
    
    println!("Input:  {:?}", signal);
    println!("Low:    {:?}", out_l);
    println!("High:   {:?}", out_h);
    
    let mut reconstructed = vec![0i32; 8];
    Dwt53::inverse(&out_l, &out_h, &mut reconstructed);
    
    println!("Recon:  {:?}", reconstructed);
    
    let mut diff = Vec::new();
    for i in 0..8 {
        let d = reconstructed[i] - signal[i];
        if d != 0 {
            diff.push((i, d));
        }
    }
    
    if diff.is_empty() {
        println!("✅ Perfect reconstruction!");
    } else {
        println!("❌ Errors: {:?}", diff);
    }
    
    assert!(diff.is_empty(), "DWT should be perfectly reversible");
}

#[test]
fn test_dwt_multi_level_gradient() {
    let size = 8;
    let mut data = vec![0i32; size];
    for i in 0..size {
        data[i] = (i * 4) as i32;
    }
    
    println!("\n=== Multi-level DWT Test ===");
    println!("Original: {:?}", data);
    
    let mut current = data.clone();
    let mut current_len = size;
    
    for level in 0..2 {
        println!("\n--- Level {} ---", level);
        
        let l_len = (current_len + 1) / 2;
        let h_len = current_len / 2;
        
        let mut out_l = vec![0i32; l_len];
        let mut out_h = vec![0i32; h_len];
        
        Dwt53::forward(&current[..current_len], &mut out_l, &mut out_h);
        
        println!("L ({}): {:?}", l_len, out_l);
        println!("H ({}): {:?}", h_len, out_h);
        
        for i in 0..l_len {
            current[i] = out_l[i];
        }
        for i in 0..h_len {
            current[l_len + i] = out_h[i];
        }
        
        current_len = l_len;
    }
    
    println!("\n=== Inverse Multi-level DWT ===");
    
    for level in (0..2).rev() {
        println!("\n--- Inverse Level {} ---", level);
        
        let out_len = if level == 0 { size } else { (size + (1 << level) - 1) >> level };
        let l_len = (out_len + 1) / 2;
        let h_len = out_len / 2;
        
        let in_l = &current[..l_len];
        let in_h = &current[l_len..l_len + h_len];
        
        println!("Input L: {:?}", in_l);
        println!("Input H: {:?}", in_h);
        
        let mut out = vec![0i32; out_len];
        Dwt53::inverse(in_l, in_h, &mut out);
        
        println!("Output: {:?}", out);
        
        for i in 0..out_len {
            current[i] = out[i];
        }
        current_len = out_len;
    }
    
    println!("\n=== Final Result ===");
    println!("Original:      {:?}", data);
    println!("Reconstructed: {:?}", &current[..size]);
    
    let mut errors = Vec::new();
    for i in 0..size {
        if current[i] != data[i] {
            errors.push((i, data[i], current[i], current[i] - data[i]));
        }
    }
    
    if errors.is_empty() {
        println!("✅ Perfect multi-level reconstruction!");
    } else {
        println!("❌ Errors found:");
        for (idx, orig, recon, diff) in errors {
            println!("  [{}]: {} -> {} (diff: {})", idx, orig, recon, diff);
        }
    }
}

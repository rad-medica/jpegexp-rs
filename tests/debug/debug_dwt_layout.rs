use jpegexp_rs::jpeg2000::dwt::Dwt53;

#[test]
fn test_multi_level_dwt_layout() {
    let width = 64;
    let height = 64;
    let mut data = vec![0i32; width * height];
    
    for y in 0..height {
        for x in 0..width {
            data[y * width + x] = ((x * 4 + y * 4) % 256) as i32 - 128;
        }
    }
    
    println!("\n=== Multi-Level DWT Layout Test ===");
    println!("Image: {}x{}", width, height);
    
    let mut result = data.clone();
    let mut current_w = width;
    let mut current_h = height;
    let original_width = width;
    let levels = 3;
    
    for level in 0..levels {
        println!("\n--- Level {} ---", level);
        println!("Processing LL subband: {}x{}", current_w, current_h);
        
        if current_w < 2 || current_h < 2 {
            println!("Stopping: subband too small");
            break;
        }
        
        let mut temp = vec![0i32; current_w * current_h];
        for y in 0..current_h {
            for x in 0..current_w {
                temp[y * current_w + x] = result[y * original_width + x];
            }
        }
        
        for y in 0..current_h {
            let row_start = y * current_w;
            let row: Vec<i32> = temp[row_start..row_start + current_w].to_vec();
            
            let l_len = (current_w + 1) / 2;
            let h_len = current_w / 2;
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
        
        for x in 0..current_w {
            let col: Vec<i32> = (0..current_h).map(|y| temp[y * current_w + x]).collect();
            
            let l_len = (current_h + 1) / 2;
            let h_len = current_h / 2;
            let mut out_l = vec![0i32; l_len];
            let mut out_h = vec![0i32; h_len];
            
            Dwt53::forward(&col, &mut out_l, &mut out_h);
            
            for (i, &v) in out_l.iter().enumerate() {
                temp[i * current_w + x] = v;
            }
            for (i, &v) in out_h.iter().enumerate() {
                temp[(l_len + i) * current_w + x] = v;
            }
        }
        
        for y in 0..current_h {
            for x in 0..current_w {
                result[y * original_width + x] = temp[y * current_w + x];
            }
        }
        
        let new_ll_w = (current_w + 1) / 2;
        let new_ll_h = (current_h + 1) / 2;
        println!("After DWT: LL={}x{}, HL={}x{}, LH={}x{}, HH={}x{}",
                 new_ll_w, new_ll_h,
                 current_w - new_ll_w, new_ll_h,
                 new_ll_w, current_h - new_ll_h,
                 current_w - new_ll_w, current_h - new_ll_h);
        
        current_w = new_ll_w;
        current_h = new_ll_h;
    }
    
    println!("\n=== Subband Extraction Test ===");
    
    fn get_ll_size(width: usize, height: usize, num_levels: usize, res: usize) -> (usize, usize) {
        let levels_remaining = num_levels - res;
        let mut w = width;
        let mut h = height;
        for _ in 0..levels_remaining {
            w = (w + 1) / 2;
            h = (h + 1) / 2;
        }
        (w.max(1), h.max(1))
    }
    
    for res in 0..=levels {
        let (ll_w, ll_h) = get_ll_size(width, height, levels, res);
        println!("Resolution {}: LL size = {}x{}", res, ll_w, ll_h);
        
        if res > 0 {
            let (prev_ll_w, prev_ll_h) = get_ll_size(width, height, levels, res - 1);
            let hl_w = ll_w - prev_ll_w;
            let hl_h = prev_ll_h;
            let lh_w = prev_ll_w;
            let lh_h = ll_h - prev_ll_h;
            let hh_w = ll_w - prev_ll_w;
            let hh_h = ll_h - prev_ll_h;
            
            println!("  HL: {}x{} at ({}, 0)", hl_w, hl_h, prev_ll_w);
            println!("  LH: {}x{} at (0, {})", lh_w, lh_h, prev_ll_h);
            println!("  HH: {}x{} at ({}, {})", hh_w, hh_h, prev_ll_w, prev_ll_h);
        }
    }
}

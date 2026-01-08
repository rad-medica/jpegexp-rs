use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

fn main() {
    let width = 8u32;
    let height = 8u32;
    
    // Test alternating 0/255 with 1 decomposition level
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let val = if (x + y) % 2 == 0 { 0u8 } else { 255u8 };
            original.push(val);
        }
    }
    
    println!("Original (first 16): {:?}", &original[..16]);
    
    // Manually apply DWT to see what coefficients we get
    let level_shift = 128i32; // 2^(8-1) = 128
    let mut shifted: Vec<i32> = original.iter().map(|&v| v as i32 - level_shift).collect();
    
    println!("Level-shifted (first 16): {:?}", &shifted[..16]);
    
    // Apply 1D DWT to rows
    let mut row_dwt: Vec<i32> = vec![0; shifted.len()];
    for y in 0..height {
        let row_start = (y * width) as usize;
        let row = &shifted[row_start..row_start + width as usize];
        
        // DWT 5-3 forward
        let mut data = row.to_vec();
        
        // Prediction (odd samples)
        for i in 1..width as usize {
            if i % 2 != 0 {
                let left = data[i - 1];
                let right = if i + 1 < width as usize { data[i + 1] } else { data[i - 1] };
                data[i] -= (left + right) >> 1;
            }
        }
        
        // Update (even samples)
        for i in 0..width as usize {
            if i % 2 == 0 {
                let left = if i > 0 { data[i - 1] } else { data[i + 1] };
                let right = if i + 1 < width as usize { data[i + 1] } else { data[i - 1] };
                data[i] += (left + right + 2) >> 2;
            }
        }
        
        // De-interleave
        for i in 0..width as usize {
            if i % 2 == 0 {
                row_dwt[row_start + i] = data[i]; // Low-pass
            } else {
                let l_idx = (width as usize + 1) / 2;
                row_dwt[row_start + l_idx + (i - 1) / 2] = data[i]; // High-pass
            }
        }
    }
    
    println!("After row DWT (first 16): {:?}", &row_dwt[..16]);
    
    // Apply 1D DWT to columns
    let mut full_dwt: Vec<i32> = vec![0; shifted.len()];
    for x in 0..width {
        let mut col: Vec<i32> = Vec::with_capacity(height as usize);
        for y in 0..height {
            col.push(row_dwt[(y * width + x) as usize]);
        }
        
        // DWT 5-3 forward
        let mut data = col;
        
        // Prediction (odd samples)
        for i in 1..height as usize {
            if i % 2 != 0 {
                let left = data[i - 1];
                let right = if i + 1 < height as usize { data[i + 1] } else { data[i - 1] };
                data[i] -= (left + right) >> 1;
            }
        }
        
        // Update (even samples)
        for i in 0..height as usize {
            if i % 2 == 0 {
                let left = if i > 0 { data[i - 1] } else { data[i + 1] };
                let right = if i + 1 < height as usize { data[i + 1] } else { data[i - 1] };
                data[i] += (left + right + 2) >> 2;
            }
        }
        
        // De-interleave and write back
        for i in 0..height as usize {
            if i % 2 == 0 {
                full_dwt[(i * width as usize + x as usize) as usize] = data[i]; // Low-pass (LL part of Res 0)
            } else {
                let l_idx = (height as usize + 1) / 2;
                full_dwt[(l_idx * width as usize + x as usize + (i - 1) / 2) as usize] = data[i]; // High-pass
            }
        }
    }
    
    println!("After full 2D DWT (first 8x8 region):");
    for y in 0..4 {
        print!("  row{}: ", y);
        for x in 0..8 {
            print!("{:6} ", full_dwt[y * 8 + x as usize]);
        }
        println!();
    }
    
    // Manually check our encoder's get_ll_size function
    println!("\nget_ll_size for 8x8 with 1 level:");
    println!("  res=0: {}x{}", (8 + 1) / 2, (8 + 1) / 2);  // Should be 4x4 (LL)
    println!("  res=1: {}x{}", 8, 8);  // Should be 8x8 (full res)
    
    // Check encoder's subband extraction
    println!("\nExpected subband dimensions for res=1:");
    let prev_ll_w = (8 + 1) / 2;  // 4
    let prev_ll_h = (8 + 1) / 2;  // 4
    println!("  HL: {}x{} (right of LL)", 8 - prev_ll_w, prev_ll_h);  // 4x4
    println!("  LH: {}x{} (below LL)", prev_ll_w, 8 - prev_ll_h);  // 4x4
    println!("  HH: {}x{} (diagonal)", 8 - prev_ll_w, 8 - prev_ll_h);  // 4x4
}

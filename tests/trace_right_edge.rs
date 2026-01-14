/// Trace the right edge coefficients
#[test]
fn trace_right_edge_coefficients() {
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::jpeg2000::dwt::Dwt53;
    use jpegexp_rs::FrameInfo;
    
    let size = 40;
    
    // Create test image: all 128 except (39,0)=129
    let mut pixels = vec![128u8; size * size];
    pixels[39] = 129;
    
    // Apply level shift
    let level_shift = 1 << 7;
    let mut data: Vec<i32> = pixels.iter().map(|&p| p as i32 - level_shift).collect();
    
    println!("After level shift:");
    println!("  (0,0) = {}", data[0]);
    println!("  (39,0) = {}", data[39]);
    println!("  (0,1) = {}", data[40]);
    println!("  (39,1) = {}", data[79]);
    
    // Apply DWT manually
    let mut coeffs = data.clone();
    
    // Row DWT
    for y in 0..size {
        let row_start = y * size;
        let row: Vec<i32> = coeffs[row_start..row_start + size].to_vec();
        let l_len = (size + 1) / 2;
        let h_len = size / 2;
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
    
    println!("\nAfter row DWT:");
    println!("  Row 0, LL (0-19): {:?}", &coeffs[0..5]);
    println!("  Row 0, HL (20-39): {:?}", &coeffs[20..25]);
    println!("  Row 0, HL last 5: {:?}", &coeffs[35..40]);
    
    // Column DWT
    for x in 0..size {
        let col: Vec<i32> = (0..size).map(|y| coeffs[y * size + x]).collect();
        let l_len = (size + 1) / 2;
        let h_len = size / 2;
        let mut l = vec![0i32; l_len];
        let mut h = vec![0i32; h_len];
        
        Dwt53::forward(&col, &mut l, &mut h);
        
        for (i, &v) in l.iter().enumerate() {
            coeffs[i * size + x] = v;
        }
        for (i, &v) in h.iter().enumerate() {
            coeffs[(l_len + i) * size + x] = v;
        }
    }
    
    println!("\nAfter 2D DWT:");
    println!("  LL (0-19, 0-19) at (0,0): {:?}", &coeffs[0..5]);
    println!("  HL (0-19, 20-39) at (20,0): {:?}", &coeffs[20..25]);
    println!("  HL column at x=39: {:?}", (0..size).map(|y| coeffs[y * size + 39]).collect::<Vec<_>>());
    println!("  HL (20-39, 0-19) at (0,20): {:?}", &coeffs[20*size..20*size+5]);
    println!("  HH (20-39, 20-39) at (20,20): {:?}", &coeffs[20*size+20..20*size+25]);
    
    // Now encode with our encoder and see what coefficients it produces
    std::env::set_var("J2K_EXTRACT_DEBUG", "1");
    
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    let mut j2k_data = vec![0u8; 1024 * 1024];
    let _ = encoder.encode(&pixels, &frame_info, &mut j2k_data).unwrap();
    
    std::env::remove_var("J2K_EXTRACT_DEBUG");
    
    // Now check the HL subband coefficients more carefully
    println!("\n=== HL Subband Analysis ===");
    println!("HL subband should be at (20,0) with size 20x20");
    println!("Coefficients at HL positions:");
    for y in 0..5 {
        let row: Vec<i32> = (20..40).map(|x| coeffs[y * size + x]).collect();
        println!("  y={}: {:?}", y, &row);
    }
    
    // Check if the +129 at (39,0) affects the DWT coefficients
    println!("\n=== Impact of (39,0)=129 ===");
    println!("Original pixel (39,0) = 129, after level shift = {}", data[39]);
    println!("Row 0, HL last coefficient (position 39 in original, 19 in HL): {}", coeffs[39]);
    
    // The coefficient at position 39 in row 0 is in HL subband at (19, 0)
    // Let's see what value it has
    if coeffs[39] != 0 {
        println!("  HL coefficient at (19,0) = {} (NON-ZERO!)", coeffs[39]);
    } else {
        println!("  HL coefficient at (19,0) = 0");
    }
}

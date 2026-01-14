/// Debug encoder coefficient storage
#[test]
fn debug_encoder_coefficients() {
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::jpeg2000::dwt::Dwt53;
    use jpegexp_rs::FrameInfo;
    
    let size = 40;
    let mut pixels = vec![0u8; size * size];
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x + y) % 256) as u8;
        }
    }
    
    // Apply level shift
    let level_shift = 1 << 7;
    let mut data: Vec<i32> = pixels.iter().map(|&p| p as i32 - level_shift).collect();
    
    // Manually compute expected DWT coefficients
    let mut expected_coeffs = data.clone();
    
    // Row DWT
    for y in 0..size {
        let row_start = y * size;
        let row: Vec<i32> = expected_coeffs[row_start..row_start + size].to_vec();
        let l_len = (size + 1) / 2;
        let h_len = size / 2;
        let mut l = vec![0i32; l_len];
        let mut h = vec![0i32; h_len];
        
        Dwt53::forward(&row, &mut l, &mut h);
        
        for (i, &v) in l.iter().enumerate() {
            expected_coeffs[row_start + i] = v;
        }
        for (i, &v) in h.iter().enumerate() {
            expected_coeffs[row_start + l_len + i] = v;
        }
    }
    
    // Column DWT
    for x in 0..size {
        let col: Vec<i32> = (0..size).map(|y| expected_coeffs[y * size + x]).collect();
        let l_len = (size + 1) / 2;
        let h_len = size / 2;
        let mut l = vec![0i32; l_len];
        let mut h = vec![0i32; h_len];
        
        Dwt53::forward(&col, &mut l, &mut h);
        
        for (i, &v) in l.iter().enumerate() {
            expected_coeffs[i * size + x] = v;
        }
        for (i, &v) in h.iter().enumerate() {
            expected_coeffs[(l_len + i) * size + x] = v;
        }
    }
    
    println!("Expected coefficients after 2D DWT:");
    println!("  LL (0,0 to 19,19): {:?}", &expected_coeffs[0..5]);
    println!("  HL (20,0 to 39,19): {:?}", &expected_coeffs[20..25]);
    println!("  LH (0,20 to 19,39): {:?}", &expected_coeffs[20*size..20*size+5]);
    println!("  HH (20,20 to 39,39): {:?}", &expected_coeffs[20*size+20..20*size+25]);
    
    // Now use the encoder
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    // We need to access the internal coefficients somehow
    // Let's use environment variable to enable debug output
    std::env::set_var("J2K_DWT_DEBUG", "1");
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    std::env::remove_var("J2K_DWT_DEBUG");
    
    println!("\nEncoder output size: {} bytes", output_size);
    
    // The encoder's coefficients are passed to encode_component_packets
    // Let's trace what coefficients are being extracted
    // Use J2K_EXTRACT_DEBUG to see extraction
    
    std::env::set_var("J2K_EXTRACT_DEBUG", "1");
    
    let output_size = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    std::env::remove_var("J2K_EXTRACT_DEBUG");
    
    println!("\nEncoded with debug output (check stderr)");
}

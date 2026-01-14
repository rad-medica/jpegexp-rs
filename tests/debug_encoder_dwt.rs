/// Debug encoder DWT to see what's happening
#[test]
fn debug_encoder_dwt() {
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
    
    // Apply level shift (center around 0)
    let level_shift = 1 << 7; // 128 for 8-bit
    let mut data: Vec<i32> = pixels.iter().map(|&p| p as i32 - level_shift).collect();
    
    println!("After level shift (first 5x5):");
    for y in 0..5 {
        print!("  ");
        for x in 0..5 {
            print!("{:4} ", data[y * size + x]);
        }
        println!();
    }
    
    // Manually apply 1-level DWT
    let mut coeffs = data.clone();
    
    // Apply 1D DWT to rows
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
    
    println!("\nAfter row DWT (first 5): {:?}", &coeffs[..5]);
    println!("After row DWT (row 0, last 5): {:?}", &coeffs[size-5..size]);
    
    // Apply 1D DWT to columns
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
    
    println!("\nAfter 2D DWT (LL 5x5 at 0,0):");
    for y in 0..5 {
        print!("  ");
        for x in 0..5 {
            print!("{:4} ", coeffs[y * size + x]);
        }
        println!();
    }
    
    println!("\nAfter 2D DWT (HL 5x5 at 20,0):");
    for y in 0..5 {
        print!("  ");
        for x in 0..5 {
            print!("{:4} ", coeffs[y * size + 20 + x]);
        }
        println!();
    }
    
    // Now use the encoder and compare
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    println!("\nEncoder output size: {} bytes", output_size);
    
    // Extract the coefficients from the encoder's internal state
    // This is tricky without access to internals, so let's decode and compare
    use std::process::Command;
    use std::fs;
    
    fs::write("debug_ours.j2k", &output[..output_size]).unwrap();
    
    let decode_result = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "debug_ours.j2k", "-o", "debug_decoded.pnm"])
        .output()
        .expect("Failed to decode");
    
    if !decode_result.status.success() {
        println!("Decode failed:");
        println!("{}", String::from_utf8_lossy(&decode_result.stderr));
        return;
    }
    
    // Parse and compare
    fn parse_pnm(data: &[u8]) -> Vec<u8> {
        let mut offset = 0;
        while offset < data.len() && data[offset] != b'\n' { offset += 1; }
        offset += 1;
        while offset < data.len() && (data[offset] == b'#' || data[offset] == b'\n') {
            while offset < data.len() && data[offset] != b'\n' { offset += 1; }
            offset += 1;
        }
        while offset < data.len() && data[offset] != b'\n' { offset += 1; }
        offset += 1;
        while offset < data.len() && data[offset] != b'\n' { offset += 1; }
        offset += 1;
        data[offset..].to_vec()
    }
    
    let decoded_data = fs::read("debug_decoded.pnm").unwrap();
    let decoded_pixels = parse_pnm(&decoded_data);
    
    // Compare
    let mut errors = 0;
    for i in 0..pixels.len().min(decoded_pixels.len()) {
        if pixels[i] != decoded_pixels[i] {
            if errors < 5 {
                let x = i % size;
                let y = i / size;
                println!("Error at ({},{}): orig={}, decoded={}", x, y, pixels[i], decoded_pixels[i]);
            }
            errors += 1;
        }
    }
    
    println!("\nTotal errors: {} / {}", errors, pixels.len());
    
    if errors == 0 {
        println!("✅ Perfect reconstruction!");
    } else {
        println!("❌ Errors remain");
    }
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::dwt::Dwt53;
use jpegexp_rs::FrameInfo;

#[test]
fn trace_dwt_gradient() {
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    
    println!("Original pixels:");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:4} ", pixels[y * 4 + x]);
        }
        println!();
    }
    
    let level_shifted: Vec<i32> = pixels.iter().map(|&p| p as i32 - 128).collect();
    println!("\nLevel-shifted:");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:4} ", level_shifted[y * 4 + x]);
        }
        println!();
    }
    
    let mut coeffs = level_shifted.clone();
    
    for y in 0..4 {
        let row: Vec<i32> = (0..4).map(|x| coeffs[y * 4 + x]).collect();
        let mut out_l = vec![0i32; 2];
        let mut out_h = vec![0i32; 2];
        Dwt53::forward(&row, &mut out_l, &mut out_h);
        
        coeffs[y * 4 + 0] = out_l[0];
        coeffs[y * 4 + 1] = out_l[1];
        coeffs[y * 4 + 2] = out_h[0];
        coeffs[y * 4 + 3] = out_h[1];
    }
    
    println!("\nAfter row DWT:");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:4} ", coeffs[y * 4 + x]);
        }
        println!();
    }
    
    for x in 0..4 {
        let col: Vec<i32> = (0..4).map(|y| coeffs[y * 4 + x]).collect();
        let mut out_l = vec![0i32; 2];
        let mut out_h = vec![0i32; 2];
        Dwt53::forward(&col, &mut out_l, &mut out_h);
        
        coeffs[0 * 4 + x] = out_l[0];
        coeffs[1 * 4 + x] = out_l[1];
        coeffs[2 * 4 + x] = out_h[0];
        coeffs[3 * 4 + x] = out_h[1];
    }
    
    println!("\nAfter column DWT (final coefficients):");
    println!("LL (top-left 2x2):");
    for y in 0..2 {
        for x in 0..2 {
            print!("{:4} ", coeffs[y * 4 + x]);
        }
        println!();
    }
    println!("\nHL (top-right 2x2):");
    for y in 0..2 {
        for x in 2..4 {
            print!("{:4} ", coeffs[y * 4 + x]);
        }
        println!();
    }
    println!("\nLH (bottom-left 2x2):");
    for y in 2..4 {
        for x in 0..2 {
            print!("{:4} ", coeffs[y * 4 + x]);
        }
        println!();
    }
    println!("\nHH (bottom-right 2x2):");
    for y in 2..4 {
        for x in 2..4 {
            print!("{:4} ", coeffs[y * 4 + x]);
        }
        println!();
    }
}

#[test]
fn trace_full_j2k_encoding() {
    use std::fs;
    use std::process::Command;
    
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let frame_info = FrameInfo {
        width: 4,
        height: 4,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; 500];
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    output.truncate(bytes_written);
    
    fs::write("trace_gradient.j2k", &output).unwrap();
    
    let pgm_header = "P5\n4 4\n255\n";
    let mut pgm_data = pgm_header.as_bytes().to_vec();
    pgm_data.extend_from_slice(&pixels);
    fs::write("trace_gradient.pgm", &pgm_data).unwrap();
    
    let _ = Command::new("libs/bin/opj_compress.exe")
        .args(&["-i", "trace_gradient.pgm", "-o", "trace_gradient_opj.j2k", "-n", "2"])
        .output();
    
    let mut sod_pos = None;
    for i in 0..output.len() - 1 {
        if output[i] == 0xFF && output[i + 1] == 0x93 {
            sod_pos = Some(i);
            break;
        }
    }
    
    if let Some(pos) = sod_pos {
        let tile_data = &output[pos + 2..];
        println!("Our tile data ({} bytes):", tile_data.len());
        println!("{}", tile_data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
    }
    
    if let Ok(opj_data) = fs::read("trace_gradient_opj.j2k") {
        for i in 0..opj_data.len() - 1 {
            if opj_data[i] == 0xFF && opj_data[i + 1] == 0x93 {
                let tile_data = &opj_data[i + 2..];
                println!("\nOpenJPEG tile data ({} bytes):", tile_data.len());
                println!("{}", tile_data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
                break;
            }
        }
    }
    
    let _ = fs::remove_file("trace_gradient.j2k");
    let _ = fs::remove_file("trace_gradient.pgm");
    let _ = fs::remove_file("trace_gradient_opj.j2k");
}

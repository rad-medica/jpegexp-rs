use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn debug_decomposition_level_sweep() {
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Decomposition Level Sweep (64x64 gradient) ===\n");
    
    for levels in 0..=5 {
        test_with_levels(&pixels, width, height, levels);
    }
}

fn test_with_levels(pixels: &[u8], width: usize, height: usize, levels: u8) {
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    // Set environment variable for debugging
    std::env::set_var("J2K_CBLK_DETAIL", "1");
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(levels);
    encoder.set_include_tlm(false);  // Disable TLM for OpenJPEG compatibility
    encoder.set_include_plt(false);  // Disable PLT for OpenJPEG compatibility
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    let our_bytes = &our_output[..our_size];
    
    let our_filename = format!("debug_ours_{}levels.j2k", levels);
    let opj_filename = format!("debug_opj_{}levels.j2k", levels);
    let raw_filename = format!("debug_input_{}levels.raw", levels);
    
    fs::write(&our_filename, our_bytes).unwrap();
    fs::write(&raw_filename, &pixels).unwrap();
    
    // Use reversible (lossless) mode - no -I flag
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", &raw_filename,
            "-o", &opj_filename,
            "-n", &format!("{}", levels + 1),
            "-r", "1",
            "-F", &format!("{},{},1,8,u", width, height),
            // No -I flag = reversible (lossless) 5-3 DWT
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    if !status.success() {
        println!("❌ Level {}: opj_compress failed", levels);
        return;
    }
    
    let opj_bytes = fs::read(&opj_filename).unwrap();
    
    let our_decoded_pnm = format!("debug_ours_decoded_{}levels.pnm", levels);
    
    let status1 = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", &our_filename, "-o", &our_decoded_pnm])
        .output()
        .expect("Failed to decode our file");
    
    if !status1.status.success() {
        println!("❌ Level {}: Decode failed", levels);
        return;
    }
    
    let our_decoded_data = fs::read(&our_decoded_pnm).unwrap();
    
    fn parse_pnm(data: &[u8]) -> Vec<u8> {
        let mut offset = 0;
        while offset < data.len() && data[offset] != b'\n' {
            offset += 1;
        }
        offset += 1;
        while offset < data.len() && (data[offset] == b'#' || data[offset] == b'\n') {
            while offset < data.len() && data[offset] != b'\n' {
                offset += 1;
            }
            offset += 1;
        }
        while offset < data.len() && data[offset] != b'\n' {
            offset += 1;
        }
        offset += 1;
        while offset < data.len() && data[offset] != b'\n' {
            offset += 1;
        }
        offset += 1;
        data[offset..].to_vec()
    }
    
    let our_pixels_decoded = parse_pnm(&our_decoded_data);
    
    // Also decode with our own decoder for comparison
    let our_own_decoded = {
        let mut reader = JpegStreamReader::new(our_bytes);
        let mut decoder = J2kDecoder::new(&mut reader);
        match decoder.decode() {
            Ok(image) => image.reconstruct_pixels().ok(),
            Err(_) => None,
        }
    };
    
    let mut sum_error = 0.0;
    let mut max_error = 0;
    let mut error_count = 0;
    
    let pixel_count = (width * height).min(our_pixels_decoded.len());
    
    for i in 0..pixel_count {
        let original = pixels[i] as i32;
        let our_decoded = our_pixels_decoded[i] as i32;
        let error = (original - our_decoded).abs();
        
        sum_error += error as f64;
        max_error = max_error.max(error);
        
        if error > 0 {
            error_count += 1;
        }
    }
    
    let mae = sum_error / pixel_count as f64;
    
    let status = if mae == 0.0 { "✅" } else { "❌" };
    
    // Calculate MAE for our own decoder
    let our_own_mae = if let Some(ref decoded) = our_own_decoded {
        let mut sum = 0.0;
        for i in 0..pixel_count.min(decoded.len()) {
            sum += (pixels[i] as i32 - decoded[i] as i32).abs() as f64;
        }
        sum / pixel_count as f64
    } else {
        -1.0
    };
    
    println!("{} Level {}: MAE={:.4} (OPJ decode), OurDec={:.4}, Max={}, Errors={}/{}, Size={}B (vs {}B OpenJPEG)",
             status, levels, mae, our_own_mae, max_error, error_count, pixel_count, our_size, opj_bytes.len());
}

/// Compare our 16-bit encoding with OpenJPEG's reference encoding
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn test_compare_with_openjpeg_16bit() {
    let size = 10;
    let mut pixels_u16 = vec![0u16; size * size];
    
    for y in 0..size {
        for x in 0..size {
            pixels_u16[y * size + x] = ((x * 4 + y * 4) % 256) as u16;
        }
    }
    
    // Convert to little-endian bytes for raw file
    let mut raw_bytes = vec![0u8; size * size * 2];
    for i in 0..pixels_u16.len() {
        let bytes = pixels_u16[i].to_le_bytes();
        raw_bytes[i*2] = bytes[0];
        raw_bytes[i*2+1] = bytes[1];
    }
    
    fs::write("test_10x10_16bit_input.raw", &raw_bytes).unwrap();
    
    // Encode with OpenJPEG
    let result = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "test_10x10_16bit_input.raw",
            "-o", "test_10x10_openjpeg_ref.j2k",
            "-n", "3", // 2 decomposition levels
            "-r", "1", // lossless
            "-F", "10,10,1,16,u", // width, height, components, bit_depth, unsigned
        ])
        .output()
        .expect("Failed to run opj_compress");
    
    println!("OpenJPEG encode stdout: {}", String::from_utf8_lossy(&result.stdout));
    println!("OpenJPEG encode stderr: {}", String::from_utf8_lossy(&result.stderr));
    
    if !result.status.success() {
        panic!("OpenJPEG encoding failed!");
    }
    
    // Encode with our encoder
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 16,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    encoder.set_include_tlm(false);
    encoder.set_include_plt(false);
    
    // Convert u16 to u8 slice (our encoder expects bytes)
    let pixels_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(
            pixels_u16.as_ptr() as *const u8,
            pixels_u16.len() * 2
        )
    };
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(pixels_bytes, &frame_info, &mut output).unwrap();
    
    fs::write("test_10x10_ours.j2k", &output[..output_size]).unwrap();
    
    println!("\nOurs: {} bytes", output_size);
    println!("OpenJPEG: {} bytes", fs::metadata("test_10x10_openjpeg_ref.j2k").unwrap().len());
    
    // Now dump both files
    println!("\n=== Dumping OpenJPEG reference ===");
    let dump_result = Command::new("libs/bin/opj_dump.exe")
        .args(&["-i", "test_10x10_openjpeg_ref.j2k"])
        .output()
        .expect("Failed to run opj_dump");
    println!("{}", String::from_utf8_lossy(&dump_result.stdout));
    
    println!("\n=== Dumping our output ===");
    let dump_result = Command::new("libs/bin/opj_dump.exe")
        .args(&["-i", "test_10x10_ours.j2k"])
        .output()
        .expect("Failed to run opj_dump");
    println!("{}", String::from_utf8_lossy(&dump_result.stdout));
    
    // Try decoding both
    println!("\n=== Decoding OpenJPEG reference ===");
    let decode_result = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_10x10_openjpeg_ref.j2k", "-o", "test_10x10_openjpeg_decoded.pnm"])
        .output()
        .expect("Failed to run opj_decompress");
    println!("{}", String::from_utf8_lossy(&decode_result.stdout));
    
    println!("\n=== Decoding our output ===");
    let decode_result = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_10x10_ours.j2k", "-o", "test_10x10_ours_decoded.pnm"])
        .output()
        .expect("Failed to run opj_decompress");
    println!("{}", String::from_utf8_lossy(&decode_result.stdout));
    println!("{}", String::from_utf8_lossy(&decode_result.stderr));
}

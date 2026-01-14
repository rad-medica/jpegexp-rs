/// Count codeblocks generated for each resolution level
/// This helps identify if we're missing codeblocks compared to OpenJPEG

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

#[test]
#[ignore]
fn count_codeblocks_by_resolution() {
    std::env::set_var("J2K_PKT_DEBUG", "1");
    
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as u8;
        }
    }
    
    println!("\n=== Codeblock Count Analysis (Level 2) ===\n");
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let mut our_output = vec![0u8; 1024 * 1024];
    let our_size = encoder.encode(&pixels, &frame_info, &mut our_output).unwrap();
    
    println!("\nTotal encoded: {} bytes", our_size);
    println!("\nExpected OpenJPEG: ~1106 bytes");
    println!("Difference: {} bytes ({}% of OpenJPEG)",
        1106 - our_size,
        100 * (1106 - our_size) / 1106);
}

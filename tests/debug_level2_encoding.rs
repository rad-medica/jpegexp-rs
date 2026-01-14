use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use std::env;

#[test]
#[ignore]
fn debug_level2_encoding_process() {
    env::set_var("J2K_PKT_DEBUG", "1");
    
    println!("\n=== Level 2 Encoding Debug ===\n");
    
    let width = 64;
    let height = 64;
    let mut image = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            image[y * width + x] = ((x + y) * 255 / (width + height - 2)) as u8;
        }
    }
    
    println!("Testing with 1 decomposition level (2 resolutions) - SHOULD WORK");
    let mut encoder1 = J2kEncoder::new();
    encoder1.decomposition_levels = 1;
    encoder1.lossless = true;
    encoder1.irreversible = false;
    
    match encoder1.encode_single_component(&image, width, height, 8) {
        Ok(data) => println!("Level 1: Encoded {} bytes\n", data.len()),
        Err(e) => println!("Level 1: ERROR - {:?}\n", e),
    }
    
    println!("\n{'='*60}\n");
    println!("Testing with 2 decomposition levels (3 resolutions) - FAILS");
    
    let mut encoder2 = J2kEncoder::new();
    encoder2.decomposition_levels = 2;
    encoder2.lossless = true;
    encoder2.irreversible = false;
    
    match encoder2.encode_single_component(&image, width, height, 8) {
        Ok(data) => println!("Level 2: Encoded {} bytes", data.len()),
        Err(e) => println!("Level 2: ERROR - {:?}", e),
    }
}

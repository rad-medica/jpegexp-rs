use jpegexp_rs::jpeg2000::encoder::J2kEncoder;

#[test]
#[ignore]
fn isolated_level2_test() {
    std::env::set_var("J2K_PKT_DEBUG", "1");
    
    let width = 64;
    let height = 64;
    let mut image = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            image[y * width + x] = ((x + y) * 255 / (width + height - 2)) as u8;
        }
    }
    
    println!("\n=== ISOLATED LEVEL 2 TEST (64x64, 2 decomp levels) ===\n");
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(2);
    
    let j2k_data = encoder.encode(&image, width as u32, height as u32, 8, false).unwrap();
    
    println!("\nEncoded {} bytes", j2k_data.len());
}

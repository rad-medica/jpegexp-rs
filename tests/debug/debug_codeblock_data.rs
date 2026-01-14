/// Debug codeblock data for edge pixel
#[test]
fn debug_codeblock_data() {
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    
    let size = 40;
    
    // Create test image: all 128 except (39,0)=129
    let mut pixels = vec![128u8; size * size];
    pixels[39] = 129;
    
    // Enable debug output
    std::env::set_var("J2K_CBLK_DETAIL", "1");
    
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
    
    std::env::remove_var("J2K_CBLK_DETAIL");
    
    // The debug output goes to stderr, so check the test output
    println!("\n⚠️  Check stderr output above for [CBLK_PRE] and [CBLK_POST] messages");
}

/// Debug to find the source of +1 error at x=39
#[test]
fn debug_single_pixel_error() {
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    
    let size = 40;
    
    // Test with our encoder - compare with OpenJPEG
    let mut pixels = vec![128u8; size * size];
    pixels[39] = 129; // Only rightmost pixel different
    
    // Encode with our encoder
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
    let j2k_size = encoder.encode(&pixels, &frame_info, &mut j2k_data).unwrap();
    let j2k_data = &j2k_data[..j2k_size];
    
    println!("Our encoder: {} bytes", j2k_size);
    
    // Write and decode with OpenJPEG
    use std::process::Command;
    use std::fs;
    
    fs::write("debug_test_ours.j2k", j2k_data).unwrap();
    
    // Also encode the same input with OpenJPEG for comparison
    fs::write("debug_test_input.raw", &pixels).unwrap();
    
    let status = Command::new("libs/bin/opj_compress.exe")
        .args(&[
            "-i", "debug_test_input.raw",
            "-o", "debug_test_opj.j2k",
            "-n", "2",  // 2 decomposition levels (1 + 1)
            "-r", "1",
            "-F", "40,40,1,8,u",
        ])
        .status()
        .expect("Failed to run opj_compress");
    
    if !status.success() {
        println!("opj_compress failed");
        return;
    }
    
    println!("OpenJPEG encoder: success");
    
    // Decode both with OpenJPEG
    let opj_ours = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "debug_test_ours.j2k", "-o", "debug_ours_decoded.pnm"])
        .output()
        .expect("Failed to run opj_decompress");
    
    let opj_opj = Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "debug_test_opj.j2k", "-o", "debug_opj_decoded.pnm"])
        .output()
        .expect("Failed to run opj_decompress");
    
    if !opj_ours.status.success() {
        println!("OpenJPEG decode of ours failed");
        return;
    }
    
    if !opj_opj.status.success() {
        println!("OpenJPEG decode of opj failed");
        return;
    }
    
    // Parse PNM files
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
    
    let ours_decoded = parse_pnm(&fs::read("debug_ours_decoded.pnm").unwrap());
    let opj_decoded = parse_pnm(&fs::read("debug_opj_decoded.pnm").unwrap());
    
    println!("\nComparison at x=35 to 39:");
    println!("  Original: {:?}", &pixels[35..40]);
    println!("  Ours->OPJ: {:?}", &ours_decoded[35..40]);
    println!("  OPJ->OPJ: {:?}", &opj_decoded[35..40]);
    
    // Count differences
    let mut ours_vs_orig = 0;
    let mut opj_vs_orig = 0;
    let mut ours_vs_opj = 0;
    
    for i in 0..pixels.len().min(ours_decoded.len()).min(opj_decoded.len()) {
        if pixels[i] != ours_decoded[i] { ours_vs_orig += 1; }
        if pixels[i] != opj_decoded[i] { opj_vs_orig += 1; }
        if ours_decoded[i] != opj_decoded[i] { ours_vs_opj += 1; }
    }
    
    println!("\nTotal pixels: {}", pixels.len());
    println!("Ours->OPJ vs Original: {} differences", ours_vs_orig);
    println!("OPJ->OPJ vs Original: {} differences", opj_vs_orig);
    println!("Ours->OPJ vs OPJ->OPJ: {} differences", ours_vs_opj);
    
    if ours_vs_opj == 0 {
        println!("\n✅ Our encoded file is DECODED identically by OpenJPEG");
        println!("   The +1 error is in OpenJPEG's decoder, not our encoder!");
    } else {
        println!("\n❌ Our encoded file differs from OpenJPEG's");
        println!("   Issue is in our ENCODER");
    }
}

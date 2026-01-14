/// Test if the issue is in bit-plane coding of edge pixels
#[test]
fn test_edge_bitplane_coding() {
    use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
    use jpegexp_rs::FrameInfo;
    use std::process::Command;
    use std::fs;
    
    let size = 40;
    
    // Test 1: All zeros - should encode perfectly
    let pixels1 = vec![128u8; size * size];
    
    // Test 2: Only (39,0) different
    let mut pixels2 = vec![128u8; size * size];
    pixels2[39] = 129;
    
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
    
    // Encode test 1
    let mut j2k_data = vec![0u8; 1024 * 1024];
    let j2k_size1 = encoder.encode(&pixels1, &frame_info, &mut j2k_data).unwrap();
    fs::write("test_all_zeros.j2k", &j2k_data[..j2k_size1]).unwrap();
    
    // Encode test 2
    let mut j2k_data = vec![0u8; 1024 * 1024];
    let j2k_size2 = encoder.encode(&pixels2, &frame_info, &mut j2k_data).unwrap();
    fs::write("test_edge_diff.j2k", &j2k_data[..j2k_size2]).unwrap();
    
    println!("Test 1 (all 128): {} bytes", j2k_size1);
    println!("Test 2 (edge diff): {} bytes", j2k_size2);
    println!("Difference: {} bytes", j2k_size2 - j2k_size1);
    
    // Decode both with OpenJPEG
    Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_all_zeros.j2k", "-o", "test_all_zeros_decoded.pnm"])
        .output()
        .expect("Failed");
    
    Command::new("libs/bin/opj_decompress.exe")
        .args(&["-i", "test_edge_diff.j2k", "-o", "test_edge_diff_decoded.pnm"])
        .output()
        .expect("Failed");
    
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
    
    let decoded1 = parse_pnm(&fs::read("test_all_zeros_decoded.pnm").unwrap());
    let decoded2 = parse_pnm(&fs::read("test_edge_diff_decoded.pnm").unwrap());
    
    println!("\nFirst row comparison:");
    for x in 35..size {
        let d1 = decoded1[x];
        let d2 = decoded2[x];
        let orig1 = pixels1[x];
        let orig2 = pixels2[x];
        println!("  x={}: orig1={}, dec1={}, orig2={}, dec2={}, diff={}", 
                 x, orig1, d1, orig2, d2, (d2 as i32 - d1 as i32));
    }
    
    // The difference in decoded images should show where the error is
    let mut diff_count = 0;
    let mut diff_sum = 0i32;
    for i in 0..decoded1.len().min(decoded2.len()) {
        let diff = decoded2[i] as i32 - decoded1[i] as i32;
        if diff != 0 {
            diff_count += 1;
            diff_sum += diff;
        }
    }
    
    println!("\nDifferences between decoded images: {} pixels", diff_count);
    println!("Sum of differences: {}", diff_sum);
    
    if diff_count == 1 && diff_sum == -1 {
        println!("\n✅ Confirmed: Edge pixel is decoded as orig-1 (systematic -1 error)");
    } else if diff_count == 0 {
        println!("\n✅ No difference - both encode/decode identically");
    } else {
        println!("\n⚠️  Multiple differences detected");
    }
}

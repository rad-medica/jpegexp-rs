use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

#[test]
fn trace_packet_detailed() {
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    
    println!("Input pixels (4x4 gradient):");
    for y in 0..4 {
        print!("  ");
        for x in 0..4 {
            print!("{:3} ", pixels[y * 4 + x]);
        }
        println!();
    }
    
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
    
    println!("\n=== Encoding (packet header trace enabled if compiled with feature) ===\n");
    
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    output.truncate(bytes_written);
    
    println!("\n=== Expected packet structure ===");
    println!("Resolution 0: 1 subband (LL), 1x1 codeblock grid");
    println!("Resolution 1: 3 subbands (HL, LH, HH), each 1x1 codeblock grid");
    println!();
    println!("From trace_full_pipeline output:");
    println!("  LL: 22 passes, 6 bytes");
    println!("  HL: 13 passes, 4 bytes");
    println!("  LH: 13 passes, 4 bytes");
    println!("  HH: all zeros (should be excluded)");
    println!();
    println!("For HL with 13 passes, 4 bytes:");
    println!("  bits_needed = floor(log2(4)) + 1 = 2 + 1 = 3");
    println!("  log2_passes = floor(log2(13)) = 3");
    println!("  increment = 3 - 3 - 3 = -3 → max(0) = 0");
    println!("  lblock = 3 + 0 = 3");
    println!("  lbits = 3 + 3 = 6");
    println!("  Comma code for increment=0: 0 (single bit)");
    println!("  Data length in 6 bits: {:06b} = {}", 4, 4);
    println!();
    println!("For LH with 13 passes, 4 bytes:");
    println!("  Same calculation as HL");
}

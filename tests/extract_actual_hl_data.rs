/// Extract the actual HL subband data from level 2 encoding
/// This will show us what coefficients we're actually encoding

use jpegexp_rs::jpeg2000::encoder::Jpeg2000Encoder;
use jpegexp_rs::jpeg2000::dwt::Dwt;

#[test]
#[ignore]
fn extract_hl_subband_level2() {
    // Create the same 64x64 gradient test image
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u16; (width * height) as usize];
    
    for y in 0..height {
        for x in 0..width {
            pixels[(y * width + x) as usize] = ((x + y * 2) % 256) as u16;
        }
    }
    
    println!("\n=== Extracting HL Subband Data from Level 2 ===");
    println!("Input: 64x64 gradient image");
    
    // Perform DWT to level 2
    let mut dwt = Dwt::new(width, height);
    let mut coeffs: Vec<i32> = pixels.iter().map(|&p| p as i32).collect();
    
    dwt.forward_5x3(&mut coeffs, 2);
    
    // After 2 levels of DWT, we have:
    // Level 0: 64x64 (original)
    // Level 1: 32x32 LL, 32x32 HL, 32x32 LH, 32x32 HH
    // Level 2: 16x16 LL, 16x16 HL, 16x16 LH, 16x16 HH (from level 1's LL)
    //          + level 1's HL, LH, HH (still 32x32)
    
    // The layout in coeffs after 2-level DWT:
    // Top-left 16x16: LL2
    // Top-right 16 cols, rows 0-15: HL2
    // Bottom-left 16 rows, cols 0-15: LH2
    // Bottom-right 16x16: HH2
    // Right half (cols 16-31, rows 0-31): HL1 (32x16)
    // Bottom half (cols 0-31, rows 16-31): LH1 (16x32)
    // Bottom-right (cols 16-31, rows 16-31): HH1 (16x16)
    
    // Wait, this is getting complex. Let me use the encoder's subband extraction
    let encoder = Jpeg2000Encoder::new(width, height, 8, 2);
    
    // Extract subbands
    let subbands = encoder.extract_subbands(&coeffs);
    
    println!("\nSubbands extracted:");
    for (i, sb) in subbands.iter().enumerate() {
        println!("  Subband {}: {}x{}, orientation={}, level={}", 
                 i, sb.width, sb.height, sb.orientation, sb.level);
    }
    
    // Find the HL subband at level 2
    let hl2_subband = subbands.iter()
        .find(|sb| sb.orientation == 1 && sb.level == 2)
        .expect("Should have HL2 subband");
    
    println!("\nHL2 Subband: {}x{}", hl2_subband.width, hl2_subband.height);
    println!("Data sample (top-left 8x8):");
    for y in 0..8.min(hl2_subband.height) {
        print!("  ");
        for x in 0..8.min(hl2_subband.width) {
            let idx = (y * hl2_subband.width + x) as usize;
            print!("{:4} ", hl2_subband.data[idx]);
        }
        println!();
    }
    
    // Now encode just this codeblock
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;
    
    let mut bpc = BitPlaneCoder::new(hl2_subband.width, hl2_subband.height, &hl2_subband.data);
    let max_bp = bpc.calculate_max_bit_plane();
    
    if let Some(max_bp) = max_bp {
        println!("\nMax bit-plane: {}", max_bp);
        
        let passes = bpc.encode_codeblock(max_bp, 0, 1); // orientation=1 for HL
        bpc.mq.flush();
        let encoded = bpc.mq.get_buffer();
        
        println!("\nEncoding complete:");
        println!("  Passes: {}", passes);
        println!("  Encoded bytes: {}", encoded.len());
        println!("  First 20 bytes: {:02X?}", &encoded[..encoded.len().min(20)]);
        
        println!("\n⚠️  Expected OpenJPEG length: 68 bytes");
        println!("⚠️  Our length: {} bytes", encoded.len());
        println!("⚠️  Difference: {} bytes", encoded.len() as i32 - 68);
        
        // Save the actual data for further analysis
        println!("\nFull HL2 data:");
        for y in 0..hl2_subband.height {
            print!("  ");
            for x in 0..hl2_subband.width {
                let idx = (y * hl2_subband.width + x) as usize;
                print!("{:4} ", hl2_subband.data[idx]);
            }
            println!();
        }
    } else {
        println!("\nHL2 subband is all zeros!");
    }
}

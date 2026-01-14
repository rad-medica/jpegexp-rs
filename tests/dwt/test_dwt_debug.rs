/// Debug DWT coefficient extraction
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;

#[test]
#[ignore]
fn test_dwt_debug() {
    let size = 40;
    
    // Create simple test pattern
    let mut pixels = vec![0u8; size * size];
    for y in 0..size {
        for x in 0..size {
            pixels[y * size + x] = ((x + y) % 16) as u8; // Simple pattern
        }
    }
    
    // Let's manually trace through the DWT
    // First, create a 40x40 i32 array
    let mut data: Vec<i32> = pixels.iter().map(|&p| p as i32).collect();
    
    println!("Original data (first 8x8):");
    for y in 0..8 {
        print!("  ");
        for x in 0..8 {
            print!("{:3} ", data[y * size + x]);
        }
        println!();
    }
    
    // Apply DWT 5/3 manually (first level)
    // Extract LL subband (20x20)
    let ll_w = (size + 1) / 2; // 20
    let ll_h = (size + 1) / 2; // 20
    
    let mut ll = vec![0i32; ll_w * ll_h];
    for y in 0..ll_h {
        for x in 0..ll_w {
            ll[y * ll_w + x] = data[y * size + x];
        }
    }
    
    println!("\nLL subband extracted (first 5x5):");
    for y in 0..5 {
        print!("  ");
        for x in 0..5 {
            print!("{:3} ", ll[y * ll_w + x]);
        }
        println!();
    }
    
    // Apply 1D DWT to first row of LL
    let mut row: Vec<i32> = ll[..20].to_vec();
    let mut out_l = vec![0i32; 10];
    let mut out_h = vec![0i32; 10];
    
    jpegexp_rs::jpeg2000::dwt::Dwt53::forward(&row, &mut out_l, &mut out_h);
    
    println!("\nFirst row of LL: {:?}", &row[..8]);
    println!("DWT L (first 5): {:?}", &out_l[..5]);
    println!("DWT H (first 5): {:?}", &out_h[..5]);
    
    // Now check what our encoder produces
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let mut output = vec![0u8; 1024 * 1024];
    let output_size = encoder.encode(&pixels, &FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 1,
    }, &mut output).unwrap();
    
    fs::write("test_dwt_debug.j2k", &output[..output_size]).unwrap();
    
    println!("\nEncoded file size: {} bytes", output_size);
    
    // The issue might be in how the DWT result is stored back
    // Let me check the encoder code more carefully
    // Looking at apply_forward_dwt_2d lines 768-773:
    // for y in 0..current_h {
    //     for x in 0..current_w {
    //         result[y * original_width + x] = temp[y * current_w + x];
    //     }
    // }
    
    // After level 0 (40x40 -> 20x20 LL):
    // result[0..20*20] = temp[0..20*20]
    // But temp is the DWT result for the 20x20 region
    // So result[0..400] contains: LL[0..100], HL[100..200], LH[200..300], HH[300..400] for the 20x20 region
    // This is WRONG! We only want to copy LL back!
    
    println!("\nLooking at the apply_forward_dwt_2d code...");
    println!("At line 768-773, the code copies temp back to result for the ENTIRE current_h x current_w region");
    println!("But temp contains LL + HL + LH + HH subbands!");
    println!("We should only copy LL subband back, not the other subbands!");
    
    println!("\nThis means HL/LH/HH coefficients are being OVERWRITTEN with LL data!");
    println!("That's why we get errors - the coefficient values are wrong!");
}

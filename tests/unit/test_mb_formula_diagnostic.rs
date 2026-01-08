/// Diagnostic test to verify M_b calculation formula
/// 
/// This test encodes a simple RGB checkerboard and examines the exact values
/// of depth, guard_bits, epsilon, max_bp, and M_b to diagnose the coefficient
/// reconstruction bug.

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_mb_formula_diagnostic() {
    // Enable debug output
    std::env::set_var("J2K_DEBUG", "1");
    
    println!("\n===================================================================");
    println!("M_b Formula Diagnostic Test");
    println!("===================================================================\n");
    
    // Create 16x16 checkerboard (smallest failing size)
    let size = 16;
    let mut pixels = Vec::with_capacity(size * size * 3);
    for y in 0..size {
        for x in 0..size {
            let is_white = ((x / 8) + (y / 8)) % 2 == 0;
            let val = if is_white { 255u8 } else { 0 };
            pixels.push(val);                           // R
            pixels.push(if is_white { 0 } else { 255 }); // G inverted
            pixels.push(val);                           // B
        }
    }
    
    let frame_info = FrameInfo {
        width: size as u32,
        height: size as u32,
        bits_per_sample: 8,
        component_count: 3,
    };
    
    println!("Image: {}x{} RGB, bits_per_sample={}", size, size, frame_info.bits_per_sample);
    println!("Pattern: Checkerboard with inverted G channel");
    println!("DWT levels: 3 (produces 2x2 LL subband)");
    println!();
    
    let mut encoded = vec![0u8; size * size * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(3);
    
    println!("ENCODING...");
    println!("-------------------------------------------------------------------");
    let encoded_len = encoder.encode(&pixels, &frame_info, &mut encoded).unwrap();
    encoded.truncate(encoded_len);
    
    println!("\nDECODING...");
    println!("-------------------------------------------------------------------");
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().unwrap();
    let decoded = image.reconstruct_pixels().unwrap();
    
    println!("\nRESULTS:");
    println!("-------------------------------------------------------------------");
    println!("Original first pixel: R={}, G={}, B={}", pixels[0], pixels[1], pixels[2]);
    println!("Decoded  first pixel: R={}, G={}, B={}", decoded[0], decoded[1], decoded[2]);
    
    let mae: f64 = pixels.iter().zip(decoded.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).abs() as u64)
        .sum::<u64>() as f64 / pixels.len() as f64;
    
    println!("MAE: {:.6}", mae);
    
    if mae > 0.01 {
        println!("\n❌ TEST FAILED - Coefficients reconstructed incorrectly!");
        println!("\nANALYSIS:");
        println!("Based on the debug output above, check:");
        println!("1. Encoder max_bp for Component 1, Resolution 0 LL");
        println!("2. Decoder max_bp for the same codeblock");
        println!("3. Epsilon values in QCD marker");
        println!("4. M_b calculation: Should be M_b = depth + guard - epsilon");
        println!("5. Current formula: M_b = guard + epsilon - 1 (WRONG!)");
    } else {
        println!("\n✅ TEST PASSED!");
    }
}

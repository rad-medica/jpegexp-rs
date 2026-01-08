/// Debug test to see RCT values during encoding

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_rct_debug() {
    println!("\n================================================================================");
    println!("Debug: RCT Transform with Simple RGB Pattern");
    println!("================================================================================\n");

    // Create a VERY simple pattern: just 4 pixels
    // Pixel 0: R=255, G=0, B=0 (red)
    // Pixel 1: R=0, G=255, B=0 (green)  
    // Pixel 2: R=0, G=0, B=255 (blue)
    // Pixel 3: R=255, G=255, B=255 (white)
    
    let width = 2;
    let height = 2;
    let pixels = vec![
        255, 0, 0,      // pixel 0: red
        0, 255, 0,      // pixel 1: green
        0, 0, 255,      // pixel 2: blue
        255, 255, 255,  // pixel 3: white
    ];
    
    println!("Input pixels (2x2):");
    println!("  [0]: R=255, G=0, B=0 (red)");
    println!("  [1]: R=0, G=255, B=0 (green)");
    println!("  [2]: R=0, G=0, B=255 (blue)");
    println!("  [3]: R=255, G=255, B=255 (white)");
    
    // Expected RCT output (with level shift -128):
    // Pixel 0: R'=127, G'=-128, B'=-128 -> Y=(127+2*(-128)+(-128))/4 = (127-256-128)/4 = -257/4 = -64.25 = -64
    //          U=B'-G'=(-128)-(-128)=0, V=R'-G'=127-(-128)=255
    // Pixel 1: R'=-128, G'=127, B'=-128 -> Y=(-128+2*127+(-128))/4 = (-128+254-128)/4 = -2/4 = 0
    //          U=-128-127=-255, V=-128-127=-255
    // Pixel 2: R'=-128, G'=-128, B'=127 -> Y=(-128+2*(-128)+127)/4 = (-128-256+127)/4 = -257/4 = -64
    //          U=127-(-128)=255, V=-128-(-128)=0
    // Pixel 3: R'=127, G'=127, B'=127 -> Y=(127+2*127+127)/4 = 508/4 = 127
    //          U=127-127=0, V=127-127=0
    
    println!("\nExpected after RCT (Y, U, V):");
    println!("  [0]: Y=-64, U=0, V=255");
    println!("  [1]: Y=0, U=-255, V=-255");
    println!("  [2]: Y=-64, U=255, V=0");
    println!("  [3]: Y=127, U=0, V=0\n");
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };
    
    // Encode with DWT level 0 (no DWT, just color transform)
    let mut encoded = vec![0u8; 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(0);
    
    let encoded_len = encoder.encode(&pixels, &frame_info, &mut encoded).unwrap();
    encoded.truncate(encoded_len);
    
    println!("Encoded {} bytes", encoded_len);
    
    // Decode
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().unwrap();
    let decoded = image.reconstruct_pixels().unwrap();
    
    println!("\nDecoded pixels:");
    for i in 0..4 {
        let idx = i * 3;
        println!("  [{}]: R={}, G={}, B={}", 
                 i, decoded[idx], decoded[idx+1], decoded[idx+2]);
    }
    
    println!("\nDifferences:");
    let mut all_match = true;
    for i in 0..12 {
        if pixels[i] != decoded[i] {
            all_match = false;
            let comp = ["R", "G", "B"][i % 3];
            let pixel = i / 3;
            println!("  Pixel {} {}: expected={}, got={}, diff={}", 
                     pixel, comp, pixels[i], decoded[i], 
                     (pixels[i] as i32 - decoded[i] as i32).abs());
        }
    }
    
    if all_match {
        println!("  ✅ All pixels match!");
    } else {
        println!("  ❌ Some pixels don't match");
    }
}

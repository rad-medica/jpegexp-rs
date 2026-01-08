/// Test 2x2 with inverted G

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_2x2_inverted_g() {
    println!("\n================================================================================");
    println!("Testing 2x2 with Inverted G Channel at DWT Level 3");
    println!("================================================================================\n");

    // 2x2 checkerboard with G inverted
    // Pattern: white=R255,G0,B255  black=R0,G255,B0
    let pixels = vec![
        255, 0, 255,    // [0,0] white
        0, 255, 0,      // [0,1] black
        0, 255, 0,      // [1,0] black
        255, 0, 255,    // [1,1] white
    ];
    
    println!("Input (2x2 checkerboard, G inverted):");
    println!("  [0,0]: R=255, G=0, B=255 (white in R/B, black in G)");
    println!("  [0,1]: R=0, G=255, B=0 (black in R/B, white in G)");
    println!("  [1,0]: R=0, G=255, B=0 (black in R/B, white in G)");
    println!("  [1,1]: R=255, G=0, B=255 (white in R/B, black in G)\n");
    
    for dwt_level in 0..=3 {
        println!("DWT Level {}:", dwt_level);
        
        let frame_info = FrameInfo {
            width: 2,
            height: 2,
            bits_per_sample: 8,
            component_count: 3,
        };
        
        let mut encoded = vec![0u8; 1024];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);
        
        let encoded_len = encoder.encode(&pixels, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);
        
        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();
        
        let mut all_match = true;
        for i in 0..12 {
            if pixels[i] != decoded[i] {
                all_match = false;
                break;
            }
        }
        
        if all_match {
            println!("  ✅ PASS - All pixels match");
        } else {
            println!("  ❌ FAIL - Decoded:");
            for i in 0..4 {
                let idx = i * 3;
                println!("    [{}]: R={}, G={}, B={} (expected R={}, G={}, B={})",
                         i, decoded[idx], decoded[idx+1], decoded[idx+2],
                         pixels[idx], pixels[idx+1], pixels[idx+2]);
            }
        }
    }
}

use jpegexp_rs::jpeg1::Jpeg1Encoder;
use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::FrameInfo;

#[test]
fn test_optimized_huffman_reduces_file_size() {
    // Create test image
    let width = 512u32;
    let height = 512u32;
    let mut rgb = vec![0u8; (width * height * 3) as usize];
    
    // Create gradient pattern which should compress well with optimized Huffman
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 3) as usize;
            rgb[idx] = (x % 256) as u8;     // R
            rgb[idx + 1] = (y % 256) as u8; // G
            rgb[idx + 2] = 128;              // B
        }
    }
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 3,
    };

    // Encode with standard Huffman
    let mut encoder_standard = Jpeg1Encoder::default();
    encoder_standard.set_quality(90);
    encoder_standard.set_optimize_huffman(false);
    
    let mut jpeg_standard = vec![0u8; width as usize * height as usize * 3]; // Sufficient buffer
    let len_standard = encoder_standard.encode(&rgb, &frame_info, &mut jpeg_standard).expect("Standard encode failed");
    jpeg_standard.truncate(len_standard);
    
    // Encode with optimized Huffman
    let mut encoder_optimized = Jpeg1Encoder::default();
    encoder_optimized.set_quality(90);
    encoder_optimized.set_optimize_huffman(true);
    
    let mut jpeg_optimized = vec![0u8; width as usize * height as usize * 3];
    let len_optimized = encoder_optimized.encode(&rgb, &frame_info, &mut jpeg_optimized).expect("Optimized encode failed");
    jpeg_optimized.truncate(len_optimized);
    
    // Verify size reduction
    println!("Standard size: {} bytes", jpeg_standard.len());
    println!("Optimized size: {} bytes", jpeg_optimized.len());
    
    let reduction_percent = 
        100.0 * (1.0 - jpeg_optimized.len() as f64 / jpeg_standard.len() as f64);
    println!("Reduction: {:.2}%", reduction_percent);
    
    // Expect 5-15% reduction for typical images, but at least some reduction
    assert!(jpeg_optimized.len() < jpeg_standard.len());
    // For this gradient, we expect significant savings
    assert!(reduction_percent >= 3.0, "Expected at least 3% reduction, got {:.2}%", reduction_percent);
}

#[test]
fn test_optimized_huffman_maintains_quality() {
    // Create test image
    let width = 256u32;
    let height = 256u32;
    let mut rgb = vec![0u8; (width * height * 3) as usize];
    
    for i in 0..rgb.len() {
        rgb[i] = (i % 255) as u8;
    }

    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 3,
    };

    // Encode with optimized Huffman
    let mut encoder = Jpeg1Encoder::default();
    encoder.set_quality(90);
    encoder.set_optimize_huffman(true);
    
    let mut encoded = vec![0u8; width as usize * height as usize * 3];
    let len_encoded = encoder.encode(&rgb, &frame_info, &mut encoded).expect("Encode failed");
    encoded.truncate(len_encoded);
    
    // Decode
    let mut decoder = Jpeg1Decoder::new(&encoded);
    decoder.read_header().expect("Read header failed");
    let mut decoded = vec![0u8; (width * height * 3) as usize];
    decoder.decode(&mut decoded).expect("Decode failed");
    
    // Compare MAE with standard encoding (should be identical pixels since quantization is same)
    
    let mut encoder_std = Jpeg1Encoder::default();
    encoder_std.set_quality(90);
    encoder_std.set_optimize_huffman(false);
    
    let mut encoded_std = vec![0u8; width as usize * height as usize * 3];
    let len_std = encoder_std.encode(&rgb, &frame_info, &mut encoded_std).expect("Standard encode failed");
    encoded_std.truncate(len_std);
    
    let mut decoder_std = Jpeg1Decoder::new(&encoded_std);
    decoder_std.read_header().expect("Read header std failed");
    let mut decoded_std = vec![0u8; (width * height * 3) as usize];
    decoder_std.decode(&mut decoded_std).expect("Decode std failed");
    
    // Verify exact pixel match between standard and optimized results
    for i in 0..decoded.len() {
        assert_eq!(decoded[i], decoded_std[i], "Pixel mismatch at index {}", i);
    }
}

//! Debug test for lossy compression

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_simple_lossy_debug() {
    // Create a simple 4x4 image with gradient
    let width = 4;
    let height = 4;
    let mut pixels = vec![0u8; width * height];
    for i in 0..pixels.len() {
        pixels[i] = (i * 16) as u8; // 0, 16, 32, 48, ..., 240
    }
    
    println!("Original pixels: {:?}", pixels);
    
    let mut encoder = J2kEncoder::new();
    encoder.set_quality(100);
    encoder.set_irreversible(true);
    encoder.set_decomposition_levels(1); // 1 level of DWT
    
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; pixels.len() * 10];
    let compressed_size = encoder
        .encode(&pixels, &frame_info, &mut output)
        .expect("Encoding failed");
    
    output.truncate(compressed_size);
    
    println!("Compressed size: {} bytes", compressed_size);
    
    // Decode
    let mut reader = JpegStreamReader::new(&output);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let decoded_pixels = image.reconstruct_pixels().expect("Reconstruction failed");
    
    println!("Decoded pixels: {:?}", decoded_pixels);
    
    // Calculate error
    let mae: f64 = pixels
        .iter()
        .zip(decoded_pixels.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).abs() as f64)
        .sum::<f64>()
        / pixels.len() as f64;
    
    println!("MAE: {:.2}", mae);
    
    // For a gradient image with quality 100, error should be small
    assert!(mae < 10.0, "MAE too high: {}. Decoded: {:?}", mae, decoded_pixels);
}

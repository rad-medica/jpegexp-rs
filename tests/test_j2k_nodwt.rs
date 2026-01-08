use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn main() {
    let width = 8u32;
    let height = 8u32;
    
    // Test alternating 0/255 with 0 decomposition levels
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let val = if (x + y) % 2 == 0 { 0u8 } else { 255u8 };
            original.push(val);
        }
    }
    
    println!("Original (first 16): {:?}", &original[..16]);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(0); // 0 levels = no DWT!
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded to {} bytes (0 levels)", encoded_len);
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    println!("Reconstructed (first 16): {:?}", &reconstructed[..16]);
    
    let mut diffs = 0;
    for i in 0..original.len() {
        if original[i] != reconstructed[i] {
            diffs += 1;
            if diffs <= 10 {
                println!("Mismatch at {}: orig={}, recon={}", i, original[i], reconstructed[i]);
            }
        }
    }
    
    println!("Total mismatches: {} / {}", diffs, original.len());
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

#[test]
fn test_main() {
    let width = 64u32;
    let height = 64u32;
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            original.push(((x + y) * 2).min(255) as u8);
        }
    }
    let frame_info = FrameInfo { width, height, bits_per_sample: 8, component_count: 1 };
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1); // ONLY 1 LEVEL
    let len = encoder.encode(&original, &frame_info, &mut encoded).unwrap();
    let mut reader = JpegStreamReader::new(&encoded[..len]);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().unwrap();
    let reconstructed = image.reconstruct_pixels().unwrap();
    println!("Original (first 10): {:?}", &original[..10]);
    println!("Reconstructed (first 10): {:?}", &reconstructed[..10]);
    
    // Debug subbands
    if !image.tiles.is_empty() && !image.tiles[0].components.is_empty() {
        let comp = &image.tiles[0].components[0];
        for (r, res) in comp.resolutions.iter().enumerate() {
            println!("Res {}: {}x{}", r, res.width, res.height);
            for (s, sb) in res.subbands.iter().enumerate() {
                if !sb.codeblocks.is_empty() {
                    let cb = &sb.codeblocks[0];
                    println!("  SB {}: coeffs={}, first 5={:?}", s, cb.coefficients.len(), &cb.coefficients[..5.min(cb.coefficients.len())]);
                }
            }
        }
    }

    let mut diff_sum: u64 = 0;
    let mut max_diff = 0;
    for i in 0..original.len() {
        let d = (original[i] as i32 - reconstructed[i] as i32).abs();
        if d > 0 {
            let x = i % width as usize;
            let y = i / width as usize;
            println!("Mismatch at ({},{}): orig={}, recon={}", x, y, original[i], reconstructed[i]);
        }
        diff_sum += d as u64;
        if d > max_diff { max_diff = d; }
    }
    println!("1 Level: MAE={:.4}, MaxDiff={}", diff_sum as f64 / original.len() as f64, max_diff);
}

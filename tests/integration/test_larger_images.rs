//! Test larger images with inverted G channel
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn mae(a: &[u8], b: &[u8]) -> f64 {
    let sum: u64 = a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| x.abs_diff(y) as u64)
        .sum();
    sum as f64 / a.len() as f64
}

#[test]
fn test_larger_images_inverted_g() {
    println!("\n================================================================================");
    println!("Testing Larger Images with Inverted G Channel");
    println!("================================================================================\n");

    let sizes = [64, 128, 256, 512];
    let block_size = 8;
    let dwt_level = 3;

    for size in sizes {
        println!(
            "Testing {}x{} with 8x8 blocks, DWT level {}...",
            size,
            size,
            dwt_level
        );

        // Create checkerboard with G inverted
        let mut pixels = Vec::with_capacity(size * size * 3);
        for y in 0..size {
            for x in 0..size {
                let is_white = (x / block_size + y / block_size) % 2 == 0;
                let val = if is_white { 255u8 } else { 0 };
                pixels.push(val); // R
                pixels.push(if is_white { 0 } else { 255 }); // G inverted
                pixels.push(val); // B
            }
        }

        let frame_info = FrameInfo {
            width: size as u32,
            height: size as u32,
            bits_per_sample: 8,
            component_count: 3,
        };

        let mut encoded = vec![0u8; size * size * 4];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(dwt_level);

        let encoded_len = encoder.encode(&pixels, &frame_info, &mut encoded).unwrap();
        encoded.truncate(encoded_len);

        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();

        let error = mae(&pixels, &decoded);

        println!(
            "  Encoded: {} bytes, MAE: {:.6} {}",
            encoded_len,
            error,
            if error < 0.01 { "✅ PASS" } else { "❌ FAIL" }
        );

        if error > 0.01 {
            // Show first few errors
            let mut shown = 0;
            for i in 0..(size * size) {
                for c in 0..3 {
                    let idx = i * 3 + c;
                    if pixels[idx] != decoded[idx] && shown < 5 {
                        let comp = ["R", "G", "B"][c];
                        println!(
                            "    Pixel {} {}: expected={}, got={}, diff={}",
                            i,
                            comp,
                            pixels[idx],
                            decoded[idx],
                            pixels[idx].abs_diff(decoded[idx])
                        );
                        shown += 1;
                    }
                }
            }
        }
    }
}

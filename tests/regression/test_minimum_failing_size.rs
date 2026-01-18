//! Find the minimum failing size
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
fn test_find_minimum_failing_size() {
    println!("\n===================================================================");
    println!("Finding Minimum Failing Size (Inverted G, 8x8 blocks, DWT=3)");
    println!("===================================================================\n");

    let sizes = vec![2, 4, 8, 16, 32, 64, 128];
    let block_size = 8;
    let dwt_level = 3;

    for size in sizes {
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

        let encoded_len = match encoder.encode(&pixels, &frame_info, &mut encoded) {
            Ok(len) => len,
            Err(_) => {
                println!(
                    "{}x{}: SKIPPED (too small for DWT level {})",
                    size,
                    size,
                    dwt_level
                );
                continue;
            }
        };
        encoded.truncate(encoded_len);

        let mut reader = JpegStreamReader::new(&encoded);
        let mut decoder = J2kDecoder::new(&mut reader);
        let image = decoder.decode().unwrap();
        let decoded = image.reconstruct_pixels().unwrap();

        let error = mae(&pixels, &decoded);

        let status = if error < 0.01 { "✅ PASS" } else { "❌ FAIL" };
        println!("{}x{}: MAE={:.6} {}", size, size, error, status);

        // If this is the first failure, show some details
        if error > 0.01 && size > 2 {
            println!("  First failure at {}x{}!", size, size);
            println!(
                "  First pixel: expected R={}, G={}, B={}",
                pixels[0],
                pixels[1],
                pixels[2]
            );
            println!(
                "  First pixel: got      R={}, G={}, B={}",
                decoded[0],
                decoded[1],
                decoded[2]
            );
            println!("  Codeblock size: 64x64 (fixed)");
            println!(
                "  After {} DWT levels, LL subband is {}x{}",
                dwt_level,
                size >> dwt_level,
                size >> dwt_level
            );
        }
    }
}

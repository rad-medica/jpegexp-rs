//! JPEG 2000 Decoder Debug Tests

use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;

/// Test that decodes a lossless J2K file
#[test]
fn test_j2k_lossless_decode() {
    // Read the test file
    let data = std::fs::read("tests/jpegls_test_images/gradient_64x64_gray_lossless.j2c")
        .expect("Failed to read test file");
    
    println!("File size: {} bytes", data.len());
    
    // Decode
    let mut reader = JpegStreamReader::new(&data);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Failed to decode");
    
    println!("Image: {}x{}, {} components", image.width, image.height, image.component_count);
    
    if let Some(cod) = &image.cod {
        println!("COD: decomp_levels={}, transform={} (0=9-7, 1=5-3)", 
            cod.decomposition_levels, cod.transformation);
        println!("COD: codeblock={}x{}", 
            1 << (cod.codeblock_width_exp + 2), 
            1 << (cod.codeblock_height_exp + 2));
    }
    
    if let Some(qcd) = &image.qcd {
        println!("QCD: style={}, step_sizes={:?}", qcd.quant_style, qcd.step_sizes);
    }
    
    // Analyze tiles
    for (ti, tile) in image.tiles.iter().enumerate() {
        println!("\nTile {} - {} components", ti, tile.components.len());
        for (ci, comp) in tile.components.iter().enumerate() {
            println!("  Component {} - {} resolutions", ci, comp.resolutions.len());
            for (ri, res) in comp.resolutions.iter().enumerate() {
                println!("    Resolution {} - {}x{}, {} subbands", 
                    ri, res.width, res.height, res.subbands.len());
                for (si, sb) in res.subbands.iter().enumerate() {
                    let cb_count = sb.codeblocks.len();
                    let coeff_count: usize = sb.codeblocks.iter()
                        .map(|cb| cb.coefficients.len()).sum();
                    let nonzero: usize = sb.codeblocks.iter()
                        .flat_map(|cb| cb.coefficients.iter())
                        .filter(|&&c| c != 0).count();
                    println!("      Subband {} ({:?}) - {}x{}, {} codeblocks, {} coeffs, {} nonzero", 
                        si, sb.orientation, sb.width, sb.height, cb_count, coeff_count, nonzero);
                    
                    // Print first codeblock details
                    for (cbi, cb) in sb.codeblocks.iter().take(1).enumerate() {
                        println!("        CB[{}]: x={}, y={}, {}x{}, passes={}, coeffs.len={}", 
                            cbi, cb.x, cb.y, cb.width, cb.height, cb.coding_passes, cb.coefficients.len());
                        if !cb.coefficients.is_empty() {
                            let first_8: Vec<_> = cb.coefficients.iter().take(8).collect();
                            println!("          First 8 coeffs: {:?}", first_8);
                        }
                    }
                }
            }
        }
    }
    
    // Reconstruct
    match image.reconstruct_pixels() {
        Ok(pixels) => {
            println!("\nReconstructed {} pixels", pixels.len());
            println!("First 16 pixels: {:?}", &pixels[..16.min(pixels.len())]);
            
            // Compare with expected raw
            let expected = std::fs::read("tests/jpegls_test_images/gradient_64x64_gray.raw")
                .expect("Failed to read raw file");
            
            let mae: f64 = pixels.iter().zip(expected.iter())
                .map(|(&p, &e)| (p as i32 - e as i32).abs() as f64)
                .sum::<f64>() / pixels.len() as f64;
            
            let max_diff: i32 = pixels.iter().zip(expected.iter())
                .map(|(&p, &e)| (p as i32 - e as i32).abs())
                .max().unwrap_or(0);
            
            println!("Expected first 16: {:?}", &expected[..16]);
            println!("MAE: {:.4}", mae);
            println!("Max diff: {}", max_diff);
            
            if mae == 0.0 {
                println!("✓ Lossless: PASS");
            } else {
                println!("✗ Lossless: FAIL (MAE={})", mae);
            }
        }
        Err(e) => {
            println!("\nReconstruction error: {}", e);
        }
    }
}

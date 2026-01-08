use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;

fn main() {
    let width = 8u32;
    let height = 8u32;
    
    // Create simple pattern: 0,1,2,3,...    
    let mut original: Vec<u8> = Vec::with_capacity((width * height) as usize);
    for i in 0..(width * height) {
        original.push(i as u8);
    }
    
    println!("Original (first 8): {:?}", &original[..8]);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoded = vec![0u8; 64 * 1024];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1); // Use only 1 level for simplicity
    
    let encoded_len = encoder.encode(&original, &frame_info, &mut encoded)
        .expect("Encoding failed");
    encoded.truncate(encoded_len);
    
    println!("Encoded to {} bytes", encoded_len);
    println!("Encoded data (hex):");
    for (i, b) in encoded.iter().enumerate().take(50) {
        print!("{:02X} ", b);
        if (i+1) % 16 == 0 { println!(); }
    }
    println!();
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("Decoding failed");
    
    println!("\nDecoded image:");
    println!("  width: {}", image.width);
    println!("  height: {}", image.height);
    println!("  components: {}", image.component_count);
    println!("  tiles: {}", image.tiles.len());
    
    if !image.tiles.is_empty() {
        let tile = &image.tiles[0];
        println!("  tile.components: {}", tile.components.len());
        if !tile.components.is_empty() {
            let comp = &tile.components[0];
            println!("  comp.resolutions: {}", comp.resolutions.len());
            if !comp.resolutions.is_empty() {
                let res = &comp.resolutions[0];
                println!("  res0.subbands: {}", res.subbands.len());
                for (i, sb) in res.subbands.iter().enumerate() {
                    println!("    sb{}: {} cbs, orient={:?}", i, sb.codeblocks.len(), sb.orientation);
                    for (j, cb) in sb.codeblocks.iter().enumerate().take(3) {
                        println!("      cb{}: coeffs={}, passes={}, layers={}", j, cb.coefficients.len(), cb.coding_passes, cb.layers_decoded);
                        if !cb.coefficients.is_empty() {
                            println!("         first 10 coeffs: {:?}", &cb.coefficients[..10.min(cb.coefficients.len())]);
                        }
                    }
                }
            }
        }
    }
    
    let reconstructed = image.reconstruct_pixels().expect("Reconstruction failed");
    
    println!("\nReconstructed (first 8): {:?}", &reconstructed[..8]);
    
    // Print internal coeffs of first row of Res 0
    if !image.tiles.is_empty() && !image.tiles[0].components.is_empty() {
        let comp = &image.tiles[0].components[0];
        let res0 = &comp.resolutions[0];
        if !res0.subbands.is_empty() && !res0.subbands[0].codeblocks.is_empty() {
            let cb = &res0.subbands[0].codeblocks[0];
            println!("Res 0 CB coeffs (fixed point *2): {:?}", &cb.coefficients[..8]);
        }
    }
    
    let mut diffs = 0;
    for i in 0..original.len() {
        if original[i] != reconstructed[i] {
            diffs += 1;
            if diffs <= 16 {
                println!("Mismatch at {}: orig={}, recon={}", i, original[i], reconstructed[i]);
            }
        }
    }
    
    println!("Total mismatches: {} / {}", diffs, original.len());
}

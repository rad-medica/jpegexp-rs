use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

#[test]
fn trace_detailed() {
    // Test with a gradient so we have non-zero coefficients
    let mut pixels = vec![0u8; 16];
    for i in 0..16 {
        pixels[i] = ((i * 16) % 256) as u8;
    }
    println!("Pixels: {:?}", pixels);
    
    let frame_info = FrameInfo {
        width: 4,
        height: 4,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let mut output = vec![0u8; 4096];
    let len = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    
    // Find SOD marker
    for i in 0..output.len()-1 {
        if output[i] == 0xFF && output[i+1] == 0x93 {
            let tile_data = &output[i+2..len];
            // Remove trailing EOC marker
            let data_len = tile_data.len().saturating_sub(2);
            println!("Tile data ({} bytes): {:02X?}", data_len, &tile_data[..data_len]);
            break;
        }
    }
}

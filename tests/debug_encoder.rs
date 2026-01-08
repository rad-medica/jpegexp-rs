use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

fn main() {
    let width = 8u32;
    let height = 8u32;
    let components = 1u32;
    let depth = 8u8;
    
    let mut original: Vec<u8> = Vec::with_capacity((width * height * components) as usize);
    for y in 0..height {
        for x in 0..width {
            for c in 0..components {
                let val = if (x + y) % 2 == 0 { 0u8 } else { 255u8 };
                original.push(val);
            }
        }
    }
    
    println!("Original (first 8): {:?}", &original[..8]);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: depth as i32,
        component_count: components as i32,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    // Temporarily modify to debug
    // Manually call encode_component_packets to see what it generates
    
    let mut encoded = vec![0u8; 64 * 1024];
    let result = encoder.encode(&original, &frame_info, &mut encoded);
    
    match result {
        Ok(len) => {
            encoded.truncate(len);
            println!("Encoded to {} bytes", len);
        }
        Err(e) => {
            println!("Encoding error: {:?}", e);
            return;
        }
    }
}

// Trace encoder step by step
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;

#[test]
fn trace_encoder_solid() {
    let pixels = vec![128u8; 16]; // 4x4 solid
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
    output.truncate(len);
    
    println!("Output len: {}", len);
    println!("Full codestream: {:02X?}", &output[..len.min(100)]);
    
    // Find SOD marker
    for i in 0..output.len()-1 {
        if output[i] == 0xFF && output[i+1] == 0x93 {
            println!("SOD at offset {}", i);
            println!("Tile data: {:02X?}", &output[i+2..]);
            break;
        }
    }
}

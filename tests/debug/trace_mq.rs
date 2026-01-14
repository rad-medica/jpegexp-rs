// Trace MQ coder step by step
use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

#[test]
fn trace_mq_solid() {
    // 4x4 solid block with value 0 (after level shift)
    let data = vec![0i32; 16];
    
    let mut bpc = BitPlaneCoder::new(4, 4, &data);
    let max_bp = bpc.calculate_max_bit_plane();
    println!("max_bp: {:?}", max_bp);
    
    // No significant bits, so nothing should be encoded
    // But we still need to encode the aggregate bits for RLC
    
    if let Some(bp) = max_bp {
        let passes = bpc.encode_codeblock(bp, 0, 0);
        bpc.mq.flush();
        let buf = bpc.mq.get_buffer();
        println!("Passes: {}", passes);
        println!("Buffer len: {}", buf.len());
        println!("Buffer: {:02X?}", buf);
    } else {
        println!("max_bp is None - all zeros");
        // Even with no data, we should output something for the empty packet
        let passes = bpc.encode_codeblock(0, 0, 0);
        bpc.mq.flush();
        let buf = bpc.mq.get_buffer();
        println!("Passes: {}", passes);
        println!("Buffer len: {}", buf.len());
        println!("Buffer: {:02X?}", buf);
    }
}

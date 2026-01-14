use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;
use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

#[test]
fn trace_gradient_codeblock() {
    let data: Vec<i32> = vec![
        0, 17, 34, 51,
        68, 85, 102, 119,
        136, 153, 170, 187,
        204, 221, 238, 255,
    ];
    
    let level_shifted: Vec<i32> = data.iter().map(|&v| v - 128).collect();
    println!("Level-shifted data:");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:4} ", level_shifted[y * 4 + x]);
        }
        println!();
    }
    
    let mut bpc = BitPlaneCoder::new(4, 4, &level_shifted);
    let max_bp = bpc.calculate_max_bit_plane().expect("Should have data");
    let min_bp = bpc.calculate_min_bit_plane();
    
    println!("\nmax_bp: {}, min_bp: {}", max_bp, min_bp);
    
    bpc.mq.init_encoder();
    bpc.encode_codeblock(max_bp, min_bp, 0);
    bpc.mq.flush();
    
    let encoded = bpc.mq.get_buffer().to_vec();
    println!("\nEncoded {} bytes:", encoded.len());
    println!("{}", encoded.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
}

#[test]  
fn trace_simple_mq_encoding() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    
    for i in 0..19 {
        mq.set_context(i, 0);
    }
    mq.set_context(0, 4 << 1);
    mq.set_context(17, 3 << 1);
    mq.set_context(18, 46 << 1);
    
    mq.init_encoder();
    
    println!("Encoding sequence for 4x4 gradient cleanup pass:");
    println!("Initial: A={:08X} C={:08X} CT={}", mq.a, mq.c, mq.ct);
    
    mq.encode(0, 17);
    println!("After AGG(0): A={:08X} C={:08X} CT={} buf={:02X?}", mq.a, mq.c, mq.ct, &mq.buffer[..]);
    
    mq.flush();
    let buf = mq.get_buffer();
    println!("\nFlushed buffer: {:02X?}", buf);
}

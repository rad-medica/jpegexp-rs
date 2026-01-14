use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

#[test]
fn trace_mq_first_byte() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    println!("Initial state: A={:08X} C={:08X} CT={}", mq.a, mq.c, mq.ct);
    
    // Encode a few bits with context 0 (MPS=0, index=0)
    // Context 0 starts at state 0 (Qe=0x5601)
    for i in 0..5 {
        mq.encode(0, 0); // Encode MPS (0)
        println!("After encode {}: A={:08X} C={:08X} CT={} buffer={:02X?}", 
            i, mq.a, mq.c, mq.ct, &mq.buffer[..]);
    }
    
    mq.flush();
    let buf = mq.get_buffer();
    println!("Final buffer: {:02X?}", buf);
}

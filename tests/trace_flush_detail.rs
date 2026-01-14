use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

#[test]
fn trace_flush_behavior() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Encode some symbols to build up state
    for i in 0..10 {
        mq.encode(i % 2, 0);
    }
    
    println!("Before flush:");
    println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    println!("  buffer.len()={}, bp_idx={}", mq.buffer.len(), mq.bp_idx);
    println!("  buffer[bp_idx]=0x{:02X}", if mq.bp_idx < mq.buffer.len() { mq.buffer[mq.bp_idx] } else { 0 });
    
    // Manually trace flush steps
    let temp_c = mq.c + mq.a;
    println!("\nFlush step 1: setbits");
    println!("  temp_c = c + a = 0x{:08X} + 0x{:08X} = 0x{:08X}", mq.c, mq.a, temp_c);
    
    mq.c |= 0xffff;
    println!("  c |= 0xffff -> c=0x{:08X}", mq.c);
    
    if mq.c >= temp_c {
        mq.c -= 0x8000;
        println!("  c >= temp_c, so c -= 0x8000 -> c=0x{:08X}", mq.c);
    }
    
    println!("\nFlush step 2: first byte_out");
    mq.c <<= mq.ct;
    println!("  c <<= ct({}) -> c=0x{:08X}", mq.ct, mq.c);
    
    let before_bp = mq.bp_idx;
    let before_len = mq.buffer.len();
    println!("  Before byte_out: bp_idx={}, buffer.len()={}", before_bp, before_len);
    
    // Call flush to complete
    mq.flush();
    
    println!("\nAfter flush:");
    println!("  bp_idx={}, buffer.len()={}", mq.bp_idx, mq.buffer.len());
    println!("  Result bytes: {}", mq.get_buffer().len());
    println!("  Buffer: {:02X?}", mq.get_buffer());
}

#[test]
fn compare_flush_with_different_states() {
    // Test flush with various MQ coder states
    let test_cases = vec![
        ("All zeros", vec![(0u8, 0usize); 20]),
        ("All ones", vec![(1u8, 0usize); 20]),
        ("Alternating", (0..20).map(|i| ((i % 2) as u8, 0)).collect()),
    ];
    
    for (name, symbols) in test_cases {
        let mut mq = MqCoder::new();
        mq.init_contexts(19);
        mq.init_encoder();
        
        for (bit, ctx) in symbols {
            mq.encode(bit, ctx);
        }
        
        let before_bp = mq.bp_idx;
        let before_len = mq.buffer.len();
        
        mq.flush();
        
        let result_len = mq.get_buffer().len();
        
        println!("{}: bp_idx {} -> {}, buffer {} -> {}, result_len={}",
                 name, before_bp, mq.bp_idx, before_len, mq.buffer.len(), result_len);
    }
}

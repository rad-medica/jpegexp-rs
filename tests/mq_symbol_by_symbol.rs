use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

/// Compare our MQ coder output symbol-by-symbol with expected OpenJPEG behavior
/// This test encodes progressively more symbols and checks when divergence occurs

#[test]
fn find_divergence_point() {
    println!("\n=== Finding Divergence Point ===\n");
    
    // Test pattern: encode increasing numbers of zeros
    for count in 1..=20 {
        let mut mq = MqCoder::new();
        mq.init_contexts(19);
        for i in 0..19 {
            mq.set_context(i, 0);
        }
        mq.set_context(0, 4 << 1);
        mq.set_context(17, 3 << 1);
        mq.set_context(18, 46 << 1);
        mq.init_encoder();
        
        for _ in 0..count {
            mq.encode(0, 0);
        }
        
        mq.flush();
        let result = mq.get_buffer();
        
        println!("{:2} symbols: {} bytes: {:02X?}", count, result.len(), result);
    }
}

#[test]
fn test_byte_out_behavior() {
    println!("\n=== Testing byte_out Behavior ===\n");
    
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    println!("Initial: a=0x{:08X}, c=0x{:08X}, ct={}, bp_idx={}, buf_len={}", 
             mq.a, mq.c, mq.ct, mq.bp_idx, mq.buffer.len());
    
    // Manually set state to trigger byte_out
    mq.a = 0x8000;
    mq.c = 0x00800000;  // High enough to trigger byte_out
    mq.ct = 0;  // Will trigger byte_out on next renorm
    
    println!("Set: a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    
    // This should trigger byte_out
    mq.encode(0, 0);
    
    println!("After encode: a=0x{:08X}, c=0x{:08X}, ct={}, bp_idx={}, buf_len={}", 
             mq.a, mq.c, mq.ct, mq.bp_idx, mq.buffer.len());
    
    if mq.buffer.len() > 1 {
        println!("Buffer: {:02X?}", &mq.buffer[1..]);
    }
}

#[test]
fn test_carry_propagation_detailed() {
    println!("\n=== Testing Carry Propagation ===\n");
    
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Manually create a situation that will cause carry
    // Set c to a value that will overflow when shifted
    mq.a = 0x8000;
    mq.c = 0x04000000;  // High bit set in bit 26
    mq.ct = 8;
    mq.bp_idx = 1;
    mq.buffer.push(0xFE);  // Previous byte is 0xFE
    
    println!("Setup: a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    println!("Buffer before: {:02X?}", &mq.buffer[1..]);
    
    // Shift c left by ct, which should trigger byte_out with carry
    mq.c <<= mq.ct;
    println!("After shift: c=0x{:08X}", mq.c);
    
    // Manually call byte_out to see what happens
    // (In real code this is called from renorm_e)
    // We can't call it directly, so let's trace what should happen:
    
    let carry = (mq.c & 0x8000000) != 0;
    println!("Carry bit set: {}", carry);
    
    if carry {
        println!("Would increment buffer[{}] from 0x{:02X} to 0x{:02X}", 
                 mq.bp_idx, mq.buffer[mq.bp_idx], mq.buffer[mq.bp_idx] + 1);
    }
}

#[test]
fn test_flush_with_different_states() {
    println!("\n=== Testing Flush with Different States ===\n");
    
    let test_cases = vec![
        ("a=0x8000, c=0", 0x8000u32, 0u32, 12i32),
        ("a=0xF000, c=0x1000", 0xF000, 0x1000, 10),
        ("a=0xA000, c=0x5000", 0xA000, 0x5000, 8),
        ("a=0x8001, c=0x7FFF", 0x8001, 0x7FFF, 12),
    ];
    
    for (name, a, c, ct) in test_cases {
        let mut mq = MqCoder::new();
        mq.init_contexts(19);
        mq.init_encoder();
        
        // Set state
        mq.a = a;
        mq.c = c;
        mq.ct = ct;
        
        println!("\n{}", name);
        println!("  Before flush: a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
        
        mq.flush();
        
        let result = mq.get_buffer();
        println!("  After flush: {} bytes: {:02X?}", result.len(), result);
        println!("  bp_idx={}, buf_len={}", mq.bp_idx, mq.buffer.len());
    }
}

#[test]
fn compare_context_states() {
    println!("\n=== Comparing Context State Evolution ===\n");
    
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    for i in 0..19 {
        mq.set_context(i, 0);
    }
    mq.set_context(0, 4 << 1);  // Start at state 4
    mq.init_encoder();
    
    println!("Context 0 state evolution when encoding MPS (0):");
    
    for i in 0..10 {
        let ctx_before = mq.contexts[0];
        let state_before = (ctx_before >> 1) as usize;
        let mps_before = ctx_before & 1;
        
        mq.encode(0, 0);
        
        let ctx_after = mq.contexts[0];
        let state_after = (ctx_after >> 1) as usize;
        let mps_after = ctx_after & 1;
        
        println!("  Symbol {}: state {} -> state {}, mps={}", 
                 i, state_before, state_after, mps_after);
    }
    
    mq.flush();
    let result = mq.get_buffer();
    println!("\nFinal output: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn test_exact_openjpeg_sequence() {
    println!("\n=== Testing Exact OpenJPEG Initialization Sequence ===\n");
    
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    
    // OpenJPEG does: opj_mqc_resetstates() then opj_mqc_setstate() for specific contexts
    // resetstates sets all contexts to mqc_states[0] (state 0, mps 0)
    for i in 0..19 {
        mq.set_context(i, 0);  // State 0, MPS 0
    }
    
    // Then it sets specific contexts
    mq.set_context(18, 46 << 1);  // UNI: state 46, MPS 0
    mq.set_context(17, 3 << 1);   // AGG: state 3, MPS 0
    mq.set_context(0, 4 << 1);    // ZC[0]: state 4, MPS 0
    
    mq.init_encoder();
    
    println!("After initialization:");
    println!("  Context 0 (ZC[0]): state {}, mps {}", mq.contexts[0] >> 1, mq.contexts[0] & 1);
    println!("  Context 17 (AGG): state {}, mps {}", mq.contexts[17] >> 1, mq.contexts[17] & 1);
    println!("  Context 18 (UNI): state {}, mps {}", mq.contexts[18] >> 1, mq.contexts[18] & 1);
    println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    
    // Encode a simple sequence
    for i in 0..5 {
        println!("\nBefore encode {}:", i);
        println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
        mq.encode(0, 0);
        println!("After encode {}:", i);
        println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    }
    
    mq.flush();
    let result = mq.get_buffer();
    println!("\nFinal: {} bytes: {:02X?}", result.len(), result);
}

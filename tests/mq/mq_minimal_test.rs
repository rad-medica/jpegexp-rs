use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

/// Test MQ coder with minimal symbol sequences to find divergence point
#[test]
fn test_mq_single_symbol() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Encode a single 0 in context 0
    mq.encode(0, 0);
    mq.flush();
    
    let result = mq.get_buffer();
    println!("Single 0: {} bytes: {:02X?}", result.len(), result);
    
    // Expected from OpenJPEG: should produce specific output
    // We'll compare this with a reference implementation
}

#[test]
fn test_mq_two_symbols() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    mq.encode(0, 0);
    mq.encode(0, 0);
    mq.flush();
    
    let result = mq.get_buffer();
    println!("Two 0s: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn test_mq_alternating() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    for i in 0..10 {
        mq.encode((i % 2) as u8, 0);
    }
    mq.flush();
    
    let result = mq.get_buffer();
    println!("Alternating 10: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn test_mq_all_zeros() {
    for count in [1, 2, 5, 10, 20, 50, 100] {
        let mut mq = MqCoder::new();
        mq.init_contexts(19);
        mq.init_encoder();
        
        for _ in 0..count {
            mq.encode(0, 0);
        }
        mq.flush();
        
        let result = mq.get_buffer();
        println!("{} zeros: {} bytes, first 8: {:02X?}", 
                 count, result.len(), &result[..result.len().min(8)]);
    }
}

#[test]
fn test_mq_all_ones() {
    for count in [1, 2, 5, 10, 20, 50, 100] {
        let mut mq = MqCoder::new();
        mq.init_contexts(19);
        mq.init_encoder();
        
        for _ in 0..count {
            mq.encode(1, 0);
        }
        mq.flush();
        
        let result = mq.get_buffer();
        println!("{} ones: {} bytes, first 8: {:02X?}", 
                 count, result.len(), &result[..result.len().min(8)]);
    }
}

#[test]
fn test_mq_pattern_0001() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Pattern: 0, 0, 0, 1 repeated
    for _ in 0..10 {
        mq.encode(0, 0);
        mq.encode(0, 0);
        mq.encode(0, 0);
        mq.encode(1, 0);
    }
    mq.flush();
    
    let result = mq.get_buffer();
    println!("Pattern 0001 x10: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn test_mq_multiple_contexts() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Use different contexts
    mq.encode(0, 0);  // Context 0
    mq.encode(1, 1);  // Context 1
    mq.encode(0, 2);  // Context 2
    mq.encode(1, 0);  // Back to context 0
    mq.flush();
    
    let result = mq.get_buffer();
    println!("Multiple contexts: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn test_mq_state_transitions() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Force state transitions by encoding MPS repeatedly
    // This should move through states 0->1->2->3...
    for i in 0..20 {
        mq.encode(0, 0);  // Always encode MPS (0)
        if i < 10 {
            println!("After {} symbols: a=0x{:08X}, c=0x{:08X}, ct={}", 
                     i+1, mq.a, mq.c, mq.ct);
        }
    }
    mq.flush();
    
    let result = mq.get_buffer();
    println!("20 MPS: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn test_mq_lps_trigger() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Encode mostly MPS, then one LPS
    for _ in 0..10 {
        mq.encode(0, 0);  // MPS
    }
    mq.encode(1, 0);  // LPS - should trigger different code path
    mq.flush();
    
    let result = mq.get_buffer();
    println!("10 MPS + 1 LPS: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn test_mq_carry_propagation() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    // Try to trigger carry propagation by encoding a pattern
    // that causes c to overflow
    for i in 0..50 {
        mq.encode((i % 3 == 0) as u8, 0);
    }
    mq.flush();
    
    let result = mq.get_buffer();
    println!("Carry test: {} bytes, first 16: {:02X?}", 
             result.len(), &result[..result.len().min(16)]);
}

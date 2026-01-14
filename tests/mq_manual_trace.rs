use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

/// Manually trace MQ coder state for debugging
#[test]
fn trace_single_zero_detailed() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    // Initialize contexts like bit-plane coder does
    for i in 0..19 {
        mq.set_context(i, 0);
    }
    mq.set_context(0, 4 << 1);   // ZC[0] context: state 4, MPS 0
    mq.set_context(17, 3 << 1);  // AGG context: state 3, MPS 0
    mq.set_context(18, 46 << 1); // UNI context: state 46, MPS 0
    mq.init_encoder();
    
    println!("\n=== Encoding single 0 in context 0 ===");
    println!("Initial state:");
    println!("  a = 0x{:08X} ({})", mq.a, mq.a);
    println!("  c = 0x{:08X}", mq.c);
    println!("  ct = {}", mq.ct);
    println!("  buffer = {:?}", mq.buffer);
    println!("  bp_idx = {}", mq.bp_idx);
    
    // Context 0 starts at state 4 (per our initialization)
    // State 4: qe=0x0521, nmps=5, nlps=29, switch=0, mps=0
    println!("\nContext 0 initial state:");
    let ctx = mq.contexts[0];
    let idx = (ctx >> 1) as usize;
    let mps = ctx & 1;
    println!("  ctx byte = 0x{:02X}", ctx);
    println!("  state index = {}", idx);
    println!("  mps = {}", mps);
    
    // Encode 0 (which is MPS)
    println!("\nEncoding symbol 0 (MPS)...");
    mq.encode(0, 0);
    
    println!("\nAfter encode:");
    println!("  a = 0x{:08X}", mq.a);
    println!("  c = 0x{:08X}", mq.c);
    println!("  ct = {}", mq.ct);
    println!("  buffer.len() = {}", mq.buffer.len());
    println!("  bp_idx = {}", mq.bp_idx);
    if mq.buffer.len() > 1 {
        println!("  buffer[1..] = {:02X?}", &mq.buffer[1..]);
    }
    
    // Now flush
    println!("\nFlushing...");
    println!("Before flush:");
    println!("  a = 0x{:08X}", mq.a);
    println!("  c = 0x{:08X}", mq.c);
    println!("  ct = {}", mq.ct);
    
    mq.flush();
    
    println!("\nAfter flush:");
    println!("  a = 0x{:08X}", mq.a);
    println!("  c = 0x{:08X}", mq.c);
    println!("  ct = {}", mq.ct);
    println!("  buffer.len() = {}", mq.buffer.len());
    println!("  bp_idx = {}", mq.bp_idx);
    println!("  buffer[0] (dummy) = 0x{:02X}", mq.buffer[0]);
    
    let result = mq.get_buffer();
    println!("\nFinal result:");
    println!("  {} bytes: {:02X?}", result.len(), result);
    
    // Expected from OpenJPEG (based on manual calculation):
    // Initial: a=0x8000, c=0, ct=12
    // After encode(0): 
    //   qe = 0x0521
    //   a = 0x8000 - 0x0521 = 0x7ADF
    //   Since (a & 0x8000) == 0, we enter renorm
    //   Since a >= qe, we do c += qe, so c = 0x0521
    //   Then renorm: a <<= 1, c <<= 1, ct -= 1
    //   Loop until a >= 0x8000
    println!("\n=== Manual calculation ===");
    let mut a = 0x8000u32;
    let mut c = 0u32;
    let mut ct = 12i32;
    let qe = 0x0521u32;
    
    println!("Initial: a=0x{:08X}, c=0x{:08X}, ct={}", a, c, ct);
    
    // Encode MPS
    a -= qe;
    println!("After a -= qe: a=0x{:08X}", a);
    
    if (a & 0x8000) == 0 {
        println!("Need renorm (a < 0x8000)");
        if a < qe {
            println!("  a < qe, so a = qe");
            a = qe;
        } else {
            println!("  a >= qe, so c += qe");
            c += qe;
        }
        println!("  Before renorm: a=0x{:08X}, c=0x{:08X}, ct={}", a, c, ct);
        
        // Renorm loop
        let mut iterations = 0;
        while a < 0x8000 {
            a <<= 1;
            c <<= 1;
            ct -= 1;
            iterations += 1;
            println!("  Renorm iter {}: a=0x{:08X}, c=0x{:08X}, ct={}", iterations, a, c, ct);
            if ct == 0 {
                println!("    ct==0, would call byte_out");
                break;
            }
        }
    }
    
    println!("\nAfter encode: a=0x{:08X}, c=0x{:08X}, ct={}", a, c, ct);
    
    // Flush
    let temp_c = c + a;
    c |= 0xffff;
    if c >= temp_c {
        c -= 0x8000;
    }
    println!("After setbits: c=0x{:08X}", c);
    
    c <<= ct;
    println!("After c <<= ct({}): c=0x{:08X}", ct, c);
    
    // First byte_out
    let byte1 = (c >> 19) as u8;
    println!("First byte_out: 0x{:02X}", byte1);
    c &= 0x7ffff;
    ct = 8;
    
    c <<= ct;
    println!("After second shift: c=0x{:08X}", c);
    
    // Second byte_out
    let byte2 = (c >> 19) as u8;
    println!("Second byte_out: 0x{:02X}", byte2);
    
    println!("\nExpected output: [0x{:02X}] (if byte2 != 0xFF, we advance past it)", byte1);
}

#[test]
fn trace_two_zeros_detailed() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    println!("\n=== Encoding two 0s in context 0 ===");
    
    // First 0
    println!("\nBefore first encode:");
    println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    mq.encode(0, 0);
    println!("After first encode:");
    println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    
    // Second 0
    println!("\nBefore second encode:");
    println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    mq.encode(0, 0);
    println!("After second encode:");
    println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    
    mq.flush();
    
    let result = mq.get_buffer();
    println!("\nFinal result: {} bytes: {:02X?}", result.len(), result);
}

#[test]
fn trace_alternating_detailed() {
    let mut mq = MqCoder::new();
    mq.init_contexts(19);
    mq.init_encoder();
    
    println!("\n=== Encoding alternating 0,1,0,1,0,1 ===");
    
    for i in 0..6 {
        let bit = (i % 2) as u8;
        println!("\nBefore encode({}):", bit);
        println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
        mq.encode(bit, 0);
        println!("After encode({}):", bit);
        println!("  a=0x{:08X}, c=0x{:08X}, ct={}", mq.a, mq.c, mq.ct);
    }
    
    mq.flush();
    
    let result = mq.get_buffer();
    println!("\nFinal result: {} bytes: {:02X?}", result.len(), result);
}

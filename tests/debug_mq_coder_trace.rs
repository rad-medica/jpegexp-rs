/// Debug test to trace MQ coder behavior for minimal case
/// Compares our MQ coder output byte-by-byte with expected OpenJPEG output

#[cfg(test)]
mod tests {
    use jpegexp_rs::jpeg2000::mq_coder::MqCoder;

    #[test]
    #[ignore]
    fn trace_mq_encoding_solid_128() {
        // For a 4x4 solid image with value 128:
        // After level shift (128 - 128 = 0), all coefficients are 0
        // All bit-planes encode zeros
        
        let mut mq = MqCoder::new();
        mq.init_encoder();
        
        // Initialize contexts like bit_plane_coder does
        mq.init_contexts(19);
        for i in 0..19 {
            mq.set_context(i, 0);
        }
        // Special contexts
        mq.set_context(17, 3 << 1);  // AGG context
        mq.set_context(18, 46 << 1); // UNI context  
        mq.set_context(0, 4 << 1);   // ZC context
        
        println!("=== Initial MQ State ===");
        println!("a={:04x}, c={:08x}, ct={}, bp_idx={}", mq.a, mq.c, mq.ct, mq.bp_idx);
        
        // Encode a few zeros in different contexts (simulating bit-plane encoding)
        let test_sequence = vec![
            (0, 0),  // Encode bit 0 in context 0
            (0, 1),  // Encode bit 0 in context 1
            (0, 2),  // Encode bit 0 in context 2
        ];
        
        for (bit, ctx) in test_sequence {
            println!("\n--- Encoding bit={} in context={} ---", bit, ctx);
            println!("Before: a={:04x}, c={:08x}, ct={}, bp_idx={}", 
                     mq.a, mq.c, mq.ct, mq.bp_idx);
            
            mq.encode(bit, ctx);
            
            println!("After:  a={:04x}, c={:08x}, ct={}, bp_idx={}", 
                     mq.a, mq.c, mq.ct, mq.bp_idx);
            println!("Buffer so far: {:02x?}", mq.get_buffer());
        }
        
        // Flush
        println!("\n=== Flushing ===");
        println!("Before flush: a={:04x}, c={:08x}, ct={}, bp_idx={}", 
                 mq.a, mq.c, mq.ct, mq.bp_idx);
        mq.flush();
        println!("After flush:  a={:04x}, c={:08x}, ct={}, bp_idx={}", 
                 mq.a, mq.c, mq.ct, mq.bp_idx);
        
        let buffer = mq.get_buffer();
        println!("\n=== Final Output ===");
        println!("Buffer length: {}", buffer.len());
        println!("Buffer contents: {:02x?}", buffer);
        println!("As hex string: {}", buffer.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" "));
        
        // OpenJPEG outputs 80 FF D9 for this case
        println!("\n=== Comparison ===");
        println!("Expected (OpenJPEG): 80 FF D9");
        println!("Actual (ours):       {:02X?}", buffer.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" "));
        
        if buffer.len() >= 1 {
            if buffer[0] == 0x80 {
                println!("✓ First byte MATCHES!");
            } else {
                println!("✗ First byte DIFFERS: expected 0x80, got 0x{:02x}", buffer[0]);
                println!("   Binary: expected 0b10000000, got 0b{:08b}", buffer[0]);
            }
        }
    }

    #[test]
    #[ignore]
    fn trace_mq_single_bit() {
        // Simplest possible case: encode a single bit
        let mut mq = MqCoder::new();
        mq.init_encoder();
        mq.init_contexts(19);
        mq.set_context(0, 0);  // Context 0, state 0
        
        println!("=== Encoding Single Bit (0) ===");
        println!("Initial: a={:04x}, c={:08x}, ct={}", mq.a, mq.c, mq.ct);
        
        mq.encode(0, 0);
        println!("After encode: a={:04x}, c={:08x}, ct={}", mq.a, mq.c, mq.ct);
        
        mq.flush();
        println!("After flush: a={:04x}, c={:08x}, ct={}", mq.a, mq.c, mq.ct);
        
        let buffer = mq.get_buffer();
        println!("Output: {:02x?}", buffer);
    }
}

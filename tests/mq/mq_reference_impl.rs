/// Reference MQ coder implementation that exactly mirrors OpenJPEG's C code
/// This is for comparison purposes to find where our implementation diverges

// MQ state table (from OpenJPEG)
const QE_VALUES: [u16; 47] = [
    0x5601, 0x3401, 0x1801, 0x0ac1, 0x0521, 0x0221, 0x5601, 0x5401,
    0x4801, 0x3801, 0x3001, 0x2401, 0x1c01, 0x1601, 0x5601, 0x5401,
    0x5101, 0x4801, 0x3801, 0x3401, 0x3001, 0x2801, 0x2401, 0x2201,
    0x1c01, 0x1801, 0x1601, 0x1401, 0x1201, 0x1101, 0x0ac1, 0x09c1,
    0x08a1, 0x0521, 0x0441, 0x02a1, 0x0221, 0x0141, 0x0111, 0x0085,
    0x0049, 0x0025, 0x0015, 0x0009, 0x0005, 0x0001, 0x5601,
];

const NMPS: [u8; 47] = [
    1, 2, 3, 4, 5, 38, 7, 8, 9, 10, 11, 12, 13, 29, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38,
    39, 40, 41, 42, 43, 44, 45, 45, 46,
];

const NLPS: [u8; 47] = [
    1, 6, 9, 12, 29, 33, 6, 14, 14, 14, 17, 18, 20, 21, 14, 14, 15, 16, 17,
    18, 19, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34,
    35, 36, 37, 38, 39, 40, 41, 42, 43, 46,
];

const SWITCH: [u8; 47] = [
    1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

struct ReferenceMQ {
    a: u32,
    c: u32,
    ct: i32,
    buffer: Vec<u8>,
    bp: usize,  // Index in buffer (equivalent to bp - start in OpenJPEG)
    contexts: Vec<u8>,  // state_index << 1 | mps
}

impl ReferenceMQ {
    fn new() -> Self {
        Self {
            a: 0,
            c: 0,
            ct: 0,
            buffer: Vec::new(),
            bp: 0,
            contexts: vec![0; 19],
        }
    }
    
    fn init_enc(&mut self) {
        self.buffer.clear();
        self.buffer.push(0);  // Dummy byte at position 0 (equivalent to bp = start - 1)
        self.bp = 0;  // Points to dummy byte initially
        self.a = 0x8000;
        self.c = 0;
        self.ct = 12;
    }
    
    fn set_context(&mut self, cx: usize, state: u8) {
        self.contexts[cx] = state;
    }
    
    fn byteout(&mut self) {
        // OpenJPEG's opj_mqc_byteout logic
        if self.buffer[self.bp] == 0xff {
            self.bp += 1;
            if self.bp >= self.buffer.len() {
                self.buffer.push(0);
            }
            self.buffer[self.bp] = (self.c >> 20) as u8;
            self.c &= 0xfffff;
            self.ct = 7;
        } else {
            if (self.c & 0x8000000) == 0 {
                self.bp += 1;
                if self.bp >= self.buffer.len() {
                    self.buffer.push(0);
                }
                self.buffer[self.bp] = (self.c >> 19) as u8;
                self.c &= 0x7ffff;
                self.ct = 8;
            } else {
                self.buffer[self.bp] = self.buffer[self.bp].wrapping_add(1);
                if self.buffer[self.bp] == 0xff {
                    self.c &= 0x7ffffff;
                    self.bp += 1;
                    if self.bp >= self.buffer.len() {
                        self.buffer.push(0);
                    }
                    self.buffer[self.bp] = (self.c >> 20) as u8;
                    self.c &= 0xfffff;
                    self.ct = 7;
                } else {
                    self.bp += 1;
                    if self.bp >= self.buffer.len() {
                        self.buffer.push(0);
                    }
                    self.buffer[self.bp] = (self.c >> 19) as u8;
                    self.c &= 0x7ffff;
                    self.ct = 8;
                }
            }
        }
    }
    
    fn renorme(&mut self) {
        loop {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byteout();
            }
            if self.a >= 0x8000 {
                break;
            }
        }
    }
    
    fn encode(&mut self, d: u8, cx: usize) {
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = QE_VALUES[idx] as u32;
        
        self.a -= qe;
        if d == mps {
            // Code MPS
            if (self.a & 0x8000) == 0 {
                if self.a < qe {
                    self.a = qe;
                } else {
                    self.c += qe;
                }
                self.contexts[cx] = (NMPS[idx] << 1) | mps;
                self.renorme();
            } else {
                self.c += qe;
            }
        } else {
            // Code LPS
            if self.a < qe {
                self.c += qe;
            } else {
                self.a = qe;
            }
            let switch = SWITCH[idx];
            let next_idx = NLPS[idx];
            let new_mps = if switch != 0 { mps ^ 1 } else { mps };
            self.contexts[cx] = (next_idx << 1) | new_mps;
            self.renorme();
        }
    }
    
    fn flush(&mut self) {
        let temp_c = self.c + self.a;
        self.c |= 0xffff;
        if self.c >= temp_c {
            self.c -= 0x8000;
        }
        self.c <<= self.ct;
        self.byteout();
        self.c <<= self.ct;
        self.byteout();
        
        if self.buffer[self.bp] != 0xff {
            self.bp += 1;
        }
    }
    
    fn get_buffer(&self) -> &[u8] {
        // OpenJPEG: numbytes = bp - start
        // bp points to last written byte (or one past if not 0xFF)
        // start is at index 1 (index 0 is dummy)
        // So we return buffer[1..bp]
        &self.buffer[1..self.bp]
    }
}

#[test]
fn test_reference_vs_ours() {
    use jpegexp_rs::jpeg2000::mq_coder::MqCoder;
    
    println!("\n=== Comparing Reference vs Our Implementation ===\n");
    
    for count in [1, 2, 5, 10, 20] {
        // Reference implementation
        let mut ref_mq = ReferenceMQ::new();
        ref_mq.init_enc();
        for i in 0..19 {
            ref_mq.set_context(i, 0);
        }
        ref_mq.set_context(0, 4 << 1);
        ref_mq.set_context(17, 3 << 1);
        ref_mq.set_context(18, 46 << 1);
        
        for _ in 0..count {
            ref_mq.encode(0, 0);
        }
        ref_mq.flush();
        let ref_result = ref_mq.get_buffer();
        
        // Our implementation
        let mut our_mq = MqCoder::new();
        our_mq.init_contexts(19);
        for i in 0..19 {
            our_mq.set_context(i, 0);
        }
        our_mq.set_context(0, 4 << 1);
        our_mq.set_context(17, 3 << 1);
        our_mq.set_context(18, 46 << 1);
        our_mq.init_encoder();
        
        for _ in 0..count {
            our_mq.encode(0, 0);
        }
        our_mq.flush();
        let our_result = our_mq.get_buffer();
        
        let match_str = if ref_result == our_result { "✓ MATCH" } else { "✗ DIFFER" };
        println!("{:2} symbols: {} | Ref: {} bytes {:02X?} | Ours: {} bytes {:02X?}", 
                 count, match_str, ref_result.len(), ref_result, our_result.len(), our_result);
        
        if ref_result != our_result {
            println!("    DIVERGENCE FOUND!");
            // Show byte-by-byte comparison
            let min_len = ref_result.len().min(our_result.len());
            for i in 0..min_len {
                if ref_result[i] != our_result[i] {
                    println!("    Byte {}: Ref=0x{:02X}, Ours=0x{:02X}", i, ref_result[i], our_result[i]);
                }
            }
            if ref_result.len() != our_result.len() {
                println!("    Length: Ref={}, Ours={}", ref_result.len(), our_result.len());
            }
        }
    }
}

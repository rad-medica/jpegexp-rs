//! MQ Arithmetic Coder (ISO/IEC 15444-1 Annex C)

// State Transition Tables (Index, Qe, NMPS, NLPS, Switch)
// Compressed format or full struct? Let's use full arrays.

#[derive(Clone, Copy)]
struct MqContextState {
    pub qe: u16,
    pub nmps: u8,
    pub nlps: u8,
    pub switch: u8,
}

// Standard Table C-2
const MQ_TABLE: [MqContextState; 47] = [
    MqContextState {
        qe: 0x5601,
        nmps: 1,
        nlps: 1,
        switch: 1,
    },
    MqContextState {
        qe: 0x3401,
        nmps: 2,
        nlps: 6,
        switch: 0,
    },
    MqContextState {
        qe: 0x1801,
        nmps: 3,
        nlps: 9,
        switch: 0,
    },
    MqContextState {
        qe: 0x0AC1,
        nmps: 4,
        nlps: 12,
        switch: 0,
    },
    MqContextState {
        qe: 0x0521,
        nmps: 5,
        nlps: 29,
        switch: 0,
    },
    MqContextState {
        qe: 0x0221,
        nmps: 38,
        nlps: 33,
        switch: 0,
    },
    MqContextState {
        qe: 0x5601,
        nmps: 7,
        nlps: 6,
        switch: 1,
    },
    MqContextState {
        qe: 0x5401,
        nmps: 8,
        nlps: 14,
        switch: 0,
    },
    MqContextState {
        qe: 0x4801,
        nmps: 9,
        nlps: 14,
        switch: 0,
    },
    MqContextState {
        qe: 0x3801,
        nmps: 10,
        nlps: 14,
        switch: 0,
    },
    MqContextState {
        qe: 0x3001,
        nmps: 11,
        nlps: 17,
        switch: 0,
    },
    MqContextState {
        qe: 0x2401,
        nmps: 12,
        nlps: 18,
        switch: 0,
    },
    MqContextState {
        qe: 0x1C01,
        nmps: 13,
        nlps: 20,
        switch: 0,
    },
    MqContextState {
        qe: 0x1601,
        nmps: 29,
        nlps: 21,
        switch: 0,
    },
    MqContextState {
        qe: 0x5601,
        nmps: 15,
        nlps: 14,
        switch: 1,
    },
    MqContextState {
        qe: 0x5401,
        nmps: 16,
        nlps: 14,
        switch: 0,
    },
    MqContextState {
        qe: 0x5101,
        nmps: 17,
        nlps: 15,
        switch: 0,
    },
    MqContextState {
        qe: 0x4801,
        nmps: 18,
        nlps: 16,
        switch: 0,
    },
    MqContextState {
        qe: 0x3801,
        nmps: 19,
        nlps: 17,
        switch: 0,
    },
    MqContextState {
        qe: 0x3401,
        nmps: 20,
        nlps: 18,
        switch: 0,
    },
    MqContextState {
        qe: 0x3001,
        nmps: 21,
        nlps: 19,
        switch: 0,
    },
    MqContextState {
        qe: 0x2801,
        nmps: 22,
        nlps: 19,
        switch: 0,
    },
    MqContextState {
        qe: 0x2401,
        nmps: 23,
        nlps: 19,
        switch: 0,
    },
    MqContextState {
        qe: 0x2201,
        nmps: 24,
        nlps: 19,
        switch: 0,
    },
    MqContextState {
        qe: 0x1C01,
        nmps: 25,
        nlps: 20,
        switch: 0,
    },
    MqContextState {
        qe: 0x1801,
        nmps: 26,
        nlps: 21,
        switch: 0,
    },
    MqContextState {
        qe: 0x1601,
        nmps: 27,
        nlps: 22,
        switch: 0,
    },
    MqContextState {
        qe: 0x1401,
        nmps: 28,
        nlps: 23,
        switch: 0,
    },
    MqContextState {
        qe: 0x1201,
        nmps: 29,
        nlps: 24,
        switch: 0,
    },
    MqContextState {
        qe: 0x1101,
        nmps: 30,
        nlps: 25,
        switch: 0,
    },
    MqContextState {
        qe: 0x0AC1,
        nmps: 31,
        nlps: 26,
        switch: 0,
    },
    MqContextState {
        qe: 0x09C1,
        nmps: 32,
        nlps: 27,
        switch: 0,
    },
    MqContextState {
        qe: 0x08A1,
        nmps: 33,
        nlps: 28,
        switch: 0,
    },
    MqContextState {
        qe: 0x0521,
        nmps: 34,
        nlps: 29,
        switch: 0,
    },
    MqContextState {
        qe: 0x0441,
        nmps: 35,
        nlps: 30,
        switch: 0,
    },
    MqContextState {
        qe: 0x02A1,
        nmps: 36,
        nlps: 31,
        switch: 0,
    },
    MqContextState {
        qe: 0x0221,
        nmps: 37,
        nlps: 32,
        switch: 0,
    },
    MqContextState {
        qe: 0x0141,
        nmps: 38,
        nlps: 33,
        switch: 0,
    },
    MqContextState {
        qe: 0x0111,
        nmps: 39,
        nlps: 34,
        switch: 0,
    },
    MqContextState {
        qe: 0x0085,
        nmps: 40,
        nlps: 35,
        switch: 0,
    },
    MqContextState {
        qe: 0x0049,
        nmps: 41,
        nlps: 36,
        switch: 0,
    },
    MqContextState {
        qe: 0x0025,
        nmps: 42,
        nlps: 37,
        switch: 0,
    },
    MqContextState {
        qe: 0x0015,
        nmps: 43,
        nlps: 38,
        switch: 0,
    },
    MqContextState {
        qe: 0x0009,
        nmps: 44,
        nlps: 39,
        switch: 0,
    },
    MqContextState {
        qe: 0x0005,
        nmps: 45,
        nlps: 40,
        switch: 0,
    },
    MqContextState {
        qe: 0x0001,
        nmps: 45,
        nlps: 41,
        switch: 0,
    },
    MqContextState {
        qe: 0x5601,
        nmps: 46,
        nlps: 46,
        switch: 0,
    },
];

pub struct MqCoder {
    // Registers
    a: u16, // Interval size (16 bits)
    c: u32, // Code register (28 bits essentially)

    // Buffer (Encoder)
    bp: Vec<u8>,
    bp_idx: usize,

    // State (Shared/Encoder)
    ct: u8,
    #[allow(dead_code)]
    b: u8,

    // State for Decoder
    source: Vec<u8>,
    src_pos: usize,
    buffer_byte: u8, // 'B' register in spec? Or temp?

    // Contexts
    contexts: Vec<u8>,
}

impl Default for MqCoder {
    fn default() -> Self {
        Self {
            a: 0x8000,
            c: 0,
            bp: Vec::new(),
            bp_idx: 0,
            ct: 12,
            b: 0,
            contexts: vec![0; 47], // Usually 19 but context indices can be higher?
            source: Vec::new(),
            src_pos: 0,
            buffer_byte: 0,
        }
    }
}

impl MqCoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init_contexts(&mut self, size: usize) {
        self.contexts = vec![0; size];
    }
    
    /// Set a specific context to a given state and MPS value
    /// state_idx: Index into MQ_TABLE (0-46)
    /// mps: Most Probable Symbol (0 or 1)
    pub fn set_context(&mut self, cx: usize, state_idx: u8, mps: u8) {
        if cx < self.contexts.len() {
            self.contexts[cx] = (state_idx << 1) | (mps & 1);
        }
    }

    // ... (Encoder methods omitted or assumed present) ...

    // Decoder Initialization (C.3.1) - Following OpenJPEG's approach
    pub fn init_decoder(&mut self, data: &[u8]) {
        self.source = data.to_vec();
        self.src_pos = 0;
        self.ct = 0;

        if data.is_empty() {
            self.c = 0xFF << 16;
        } else {
            self.c = (data[0] as u32) << 16;
        }

        self.byte_in();
        self.c <<= 7;
        self.ct = self.ct.saturating_sub(7);
        self.a = 0x8000;
    }

    fn byte_in(&mut self) {
        // Following OpenJPEG's bytein logic
        // Looks at current byte and next byte
        if self.src_pos >= self.source.len() {
            // End of stream - add 0xFF00 pattern
            if std::env::var("MQ_TRACE").is_ok() {
                eprintln!("byte_in: EOS at pos {}, adding 0xFF00", self.src_pos);
            }
            self.c += 0xFF00;
            self.ct = 8;
            return;
        }
        
        let current = self.source[self.src_pos];
        
        if current == 0xFF {
            let next = if self.src_pos + 1 < self.source.len() {
                self.source[self.src_pos + 1]
            } else {
                0xFF
            };
            
            if next > 0x8F {
                // Marker detected - don't consume, add 0xFF00
                if std::env::var("MQ_TRACE").is_ok() {
                    eprintln!("byte_in: pos={}, current=0xFF, marker detected, adding 0xFF00", self.src_pos);
                }
                self.c += 0xFF00;
                self.ct = 8;
            } else {
                // Bit stuffing - the byte after 0xFF has only 7 valid bits
                self.src_pos += 1;
                if std::env::var("MQ_TRACE").is_ok() {
                    eprintln!("byte_in: pos={}, byte={:#x} (after 0xFF, 7 bits)", self.src_pos, self.source[self.src_pos]);
                }
                self.c += (self.source[self.src_pos] as u32) << 9;
                self.ct = 7;
            }
        } else {
            // Normal case - read next byte
            self.src_pos += 1;
            if self.src_pos < self.source.len() {
                let byte = self.source[self.src_pos];
                if std::env::var("MQ_TRACE").is_ok() {
                    eprintln!("byte_in: pos={}, byte={:#x}", self.src_pos, byte);
                }
                self.c += (byte as u32) << 8;
                self.ct = 8;
            } else {
                // End of stream
                if std::env::var("MQ_TRACE").is_ok() {
                    eprintln!("byte_in: pos={}, EOS, adding 0xFF00", self.src_pos);
                }
                self.c += 0xFF00;
                self.ct = 8;
            }
        }
    }

    // C.3.2 Decoding a symbol
    pub fn decode_bit(&mut self, cx: usize) -> u8 {
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = MQ_TABLE[idx].qe;
        
        if std::env::var("MQ_TRACE").is_ok() {
            eprintln!("MQ_DEC: cx={} mps={} idx={} qe={:#06x} A={:#06x} C={:#010x}", 
                cx, mps, idx, qe, self.a, self.c);
        }
        
        // Debug tracing (only for first few calls if env var set)
        let _debug = std::env::var("MQ_DEBUG").is_ok();

        // This corresponds to "DECODE" procedure
        // My Encoder put LPS at [0, Qe), MPS at [Qe, A).
        // So check C against Qe.
        // Wait, C is high 16 bits of register?
        // C register is 28 bits?
        // "C register ... 16 bits active + spacers"
        // Let's check `c` magnitude relative to `a` (0x8000..0xFFFF).
        // My `c` is u32.
        // If I mapped encoder `c += qe` (move base up).
        // Then `c` represents the base.
        // The *incoming* stream represents a value `V`.
        // `C_reg` holds bits from `V`?
        // "Chigh contains the most significant 16 bits of the code"
        // In my `init_decoder`, I fill `c` with shifts.
        // `a` is 16 bits (0x8000).
        // So `c-buffers` needs to align with `a`.
        // C should be effectively 16 bits for comparison?
        // `c` variable here holds `Chigh` (16 bits) + `Clow` (12 bits) = 28 bits?
        // Let's assume `c` (u32) holds `Chigh << 16 | Clow`.
        // `a` operates on `Chigh`.

        // Comparison: `Chigh < Qe` is impossible if `Chigh` is 16 bits and `Qe` is 16 bits?
        // `Chigh` is normalized to be roughly in range of `a`?
        // Spec C.3.2:
        // `A -= Qe`
        // `if (Chigh < Qe)` -> LPS (Exchange intervals?)
        // `else` -> MPS

        // Chigh is `self.c >> 16`.

        self.a -= qe;
        let chigh = (self.c >> 16) as u16;
        
        let mq_debug = std::env::var("MQ_DEBUG").is_ok();
        if mq_debug && cx == 17 {
            eprintln!("MQ decode_bit cx={}: A_before_sub={:#06x}, qe={:#06x}, A={:#06x}, chigh={:#06x}, C={:#010x}, mps={}, idx={}",
                cx, self.a + qe, qe, self.a, chigh, self.c, mps, idx);
        }

        let d;
        // OpenJPEG-compatible MQ Decoding:
        // The interval [0, A_old) is split into:
        //   MPS: [0, A_new) where A_new = A_old - Qe
        //   LPS: [A_new, A_old) (size Qe)
        // 
        // If Chigh < A_new: We're in MPS sub-interval
        // If Chigh >= A_new: We're in LPS sub-interval, C -= A_new
        if chigh >= self.a {
            // LPS path - Chigh >= A (after A -= Qe)
            // C -= A to renormalize into LPS sub-interval
            if std::env::var("MQ_TRACE").is_ok() {
                eprintln!("  -> LPS path (chigh {:#x} >= A {:#x})", chigh, self.a);
            }
            self.c -= (self.a as u32) << 16;
            
            // Standard LPS exchange logic
            if self.a < qe {
                // Conditional exchange: return MPS, use NMPS context
                d = mps;
                self.contexts[cx] = (MQ_TABLE[idx].nmps << 1) | mps;
            } else {
                // Normal LPS: return LPS, use NLPS context
                d = 1 - mps;
                if std::env::var("MQ_TRACE").is_ok() {
                    eprintln!("  DEC LPS normal: returning LPS={}", d);
                }
                let switch = MQ_TABLE[idx].switch;
                let next_idx = MQ_TABLE[idx].nlps;
                let next_mps = if switch == 1 { 1 - mps } else { mps };
                self.contexts[cx] = (next_idx << 1) | next_mps;
            }
            self.a = qe;
            self.renormalize_input();
            d
        } else {
            // MPS path - Chigh < A
            // C stays the same for MPS (we're in the lower portion of interval)
            if std::env::var("MQ_TRACE").is_ok() {
                eprintln!("  -> MPS path (chigh {:#x} < A {:#x})", chigh, self.a);
            }
            if self.a < 0x8000 {
                // Need renormalization - apply MPS exchange
                if self.a < qe {
                    // Conditional exchange: LPS sub-interval is larger than MPS
                    // Return LPS, use NLPS context
                    d = 1 - mps;
                    self.a = qe;
                    let switch = MQ_TABLE[idx].switch;
                    let next_idx = MQ_TABLE[idx].nlps;
                    let next_mps = if switch == 1 { 1 - mps } else { mps };
                    self.contexts[cx] = (next_idx << 1) | next_mps;
                } else {
                    // Normal MPS: return MPS, use NMPS context
                    d = mps;
                    self.contexts[cx] = (MQ_TABLE[idx].nmps << 1) | mps;
                }
                self.renormalize_input();
                d
            } else {
                mps
            }
        }
    }

    fn renormalize_input(&mut self) {
        // Following OpenJPEG's opj_mqc_renormd_macro
        let mut shifts = 0;
        loop {
            if self.ct == 0 {
                self.byte_in(); // byte_in already adds to c
            }
            self.a <<= 1;
            self.c <<= 1;
            self.ct = self.ct.saturating_sub(1);
            shifts += 1;
            if self.a >= 0x8000 {
                break;
            }
        }
        if std::env::var("MQ_TRACE").is_ok() {
            eprintln!("  DEC renorm: {} shifts, A={:#x}, C={:#x}, ct={}", shifts, self.a, self.c, self.ct);
        }
    }

    // Encoder methods...
    // Using OpenJPEG-compatible convention where:
    // - MPS occupies [0, A-Qe) in the interval
    // - LPS occupies [A-Qe, A) in the interval
    // So C stays low for MPS, C += A for LPS
    pub fn encode(&mut self, d: u8, cx: usize) {
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = MQ_TABLE[idx].qe;
        
        if std::env::var("MQ_TRACE").is_ok() {
            eprintln!("MQ_ENC: cx={} d={} mps={} idx={} qe={:#06x} A={:#06x} C={:#010x}", 
                cx, d, mps, idx, qe, self.a, self.c);
        }

        self.a -= qe;

        if d == mps {
            // MPS path: stay in lower part of interval, C unchanged
            if self.a < 0x8000 {
                if self.a < qe {
                    // Conditional exchange: MPS gets Qe sub-interval
                    self.c += self.a as u32;
                    self.a = qe;
                }
                // NMPS context update
                let next = MQ_TABLE[idx].nmps;
                self.contexts[cx] = (next << 1) | mps;
                self.renormalize();
            }
            // If A >= 0x8000, no renormalization needed, C stays the same
        } else {
            // LPS path
            if qe > self.a {
                // Conditional exchange: LPS is now in the LOWER sub-interval
                // Don't add to C (stay in lower), set A = qe for renorm
                // This makes decoder take MPS-path (chigh < A)
                // MPS-path-with-exchange returns LPS with NLPS transition
                self.a = qe;
                // Use NLPS with switch (matching decoder's MPS-path-with-exchange)
                let switch = MQ_TABLE[idx].switch;
                let next = MQ_TABLE[idx].nlps;
                if switch == 1 {
                    self.contexts[cx] = (next << 1) | (1 - mps);
                } else {
                    self.contexts[cx] = (next << 1) | mps;
                }
            } else {
                // Normal LPS: C += A, A = qe
                let old_c = self.c;
                let old_a = self.a;
                self.c += self.a as u32;
                self.a = qe;
                if std::env::var("MQ_TRACE").is_ok() {
                    eprintln!("  LPS normal: C {:#x} + A {:#x} = {:#x}, new A={:#x}", 
                        old_c, old_a, self.c, self.a);
                }
                // Use NLPS with switch
                let switch = MQ_TABLE[idx].switch;
                let next = MQ_TABLE[idx].nlps;
                if switch == 1 {
                    self.contexts[cx] = (next << 1) | (1 - mps);
                } else {
                    self.contexts[cx] = (next << 1) | mps;
                }
            }

            if std::env::var("MQ_TRACE").is_ok() {
                eprintln!("  Before renorm: A={:#x}, C={:#x}, ct={}", self.a, self.c, self.ct);
            }
            self.renormalize();
            if std::env::var("MQ_TRACE").is_ok() {
                eprintln!("  After renorm: A={:#x}, C={:#x}, ct={}", self.a, self.c, self.ct);
            }
        }
    }

    fn renormalize(&mut self) {
        loop {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byte_out();
                self.ct = 8;
            }
            if self.a >= 0x8000 {
                break;
            }
        }
    }

    fn byte_out(&mut self) {
        let b_out = (self.c >> 19) as u8;
        if std::env::var("MQ_TRACE").is_ok() {
            eprintln!("  byte_out: C={:#x} → byte={:#x}", self.c, b_out);
        }
        if b_out == 0xFF {
            self.ct = 7;
        }
        self.c &= 0x7FFFF;
        self.bp.push(b_out);
        self.bp_idx += 1;
    }

    /// Flush the encoder - must be called after encoding to finalize the bitstream
    /// Per JPEG2000 spec C.2.9
    pub fn flush(&mut self) {
        if std::env::var("MQ_TRACE").is_ok() {
            eprintln!("MQ flush: before C={:#010x} A={:#06x} ct={}", self.c, self.a, self.ct);
        }
        
        // Set bits in c to 1 (SETBITS procedure)
        let temp = self.c + self.a as u32;
        self.c |= 0xFFFF;
        if self.c >= temp {
            self.c -= 0x8000;
        }
        
        if std::env::var("MQ_TRACE").is_ok() {
            eprintln!("MQ flush: after SETBITS C={:#010x}", self.c);
        }

        // Shift out the final bytes - must output enough to capture all bits
        // Following OpenJPEG: output at least 2 bytes, then check if more needed
        for _ in 0..4 {  // Output up to 4 bytes to be safe
            self.c <<= self.ct;
            self.byte_out();
            if self.c == 0 {
                break;
            }
        }
        
        if std::env::var("MQ_TRACE").is_ok() {
            eprintln!("MQ flush: output {} bytes", self.bp.len());
        }

        // Remove trailing 0xFF if present (marker avoidance)
        while self.bp.len() > 1 && *self.bp.last().unwrap_or(&0) == 0xFF {
            self.bp.pop();
        }
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.bp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mq_init() {
        let mq = MqCoder::new();
        assert_eq!(mq.a, 0x8000);
        assert_eq!(mq.ct, 12);
    }

    #[test]
    fn test_mq_encode_update() {
        let mut mq = MqCoder::new();
        mq.init_contexts(5);
        // Encode a few MPS symbols
        let cx = 0;
        let _original_c = mq.c;
        mq.encode(0, cx); // MPS (0 is default MPS for 0 index table?)

        // After encoding, A should be renormalized to >= 0x8000
        assert!(mq.a >= 0x8000);
    }

    #[test]
    fn test_mq_encode_decode_roundtrip() {
        let mut mq_enc = MqCoder::new();
        mq_enc.init_contexts(3);

        // Encode sequence: 0, 0, 1, 0, 1, 1, 0, 1, 0, 0 (Context 0)
        let bits: Vec<u8> = vec![0, 0, 1, 0, 1, 1, 0, 1, 0, 0];
        for &b in &bits {
            mq_enc.encode(b, 0);
        }
        mq_enc.flush();
        let encoded = mq_enc.get_buffer().to_vec();
        
        // Decode
        let mut mq_dec = MqCoder::new();
        mq_dec.init_contexts(3);
        mq_dec.init_decoder(&encoded);
        
        let mut decoded = Vec::new();
        for _ in 0..bits.len() {
            decoded.push(mq_dec.decode_bit(0));
        }
        
        assert_eq!(bits, decoded, "MQ roundtrip failed: encoded {:?}, decoded {:?}", bits, decoded);
    }

    #[test]
    fn test_mq_multi_context_roundtrip() {
        // Test with context 17 (RUN context) initialized like BitPlaneCoder does
        let mut mq_enc = MqCoder::new();
        mq_enc.init_contexts(19);
        
        // Initialize context 17 (RUN) to state 3, like BitPlaneCoder does
        mq_enc.set_context(17, 3, 0);
        // Initialize context 18 (UNIFORM) to state 46
        mq_enc.set_context(18, 46, 0);
        
        // Simple sequence using RUN context: MPS, MPS, LPS, MPS
        let operations: Vec<(u8, usize)> = vec![
            (0, 17), // MPS for RUN context
            (0, 17), // MPS
            (1, 17), // LPS - this is the tricky one
            (0, 17), // MPS
        ];
        
        for &(bit, ctx) in &operations {
            mq_enc.encode(bit, ctx);
        }
        mq_enc.flush();
        let encoded = mq_enc.get_buffer().to_vec();
        
        println!("Simple encoded {} bytes: {:02X?}", encoded.len(), &encoded);
        
        // Decode - must have matching context initialization
        let mut mq_dec = MqCoder::new();
        mq_dec.init_contexts(19);
        mq_dec.set_context(17, 3, 0);
        mq_dec.set_context(18, 46, 0);
        mq_dec.init_decoder(&encoded);
        
        let mut decoded = Vec::new();
        for &(_, ctx) in &operations {
            decoded.push((mq_dec.decode_bit(ctx), ctx));
        }
        
        println!("Expected: {:?}", operations);
        println!("Decoded:  {:?}", decoded);
        
        for (i, (&(expected_bit, ctx), (decoded_bit, _))) in operations.iter().zip(decoded.iter()).enumerate() {
            assert_eq!(expected_bit, *decoded_bit, 
                "Mismatch at op {}: ctx={}, expected={}, decoded={}", 
                i, ctx, expected_bit, decoded_bit);
        }
    }
}

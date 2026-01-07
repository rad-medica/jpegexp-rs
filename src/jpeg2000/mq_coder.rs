//! MQ Arithmetic Coder (ISO/IEC 15444-1 Annex C)
//! Based on OpenJPEG implementation

// State Transition Tables (Index, Qe, NMPS, NLPS, Switch)
#[derive(Clone, Copy)]
struct MqContextState {
    pub qe: u16,
    pub nmps: u8,
    pub nlps: u8,
    pub switch: u8,
}

// Standard Table C-2 from ISO/IEC 15444-1
const MQ_TABLE: [MqContextState; 47] = [
    MqContextState { qe: 0x5601, nmps: 1,  nlps: 1,  switch: 1 },  // 0
    MqContextState { qe: 0x3401, nmps: 2,  nlps: 6,  switch: 0 },  // 1
    MqContextState { qe: 0x1801, nmps: 3,  nlps: 9,  switch: 0 },  // 2
    MqContextState { qe: 0x0AC1, nmps: 4,  nlps: 12, switch: 0 },  // 3
    MqContextState { qe: 0x0521, nmps: 5,  nlps: 29, switch: 0 },  // 4
    MqContextState { qe: 0x0221, nmps: 38, nlps: 33, switch: 0 },  // 5
    MqContextState { qe: 0x5601, nmps: 7,  nlps: 6,  switch: 1 },  // 6
    MqContextState { qe: 0x5401, nmps: 8,  nlps: 14, switch: 0 },  // 7
    MqContextState { qe: 0x4801, nmps: 9,  nlps: 14, switch: 0 },  // 8
    MqContextState { qe: 0x3801, nmps: 10, nlps: 14, switch: 0 },  // 9
    MqContextState { qe: 0x3001, nmps: 11, nlps: 17, switch: 0 },  // 10
    MqContextState { qe: 0x2401, nmps: 12, nlps: 18, switch: 0 },  // 11
    MqContextState { qe: 0x1C01, nmps: 13, nlps: 20, switch: 0 },  // 12
    MqContextState { qe: 0x1601, nmps: 29, nlps: 21, switch: 0 },  // 13
    MqContextState { qe: 0x5601, nmps: 15, nlps: 14, switch: 1 },  // 14
    MqContextState { qe: 0x5401, nmps: 16, nlps: 14, switch: 0 },  // 15
    MqContextState { qe: 0x5101, nmps: 17, nlps: 15, switch: 0 },  // 16
    MqContextState { qe: 0x4801, nmps: 18, nlps: 16, switch: 0 },  // 17
    MqContextState { qe: 0x3801, nmps: 19, nlps: 17, switch: 0 },  // 18
    MqContextState { qe: 0x3401, nmps: 20, nlps: 18, switch: 0 },  // 19
    MqContextState { qe: 0x3001, nmps: 21, nlps: 19, switch: 0 },  // 20
    MqContextState { qe: 0x2801, nmps: 22, nlps: 19, switch: 0 },  // 21
    MqContextState { qe: 0x2401, nmps: 23, nlps: 20, switch: 0 },  // 22
    MqContextState { qe: 0x2201, nmps: 24, nlps: 21, switch: 0 },  // 23
    MqContextState { qe: 0x1C01, nmps: 25, nlps: 22, switch: 0 },  // 24
    MqContextState { qe: 0x1801, nmps: 26, nlps: 23, switch: 0 },  // 25
    MqContextState { qe: 0x1601, nmps: 27, nlps: 24, switch: 0 },  // 26
    MqContextState { qe: 0x1401, nmps: 28, nlps: 25, switch: 0 },  // 27
    MqContextState { qe: 0x1201, nmps: 29, nlps: 26, switch: 0 },  // 28
    MqContextState { qe: 0x1101, nmps: 30, nlps: 27, switch: 0 },  // 29
    MqContextState { qe: 0x0AC1, nmps: 31, nlps: 28, switch: 0 },  // 30
    MqContextState { qe: 0x09C1, nmps: 32, nlps: 29, switch: 0 },  // 31
    MqContextState { qe: 0x08A1, nmps: 33, nlps: 30, switch: 0 },  // 32
    MqContextState { qe: 0x0521, nmps: 34, nlps: 31, switch: 0 },  // 33
    MqContextState { qe: 0x0441, nmps: 35, nlps: 32, switch: 0 },  // 34
    MqContextState { qe: 0x02A1, nmps: 36, nlps: 33, switch: 0 },  // 35
    MqContextState { qe: 0x0221, nmps: 37, nlps: 34, switch: 0 },  // 36
    MqContextState { qe: 0x0141, nmps: 38, nlps: 35, switch: 0 },  // 37
    MqContextState { qe: 0x0111, nmps: 39, nlps: 36, switch: 0 },  // 38
    MqContextState { qe: 0x0085, nmps: 40, nlps: 37, switch: 0 },  // 39
    MqContextState { qe: 0x0049, nmps: 41, nlps: 38, switch: 0 },  // 40
    MqContextState { qe: 0x0025, nmps: 42, nlps: 39, switch: 0 },  // 41
    MqContextState { qe: 0x0015, nmps: 43, nlps: 40, switch: 0 },  // 42
    MqContextState { qe: 0x0009, nmps: 44, nlps: 41, switch: 0 },  // 43
    MqContextState { qe: 0x0005, nmps: 45, nlps: 42, switch: 0 },  // 44
    MqContextState { qe: 0x0001, nmps: 45, nlps: 43, switch: 0 },  // 45
    MqContextState { qe: 0x5601, nmps: 46, nlps: 46, switch: 0 },  // 46 (Uniform)
];

pub struct MqCoder {
    // Registers
    a: u32,   // Interval (lower 16 bits meaningful)
    c: u32,   // Code register (28 effective bits)
    ct: i32,  // Counter (bits available before byte_out/byte_in)

    // Encoder buffer
    bp: Vec<u8>,
    last_byte: u8,  // Last byte written (for bit-stuffing check)

    // Decoder source
    source: Vec<u8>,
    src_pos: usize,

    // Contexts: Each context stores (index << 1 | mps)
    contexts: Vec<u8>,
}

impl Default for MqCoder {
    fn default() -> Self {
        Self {
            a: 0x8000,
            c: 0,
            ct: 12,
            bp: Vec::new(),
            last_byte: 0,
            contexts: vec![0; 47],
            source: Vec::new(),
            src_pos: 0,
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

    pub fn set_context(&mut self, cx: usize, state: u8) {
        if cx >= self.contexts.len() {
            self.contexts.resize(cx + 1, 0);
        }
        self.contexts[cx] = state;
    }

    // ======== ENCODER ========

    /// Initialize encoder
    pub fn init_encoder(&mut self) {
        self.a = 0x8000;
        self.c = 0;
        self.ct = 12;
        self.bp.clear();
        self.last_byte = 0;
    }

    /// Encode a symbol (OpenJPEG style)
    pub fn encode(&mut self, d: u8, cx: usize) {
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = MQ_TABLE[idx].qe as u32;

        self.a -= qe;

        if d == mps {
            // Encode MPS (opj_mqc_codemps_macro)
            if (self.a & 0x8000) == 0 {
                // Need renormalization
                if self.a < qe {
                    // Conditional exchange: MPS placed in lower sub-interval
                    self.a = qe;
                } else {
                    // MPS placed in upper sub-interval
                    self.c += qe;
                }
                let next_idx = MQ_TABLE[idx].nmps;
                self.contexts[cx] = (next_idx << 1) | mps;
                self.renormalize_enc();
            } else {
                // No renormalization needed - MPS in upper sub-interval
                self.c += qe;
            }
        } else {
            // Encode LPS (opj_mqc_codelps_macro)
            if self.a < qe {
                // Conditional exchange: LPS placed in upper sub-interval
                self.c += qe;
            } else {
                // LPS placed in lower sub-interval
                self.a = qe;
            }
            // LPS state transition (may switch MPS sense)
            let switch = MQ_TABLE[idx].switch;
            let next_idx = MQ_TABLE[idx].nlps;
            let new_mps = if switch == 1 { mps ^ 1 } else { mps };
            self.contexts[cx] = (next_idx << 1) | new_mps;
            self.renormalize_enc();
        }
    }

    fn renormalize_enc(&mut self) {
        while self.a < 0x8000 {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byte_out();
            }
        }
    }

    /// Output a byte (OpenJPEG style)
    fn byte_out(&mut self) {
        if self.last_byte == 0xFF {
            // After 0xFF, output only 7 bits
            let b = (self.c >> 20) as u8;
            self.c &= 0xFFFFF;
            self.ct = 7;
            self.bp.push(b);
            self.last_byte = b;
        } else {
            // Check for carry propagation
            if (self.c & 0x8000000) != 0 {
                // Carry: increment last byte
                if !self.bp.is_empty() {
                    let len = self.bp.len();
                    self.bp[len - 1] = self.bp[len - 1].wrapping_add(1);
                    self.last_byte = self.bp[len - 1];
                }
                self.c &= 0x7FFFFFF; // Clear carry bit

                if self.last_byte == 0xFF {
                    // After incrementing to 0xFF, output only 7 bits
                    let b = (self.c >> 20) as u8;
                    self.c &= 0xFFFFF;
                    self.ct = 7;
                    self.bp.push(b);
                    self.last_byte = b;
                } else {
                    // Normal output after carry
                    let b = (self.c >> 19) as u8;
                    self.c &= 0x7FFFF;
                    self.ct = 8;
                    self.bp.push(b);
                    self.last_byte = b;
                }
            } else {
                // No carry: normal output
                let b = (self.c >> 19) as u8;
                self.c &= 0x7FFFF;
                self.ct = 8;
                self.bp.push(b);
                self.last_byte = b;
            }
        }
    }

    /// Flush the encoder (FLUSH procedure - Figure C.11)
    pub fn flush(&mut self) {
        // SETBITS: Set low-order bits of C to force correct termination
        let temp = self.c + self.a;
        self.c |= 0xFFFF;
        if self.c >= temp {
            self.c -= 0x8000;
        }

        // Output remaining bytes
        self.c <<= self.ct;
        self.byte_out();
        self.c <<= self.ct;
        self.byte_out();

        // Remove trailing 0xFF
        while let Some(&b) = self.bp.last() {
            if b == 0xFF {
                self.bp.pop();
            } else {
                break;
            }
        }
    }

    pub fn get_buffer(&self) -> &[u8] {
        &self.bp
    }

    // ======== DECODER ========

    /// Initialize decoder (INITDEC - Figure C.19)
    pub fn init_decoder(&mut self, data: &[u8]) {
        // Copy data and add artificial end markers
        self.source = data.to_vec();
        // Add 0xFF 0xFF at end for bytein to stop on
        self.source.push(0xFF);
        self.source.push(0xFF);

        self.src_pos = 0;
        self.a = 0x8000;

        if data.is_empty() {
            self.c = 0xFF << 16;
        } else {
            self.c = (self.source[0] as u32) << 16;
        }

        self.byte_in();
        self.c <<= 7;
        self.ct -= 7;
    }

    /// Read a byte (bytein - OpenJPEG style)
    fn byte_in(&mut self) {
        // Read the NEXT byte (at src_pos + 1)
        let next_pos = self.src_pos + 1;
        let next_byte = if next_pos < self.source.len() {
            self.source[next_pos]
        } else {
            0xFF
        };

        // Check if current byte is 0xFF
        let cur_byte = if self.src_pos < self.source.len() {
            self.source[self.src_pos]
        } else {
            0xFF
        };

        if cur_byte == 0xFF {
            if next_byte > 0x8F {
                // Marker detected - don't advance, pad with 0xFF
                self.c += 0xFF00;
                self.ct = 8;
            } else {
                // Bit-stuffed: next byte has only 7 valid bits
                self.src_pos += 1;
                self.c += (next_byte as u32) << 9;
                self.ct = 7;
            }
        } else {
            // Normal byte
            self.src_pos += 1;
            self.c += (next_byte as u32) << 8;
            self.ct = 8;
        }
    }

    /// Decode a symbol (OpenJPEG style - C.3.2 DECODE)
    pub fn decode_bit(&mut self, cx: usize) -> u8 {
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = MQ_TABLE[idx].qe as u32;

        // A = A - Qe
        self.a -= qe;

        // Check if code value is in LPS sub-interval (upper part)
        // C bits [23:16] contain the 8-bit code value to compare with Qe
        let c_active = self.c >> 16;

        if c_active < qe {
            // Code value in LPS sub-interval (lower part of the interval)
            // LPS exchange logic
            let d = self.lps_exchange(cx, idx, mps);
            self.renormalize_dec();
            d
        } else {
            // Code value in MPS sub-interval (upper part)
            // Subtract Qe << 16 from C
            self.c -= qe << 16;

            if (self.a & 0x8000) == 0 {
                // Need renormalization - check for conditional exchange
                let d = self.mps_exchange(cx, idx, mps);
                self.renormalize_dec();
                d
            } else {
                // No renormalization needed, definitely MPS
                // IMPORTANT: Do NOT update context when no renorm needed!
                mps
            }
        }
    }

    /// MPS exchange logic - handles conditional exchange when in MPS sub-interval
    fn mps_exchange(&mut self, cx: usize, idx: usize, mps: u8) -> u8 {
        let qe = MQ_TABLE[idx].qe as u32;
        if self.a < qe {
            // Conditional exchange: decode LPS
            let switch = MQ_TABLE[idx].switch;
            let next_idx = MQ_TABLE[idx].nlps;
            let new_mps = if switch == 1 { mps ^ 1 } else { mps };
            self.contexts[cx] = (next_idx << 1) | new_mps;
            mps ^ 1 // Return LPS
        } else {
            // Normal MPS decode
            let next_idx = MQ_TABLE[idx].nmps;
            self.contexts[cx] = (next_idx << 1) | mps;
            mps
        }
    }

    /// LPS exchange logic - handles conditional exchange when in LPS sub-interval
    fn lps_exchange(&mut self, cx: usize, idx: usize, mps: u8) -> u8 {
        let qe = MQ_TABLE[idx].qe as u32;
        if self.a < qe {
            // Conditional exchange: decode MPS
            self.a = qe;
            let next_idx = MQ_TABLE[idx].nmps;
            self.contexts[cx] = (next_idx << 1) | mps;
            mps
        } else {
            // Normal LPS decode
            self.a = qe;
            let switch = MQ_TABLE[idx].switch;
            let next_idx = MQ_TABLE[idx].nlps;
            let new_mps = if switch == 1 { mps ^ 1 } else { mps };
            self.contexts[cx] = (next_idx << 1) | new_mps;
            mps ^ 1 // Return LPS
        }
    }

    fn renormalize_dec(&mut self) {
        while self.a < 0x8000 {
            if self.ct == 0 {
                self.byte_in();
            }
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
        }
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
    fn test_mq_roundtrip_simple() {
        // Test simple sequence of symbols with single context
        let symbols: Vec<u8> = vec![0, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0];

        let mut encoder = MqCoder::new();
        encoder.init_contexts(5);
        encoder.init_encoder();

        // Encode
        for &sym in &symbols {
            encoder.encode(sym, 0);
        }
        encoder.flush();
        let encoded = encoder.get_buffer().to_vec();

        // Decode
        let mut decoder = MqCoder::new();
        decoder.init_contexts(5);
        decoder.init_decoder(&encoded);

        let mut decoded = Vec::new();
        for _ in 0..symbols.len() {
            let sym = decoder.decode_bit(0);
            decoded.push(sym);
        }

        assert_eq!(symbols, decoded, "MQ roundtrip failed");
    }

    #[test]
    fn test_mq_roundtrip_many_zeros() {
        // Test many zeros (common in run-length coding)
        let symbols: Vec<u8> = vec![0; 100];

        let mut encoder = MqCoder::new();
        encoder.init_contexts(5);
        encoder.init_encoder();

        for &sym in &symbols {
            encoder.encode(sym, 0);
        }
        encoder.flush();
        let encoded = encoder.get_buffer().to_vec();

        let mut decoder = MqCoder::new();
        decoder.init_contexts(5);
        decoder.init_decoder(&encoded);

        for i in 0..symbols.len() {
            let sym = decoder.decode_bit(0);
            assert_eq!(sym, 0, "Mismatch at position {}", i);
        }
    }

    #[test]
    fn test_mq_roundtrip_many_ones() {
        // Test many ones
        let symbols: Vec<u8> = vec![1; 100];

        let mut encoder = MqCoder::new();
        encoder.init_contexts(5);
        encoder.init_encoder();

        for &sym in &symbols {
            encoder.encode(sym, 0);
        }
        encoder.flush();
        let encoded = encoder.get_buffer().to_vec();

        let mut decoder = MqCoder::new();
        decoder.init_contexts(5);
        decoder.init_decoder(&encoded);

        for i in 0..symbols.len() {
            let sym = decoder.decode_bit(0);
            assert_eq!(sym, 1, "Mismatch at position {}", i);
        }
    }

    #[test]
    fn test_mq_roundtrip_alternating() {
        // Test alternating pattern
        let symbols: Vec<u8> = (0..50).map(|i| (i % 2) as u8).collect();

        let mut encoder = MqCoder::new();
        encoder.init_contexts(5);
        encoder.init_encoder();

        for &sym in &symbols {
            encoder.encode(sym, 0);
        }
        encoder.flush();
        let encoded = encoder.get_buffer().to_vec();

        let mut decoder = MqCoder::new();
        decoder.init_contexts(5);
        decoder.init_decoder(&encoded);

        let mut decoded = Vec::new();
        for _ in 0..symbols.len() {
            let sym = decoder.decode_bit(0);
            decoded.push(sym);
        }

        assert_eq!(symbols, decoded, "Alternating pattern roundtrip failed");
    }

    #[test]
    fn test_mq_roundtrip_multiple_contexts() {
        // Test with multiple contexts like bit-plane coding uses
        let symbols: Vec<(u8, usize)> = vec![
            (0, 0), (0, 1), (1, 2), (0, 0), (1, 1), (1, 2),
            (0, 3), (0, 4), (1, 0), (1, 1), (0, 2), (0, 3),
        ];

        let mut encoder = MqCoder::new();
        encoder.init_contexts(10);
        encoder.init_encoder();

        for &(sym, cx) in &symbols {
            encoder.encode(sym, cx);
        }
        encoder.flush();
        let encoded = encoder.get_buffer().to_vec();

        let mut decoder = MqCoder::new();
        decoder.init_contexts(10);
        decoder.init_decoder(&encoded);

        for (i, &(exp_sym, cx)) in symbols.iter().enumerate() {
            let sym = decoder.decode_bit(cx);
            assert_eq!(sym, exp_sym, "Mismatch at position {} (cx={})", i, cx);
        }
    }
}

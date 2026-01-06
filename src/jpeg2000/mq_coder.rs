//! MQ Arithmetic Coder (ISO/IEC 15444-1 Annex C)

// State Transition Tables (Index, Qe, NMPS, NLPS, Switch)
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
    c: u32, // Code register (32 bits, but effectively 28 bits active)

    // Buffer (Encoder)
    bp: Vec<u8>,
    // bp_idx: usize, // Unused with Vec push

    // State (Shared/Encoder)
    ct: u8,
    b: u8, // Buffered byte B

    // State for Decoder
    source: Vec<u8>,
    src_pos: usize,

    // Contexts
    contexts: Vec<u8>,
}

impl Default for MqCoder {
    fn default() -> Self {
        Self {
            a: 0x8000,
            c: 0,
            bp: Vec::new(),
            // bp_idx: 0,
            ct: 12,
            b: 0,
            contexts: vec![0; 47], // Usually 19 but context indices can be higher?
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

    pub fn set_context(&mut self, cx: usize, value: u8) {
        if cx < self.contexts.len() {
            self.contexts[cx] = value;
        }
    }

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
            self.c += 0xFF00;
            self.ct = 8;
            return;
        }

        let current = self.source[self.src_pos];
        let next = if self.src_pos + 1 < self.source.len() {
            self.source[self.src_pos + 1]
        } else {
            0xFF
        };

        if current == 0xFF {
            if next > 0x8F {
                // Marker detected - don't consume, add 0xFF00
                self.c += 0xFF00;
                self.ct = 8;
            } else {
                // Bit stuffing or regular data after 0xFF
                self.src_pos += 1;
                self.c += (next as u32) << 9; // Shift by 9 for bit stuffing
                self.ct = 7;
            }
        } else {
            self.src_pos += 1;
            self.c += (next as u32) << 8;
            self.ct = 8;
        }
    }

    // C.3.2 Decoding a symbol
    pub fn decode_bit(&mut self, cx: usize) -> u8 {
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = MQ_TABLE[idx].qe;

        self.a -= qe;
        let chigh = (self.c >> 16) as u16;

        let d;
        if chigh < qe {
            // LPS path - C_high < Qe
            if self.a < qe {
                // Conditional exchange: return MPS, use NMPS context
                self.a = qe;
                d = mps;
                self.contexts[cx] = (MQ_TABLE[idx].nmps << 1) | mps;
            } else {
                // Normal LPS: return LPS, use NLPS context
                self.a = qe;
                d = 1 - mps;
                let switch = MQ_TABLE[idx].switch;
                let next_idx = MQ_TABLE[idx].nlps;
                let next_mps = if switch == 1 { 1 - mps } else { mps };
                self.contexts[cx] = (next_idx << 1) | next_mps;
            }
            self.renormalize_input();
            d
        } else {
            // MPS path - C_high >= Qe
            self.c -= (qe as u32) << 16;

            if self.a < 0x8000 {
                // Need renormalization - apply MPS exchange
                if self.a < qe {
                    // Conditional exchange: return LPS, use NLPS context
                    d = 1 - mps;
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
        loop {
            if self.ct == 0 {
                self.byte_in(); // byte_in already adds to c
            }
            self.a <<= 1;
            self.c <<= 1;
            self.ct = self.ct.saturating_sub(1);
            if self.a >= 0x8000 {
                break;
            }
        }
    }

    // Encoder methods...
    pub fn encode(&mut self, d: u8, cx: usize) {
        // Renormalization driven encoding
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;

        let qe = MQ_TABLE[idx].qe;

        if d == mps {
            self.a -= qe;
            if self.a < 0x8000 {
                if self.a < qe {
                    self.a = qe;
                } else {
                    self.c += qe as u32;
                }
                // NMPS
                let next = MQ_TABLE[idx].nmps;
                self.contexts[cx] = (next << 1) | mps;
                self.renormalize();
            } else {
                self.c += qe as u32;
            }
        } else {
            // LPS
            self.a -= qe;
            if self.a < qe {
                self.c += qe as u32;
            } else {
                self.a = qe;
            }

            // Update Context
            let switch = MQ_TABLE[idx].switch;
            let next = MQ_TABLE[idx].nlps;
            if switch == 1 {
                self.contexts[cx] = (next << 1) | (1 - mps);
            } else {
                self.contexts[cx] = (next << 1) | mps;
            }

            self.renormalize();
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

    // Correct BYTEOUT procedure (C.2.3)
    fn byte_out(&mut self) {
        // C register: 28 bits (C27..C0)
        // Check carry (bit 27, mask 0x8000000? No, bit 27 is 0x8000000)
        // Wait, OpenJPEG uses 28 bits?
        // OpenJPEG uses `c` as 32 bits.
        // `byte_out` output `c >> 19` (bits 19..27 approx).
        // If `c` overflows 27 bits, it carries into 28.

        // Standard says:
        // 1. If B == 0xFF:
        //    Write (B). ct=7.
        //    (Actually we wrote B already, we just check overflow)

        // Let's implement the buffering logic:
        // We accumulate in `C`. When `ct` triggers `byte_out`:
        // We have 8 (or 7) bits ready in the high part of `C`.
        // We verify if `C` has carried into `B`.

        // At start (new): B=0?
        // First byte is not written until second byte_out?

        // Let's follow OpenJPEG `opj_mqc_byteout_macro`.
        // `if (*bp == 0xff) { bp++; *bp = c >> 20; c &= 0xfffff; ct = 7; } else ...`
        // OpenJPEG uses `c` shifted by 20?
        // My previous code used 19.

        // Standard C.2.3 BYTEOUT:
        // T = (C >> 19) & 0xFF? Or just C >> 19?
        // "Transfer the MSBs of C to B".

        // Let's stick to C.2.3 strict interpretation.
        // C is 28 bits (0..27).
        // A is added to C at bit 16?
        // If A=0x8000 (16 bits), we add at bit 0 of A.
        // Wait, C is 1.5 + 0.5?

        // Let's use the implementation that mirrors OpenJPEG which is proven.
        // OpenJPEG state: c (32), a (32), ct (32), bp (ptr).
        // Init: c=0, a=0x8000, ct=12, bp=start-1.

        // ByteOut:
        // if (*bp == 0xff) {
        //    *++bp = c >> 20;
        //    c &= 0xfffff;
        //    ct = 7;
        // } else {
        //    if ((c >> 27) == 1) { // Carry!
        //       *bp += 1; // Propagate carry to B
        //       if (*bp == 0xff) { // B became FF
        //           // Stuffing needed
        //           *++bp = c >> 20;
        //           c &= 0xfffff;
        //           ct = 7;
        //       } else {
        //           *++bp = c >> 19;
        //           c &= 0x7ffff;
        //           ct = 8;
        //       }
        //    } else {
        //       *++bp = c >> 19;
        //       c &= 0x7ffff;
        //       ct = 8;
        //    }
        // }

        // My struct has `b` which acts as `*bp` (the LAST written byte).
        // Since we push to Vec, we can modify the last element if we need to propagate carry.

        if self.bp.is_empty() {
            // First time called (after init ct=12)
            // Just output. Carry is impossible?
            let b = (self.c >> 19) as u8;
            self.c &= 0x7FFFF;
            self.ct = 8;
            self.bp.push(b);
            // self.b = b; // Not needed, we inspect bp.last()
            return;
        }

        // Inspect B (last byte written)
        let last_byte = *self.bp.last().unwrap();

        if last_byte == 0xFF {
            // Previous byte was 0xFF.
            // We output 7 bits (from 20..26).
            // Bit 27 is Spacer/Carry?
            let b = (self.c >> 20) as u8;
            self.c &= 0xFFFFF;
            self.ct = 7;
            self.bp.push(b);
        } else {
            // Previous byte < 0xFF.
            // Check for carry (Bit 27)
            if (self.c >> 27) & 1 == 1 {
                // Carry occurred!
                // Add to last byte
                let len = self.bp.len();
                self.bp[len - 1] += 1;
                let new_last = self.bp[len - 1];

                // If it became 0xFF, we must stuff
                if new_last == 0xFF {
                    let b = (self.c >> 20) as u8;
                    self.c &= 0xFFFFF;
                    self.ct = 7;
                    self.bp.push(b);
                } else {
                    let b = (self.c >> 19) as u8;
                    self.c &= 0x7FFFF;
                    self.ct = 8;
                    self.bp.push(b);
                }
            } else {
                // No carry
                let b = (self.c >> 19) as u8;
                self.c &= 0x7FFFF;
                self.ct = 8;
                self.bp.push(b);
            }
        }
    }

    /// Flush the encoder - must be called after encoding to finalize the bitstream
    /// Per JPEG2000 spec C.2.9
    pub fn flush(&mut self) {
        // SETBITS
        let temp = self.c + self.a as u32;
        self.c |= 0xFFFF;
        if self.c >= temp {
            self.c -= 0x8000;
        }

        // Output remaining bits
        let remaining = self.ct;
        self.c <<= remaining;
        self.byte_out();

        // C.2.9 says:
        // C <<= CT; BYTEOUT();
        // C <<= CT; BYTEOUT();
        // Discard any 0xFF at end.

        // We already did shift in loop? No.
        // byte_out consumes bits from C high.
        // We just shifted remaining bits up.

        // We might need another flush byte to clear buffer?
        // Typically length is sufficient.
        // Standard says "shifted ... until all bits are output".

        // OpenJPEG does:
        // opj_mqc_setbits_macro
        // c <<= ct; byteout();
        // c <<= ct; byteout();
        // if (bp != start) bp--; // Backtrack one?

        self.c <<= self.ct; // ct set by byte_out
        self.byte_out();

        // Remove trailing 0xFF if present (per C.2.9)
        // "The byte pointed to by BP is discarded if it is 0xFF."
        if let Some(&last) = self.bp.last() {
            if last == 0xFF {
                self.bp.pop();
            }
        }
    }

    pub fn get_buffer(&self) -> &[u8] {
        // Some implementations skip the very first byte if it was a dummy?
        // My implementation pushes first byte immediately.
        // OpenJPEG initializes `bp = start - 1`. First byte_out writes to `start`.
        // So first byte IS valid.

        // But wait, in Init: ct=12.
        // First byte_out happens after 12 shifts.
        // Is that byte significant?
        // C=0 initially. 12 shifts -> C=0.
        // byte_out writes 0.
        // Is the first 0 byte part of stream?
        // Yes, C.2.5 InitEnc: "The first byte ... is 0".
        // But do we transmit it?
        // "The first byte of the codestream ... is the byte pointed to by BP".
        // So yes.
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
}

//! MQ Arithmetic Coder (ISO/IEC 15444-1 Annex C)
//! Exactly matching OpenJPEG implementation (mqc.c)

const MQ_TABLE: [MqState; 47] = [
    MqState {
        qe: 0x5601,
        nmps: 1,
        nlps: 1,
        switch: 1,
    },
    MqState {
        qe: 0x3401,
        nmps: 2,
        nlps: 6,
        switch: 0,
    },
    MqState {
        qe: 0x1801,
        nmps: 3,
        nlps: 9,
        switch: 0,
    },
    MqState {
        qe: 0x0ac1,
        nmps: 4,
        nlps: 12,
        switch: 0,
    },
    MqState {
        qe: 0x0521,
        nmps: 5,
        nlps: 29,
        switch: 0,
    },
    MqState {
        qe: 0x0221,
        nmps: 38,
        nlps: 33,
        switch: 0,
    },
    MqState {
        qe: 0x5601,
        nmps: 7,
        nlps: 6,
        switch: 1,
    },
    MqState {
        qe: 0x5401,
        nmps: 8,
        nlps: 14,
        switch: 0,
    },
    MqState {
        qe: 0x4801,
        nmps: 9,
        nlps: 14,
        switch: 0,
    },
    MqState {
        qe: 0x3801,
        nmps: 10,
        nlps: 14,
        switch: 0,
    },
    MqState {
        qe: 0x3001,
        nmps: 11,
        nlps: 17,
        switch: 0,
    },
    MqState {
        qe: 0x2401,
        nmps: 12,
        nlps: 18,
        switch: 0,
    },
    MqState {
        qe: 0x1c01,
        nmps: 13,
        nlps: 20,
        switch: 0,
    },
    MqState {
        qe: 0x1601,
        nmps: 29,
        nlps: 21,
        switch: 0,
    },
    MqState {
        qe: 0x5601,
        nmps: 15,
        nlps: 14,
        switch: 1,
    },
    MqState {
        qe: 0x5401,
        nmps: 16,
        nlps: 14,
        switch: 0,
    },
    MqState {
        qe: 0x5101,
        nmps: 17,
        nlps: 15,
        switch: 0,
    },
    MqState {
        qe: 0x4801,
        nmps: 18,
        nlps: 16,
        switch: 0,
    },
    MqState {
        qe: 0x3801,
        nmps: 19,
        nlps: 17,
        switch: 0,
    },
    MqState {
        qe: 0x3401,
        nmps: 20,
        nlps: 18,
        switch: 0,
    },
    MqState {
        qe: 0x3001,
        nmps: 21,
        nlps: 19,
        switch: 0,
    },
    MqState {
        qe: 0x2801,
        nmps: 22,
        nlps: 19,
        switch: 0,
    },
    MqState {
        qe: 0x2401,
        nmps: 23,
        nlps: 20,
        switch: 0,
    },
    MqState {
        qe: 0x2201,
        nmps: 24,
        nlps: 21,
        switch: 0,
    },
    MqState {
        qe: 0x1c01,
        nmps: 25,
        nlps: 22,
        switch: 0,
    },
    MqState {
        qe: 0x1801,
        nmps: 26,
        nlps: 23,
        switch: 0,
    },
    MqState {
        qe: 0x1601,
        nmps: 27,
        nlps: 24,
        switch: 0,
    },
    MqState {
        qe: 0x1401,
        nmps: 28,
        nlps: 25,
        switch: 0,
    },
    MqState {
        qe: 0x1201,
        nmps: 29,
        nlps: 26,
        switch: 0,
    },
    MqState {
        qe: 0x1101,
        nmps: 30,
        nlps: 27,
        switch: 0,
    },
    MqState {
        qe: 0x0ac1,
        nmps: 31,
        nlps: 28,
        switch: 0,
    },
    MqState {
        qe: 0x09c1,
        nmps: 32,
        nlps: 29,
        switch: 0,
    },
    MqState {
        qe: 0x08a1,
        nmps: 33,
        nlps: 30,
        switch: 0,
    },
    MqState {
        qe: 0x0521,
        nmps: 34,
        nlps: 31,
        switch: 0,
    },
    MqState {
        qe: 0x0441,
        nmps: 35,
        nlps: 32,
        switch: 0,
    },
    MqState {
        qe: 0x02a1,
        nmps: 36,
        nlps: 33,
        switch: 0,
    },
    MqState {
        qe: 0x0221,
        nmps: 37,
        nlps: 34,
        switch: 0,
    },
    MqState {
        qe: 0x0141,
        nmps: 38,
        nlps: 35,
        switch: 0,
    },
    MqState {
        qe: 0x0111,
        nmps: 39,
        nlps: 36,
        switch: 0,
    },
    MqState {
        qe: 0x0085,
        nmps: 40,
        nlps: 37,
        switch: 0,
    },
    MqState {
        qe: 0x0049,
        nmps: 41,
        nlps: 38,
        switch: 0,
    },
    MqState {
        qe: 0x0025,
        nmps: 42,
        nlps: 39,
        switch: 0,
    },
    MqState {
        qe: 0x0015,
        nmps: 43,
        nlps: 40,
        switch: 0,
    },
    MqState {
        qe: 0x0009,
        nmps: 44,
        nlps: 41,
        switch: 0,
    },
    MqState {
        qe: 0x0005,
        nmps: 45,
        nlps: 42,
        switch: 0,
    },
    MqState {
        qe: 0x0001,
        nmps: 45,
        nlps: 43,
        switch: 0,
    },
    MqState {
        qe: 0x5601,
        nmps: 46,
        nlps: 46,
        switch: 0,
    },
];

#[derive(Clone, Copy)]
struct MqState {
    pub qe: u16,
    pub nmps: u8,
    pub nlps: u8,
    pub switch: u8,
}

pub struct MqCoder {
    pub a: u32,
    pub c: u32,
    pub ct: i32,
    pub buffer: Vec<u8>,
    pub bp_idx: usize, // Index in buffer
    pub source: Vec<u8>,
    pub src_pos: usize,
    pub contexts: Vec<u8>,
    pub symbol_count: usize, // Debug: count encoded symbols
}

impl Default for MqCoder {
    fn default() -> Self {
        Self {
            a: 0,
            c: 0,
            ct: 0,
            buffer: Vec::new(),
            bp_idx: 0,
            source: Vec::new(),
            src_pos: 0,
            contexts: vec![0; 19],
            symbol_count: 0,
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

    // --- Encoder ---

    pub fn init_encoder(&mut self) {
        self.buffer.clear();
        self.buffer.push(0); // Dummy byte for bp[0] increment
        self.bp_idx = 0;
        self.a = 0x8000;
        self.c = 0;
        self.ct = 12; // Standard says 12, OpenJPEG uses 12
        self.symbol_count = 0;
    }

    pub fn encode(&mut self, d: u8, cx: usize) {
        self.symbol_count += 1;

        // Trace symbol encoding if MQ_SYMBOL_TRACE is set
        if std::env::var("MQ_SYMBOL_TRACE").is_ok() {
            eprintln!("[MQ] Symbol #{}: d={}, cx={}", self.symbol_count, d, cx);
        }

        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = MQ_TABLE[idx].qe as u32;

        self.a -= qe;
        if d == mps {
            if (self.a & 0x8000) == 0 {
                if self.a < qe {
                    self.a = qe;
                } else {
                    self.c += qe;
                }
                self.contexts[cx] = (MQ_TABLE[idx].nmps << 1) | mps;
                self.renorm_e();
            } else {
                self.c += qe;
            }
        } else {
            if self.a < qe {
                self.c += qe;
            } else {
                self.a = qe;
            }
            let switch = MQ_TABLE[idx].switch;
            let next_idx = MQ_TABLE[idx].nlps;
            let new_mps = if switch != 0 { mps ^ 1 } else { mps };
            self.contexts[cx] = (next_idx << 1) | new_mps;
            self.renorm_e();
        }
    }

    fn renorm_e(&mut self) {
        loop {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byte_out();
            }
            if self.a >= 0x8000 {
                break;
            }
        }
    }

    fn byte_out(&mut self) {
        // OpenJPEG-compatible byte_out implementation (ISO 15444-1 Annex C)
        // Key: Only clear carry bit when it actually propagates to create 0xFF
        if self.buffer[self.bp_idx] == 0xff {
            // Previous byte was 0xFF: bit-stuffing mode
            // Output 7 bits instead of 8 to avoid creating marker-like sequences
            self.bp_idx += 1;
            self.buffer.push((self.c >> 20) as u8);
            self.c &= 0xfffff;
            self.ct = 7;
        } else if (self.c & 0x8000000) == 0 {
            // No carry: normal output
            self.bp_idx += 1;
            self.buffer.push((self.c >> 19) as u8);
            self.c &= 0x7ffff;
            self.ct = 8;
        } else {
            // Carry bit is set: propagate carry to previous byte
            let v = self.buffer[self.bp_idx] + 1;
            self.buffer[self.bp_idx] = v;

            if v == 0xff {
                // Carry propagation created 0xFF: enter bit-stuffing mode
                self.c &= 0x7ffffff; // Clear carry bit ONLY when creating 0xFF
                self.bp_idx += 1;
                self.buffer.push((self.c >> 20) as u8);
                self.c &= 0xfffff;
                self.ct = 7;
            } else {
                // Normal carry propagation complete
                self.bp_idx += 1;
                self.buffer.push((self.c >> 19) as u8);
                self.c &= 0x7ffff;
                self.ct = 8;
            }
        }
    }

    pub fn flush(&mut self) {
        let temp_c = self.c + self.a;
        self.c |= 0xffff;
        if self.c >= temp_c {
            self.c -= 0x8000;
        }
        self.c <<= self.ct;
        self.byte_out();
        self.c <<= self.ct;
        self.byte_out();

        // OpenJPEG: if (*mqc->bp != 0xff) { mqc->bp++; }
        if self.bp_idx < self.buffer.len() && self.buffer[self.bp_idx] != 0xff {
            self.bp_idx += 1;
        }

        if std::env::var("MQ_DEBUG").is_ok() {
            eprintln!(
                "[MQ_FLUSH] symbols={}, bp_idx={}, buffer.len()={}, last_byte=0x{:02X}, result_len={}",
                self.symbol_count,
                self.bp_idx,
                self.buffer.len(),
                if self.bp_idx > 0 && self.bp_idx <= self.buffer.len() {
                    self.buffer[self.bp_idx - 1]
                } else {
                    0
                },
                self.get_buffer().len()
            );
        }
    }

    pub fn get_buffer(&self) -> &[u8] {
        // Return bytes from 1 (skip dummy) to bp_idx (exclusive)
        // OpenJPEG returns (bp - start) bytes
        if self.bp_idx > 0 {
            &self.buffer[1..self.bp_idx]
        } else {
            &[]
        }
    }

    // --- Decoder ---

    pub fn init_decoder(&mut self, data: &[u8]) {
        self.source = data.to_vec();
        // Append 0xff 0xff for safety
        self.source.push(0xff);
        self.source.push(0xff);

        self.src_pos = 0;
        self.a = 0x8000;
        if data.is_empty() {
            self.c = 0xff0000;
        } else {
            self.c = (self.source[0] as u32) << 16;
        }

        self.byte_in();
        self.c <<= 7;
        self.ct -= 7;
    }

    fn byte_in(&mut self) {
        if self.src_pos < self.source.len() {
            if self.source[self.src_pos] == 0xff {
                let next = if self.src_pos + 1 < self.source.len() {
                    self.source[self.src_pos + 1]
                } else {
                    0xff
                };
                if next > 0x8f {
                    self.c += 0xff00;
                    self.ct = 8;
                } else {
                    self.src_pos += 1;
                    self.c += (self.source[self.src_pos] as u32) << 9;
                    self.ct = 7;
                }
            } else {
                self.src_pos += 1;
                self.c += (self.source[self.src_pos] as u32) << 8;
                self.ct = 8;
            }
        } else {
            self.c += 0xff00;
            self.ct = 8;
        }
    }

    pub fn decode_bit(&mut self, cx: usize) -> u8 {
        let ctx = self.contexts[cx];
        let idx = (ctx >> 1) as usize;
        let mps = ctx & 1;
        let qe = MQ_TABLE[idx].qe as u32;

        self.a -= qe;
        let d = if (self.c >> 16) < qe {
            let d = self.lps_exchange(cx, idx, mps);
            self.renorm_d();
            d
        } else {
            self.c -= qe << 16;
            if self.a < 0x8000 {
                let d = self.mps_exchange(cx, idx, mps);
                self.renorm_d();
                d
            } else {
                mps
            }
        };

        if std::env::var("MQ_SYMBOL_TRACE").is_ok() {
            self.symbol_count += 1;
            eprintln!("[MQ] Symbol #{}: d={}, cx={}", self.symbol_count, d, cx);
        }

        d
    }

    fn mps_exchange(&mut self, cx: usize, idx: usize, mps: u8) -> u8 {
        let qe = MQ_TABLE[idx].qe as u32;
        if self.a < qe {
            let next_idx = MQ_TABLE[idx].nlps;
            let switch = MQ_TABLE[idx].switch;
            let new_mps = if switch != 0 { mps ^ 1 } else { mps };
            self.contexts[cx] = (next_idx << 1) | new_mps;
            mps ^ 1
        } else {
            let next_idx = MQ_TABLE[idx].nmps;
            self.contexts[cx] = (next_idx << 1) | mps;
            mps
        }
    }

    fn lps_exchange(&mut self, cx: usize, idx: usize, mps: u8) -> u8 {
        let qe = MQ_TABLE[idx].qe as u32;
        if self.a < qe {
            self.a = qe;
            let next_idx = MQ_TABLE[idx].nmps;
            self.contexts[cx] = (next_idx << 1) | mps;
            mps
        } else {
            self.a = qe;
            let next_idx = MQ_TABLE[idx].nlps;
            let switch = MQ_TABLE[idx].switch;
            let new_mps = if switch != 0 { mps ^ 1 } else { mps };
            self.contexts[cx] = (next_idx << 1) | new_mps;
            mps ^ 1
        }
    }

    fn renorm_d(&mut self) {
        loop {
            if self.ct == 0 {
                self.byte_in();
            }
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.a >= 0x8000 {
                break;
            }
        }
    }
}

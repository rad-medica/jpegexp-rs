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
    MqContextState { qe: 0x5601, nmps: 1, nlps: 1, switch: 1 },
    MqContextState { qe: 0x3401, nmps: 2, nlps: 6, switch: 0 },
    MqContextState { qe: 0x1801, nmps: 3, nlps: 9, switch: 0 },
    MqContextState { qe: 0x0AC1, nmps: 4, nlps: 12, switch: 0 },
    MqContextState { qe: 0x0521, nmps: 5, nlps: 29, switch: 0 },
    MqContextState { qe: 0x0221, nmps: 38, nlps: 33, switch: 0 },
    MqContextState { qe: 0x5601, nmps: 7, nlps: 6, switch: 1 },
    MqContextState { qe: 0x5401, nmps: 8, nlps: 14, switch: 0 },
    MqContextState { qe: 0x4801, nmps: 9, nlps: 14, switch: 0 },
    MqContextState { qe: 0x3801, nmps: 10, nlps: 14, switch: 0 },
    MqContextState { qe: 0x3001, nmps: 11, nlps: 17, switch: 0 },
    MqContextState { qe: 0x2401, nmps: 12, nlps: 18, switch: 0 },
    MqContextState { qe: 0x1C01, nmps: 13, nlps: 20, switch: 0 },
    MqContextState { qe: 0x1601, nmps: 29, nlps: 21, switch: 0 },
    MqContextState { qe: 0x5601, nmps: 15, nlps: 14, switch: 1 },
    MqContextState { qe: 0x5401, nmps: 16, nlps: 14, switch: 0 },
    MqContextState { qe: 0x5101, nmps: 17, nlps: 15, switch: 0 },
    MqContextState { qe: 0x4801, nmps: 18, nlps: 16, switch: 0 },
    MqContextState { qe: 0x3801, nmps: 19, nlps: 17, switch: 0 },
    MqContextState { qe: 0x3401, nmps: 20, nlps: 18, switch: 0 },
    MqContextState { qe: 0x3001, nmps: 21, nlps: 19, switch: 0 },
    MqContextState { qe: 0x2801, nmps: 22, nlps: 19, switch: 0 },
    MqContextState { qe: 0x2401, nmps: 23, nlps: 19, switch: 0 },
    MqContextState { qe: 0x2201, nmps: 24, nlps: 19, switch: 0 },
    MqContextState { qe: 0x1C01, nmps: 25, nlps: 20, switch: 0 },
    MqContextState { qe: 0x1801, nmps: 26, nlps: 21, switch: 0 },
    MqContextState { qe: 0x1601, nmps: 27, nlps: 22, switch: 0 },
    MqContextState { qe: 0x1401, nmps: 28, nlps: 23, switch: 0 },
    MqContextState { qe: 0x1201, nmps: 29, nlps: 24, switch: 0 },
    MqContextState { qe: 0x1101, nmps: 30, nlps: 25, switch: 0 },
    MqContextState { qe: 0x0AC1, nmps: 31, nlps: 26, switch: 0 },
    MqContextState { qe: 0x09C1, nmps: 32, nlps: 27, switch: 0 },
    MqContextState { qe: 0x08A1, nmps: 33, nlps: 28, switch: 0 },
    MqContextState { qe: 0x0521, nmps: 34, nlps: 29, switch: 0 },
    MqContextState { qe: 0x0441, nmps: 35, nlps: 30, switch: 0 },
    MqContextState { qe: 0x02A1, nmps: 36, nlps: 31, switch: 0 },
    MqContextState { qe: 0x0221, nmps: 37, nlps: 32, switch: 0 },
    MqContextState { qe: 0x0141, nmps: 38, nlps: 33, switch: 0 },
    MqContextState { qe: 0x0111, nmps: 39, nlps: 34, switch: 0 },
    MqContextState { qe: 0x0085, nmps: 40, nlps: 35, switch: 0 },
    MqContextState { qe: 0x0049, nmps: 41, nlps: 36, switch: 0 },
    MqContextState { qe: 0x0025, nmps: 42, nlps: 37, switch: 0 },
    MqContextState { qe: 0x0015, nmps: 43, nlps: 38, switch: 0 },
    MqContextState { qe: 0x0009, nmps: 44, nlps: 39, switch: 0 },
    MqContextState { qe: 0x0005, nmps: 45, nlps: 40, switch: 0 },
    MqContextState { qe: 0x0001, nmps: 45, nlps: 41, switch: 0 },
    MqContextState { qe: 0x5601, nmps: 46, nlps: 46, switch: 0 },
];

pub struct MqCoder {
    // Registers
    a: u16, // Interval size (16 bits)
    c: u32, // Code register (28 bits essentially)
    
    // Buffer
    bp: Vec<u8>,
    bp_idx: usize,
    
    // State
    ct: u8, // bit counter needed for next byte output
    b: u8, // byte buffer
    
    // Contexts (I_k)
    // each index maps to a state in MQ_TABLE (0..46)
    // and MPS value (0 or 1)
    // structure: (index << 1) | mps
    contexts: Vec<u8>, 
}

impl MqCoder {
    pub fn new() -> Self {
        Self {
            a: 0x8000,
            c: 0,
            bp: Vec::new(),
            bp_idx: 0,
            ct: 12,
            b: 0,
            contexts: vec![0; 19], // Standard 19 contexts? (Usually J2K uses 19 for LL)
        }
    }
    
    pub fn init_contexts(&mut self, size: usize) {
        self.contexts = vec![0; size];
    }

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
            if self.a >= 0x8000 { break; }
        }
    }

    fn byte_out(&mut self) {
        if self.bp_idx == 0 {
            // First byte logic if needed, usually init handles it
        }
        
        // This logic is complex in spec C.3.2
        // Simplified placeholder:
        let b_out = (self.c >> 19) as u8;
        if b_out == 0xFF {
            self.ct = 7;
        }
        self.c &= 0x7FFFF; // keep lower bits
        self.bp.push(b_out);
        self.bp_idx += 1;
    }
    
    // Note: Decoder logic is symmetric but requires buffer read.
    pub fn decode(&mut self, cx: usize) -> u8 {
        // Placeholder for decoder
        // D = MPS or LPS based on interval check
        0
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
        let original_c = mq.c;
        mq.encode(0, cx); // MPS (0 is default MPS for 0 index table?)
        
        // After encoding, A should be renormalized to >= 0x8000
        assert!(mq.a >= 0x8000);
        // C might change or BP might get data
        // For the first symbol (MPS of 0.56 probability), A shrinks, then renormalizes.
        // C might not change much if MPS is at bottom of interval, but let's just check no panic.
    }
}

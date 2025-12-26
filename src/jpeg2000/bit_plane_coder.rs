use super::mq_coder::MqCoder;

pub struct BitPlaneCoder<'a> {
    pub width: u32,
    pub height: u32,
    pub data: &'a [i32], // Quantized coefficients
    
    // State: Significance, Visited, Refinement
    // Bitmasks per sample? Or separate arrays?
    // Standard usually uses a state array.
    // Bit 0: Sig, Bit 1: Visited, Bit 2: Refined...
    pub state: Vec<u8>, 
    
    pub mq: MqCoder,
}

impl<'a> BitPlaneCoder<'a> {
    pub fn new(width: u32, height: u32, data: &'a [i32]) -> Self {
        let size = (width * height) as usize;
        let mut mq = MqCoder::new();
        mq.init_contexts(19); // 19 contexts for LL subband usually
        
        Self {
            width,
            height,
            data,
            state: vec![0; size],
            mq,
        }
    }
    
    // State Bit Definitions
    const SIG: u8 = 1 << 0;
    const VISITED: u8 = 1 << 1;
    const REFINE: u8 = 1 << 2;
    const SIGN: u8 = 1 << 3; // 0=pos, 1=neg

    // Zero Coding Tables (LL, HL, LH, HH) - Simplified for LL (Index 0)
    // Actually J2K has specific tables based on band.
    // Let's assume LL for simplicity for now or implement full logic.
    // Table C-1: Contexts for SigProp (ZC)
    // We need logic to map neighbor counts (H, V, D) to Context (0-8)
    
    pub fn get_neighbors(&self, x: u32, y: u32) -> (u8, u8, u8) {
        // Returns count of significant neighbors (H, V, D)
        let w = self.width as i32;
        let h = self.height as i32;
        let ix = x as i32;
        let iy = y as i32;
        
        let mut h_cnt = 0;
        let mut v_cnt = 0;
        let mut d_cnt = 0;
        
        let idx = |cnx, cny| (cny * w + cnx) as usize;
        
        // H: (x-1, y), (x+1, y)
        if ix > 0 && (self.state[idx(ix-1, iy)] & Self::SIG) != 0 { h_cnt += 1; }
        if ix < w-1 && (self.state[idx(ix+1, iy)] & Self::SIG) != 0 { h_cnt += 1; }
        
        // V: (x, y-1), (x, y+1)
        if iy > 0 && (self.state[idx(ix, iy-1)] & Self::SIG) != 0 { v_cnt += 1; }
        if iy < h-1 && (self.state[idx(ix, iy+1)] & Self::SIG) != 0 { v_cnt += 1; }
        
        // D: Diagonals
        if ix > 0 && iy > 0 && (self.state[idx(ix-1, iy-1)] & Self::SIG) != 0 { d_cnt += 1; }
        if ix < w-1 && iy > 0 && (self.state[idx(ix+1, iy-1)] & Self::SIG) != 0 { d_cnt += 1; }
        if ix > 0 && iy < h-1 && (self.state[idx(ix-1, iy+1)] & Self::SIG) != 0 { d_cnt += 1; }
        if ix < w-1 && iy < h-1 && (self.state[idx(ix+1, iy+1)] & Self::SIG) != 0 { d_cnt += 1; }
        
        (h_cnt, v_cnt, d_cnt)
    }

    fn get_zc_context(&self, band: u8, h: u8, v: u8, d: u8) -> usize {
        // Simplified Table C-8 for LL band (band=0)
        // (This is huge, usually table driven. Using minimal logic for flow)
        // Returns context label 0..8
        match band {
            0 | 1 => { // LL and LH
                match (h, v, d) {
                    (2, _, _) => 8,
                    (1, v, _) if v >= 1 => 7,
                    (1, 0, d) if d >= 1 => 6,
                    (1, 0, 0) => 5,
                    (0, 2, _) => 4,
                    (0, 1, _) => 3,
                    (0, 0, d) if d >= 2 => 2,
                    (0, 0, 1) => 1,
                    _ => 0,
                }
            },
            // HL follows LH derived (transpose H/V), HH is separate.
            // Placeholder:
            _ => 0 
        }
    }
    
    pub fn encode_codeblock(&mut self) {
        // Iterate bitplanes from MSB to LSB
        // Assume 30 down to 0? Or determined by max value.
        // For testing, let's start at bit 5.
        for bp in (0..5).rev() {
             self.significance_propagation(bp);
             self.magnitude_refinement(bp);
             self.cleanup(bp);
        }
    }
    
    fn significance_propagation(&mut self, bit_plane: u8) {
        // Iterate scan order (simple raster for simplicity, J2K stripes 4 rows)
        // Correct J2K is stripe order: 4 rows column-wise.
        let w = self.width;
        let h = self.height;
        
        for y in 0..h {
            for x in 0..w {
                 let idx = (y * w + x) as usize;
                 let state = self.state[idx];
                 
                 // If insignificant and not visited
                 if (state & (Self::SIG | Self::VISITED)) == 0 {
                     let (hc, vc, dc) = self.get_neighbors(x, y);
                     if hc > 0 || vc > 0 || dc > 0 {
                         // Propagate Importance
                         let val = self.data[idx];
                         let bit = (val.abs() >> bit_plane) & 1;
                         
                         // Encode ZC
                         let cx = self.get_zc_context(0, hc as u8, vc as u8, dc as u8); // band 0 assumed
                         self.mq.encode(bit as u8, cx);
                         
                         if bit == 1 {
                             // Became Significant: Update State
                             let sign = if val < 0 { 1 } else { 0 };
                             self.state[idx] |= Self::SIG | Self::VISITED;
                             if sign == 1 { self.state[idx] |= Self::SIGN; }
                             
                             // Encode Sign (SC)
                             // Context depends on neighbor signs. Simplified:
                             let sc_ctx = 9; // Sign Context Base
                             self.mq.encode(sign as u8, sc_ctx); // Wrong context logic but placeholder
                         } else {
                             // Visited but not significant
                             self.state[idx] |= Self::VISITED;
                         }
                     }
                 }
            }
        }
    }

    fn magnitude_refinement(&mut self, bit_plane: u8) {
         let w = self.width;
         let h = self.height;
         for i in 0..(w*h) as usize {
             let state = self.state[i];
             // If already significant and NOT visited in SigProp (i.e., became sig in prev bitplane)
             if (state & Self::SIG) != 0 && (state & Self::VISITED) == 0 {
                  self.state[i] |= Self::VISITED; // Mark visited for this bitplane
                  let val = self.data[i];
                  let bit = (val.abs() >> bit_plane) & 1;
                  
                  // MR Context
                  let mr_ctx = 14; 
                  // Refinement logic: uses Neighbors+RefineBit state.
                  self.mq.encode(bit as u8, mr_ctx);
                  self.state[i] |= Self::REFINE; // First refinement done
             }
         }
    }
    
    fn cleanup(&mut self, bit_plane: u8) {
        // Encode remaining insignificant samples
        let w = self.width;
        let h = self.height;
        
        // RLC logic would go here: check if run of 4 is all insignificant
        
        for y in 0..h {
            for x in 0..w {
                 let idx = (y * w + x) as usize;
                 let state = self.state[idx];
                 if (state & Self::VISITED) == 0 {
                     // Not visited: Must be insignificant so far
                     let (hc, vc, dc) = self.get_neighbors(x, y);
                     
                     // ZC Context
                     let cx = self.get_zc_context(0, hc as u8, vc as u8, dc as u8);
                     let val = self.data[idx];
                     let bit = (val.abs() >> bit_plane) & 1;
                     
                     self.mq.encode(bit as u8, cx);
                     
                     if bit == 1 {
                          // Became Significant
                          let sign = if val < 0 { 1 } else { 0 };
                          self.state[idx] |= Self::SIG;
                          if sign == 1 { self.state[idx] |= Self::SIGN; }
                          
                          let sc_ctx = 9; 
                          self.mq.encode(sign as u8, sc_ctx);
                     }
                 }
                 // Reset VISITED for next bitplane
                 self.state[idx] &= !Self::VISITED;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_plane_coding_simple() {
        // 4x4 block
        let data = [
             10, 0, 0, 0,
              0, 5, 0, 0,
              0, 0, -3, 0,
              0, 0, 0, 1
        ];
        let mut bpc = BitPlaneCoder::new(4, 4, &data);
        
        // This should not panic
        bpc.encode_codeblock();
        
        // We can't easily verify exact bytes without full J2K compliance check,
        // but we can check that state updated (e.g., significant samples marked)
        
        // At end, indices 0, 5, 10, 15 should be SIGNIFICANT
        let sig = BitPlaneCoder::SIG;
        assert_eq!(bpc.state[0] & sig, sig, "Index 0 should be significant");
        assert_eq!(bpc.state[5] & sig, sig, "Index 5 should be significant");
        // -3 is abs 3. Max bit plane was 5. 3 is binary 011. It should eventually become significant.
        // wait, we ran loop 5..0. 0..5 rev. 
        // 3 is 000011.
        // Bit 1: 3>>1 & 1 = 1. Yes.
        assert_eq!(bpc.state[10] & sig, sig, "Index 10 (-3) should be significant");
    }
}

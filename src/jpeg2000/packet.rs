use super::bit_io::{BitIoError, J2kBitReader};
use super::tag_tree::TagTree;

pub struct SubbandState {
    pub grid_width: usize,
    pub grid_height: usize,
    pub inclusion_tree: TagTree,
    pub zero_bp_tree: TagTree,
    /// Whether a code-block has been included by any previous layer (per leaf).
    pub included: Vec<bool>,
    /// Lblock value per code-block (per leaf). Starts at 3 per spec.
    pub lblock: Vec<u8>,
}

impl SubbandState {
    pub fn new(w: usize, h: usize) -> Self {
        let count = w.saturating_mul(h);
        Self {
            grid_width: w,
            grid_height: h,
            inclusion_tree: TagTree::new(w, h),
            zero_bp_tree: TagTree::new(w, h),
            included: vec![false; count],
            lblock: vec![3u8; count],
        }
    }

    pub fn reset(&mut self) {
        self.inclusion_tree.reset();
        self.zero_bp_tree.reset();
        self.included.fill(false);
        self.lblock.fill(3);
    }
}

/// Represents the state of a Precinct during parsing.
pub struct PrecinctState {
    /// Trees for each subband (resolution 0 has 1, others have 3)
    pub subbands: Vec<SubbandState>,
}

impl PrecinctState {
    pub fn new() -> Self {
        Self { subbands: Vec::new() }
    }

    pub fn reset(&mut self) {
        for sb in &mut self.subbands {
            sb.reset();
        }
    }
}

pub struct PacketHeader {
    pub packet_seq_num: u32,
    pub empty: bool,
    pub layer_index: u32,
    pub included_cblks: Vec<CodeBlockInfo>,
}

#[derive(Debug, Clone)]
pub struct CodeBlockInfo {
    pub x: usize,
    pub y: usize,
    pub subband_index: u8,
    pub included: bool,
    pub num_passes: u8,
    pub data_len: u32,
    pub zero_bp: u8,
}

impl PacketHeader {
    /// Read a packet header from the bit stream.
    pub fn read(
        reader: &mut J2kBitReader<'_, '_>,
        state: &mut PrecinctState,
        layer: u32,
        subband_grids: &[(usize, usize)],
    ) -> Result<Self, BitIoError> {
        let mut header = PacketHeader {
            packet_seq_num: 0,
            empty: false,
            layer_index: layer,
            included_cblks: Vec::new(),
        };

        // 1. Zero-length packet bit
        let bit = reader.read_bit()?;
        if bit == 0 {
            header.empty = true;
            return Ok(header);
        }

        // 2. Code-block inclusion and header info
        for (s, &(grid_width, grid_height)) in subband_grids.iter().enumerate() {
            if state.subbands.len() <= s {
                state
                    .subbands
                    .resize_with(s + 1, || SubbandState::new(grid_width, grid_height));
            }
            // If dimensions changed (can happen across subbands), reset state to avoid desync.
            if state.subbands[s].grid_width != grid_width || state.subbands[s].grid_height != grid_height
            {
                state.subbands[s] = SubbandState::new(grid_width, grid_height);
            }

            let subband_state = &mut state.subbands[s];

            for y in 0..grid_height {
                for x in 0..grid_width {
                    let leaf_idx = y * grid_width + x;
                    let was_included = subband_state
                        .included
                        .get(leaf_idx)
                        .copied()
                        .unwrap_or(false);

                    // Determine inclusion
                    let mut process_block = false;
                    if was_included {
                        // Already included in a previous layer: a single bit indicates whether
                        // this code-block contributes (has new passes) in the current layer.
                        if reader.read_bit()? == 1 {
                            process_block = true;
                        }
                    } else {
                        let threshold = (layer + 1) as i32;
                        let not_included_yet = subband_state
                            .inclusion_tree
                            .decode(reader, x, y, threshold)?;
                        if !not_included_yet {
                            process_block = true;
                            if let Some(slot) = subband_state.included.get_mut(leaf_idx) {
                                *slot = true;
                            }
                        }
                    }

                    if process_block {
                        /*
                        eprintln!(
                            "DEBUG: LBlock val={}, reading {} bits. New LBlock={}",
                            lbits - 3, lbits,
                            state.lblock[s][cb_idx]
                        );
                        */
                        /*
                        eprintln!("DEBUG: LBlock val={}, reading {} bits", lbits - 3, lbits);
                        */
                        /*
                        eprintln!(
                            "DEBUG: CB {},{} included in subband {}",
                            x, y, s
                        );
                        */

                        // Decode Zero Bit Planes
                        // Only present if this is the first time included
                        if !was_included {
                            subband_state.zero_bp_tree.decode(reader, x, y, 128)?;
                        }
                        let zero_bp = subband_state.zero_bp_tree.get_current_value(x, y) as u8;

                        // Decode Number of Passes
                        let num_passes = Self::read_coding_passes(reader)?;

                        // Lblock + segment length (ISO/IEC 15444-1, packet header syntax).
                        //
                        // Lblock starts at 3 and is incremented by reading 1-bits until a 0-bit.
                        // Then, the segment length is coded in:
                        //   Lblock + floor(log2(num_passes)) bits.
                        let mut lblock_val = subband_state.lblock.get(leaf_idx).copied().unwrap_or(3);
                        while reader.read_bit()? == 1 {
                            lblock_val = lblock_val.saturating_add(1);
                        }
                        if let Some(slot) = subband_state.lblock.get_mut(leaf_idx) {
                            *slot = lblock_val;
                        }
                        // Spec uses ceil(log2(num_passes)) for the length information bit-width.
                        // Using floor(log2) under-reads the segment length field for non-powers of two,
                        // desynchronizing packet header/body parsing.
                        let k = if num_passes <= 1 {
                            0
                        } else {
                            let v = num_passes.saturating_sub(1);
                            (7 - v.leading_zeros() as u8).saturating_add(1)
                        };
                        let lbits = lblock_val.saturating_add(k);
                        let data_len = reader.read_bits(lbits)?;

                        header.included_cblks.push(CodeBlockInfo {
                            x,
                            y,
                            subband_index: s as u8,
                            included: true,
                            num_passes,
                            data_len,
                            zero_bp,
                        });
                    }
                }
            }
        }

        Ok(header)
    }

    /// Reads the number of coding passes using J2K codeword table (Table B.4).
    fn read_coding_passes(reader: &mut J2kBitReader<'_, '_>) -> Result<u8, BitIoError> {
        if reader.read_bit()? == 0 {
            // eprintln!("DEBUG: passes codework 0 -> 1");
            return Ok(1);
        }
        if reader.read_bit()? == 0 {
            // eprintln!("DEBUG: passes codework 10 -> 2");
            return Ok(2);
        }
        let bits = reader.read_bits(2)?;
        if bits < 3 {
            // eprintln!("DEBUG: passes codeword 11{} -> {}", bits, 3 + bits);
            return Ok((3 + bits) as u8);
        }
        let bits = reader.read_bits(5)?;
        if bits < 31 {
            // eprintln!("DEBUG: passes codeword 1111{} -> {}", bits, 6 + bits);
            return Ok((6 + bits) as u8);
        }
        // Extension: 32 + 5 bits... (Very rare for typical images)
        let bits2 = reader.read_bits(5)?;
        // eprintln!("DEBUG: passes codeword extension -> {}", 37 + bits2);
        Ok((37 + bits2) as u8)
    }

    /// Write a packet header to the bit stream.
    pub fn write(
        &self,
        writer: &mut crate::jpeg2000::bit_io::J2kBitWriter,
        state: &mut PrecinctState,
        subband_grids: &[(usize, usize)],
    ) {
        if self.empty {
            writer.write_bit(0);
            return;
        }
        writer.write_bit(1);

        for (s, &(grid_width, grid_height)) in subband_grids.iter().enumerate() {
            if state.subbands.len() <= s {
                state
                    .subbands
                    .resize_with(s + 1, || SubbandState::new(grid_width, grid_height));
            }
            if state.subbands[s].grid_width != grid_width || state.subbands[s].grid_height != grid_height
            {
                state.subbands[s] = SubbandState::new(grid_width, grid_height);
            }
            let subband_state = &mut state.subbands[s];

            for y in 0..grid_height {
                for x in 0..grid_width {
                    let leaf_idx = y * grid_width + x;
                    let cb_info = self
                        .included_cblks
                        .iter()
                        .find(|c| c.x == x && c.y == y && c.subband_index == s as u8);

                    let included_now = cb_info.is_some() && cb_info.unwrap().included;
                    let was_included = subband_state
                        .included
                        .get(leaf_idx)
                        .copied()
                        .unwrap_or(false);

                    if was_included {
                        writer.write_bit(if included_now { 1 } else { 0 });
                    } else {
                        // Encode inclusion tag-tree for this layer threshold.
                        subband_state
                            .inclusion_tree
                            .encode(writer, x, y, (self.layer_index + 1) as i32);
                        if included_now {
                            if let Some(slot) = subband_state.included.get_mut(leaf_idx) {
                                *slot = true;
                            }
                        }
                    }

                    if included_now {
                        let cb = cb_info.unwrap();
                        if !was_included {
                            subband_state.zero_bp_tree.set_value(x, y, cb.zero_bp as i32);
                            subband_state.zero_bp_tree.encode(writer, x, y, 128);
                        }

                        let num_passes = cb.num_passes.max(1);
                        // Encode number of coding passes (inverse of read_coding_passes).
                        match num_passes {
                            1 => writer.write_bit(0),
                            2 => {
                                writer.write_bit(1);
                                writer.write_bit(0);
                            }
                            3 | 4 | 5 => {
                                writer.write_bit(1);
                                writer.write_bit(1);
                                writer.write_bits((num_passes - 3) as u32, 2);
                            }
                            _ => {
                                // Fallback: encode using the 1111 + 5-bit form (supports up to 36).
                                writer.write_bit(1);
                                writer.write_bit(1);
                                writer.write_bits(3, 2); // 11 + '11' => 1111 prefix
                                let v = (num_passes as i32 - 6).clamp(0, 31) as u32;
                                writer.write_bits(v, 5);
                            }
                        }

                        // Lblock increment bits (we keep lblock fixed in this stub path).
                        writer.write_bit(0);

                        let k = if num_passes <= 1 {
                            0
                        } else {
                            let v = num_passes.saturating_sub(1);
                            (7 - v.leading_zeros() as u8).saturating_add(1)
                        };
                        let lblock_val = subband_state.lblock.get(leaf_idx).copied().unwrap_or(3);
                        let lbits = lblock_val.saturating_add(k);
                        writer.write_bits(cb.data_len, lbits as u8);
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_read_empty() {
        let data = vec![0x00];
        let mut buf_reader = crate::jpeg_stream_reader::JpegStreamReader::new(&data);
        let mut reader = J2kBitReader::new(&mut buf_reader);
        let mut state = PrecinctState::new();

        let header = PacketHeader::read(&mut reader, &mut state, 0, &[(2, 2)]).unwrap();
        assert!(header.empty);
    }
}

use super::bit_io::{BitIoError, J2kBitReader};
use super::tag_tree::TagTree;

pub struct SubbandState {
    pub inclusion_tree: TagTree,
    pub zero_bp_tree: TagTree,
    pub lblock_tree: TagTree,
}

impl SubbandState {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            inclusion_tree: TagTree::new(w, h),
            zero_bp_tree: TagTree::new(w, h),
            lblock_tree: TagTree::new(w, h),
        }
    }

    pub fn reset(&mut self) {
        self.inclusion_tree.reset();
        self.zero_bp_tree.reset();
        self.lblock_tree.reset();
    }
}

/// Represents the state of a Precinct during parsing.
pub struct PrecinctState {
    /// Trees for each subband (resolution 0 has 1, others have 3)
    pub subbands: Vec<SubbandState>,
}

impl PrecinctState {
    pub fn new(_w: usize, _h: usize) -> Self {
        let subbands = Vec::with_capacity(3);
        Self { subbands }
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
        grid_width: usize,
        grid_height: usize,
        num_subbands: usize,
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
        for s in 0..num_subbands {
            if state.subbands.len() <= s {
                state
                    .subbands
                    .push(SubbandState::new(grid_width, grid_height));
            }
            let subband_state = &mut state.subbands[s];

            for y in 0..grid_height {
                for x in 0..grid_width {
                    // Determine inclusion
                    let threshold = (layer + 1) as i32;
                    let already_included =
                        subband_state.inclusion_tree.get_current_value(x, y) < threshold;

                    let mut process_block = false;
                    if already_included {
                        if reader.read_bit()? == 1 {
                            process_block = true;
                        }
                    } else {
                        let not_included_yet = subband_state
                            .inclusion_tree
                            .decode(reader, x, y, threshold)?;
                        if !not_included_yet {
                            process_block = true;
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
                        if !already_included {
                            subband_state.zero_bp_tree.decode(reader, x, y, 128)?;
                        }
                        let zero_bp = subband_state.zero_bp_tree.get_current_value(x, y) as u8;

                        // Decode Number of Passes
                        let num_passes = Self::read_coding_passes(reader)?;

                        // Data Length
                        // Decode LBlock parameter with arbitrary threshold (32)
                        let _ = subband_state.lblock_tree.decode(reader, x, y, 32)?;
                        let lbits = subband_state.lblock_tree.get_current_value(x, y) + 3;

                        let data_len = reader.read_bits(lbits as u8)?;

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
        grid_width: usize,
        grid_height: usize,
        num_subbands: usize,
    ) {
        if self.empty {
            writer.write_bit(0);
            return;
        }
        writer.write_bit(1);

        for s in 0..num_subbands {
            if state.subbands.len() <= s {
                state
                    .subbands
                    .push(SubbandState::new(grid_width, grid_height));
            }
            let subband_state = &mut state.subbands[s];

            for y in 0..grid_height {
                for x in 0..grid_width {
                    let cb_info = self
                        .included_cblks
                        .iter()
                        .find(|c| c.x == x && c.y == y && c.subband_index == s as u8);

                    let included_now = cb_info.is_some() && cb_info.unwrap().included;

                    if included_now {
                        subband_state.inclusion_tree.encode(
                            writer,
                            x,
                            y,
                            (self.layer_index + 1) as i32,
                        );

                        let cb = cb_info.unwrap();
                        subband_state
                            .zero_bp_tree
                            .set_value(x, y, cb.zero_bp as i32);
                        subband_state.zero_bp_tree.encode(
                            writer,
                            x,
                            y,
                            (self.layer_index + 1) as i32,
                        );

                        let num_passes = cb.num_passes.max(1);
                        for _ in 0..(num_passes - 1) {
                            writer.write_bit(1);
                        }
                        writer.write_bit(0);

                        if cb.data_len > 0 {
                            let bits_needed = (32 - cb.data_len.leading_zeros()).max(3);
                            let val = bits_needed as i32 - 3;

                            subband_state
                                .lblock_tree
                                .set_value(x, y, val);
                            subband_state.lblock_tree.encode(
                                writer,
                                x,
                                y,
                                (self.layer_index + 1) as i32,
                            );
                            writer.write_bits(cb.data_len, bits_needed as u8);
                        } else {
                            subband_state.lblock_tree.set_value(x, y, 0);
                            subband_state.lblock_tree.encode(
                                writer,
                                x,
                                y,
                                (self.layer_index + 1) as i32,
                            );
                        }
                    } else {
                        subband_state.inclusion_tree.encode(
                            writer,
                            x,
                            y,
                            (self.layer_index + 1) as i32,
                        );
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
        let mut state = PrecinctState::new(2, 2);

        let header = PacketHeader::read(&mut reader, &mut state, 0, 2, 2, 1).unwrap();
        assert!(header.empty);
    }
}

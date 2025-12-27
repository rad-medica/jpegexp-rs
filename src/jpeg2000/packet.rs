use super::bit_io::J2kBitReader;
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
    pub fn new(w: usize, h: usize) -> Self {
        // Initialize with capacity appropriate for most cases
        let mut subbands = Vec::with_capacity(3);
        // It will be populated on demand or we can pre-populate if we knew subbands count
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
        reader: &mut J2kBitReader,
        state: &mut PrecinctState,
        layer: u32,
        grid_width: usize,
        grid_height: usize,
        num_subbands: usize,
    ) -> Result<Self, ()> {
        let mut header = PacketHeader {
            packet_seq_num: 0,
            empty: false,
            layer_index: layer,
            included_cblks: Vec::new(),
        };

        // 1. Zero-length packet bit
        // Read 1 bit. If 0, packet is empty.
        let bit = reader.read_bit()?;
        if bit == 0 {
            header.empty = true;
            return Ok(header);
        }

        // 2. Code-block inclusion and header info
        for s in 0..num_subbands {
            eprintln!("DEBUG: Packet S{} L{}", s, layer);
            // Ensure state has trees for this subband
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
                    let not_included_yet = subband_state
                        .inclusion_tree
                        .decode(reader, x, y, threshold)?;
                    let included = !not_included_yet;

                    if included {
                        eprintln!("DEBUG: Included CB {},{} in S{}", x, y, s);
                        let zero_bp = 0;
                        let num_passes = 1;

                        // Data Length
                        let _has_lblock =
                            subband_state.lblock_tree.decode(reader, x, y, threshold)?;
                        let data_len = reader.read_bits(16)?;

                        header.included_cblks.push(CodeBlockInfo {
                            x,
                            y,
                            subband_index: s as u8,
                            included,
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

    /// Write a packet header to the bit stream.
    pub fn write(
        &self,
        writer: &mut crate::jpeg2000::bit_io::J2kBitWriter,
        state: &mut PrecinctState,
        grid_width: usize,
        grid_height: usize,
        num_subbands: usize,
    ) {
        // 1. Zero-length packet bit
        if self.empty {
            writer.write_bit(0);
            return;
        }
        writer.write_bit(1);

        // 2. Code-block inclusion and header info
        for s in 0..num_subbands {
            // Ensure state has trees for this subband
            if state.subbands.len() <= s {
                state
                    .subbands
                    .push(SubbandState::new(grid_width, grid_height));
            }
            let subband_state = &mut state.subbands[s];

            for y in 0..grid_height {
                for x in 0..grid_width {
                    // Check if codeblock is included in this packet
                    let cb_info = self
                        .included_cblks
                        .iter()
                        .find(|c| c.x == x && c.y == y && c.subband_index == s as u8);

                    let included_now = cb_info.is_some() && cb_info.unwrap().included;

                    if included_now {
                        // Tag tree encode:
                        subband_state.inclusion_tree.encode(
                            writer,
                            x,
                            y,
                            (self.layer_index + 1) as i32,
                        );

                        // Zero BP, passes, length...
                        let cb = cb_info.unwrap();

                        // Zero Bit Planes (Tag Tree)
                        subband_state
                            .zero_bp_tree
                            .set_value(x, y, cb.zero_bp as i32);
                        subband_state.zero_bp_tree.encode(
                            writer,
                            x,
                            y,
                            (self.layer_index + 1) as i32,
                        );

                        // Number of Passes
                        let num_passes = cb.num_passes.max(1); // Ensure at least 1
                        for _ in 0..(num_passes - 1) {
                            writer.write_bit(1);
                        }
                        writer.write_bit(0); // Terminate unary encoding

                        // Data Length (Lblock encoding)
                        if cb.data_len > 0 {
                            subband_state
                                .lblock_tree
                                .set_value(x, y, cb.data_len as i32);
                            subband_state.lblock_tree.encode(
                                writer,
                                x,
                                y,
                                (self.layer_index + 1) as i32,
                            );
                            // Write the actual data length value (16 bits for simplified encoding)
                            writer.write_bits(cb.data_len, 16);
                        } else {
                            // No data length - tag tree encodes absence
                            subband_state.lblock_tree.set_value(x, y, 0);
                            subband_state.lblock_tree.encode(
                                writer,
                                x,
                                y,
                                (self.layer_index + 1) as i32,
                            );
                        }
                    } else {
                        // Not included
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
        let data = vec![0x00]; // 0 bit -> empty
        let mut reader = J2kBitReader::new(&data);
        let mut state = PrecinctState::new(2, 2);

        let header = PacketHeader::read(&mut reader, &mut state, 0, 2, 2, 1).unwrap();
        assert!(header.empty);
    }
}

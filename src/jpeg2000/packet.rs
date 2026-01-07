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
        subband_grids: &[(usize, usize)],
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
        if std::env::var("J2K_DEBUG").is_ok() {
            eprintln!(
                "PACKET: layer={}, subbands={}, empty_bit={}",
                layer, num_subbands, bit
            );
        }
        if bit == 0 {
            header.empty = true;
            return Ok(header);
        }

        // 2. Code-block inclusion and header info
        for s in 0..num_subbands {
            let (grid_width, grid_height) = if s < subband_grids.len() {
                subband_grids[s]
            } else {
                (0, 0)
            };
            
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
                    // A codeblock is "already included" only if we have decoded its exact
                    // inclusion layer in a previous layer AND that layer is below current threshold
                    let already_included = subband_state
                        .inclusion_tree
                        .is_known_below_threshold(x, y, threshold);

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
                        // Decode Zero Bit Planes
                        // Only present if this is the first time included
                        if !already_included {
                            subband_state.zero_bp_tree.decode(reader, x, y, 128)?;
                        }
                        let zero_bp = subband_state.zero_bp_tree.get_current_value(x, y) as u8;

                        // Decode Number of Passes
                        let num_passes = Self::read_coding_passes(reader)?;

                        // Data Length
                        // Decode LBlock parameter with Comma Code
                        // Note: This assumes 1 layer or state reset.
                        // Ideally we should track LBlock state in subband_state.
                        // But currently we use new state per packet.
                        // The base LBlock is 3.
                        // let _ = subband_state.lblock_tree.decode(reader, x, y, 32)?;
                        // let lblock = subband_state.lblock_tree.get_current_value(x, y) + 3;
                        
                        let lblock_inc = Self::read_comma_code(reader)?;
                        let lblock = (lblock_inc + 3) as i32;

                        // Calculate Lbits = Lblock + floor(log2(num_passes))
                        let log2_passes = if num_passes > 0 {
                            (u32::BITS - 1 - (num_passes as u32).leading_zeros()) as i32
                        } else {
                            0
                        };
                        let lbits = lblock + log2_passes;

                        let data_len = reader.read_bits(lbits as u8)?;

                        if std::env::var("J2K_DEBUG").is_ok() {
                            eprintln!(
                                "  CB[{},{}] subband={}: zero_bp={}, passes={}, lblock={}, lbits={}, len={}",
                                x, y, s, zero_bp, num_passes, lblock, lbits, data_len
                            );
                        }

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

    /// Write coding passes using Table B.4
    fn write_coding_passes(writer: &mut crate::jpeg2000::bit_io::J2kBitWriter, passes: u8) {
        match passes {
            1 => writer.write_bit(0),
            2 => {
                writer.write_bit(1);
                writer.write_bit(0);
            }
            3..=5 => {
                writer.write_bit(1);
                writer.write_bit(1);
                writer.write_bits((passes - 3) as u32, 2);
            }
            6..=36 => {
                writer.write_bit(1);
                writer.write_bit(1);
                writer.write_bits(3, 2);
                writer.write_bits((passes - 6) as u32, 5);
            }
            _ => {
                writer.write_bit(1);
                writer.write_bit(1);
                writer.write_bits(3, 2);
                writer.write_bits(31, 5);
            }
        }
    }

    /// Writes a comma code (n ones followed by a zero)
    fn write_comma_code(writer: &mut crate::jpeg2000::bit_io::J2kBitWriter, n: i32) {
        for _ in 0..n {
            writer.write_bit(1);
        }
        writer.write_bit(0);
    }

    /// Reads a comma code (sequence of ones terminated by a zero)
    fn read_comma_code(reader: &mut J2kBitReader<'_, '_>) -> Result<u32, BitIoError> {
        let mut count = 0;
        loop {
            let bit = reader.read_bit()?;
            if bit == 0 {
                break;
            }
            count += 1;
        }
        Ok(count)
    }

    /// Write a packet header to the bit stream.
    pub fn write(
        &self,
        writer: &mut crate::jpeg2000::bit_io::J2kBitWriter,
        state: &mut PrecinctState,
        subband_grids: &[(usize, usize)],
        num_subbands: usize,
    ) {
        if self.empty {
            writer.write_bit(0);
            return;
        }
        writer.write_bit(1);

        for s in 0..num_subbands {
            let (grid_width, grid_height) = if s < subband_grids.len() {
                subband_grids[s]
            } else {
                (0, 0)
            };

            if state.subbands.len() <= s {
                state
                    .subbands
                    .push(SubbandState::new(grid_width, grid_height));
            }
            let subband_state = &mut state.subbands[s];

            for y in 0..grid_height {
                for x in 0..grid_width {
                    // Find codeblock info for this position
                    let cb_info = self
                        .included_cblks
                        .iter()
                        .find(|c| c.x == x && c.y == y && c.subband_index == s as u8);

                    let included = cb_info.map_or(false, |c| c.included);
                    let threshold = (self.layer_index + 1) as i32;

                    // Tag Tree Encoding for Inclusion
                    // If not previously included, we must encode inclusion up to current layer
                    // We assume `state` tracks previous inclusions.
                    // TagTree::encode ensures the value is encoded if not already known.

                    if included {
                        let cb = cb_info.unwrap();

                        // Set inclusion value to current layer (assuming 0-based layer index)
                        subband_state
                            .inclusion_tree
                            .set_value(x, y, self.layer_index as i32);
                        subband_state.inclusion_tree.encode(writer, x, y, threshold);

                        // Zero bit-planes
                        // TagTree handles "already encoded" check internally?
                        // We need to ensure we only encode this ONCE (first time included).
                        // But TagTree::encode re-encodes if we call it again?
                        // We should rely on TagTree state. If it was already encoded (value known < threshold), it does nothing.
                        // But ZBP is constant. We set it once.

                        subband_state
                            .zero_bp_tree
                            .set_value(x, y, cb.zero_bp as i32);
                        // Encode ZBP. We need to know "current bitplane" context?
                        // ZBP tree encodes the value `cb.zero_bp`.
                        // We need to pass a threshold? No, we encode the exact value.
                        // But typically we encode relative to a max?
                        // Let's assume TagTree encodes the value fully.
                        // We pass `cb.zero_bp + 1` as threshold to ensure we encode up to the value?
                        // Actually, just encoding the value is sufficient if we want it known.
                        // Using a large threshold guarantees it is fully output.
                        subband_state
                            .zero_bp_tree
                            .encode(writer, x, y, cb.zero_bp as i32 + 1);

                        // Number of coding passes (Table B.4)
                        Self::write_coding_passes(writer, cb.num_passes);

                        // LBlock
                        // bits = Lblock + floor(log2(num_passes))
                        // We need bits >= bits_needed
                        let bits_needed = if cb.data_len > 0 {
                            (u32::BITS - cb.data_len.leading_zeros()) as i32
                        } else {
                            0
                        };
                        
                        let log2_passes = if cb.num_passes > 0 {
                            (u32::BITS - 1 - (cb.num_passes as u32).leading_zeros()) as i32
                        } else {
                            0
                        };
                        
                        // Lblock >= bits_needed - log2_passes
                        let min_lblock = bits_needed - log2_passes;
                        let lblock = min_lblock.max(3);
                        let lblock_inc = (lblock - 3).max(0);
                        
                        let lbits = lblock + log2_passes;

                        if std::env::var("J2K_DEBUG").is_ok() {
                            eprintln!("    LBlock Calc: len={} bits_needed={} passes={} log2={} lblock={} inc={} lbits={}", 
                                cb.data_len, bits_needed, cb.num_passes, log2_passes, lblock, lblock_inc, lbits);
                        }

                        // Use Comma Code for LBlock increment instead of TagTree
                        // Standard B.10.5 says LBlock is encoded using a unary code (comma code)
                        // Note: TagTree logic for 1x1 grid with val=0 writes '1' (found).
                        // Comma Code for 0 writes '0'. They are inverted.
                        // Since OpenJPEG uses Comma Code, we must use Comma Code.
                        Self::write_comma_code(writer, lblock_inc);

                        /* 
                        subband_state.lblock_tree.set_value(x, y, lblock_inc);
                        subband_state
                            .lblock_tree
                            .encode(writer, x, y, lblock_inc + 1);
                        */

                        // Write data length
                        writer.write_bits(cb.data_len, lbits as u8);
                    } else {
                        // Not included - encode "not included yet"
                        // We set value to MAX (or > threshold) so it encodes 0 bits up to threshold
                        subband_state.inclusion_tree.set_value(x, y, threshold + 1);
                        subband_state.inclusion_tree.encode(writer, x, y, threshold);
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
    fn test_packet_header_roundtrip_complex() {
        let mut header = PacketHeader {
            packet_seq_num: 0,
            empty: false,
            layer_index: 0,
            included_cblks: Vec::new(),
        };

        // Subband 0
        header.included_cblks.push(CodeBlockInfo {
            x: 0,
            y: 0,
            subband_index: 0,
            included: true,
            num_passes: 3,
            data_len: 15,
            zero_bp: 3,
        });
        // Subband 1
        header.included_cblks.push(CodeBlockInfo {
            x: 0,
            y: 0,
            subband_index: 1,
            included: true,
            num_passes: 1,
            data_len: 31,
            zero_bp: 0,
        });
        // Subband 2
        header.included_cblks.push(CodeBlockInfo {
            x: 0,
            y: 0,
            subband_index: 2,
            included: true,
            num_passes: 1,
            data_len: 7,
            zero_bp: 0,
        });

        let mut writer = crate::jpeg2000::bit_io::J2kBitWriter::new();
        let mut state_enc = PrecinctState::new(1, 1);
        let grids = vec![(1, 1); 3];
        header.write(&mut writer, &mut state_enc, &grids, 3);
        let buffer = writer.finish();

        let mut buf_reader = crate::jpeg_stream_reader::JpegStreamReader::new(&buffer);
        let mut reader = J2kBitReader::new(&mut buf_reader);
        let mut state_dec = PrecinctState::new(1, 1);

        let decoded = PacketHeader::read(&mut reader, &mut state_dec, 0, &grids, 3).unwrap();

        assert_eq!(decoded.included_cblks.len(), 3);

        // Check Subband 0
        let cb0 = &decoded.included_cblks[0];
        assert_eq!(cb0.subband_index, 0);
        assert_eq!(cb0.num_passes, 3);
        assert_eq!(cb0.data_len, 15);
        assert_eq!(cb0.zero_bp, 3);

        // Check Subband 1
        let cb1 = &decoded.included_cblks[1];
        assert_eq!(cb1.subband_index, 1);
        assert_eq!(cb1.num_passes, 1);
        assert_eq!(cb1.data_len, 31);
        assert_eq!(cb1.zero_bp, 0);

        // Check Subband 2
        let cb2 = &decoded.included_cblks[2];
        assert_eq!(cb2.subband_index, 2);
        assert_eq!(cb2.num_passes, 1);
        assert_eq!(cb2.data_len, 7);
        assert_eq!(cb2.zero_bp, 0);
    }
}

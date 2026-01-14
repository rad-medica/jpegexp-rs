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
    pub numlenbits: u8,
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
            reader.align_to_byte();
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
                        if !already_included {
                            subband_state.zero_bp_tree.decode(reader, x, y, 128)?;
                        }
                        let zero_bp = subband_state.zero_bp_tree.get_current_value(x, y) as u8;

                        // Decode Number of Passes
                        let num_passes = Self::read_coding_passes(reader)?;

                        // Data Length
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
                                "  DEC CB[{},{}] subband={}: zero_bp={}, passes={}, lblock_inc={}, lblock={}, log2_passes={}, lbits={}, len={}",
                                x, y, s, zero_bp, num_passes, lblock_inc, lblock, log2_passes, lbits, data_len
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
                            numlenbits: 3,
                        });
                    }
                }
            }
        }
        reader.align_to_byte();
        Ok(header)
    }

    /// Reads the number of coding passes using J2K codeword table (Table B.4).
    fn read_coding_passes(reader: &mut J2kBitReader<'_, '_>) -> Result<u8, BitIoError> {
        if reader.read_bit()? == 0 {
            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!("    DECODE num_passes: 1");
            }
            return Ok(1);
        }
        if reader.read_bit()? == 0 {
            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!("    DECODE num_passes: 2");
            }
            return Ok(2);
        }
        let bits = reader.read_bits(2)?;
        if bits < 3 {
            let result = (3 + bits) as u8;
            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!("    DECODE num_passes: {}", result);
            }
            return Ok(result);
        }
        let bits = reader.read_bits(5)?;
        if bits < 31 {
            let result = (6 + bits) as u8;
            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!("    DECODE num_passes: {}", result);
            }
            return Ok(result);
        }
        // 37-164 passes: read 7 more bits
        let bits2 = reader.read_bits(7)?;
        let result = (37 + bits2) as u8;
        if std::env::var("J2K_DEBUG").is_ok() {
            eprintln!("    DECODE num_passes: {}", result);
        }
        Ok(result)
    }

    /// Writes the number of coding passes using J2K codeword table (Table B.4).
    fn write_coding_passes(writer: &mut crate::jpeg2000::bit_io::J2kBitWriter, passes: u8) {
        if std::env::var("J2K_PKT_TRACE").is_ok() {
            eprint!("[PKT] Write passes({}): ", passes);
        }
        match passes {
            1 => {
                writer.write_bit(0);
                if std::env::var("J2K_PKT_TRACE").is_ok() {
                    eprintln!("0");
                }
            }
            2 => {
                writer.write_bit(1);
                writer.write_bit(0);
                if std::env::var("J2K_PKT_TRACE").is_ok() {
                    eprintln!("10");
                }
            }
            3..=5 => {
                writer.write_bit(1);
                writer.write_bit(1);
                writer.write_bits((passes - 3) as u32, 2);
                if std::env::var("J2K_PKT_TRACE").is_ok() {
                    eprintln!("11 + {:02b}", passes - 3);
                }
            }
            6..=36 => {
                writer.write_bit(1);
                writer.write_bit(1);
                writer.write_bits(3, 2);
                writer.write_bits((passes - 6) as u32, 5);
                if std::env::var("J2K_PKT_TRACE").is_ok() {
                    eprintln!("1111 + {:05b}", passes - 6);
                }
            }
            _ => {
                // 37-164 passes: write 16 bits total (9-bit prefix + 7-bit suffix)
                // Prefix: 1111 1111 1 (0xff80 >> 7 = 0x1ff)
                // Suffix: 7 bits for (passes - 37)
                writer.write_bit(1);
                writer.write_bit(1);
                writer.write_bits(3, 2);
                writer.write_bits(31, 5);
                writer.write_bits((passes - 37) as u32, 7);
                if std::env::var("J2K_PKT_TRACE").is_ok() {
                    eprintln!("1111 11111 + {:07b}", passes - 37);
                }
            }
        }
    }

    /// Writes a comma code (n ones followed by a zero)
    fn write_comma_code(writer: &mut crate::jpeg2000::bit_io::J2kBitWriter, n: i32) {
        if std::env::var("J2K_PKT_TRACE").is_ok() {
            eprint!("[PKT] Write comma_code({}): ", n);
        }
        for _ in 0..n {
            writer.write_bit(1);
        }
        writer.write_bit(0);
        if std::env::var("J2K_PKT_TRACE").is_ok() {
            eprintln!("{}", "1".repeat(n as usize) + "0");
        }
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
            writer.align_to_byte();
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
                    let cb_info = self
                        .included_cblks
                        .iter()
                        .find(|c| c.x == x && c.y == y && c.subband_index == s as u8);

                    let included = cb_info.map_or(false, |c| c.included);
                    let threshold = (self.layer_index + 1) as i32;

                    if included {
                        let cb = cb_info.unwrap();
                        
                        if std::env::var("J2K_PKT_DEBUG").is_ok() {
                            eprintln!("[PKT] Subband {} CB({},{}) included: num_passes={}, zero_bp={}, data_len={}, numlenbits={}", 
                                      s, x, y, cb.num_passes, cb.zero_bp, cb.data_len, cb.numlenbits);
                        }
                        
                        subband_state
                            .inclusion_tree
                            .set_value(x, y, self.layer_index as i32);
                        subband_state.inclusion_tree.encode(writer, x, y, threshold);

                        subband_state
                            .zero_bp_tree
                            .set_value(x, y, cb.zero_bp as i32);
                        // OpenJPEG uses threshold 999 to encode the full value
                        subband_state
                            .zero_bp_tree
                            .encode(writer, x, y, 999);

                        Self::write_coding_passes(writer, cb.num_passes);

                        let bits_needed = if cb.data_len > 0 {
                            (32 - cb.data_len.leading_zeros()) as i32
                        } else {
                            1
                        };

                        let log2_passes = if cb.num_passes > 0 {
                            (31 - (cb.num_passes as u32).leading_zeros()) as i32
                        } else {
                            0
                        };

                        let numlenbits = cb.numlenbits as i32;
                        let increment = (bits_needed - numlenbits - log2_passes).max(0);
                        let lblock = numlenbits + increment;
                        let lbits = lblock + log2_passes;

                        #[cfg(feature = "trace_packet_header")]
                        eprintln!("[PKT]   bits_needed={}, log2_passes={}, increment={}, lblock={}, lbits={}", 
                                  bits_needed, log2_passes, increment, lblock, lbits);

                        Self::write_comma_code(writer, increment);
                        if std::env::var("J2K_PKT_TRACE").is_ok() {
                            eprintln!("[PKT] Write data_len: {} in {} bits = {:0width$b}", 
                                     cb.data_len, lbits, cb.data_len, width = lbits as usize);
                        }
                        writer.write_bits(cb.data_len, lbits as u8);
                    } else {
                        subband_state.inclusion_tree.set_value(x, y, threshold + 1);
                        subband_state.inclusion_tree.encode(writer, x, y, threshold);
                    }
                }
            }
        }
        writer.align_to_byte();
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
        header.included_cblks.push(CodeBlockInfo {
            x: 0,
            y: 0,
            subband_index: 0,
            included: true,
            num_passes: 3,
            data_len: 15,
            zero_bp: 3,
            numlenbits: 3,
        });
        header.included_cblks.push(CodeBlockInfo {
            x: 0,
            y: 0,
            subband_index: 1,
            included: true,
            num_passes: 1,
            data_len: 31,
            zero_bp: 0,
            numlenbits: 3,
        });
        header.included_cblks.push(CodeBlockInfo {
            x: 0,
            y: 0,
            subband_index: 2,
            included: true,
            num_passes: 1,
            data_len: 7,
            zero_bp: 0,
            numlenbits: 3,
        });
        let mut writer = crate::jpeg2000::bit_io::J2kBitWriter::new();
        let mut state_enc = PrecinctState::new(1, 1);
        let grids = vec![(1, 1); 3];
        header.write(&mut writer, &mut state_enc, &grids, 3);
        let buffer = writer.finish();
        let mut buf_reader = crate::jpeg_stream_reader::JpegStreamReader::new(&buffer);
        let mut reader = crate::jpeg2000::bit_io::J2kBitReader::new(&mut buf_reader);
        let mut state_dec = PrecinctState::new(1, 1);
        let decoded = PacketHeader::read(&mut reader, &mut state_dec, 0, &grids, 3).unwrap();
        assert_eq!(decoded.included_cblks.len(), 3);
        assert_eq!(decoded.included_cblks[0].zero_bp, 3);
        assert_eq!(decoded.included_cblks[1].data_len, 31);
    }
}

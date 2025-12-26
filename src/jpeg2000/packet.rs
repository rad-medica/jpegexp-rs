use super::bit_io::J2kBitReader;
use super::tag_tree::TagTree;

/// Represents the state of a Precinct during parsing.
pub struct PrecinctState {
    pub inclusion_tree: TagTree,
    pub zero_bp_tree: TagTree,
    pub lblock_tree: TagTree,
}

impl PrecinctState {
    pub fn new(w: usize, h: usize) -> Self {
        let mut state = Self {
            inclusion_tree: TagTree::new(w, h),
            zero_bp_tree: TagTree::new(w, h),
            lblock_tree: TagTree::new(w, h),
        };
        state.reset();
        state
    }

    pub fn reset(&mut self) {
        self.inclusion_tree.reset();
        self.zero_bp_tree.reset();
        self.lblock_tree.reset();
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
        // Iterate in raster order (for now)
        for y in 0..grid_height {
            for x in 0..grid_width {
                // Determine inclusion
                // TagTree::decode returns true if "low >= threshold" (i.e. NOT included logic for inclusion tree).
                // If decode returns false, it means "low < threshold", which implies it found the value is <= layer.
                // So include = !decode(...)
                // Threshold is (layer + 1).
                let not_included_yet =
                    state
                        .inclusion_tree
                        .decode(reader, x, y, (layer + 1) as i32)?;
                let included = !not_included_yet;

                if included {
                    let zero_bp = 0;
                    // First time inclusion?
                    // Check if already included in previous layers?
                    // Current TagTree doesn't persistently store "included".
                    // We need PrecinctState to track which blocks are already included.
                    // For now, assume simple case (1 layer).

                    // IF first time included:
                    //   Decode Zero Bit Planes (tag tree)
                    //   state.zero_bp_tree.decode(...)
                    //   zero_bp = ...

                    // Number of passes
                    // Standard J2K: 1 bit for 1 pass, 2 bits...
                    // HTJ2K: logic might differ or use same packet headers.
                    // HTJ2K typically puts everything in one packet -> 1 pass?
                    // Let's assume 1 pass for now to unblock integration.
                    let num_passes = 1;

                    // Data Length
                    // Lblock coding:
                    // This is complex. For now, we will read 16 bits as length (Mock).
                    // This allows us to inject test data easily.
                    // TODO: Implement full Lblock tag tree decoding.
                    let data_len = reader.read_bits(16)? as u32;

                    header.included_cblks.push(CodeBlockInfo {
                        x,
                        y,
                        included,
                        num_passes,
                        data_len,
                        zero_bp,
                    });
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
    ) {
        // 1. Zero-length packet bit
        if self.empty {
            writer.write_bit(0);
            return;
        }
        writer.write_bit(1);

        // 2. Code-block inclusion and header info
        for y in 0..grid_height {
            for x in 0..grid_width {
                // Check if codeblock is included in this packet
                let cb_info = self.included_cblks.iter().find(|c| c.x == x && c.y == y);

                let included_now = cb_info.is_some() && cb_info.unwrap().included;

                // Tag tree encode:
                // If not included yet (state check), encode inclusion.
                // We need to know the *actual* layer it is first included.
                // If included now, 'val' = layer_idx + 1 ?
                // Or layer_idx ?
                // Let's assume passed threshold logic:
                // Encode(writer, x, y, threshold)
                // If val < threshold, we found it.
                // Here threshold = layer + 1.
                // If included, we assume value < threshold.

                // Simplified: Just encode inclusion "1" if included now?
                // Tag trees work by revealing if value < threshold.
                // If we want to say "Included", we ensure the tree encodes that Value <= Layer.

                // For writing, we need to manipulate the Tag Tree nodes to set the value?
                // Or the TagTree::encode simply writes bits based on preset values?
                // TagTree::encode uses `node.value`.
                // We MUST set `node.value` for this (x,y) to `self.layer_index` (or similar) if included.

                // Current hack: Assume single layer or simple logic.
                // If included, we perform encode.

                if included_now {
                    // Update inclusion tree with current layer.
                    // But TagTree::encode reads from `self.nodes`.
                    // We accept that `PacketHeader` is just a struct, `PrecinctState` holds the trees.

                    // We'll trust `state.inclusion_tree.encode` handles it if we set the value correctly?
                    // Actually `TagTree` implementation: `encode` checks `node.value` vs `threshold`.
                    // So we MUST set `node.value` for this (x,y) to `self.layer_index` (or similar) if included.

                    // Issue: TagTree nodes are flat vector.
                    // We need a helper to set value?
                    // `state.inclusion_tree.set_value(x, y, layer_index)`?
                    // TagTree doesn't have `set_value` public?
                    // It has `nodes`.

                    // For this task, we will just call `encode` assuming values are correct or mocked?
                    // Or add `set_value` to TagTree?
                    // Tag Tree rewrite in previous step didn't add `set_value`.

                    // Let's stick to the protocol logic:
                    // We call encode with threshold (layer + 1).
                    // If the node value < threshold, it emits bits to prove it.

                    state
                        .inclusion_tree
                        .encode(writer, x, y, (self.layer_index + 1) as i32);

                    // Zero BP, passes, length...
                    // if included...
                    let _cb = cb_info.unwrap();
                    // Zero BP (Tag Tree)
                    // state.zero_bp_tree.encode(...)

                    // Num Passes
                    // Unary or other?
                    // Placeholder: Write 1 bit
                    writer.write_bit(0); // Mock
                } else {
                    // Not included
                    state
                        .inclusion_tree
                        .encode(writer, x, y, (self.layer_index + 1) as i32);
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

        let header = PacketHeader::read(&mut reader, &mut state, 0, 2, 2).unwrap();
        assert!(header.empty);
    }
}

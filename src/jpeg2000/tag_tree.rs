use crate::jpeg2000::bit_io::{BitIoError, J2kBitReader, J2kBitWriter};

/// Tag Tree for JPEG 2000 Packet Header coding.
/// Represents a quad-tree structure used to encode 2D arrays of values (e.g. inclusion, zero bit-planes).
pub struct TagTree {
    nodes: Vec<TagTreeNode>,
    leaf_width: usize,
    leaf_height: usize,
}

#[derive(Clone, Debug)]
struct TagTreeNode {
    value: i32,
    low: i32,
    known: bool,
    parent_index: Option<usize>,
}

impl Default for TagTreeNode {
    fn default() -> Self {
        Self {
            value: 99999,  // Match OpenJPEG's initialization value
            low: 0,
            known: false,
            parent_index: None,
        }
    }
}

impl TagTree {
    /// Create a new TagTree for a grid of `w` x `h` leaves.
    pub fn new(w: usize, h: usize) -> Self {
        let mut nodes = Vec::new();
        let mut levels = Vec::new();

        // Level 0 (Leaves)
        let mut current_level_start = 0;
        let mut current_w = w;
        let mut current_h = h;

        levels.push((current_level_start, current_w, current_h));
        // Allocate leaves
        for _ in 0..(w * h) {
            nodes.push(TagTreeNode::default());
        }

        // Build levels up to root
        while current_w > 1 || current_h > 1 {
            #[allow(clippy::manual_div_ceil)]
            let next_w = (current_w + 1) / 2;
            #[allow(clippy::manual_div_ceil)]
            let next_h = (current_h + 1) / 2;
            let next_level_start = nodes.len();

            for _ in 0..(next_w * next_h) {
                nodes.push(TagTreeNode::default());
            }

            // Link children to parents
            for y in 0..current_h {
                for x in 0..current_w {
                    let child_idx = current_level_start + y * current_w + x;
                    let parent_y = y / 2;
                    let parent_x = x / 2;
                    let parent_idx = next_level_start + parent_y * next_w + parent_x;
                    nodes[child_idx].parent_index = Some(parent_idx);
                }
            }

            current_w = next_w;
            current_h = next_h;
            current_level_start = next_level_start;
            levels.push((current_level_start, current_w, current_h));
        }

        Self {
            nodes,
            leaf_width: w,
            leaf_height: h,
        }
    }

    /// Reset the tree state (values and known status).
    pub fn reset(&mut self) {
        for node in &mut self.nodes {
            node.value = 99999;
            node.low = 0;
            node.known = false;
        }
    }

    /// Get the current value (low) at a leaf coordinate (x, y).
    pub fn get_current_value(&self, x: usize, y: usize) -> i32 {
        if x >= self.leaf_width || y >= self.leaf_height {
            return 0;
        }
        let leaf_idx = y * self.leaf_width + x;
        self.nodes[leaf_idx].low
    }

    /// Check if the value at (x, y) is known and less than threshold.
    /// This is used for determining if a codeblock was already included in a previous layer.
    pub fn is_known_below_threshold(&self, x: usize, y: usize, threshold: i32) -> bool {
        if x >= self.leaf_width || y >= self.leaf_height {
            return false;
        }
        let leaf_idx = y * self.leaf_width + x;
        let node = &self.nodes[leaf_idx];
        // A codeblock is "already included" if:
        // 1. We know its exact inclusion layer (known=true), AND
        // 2. That layer is less than the current threshold
        node.known && node.low < threshold
    }

    /// Set the value at a leaf coordinate (x, y).
    /// This propagates the value up the tree to all parent nodes (matching OpenJPEG's opj_tgt_setvalue).
    /// Each parent node stores the minimum value of all its children.
    pub fn set_value(&mut self, x: usize, y: usize, value: i32) {
        if x >= self.leaf_width || y >= self.leaf_height {
            return;
        }
        let leaf_idx = y * self.leaf_width + x;
        
        if std::env::var("J2K_TT_TRACE").is_ok() {
            eprintln!("TT set_value: leaf=({},{}) idx={} value={} (before: leaf.value={})", 
                      x, y, leaf_idx, value, self.nodes[leaf_idx].value);
        }
        
        // Propagate value up the tree (matching OpenJPEG's logic in opj_tgt_setvalue)
        let mut idx = Some(leaf_idx);
        while let Some(curr_idx) = idx {
            let node = &mut self.nodes[curr_idx];
            if node.value <= value {
                // Parent already has a smaller value from another child, stop propagation
                if std::env::var("J2K_TT_TRACE").is_ok() {
                    eprintln!("  TT[{}]: stop propagation (node.value={} <= value={})", curr_idx, node.value, value);
                }
                break;
            }
            if std::env::var("J2K_TT_TRACE").is_ok() {
                eprintln!("  TT[{}]: set value {} (was {})", curr_idx, value, node.value);
            }
            node.value = value;
            idx = node.parent_index;
        }
    }

    /// Encode the value for leaf at (x, y) given a threshold.
    /// Tag tree coding in Packet Headers uses J2kBitWriter (Raw bits with stuffing).
    /// 
    /// This implementation matches OpenJPEG's opj_tgt_encode exactly.
    pub fn encode(&mut self, writer: &mut J2kBitWriter, x: usize, y: usize, threshold: i32) {
        if x >= self.leaf_width || y >= self.leaf_height {
            return;
        }
        let leaf_idx = y * self.leaf_width + x;
        
        if std::env::var("J2K_TT_TRACE").is_ok() {
            eprintln!("TT Encode: leaf=({},{}) idx={} value={} threshold={} current_low={}", 
                      x, y, leaf_idx, self.nodes[leaf_idx].value, threshold, self.nodes[leaf_idx].low);
        }

        // Build stack from leaf to root (matching OpenJPEG's approach)
        let mut stack: Vec<usize> = Vec::new();
        let mut idx = leaf_idx;
        
        // Walk up to root, pushing nodes onto stack
        loop {
            stack.push(idx);
            if let Some(parent) = self.nodes[idx].parent_index {
                idx = parent;
            } else {
                break;
            }
        }

        // Process from root to leaf (pop from stack)
        // Use a local `low` variable that propagates through the tree (like OpenJPEG)
        let mut low: i32 = 0;
        
        while let Some(curr_idx) = stack.pop() {
            let node = &mut self.nodes[curr_idx];
            
            // Sync low: take max of local low and node->low (OpenJPEG lines 281-285)
            if low > node.low {
                node.low = low;
            } else {
                low = node.low;
            }

            // Encode bits while low < threshold (OpenJPEG lines 287-297)
            while low < threshold {
                if low >= node.value {
                    // Value found at current low level
                    if !node.known {
                        if std::env::var("J2K_TT_TRACE").is_ok() {
                            eprintln!("  TT[{}]: Write 1 (low {} >= value {})", curr_idx, low, node.value);
                        }
                        writer.write_bit(1);
                        node.known = true;
                    }
                    break;
                }
                // Value is higher, write 0 and increment low
                if std::env::var("J2K_TT_TRACE").is_ok() {
                    eprintln!("  TT[{}]: Write 0 (low {} < value {})", curr_idx, low, node.value);
                }
                writer.write_bit(0);
                low += 1;
            }

            // Update node->low after the loop (OpenJPEG line 299)
            node.low = low;
        }
    }

    /// Decode the tag tree for leaf (x,y) up to threshold.
    pub fn decode(
        &mut self,
        reader: &mut J2kBitReader<'_, '_>,
        x: usize,
        y: usize,
        threshold: i32,
    ) -> Result<bool, BitIoError> {
        if x >= self.leaf_width || y >= self.leaf_height {
            return Ok(false);
        }
        let leaf_idx = y * self.leaf_width + x;

        let mut idx = leaf_idx;
        let mut stack = Vec::new();

        loop {
            stack.push(idx);
            let node = &self.nodes[idx];
            if node.low >= threshold || node.known {
                break;
            }
            if let Some(parent) = node.parent_index {
                idx = parent;
            } else {
                break;
            }
        }

        while let Some(curr_idx) = stack.pop() {
            let parent_low = if let Some(p_idx) = self.nodes[curr_idx].parent_index {
                self.nodes[p_idx].low
            } else {
                0
            };

            let node = &mut self.nodes[curr_idx];
            if node.low < parent_low {
                node.low = parent_low;
            }

            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!(
                    "    TT Decode[{}]: parent_low={} low={} thresh={}",
                    curr_idx, parent_low, node.low, threshold
                );
            }

            while node.low < threshold {
                if node.known {
                    break;
                }
                let bit = reader.read_bit()?;
                if std::env::var("J2K_DEBUG").is_ok() {
                    eprintln!(
                        "    TT[{}]: bit={} low={} known={} threshold={}",
                        curr_idx, bit, node.low, node.known, threshold
                    );
                }
                // JPEG 2000 tag tree semantics (per OpenJPEG):
                // bit=1 means "value equals current low" (found!)
                // bit=0 means "value is higher than current low" (continue)
                if bit == 1 {
                    node.known = true;
                    // node.low stays at current value (which is the actual value)
                    break;
                } else {
                    node.low += 1;
                }
            }
        }

        let result = self.nodes[leaf_idx].low >= threshold;
        if std::env::var("J2K_DEBUG").is_ok() {
            eprintln!(
                "    TT result: low={} >= threshold={} ? {}",
                self.nodes[leaf_idx].low, threshold, result
            );
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_tree_structure() {
        let tt = TagTree::new(3, 3);
        assert_eq!(tt.nodes.len(), 14);

        let leaf0 = &tt.nodes[0];
        assert_eq!(leaf0.parent_index, Some(9));

        let leaf8 = &tt.nodes[8];
        assert_eq!(leaf8.parent_index, Some(12));
    }

    #[test]
    fn test_tag_tree_roundtrip() {
        let mut tt_enc = TagTree::new(2, 2);
        tt_enc.set_value(0, 0, 5);
        tt_enc.set_value(1, 0, 2);
        tt_enc.set_value(0, 1, 10);
        tt_enc.set_value(1, 1, 0);

        let mut writer = J2kBitWriter::new();
        tt_enc.encode(&mut writer, 0, 0, 6);
        tt_enc.encode(&mut writer, 1, 0, 6);
        let buffer = writer.finish();

        let mut tt_dec = TagTree::new(2, 2);
        let mut buf_reader = crate::jpeg_stream_reader::JpegStreamReader::new(&buffer);
        let mut reader = J2kBitReader::new(&mut buf_reader);

        let res1 = tt_dec.decode(&mut reader, 0, 0, 6).unwrap();
        assert!(!res1);

        let res2 = tt_dec.decode(&mut reader, 1, 0, 6).unwrap();
        assert!(!res2);

        let mut tt_enc3 = TagTree::new(1, 1);
        tt_enc3.set_value(0, 0, 5);
        let mut writer3 = J2kBitWriter::new();
        tt_enc3.encode(&mut writer3, 0, 0, 5);
        let buf3 = writer3.finish();

        let mut tt_dec3 = TagTree::new(1, 1);
        let mut buf_reader3 = crate::jpeg_stream_reader::JpegStreamReader::new(&buf3);
        let mut reader3 = J2kBitReader::new(&mut buf_reader3);
        let res3 = tt_dec3.decode(&mut reader3, 0, 0, 5).unwrap();
        assert!(res3);
    }
}

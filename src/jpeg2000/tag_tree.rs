use crate::jpeg2000::bit_io::{J2kBitReader, J2kBitWriter};

/// Tag Tree for JPEG 2000 Packet Header coding.
/// Represents a quad-tree structure used to encode 2D arrays of values (e.g. inclusion, zero bit-planes).
pub struct TagTree {
    nodes: Vec<TagTreeNode>,
    leaf_width: usize,
    leaf_height: usize,
}

#[derive(Clone, Default, Debug)]
struct TagTreeNode {
    value: i32,
    low: i32,
    known: bool,
    parent_index: Option<usize>,
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

    /// Set the value at a leaf coordinate (x, y).
    pub fn set_value(&mut self, x: usize, y: usize, value: i32) {
        if x >= self.leaf_width || y >= self.leaf_height {
            return;
        }
        let leaf_idx = y * self.leaf_width + x;
        self.nodes[leaf_idx].value = value;
    }

    /// Encode the value for leaf at (x, y) given a threshold.
    /// Tag tree coding in Packet Headers uses J2kBitWriter (Raw bits with stuffing).
    pub fn encode(&mut self, writer: &mut J2kBitWriter, x: usize, y: usize, threshold: i32) {
        if x >= self.leaf_width || y >= self.leaf_height {
            return;
        }
        let leaf_idx = y * self.leaf_width + x;

        let mut idx = leaf_idx;
        let mut stack = Vec::new();

        // Find start node
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

        // Encode
        while let Some(curr_idx) = stack.pop() {
            let node = &mut self.nodes[curr_idx];
            while node.low < threshold {
                if node.value > node.low {
                    writer.write_bit(1);
                    node.low += 1;
                } else {
                    writer.write_bit(0);
                    break;
                }
            }
            node.known = node.low < threshold;
        }
    }

    /// Decode the tag tree for leaf (x,y) up to threshold.
    pub fn decode(
        &mut self,
        reader: &mut J2kBitReader,
        x: usize,
        y: usize,
        threshold: i32,
    ) -> Result<bool, ()> {
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

            while node.low < threshold {
                if node.known {
                    break;
                }
                let bit = reader.read_bit()?;
                if bit == 1 {
                    node.low += 1;
                } else {
                    node.known = true;
                    break;
                }
            }
        }

        Ok(self.nodes[leaf_idx].low >= threshold)
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
        let mut reader = J2kBitReader::new(&buffer);

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
        let mut reader3 = J2kBitReader::new(&buf3);
        let res3 = tt_dec3.decode(&mut reader3, 0, 0, 5).unwrap();
        assert!(res3);
    }
}

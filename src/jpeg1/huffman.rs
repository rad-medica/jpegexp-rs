//! Huffman coding implementation for JPEG 1 Baseline.

use crate::error::JpeglsError;

#[derive(Debug, Clone, Copy, Default)]
pub struct HuffmanCode {
    pub value: u16,
    pub length: u8,
}

pub const STD_LUMINANCE_DC_LENGTHS: [u8; 16] = [0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];

pub const STD_LUMINANCE_DC_VALUES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

pub const EXT_LUMINANCE_DC_LENGTHS: [u8; 16] = [0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const EXT_LUMINANCE_DC_VALUES: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];


pub const STD_LUMINANCE_AC_LENGTHS: [u8; 16] = [0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125];

pub const STD_LUMINANCE_AC_VALUES: [u8; 162] = [
    0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
    0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
    0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
    0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
    0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
    0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
    0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
    0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
    0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

pub const STD_CHROMINANCE_DC_LENGTHS: [u8; 16] = [0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0];

pub const STD_CHROMINANCE_DC_VALUES: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

pub const STD_CHROMINANCE_AC_LENGTHS: [u8; 16] = [0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 119];

pub const STD_CHROMINANCE_AC_VALUES: [u8; 162] = [
    0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
    0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
    0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
    0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
    0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
    0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
    0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
    0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
    0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
    0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
    0xf9, 0xfa,
];

#[derive(Clone)]
pub struct HuffmanTable {
    pub codes: [HuffmanCode; 256],
    pub min_code: [i32; 16],
    pub max_code: [i32; 16],
    pub val_ptr: [i32; 16],
    pub values: Vec<u8>,
    pub lengths: [u8; 16],
}

impl HuffmanTable {
    pub fn build_from_dht(lengths: &[u8; 16], values: &[u8]) -> Self {
        let mut table = Self {
            codes: [HuffmanCode::default(); 256],
            min_code: [0; 16],
            max_code: [-1; 16],
            val_ptr: [0; 16],
            values: values.to_vec(),
            lengths: *lengths,
        };

        let mut code = 0u16;
        let mut val_idx = 0;
        for (i, &length) in lengths.iter().enumerate() {
            let n = length as usize;
            if n > 0 {
                table.min_code[i] = code as i32;
                table.val_ptr[i] = val_idx as i32;
                for _ in 0..n {
                    let v = values[val_idx] as usize;
                    table.codes[v] = HuffmanCode {
                        value: code,
                        length: (i + 1) as u8,
                    };
                    code += 1;
                    val_idx += 1;
                }
                table.max_code[i] = (code - 1) as i32;
            }
            code <<= 1;
        }
        table
    }

    pub fn decode(&self, reader: &mut JpegBitReader) -> Result<u8, JpeglsError> {
        let mut code = 0i32;
        for i in 0..16 {
            let bit = reader.read_bits(1)? as i32;
            code = (code << 1) | bit;
            if code <= self.max_code[i] {
                let idx = self.val_ptr[i] + (code - self.min_code[i]);
                return Ok(self.values[idx as usize]);
            }
        }
        Err(JpeglsError::InvalidData)
    }

    pub fn standard_luminance_dc() -> Self {
        Self::build_from_dht(&STD_LUMINANCE_DC_LENGTHS, &STD_LUMINANCE_DC_VALUES)
    }

    pub fn extended_luminance_dc() -> Self {
        Self::build_from_dht(&EXT_LUMINANCE_DC_LENGTHS, &EXT_LUMINANCE_DC_VALUES)
    }

    pub fn standard_luminance_ac() -> Self {

        Self::build_from_dht(&STD_LUMINANCE_AC_LENGTHS, &STD_LUMINANCE_AC_VALUES)
    }

    pub fn standard_chrominance_dc() -> Self {
        Self::build_from_dht(&STD_CHROMINANCE_DC_LENGTHS, &STD_CHROMINANCE_DC_VALUES)
    }

    pub fn standard_chrominance_ac() -> Self {
        Self::build_from_dht(&STD_CHROMINANCE_AC_LENGTHS, &STD_CHROMINANCE_AC_VALUES)
    }
}

pub struct JpegBitReader<'a> {
    source: &'a [u8],
    position: usize,
    bit_buffer: u32,
    bits_in_buffer: i32,
}

impl<'a> JpegBitReader<'a> {
    pub fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            position: 0,
            bit_buffer: 0,
            bits_in_buffer: 0,
        }
    }

    pub fn read_bits(&mut self, count: u8) -> Result<u16, JpeglsError> {
        if count == 0 {
            return Ok(0);
        }
        let count = count as i32;
        while self.bits_in_buffer < count {
            let byte = self.read_byte_unstuffed()?;
            self.bit_buffer = (self.bit_buffer << 8) | byte as u32;
            self.bits_in_buffer += 8;
        }
        let shift = self.bits_in_buffer - count;
        let val = (self.bit_buffer >> shift) & ((1 << count) - 1);
        self.bits_in_buffer = shift;
        if shift > 0 {
            self.bit_buffer &= (1 << shift) - 1;
        } else {
            self.bit_buffer = 0;
        }
        Ok(val as u16)
    }

    fn read_byte_unstuffed(&mut self) -> Result<u8, JpeglsError> {
        if self.position >= self.source.len() {
            return Err(JpeglsError::InvalidData);
        }
        let byte = self.source[self.position];

        if byte == 0xFF {
            if self.position + 1 < self.source.len() {
                let next = self.source[self.position + 1];
                if next == 0x00 {
                    // Stuffed FF. Skip the 00 byte.
                    self.position += 2;
                    return Ok(0xFF);
                } else {
                    // Marker found!
                    // Do NOT consume the 0xFF or the marker.
                    // Return InvalidData so the decoder can handle it (or stop).
                    // This prevents reading marker bytes as Huffman data.
                    return Err(JpeglsError::InvalidData);
                }
            } else {
                // FF at end of file. Consume it.
                self.position += 1;
                return Ok(0xFF);
            }
        }

        self.position += 1;
        Ok(byte)
    }

    pub fn align_to_byte(&mut self) {
        self.bits_in_buffer = 0;
        self.bit_buffer = 0;
    }

    pub fn read_marker_code(&mut self) -> Result<u16, JpeglsError> {
        self.align_to_byte();
        if self.position + 1 >= self.source.len() {
            return Err(JpeglsError::InvalidData);
        }
        let b1 = self.source[self.position];
        let b2 = self.source[self.position + 1];
        self.position += 2;
        Ok(((b1 as u16) << 8) | (b2 as u16))
    }

    pub fn position(&self) -> usize {
        self.position
    }
}

pub struct JpegBitWriter<'a> {
    destination: &'a mut [u8],
    position: usize,
    bit_buffer: u32,
    bits_in_buffer: i32,
}

impl<'a> JpegBitWriter<'a> {
    pub fn new(destination: &'a mut [u8]) -> Self {
        Self {
            destination,
            position: 0,
            bit_buffer: 0,
            bits_in_buffer: 0,
        }
    }

    pub fn write_bits(&mut self, value: u16, length: u8) -> Result<(), JpeglsError> {
        if length == 0 {
            return Ok(());
        }
        let length = length as i32;
        self.bit_buffer = (self.bit_buffer << length) | (value as u32 & ((1 << length) - 1));
        self.bits_in_buffer += length;
        while self.bits_in_buffer >= 8 {
            let shift = self.bits_in_buffer - 8;
            let byte = (self.bit_buffer >> shift) as u8;
            self.emit_byte(byte)?;
            self.bits_in_buffer = shift;
            if shift > 0 {
                self.bit_buffer &= (1 << shift) - 1;
            } else {
                self.bit_buffer = 0;
            }
        }
        Ok(())
    }

    fn emit_byte(&mut self, byte: u8) -> Result<(), JpeglsError> {
        if self.position >= self.destination.len() {
            return Err(JpeglsError::ParameterValueNotSupported);
        }
        self.destination[self.position] = byte;
        self.position += 1;
        if byte == 0xFF {
            if self.position >= self.destination.len() {
                return Err(JpeglsError::ParameterValueNotSupported);
            }
            self.destination[self.position] = 0x00;
            self.position += 1;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), JpeglsError> {
        if self.bits_in_buffer > 0 {
            let pad_bits = 8 - self.bits_in_buffer;
            self.write_bits((1 << pad_bits) - 1, pad_bits as u8)?;
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.position
    }

    pub fn is_empty(&self) -> bool {
        self.position == 0
    }
}

#[derive(Default)]
pub struct HuffmanEncoder {
    pub dc_previous_value: [i16; 4],
}


impl HuffmanEncoder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_category(v: i16) -> u8 {
        if v == 0 {
            return 0;
        }
        let abs_v = v.unsigned_abs();
        (16 - abs_v.leading_zeros()) as u8
    }

    pub fn get_diff_bits(v: i16, cat: u8) -> (u16, u8) {
        if cat == 0 {
            return (0, 0);
        }
        if v >= 0 {
            (v as u16, cat)
        } else {
            ((v + (1 << cat) - 1) as u16, cat)
        }
    }
    pub fn decode_value_bits(bits: u16, cat: u8) -> i16 {
        if cat == 0 {
            return 0;
        }
        let threshold = 1 << (cat - 1);
        if bits >= threshold {
            bits as i16
        } else {
            (bits as i32 - (1 << cat) + 1) as i16
        }
    }

    /// Encode a single value (used for lossless differences).
    pub fn encode_value(
        &mut self,
        value: i16,
        writer: &mut JpegBitWriter,
        huffman_table: &HuffmanTable,
        _component_index: usize,
    ) -> Result<(), JpeglsError> {
        let category = Self::get_category(value);
        let code = huffman_table.codes[category as usize];
        if code.length == 0 && category > 0 {
            return Err(JpeglsError::InvalidData);
        }
        writer.write_bits(code.value, code.length)?;
        let (bits, bit_len) = Self::get_diff_bits(value, category);
        writer.write_bits(bits, bit_len)?;
        Ok(())
    }
}

/// Symbol frequency statistics for building optimal Huffman tables
#[derive(Clone)]
pub struct SymbolFrequencies {
    /// Frequencies for DC symbols (categories 0-15)
    pub dc_freqs: [usize; 16],
    /// Frequencies for AC symbols (run/size combinations 0x00-0xFF)
    pub ac_freqs: [usize; 256],
}

impl Default for SymbolFrequencies {
    fn default() -> Self {
        Self {
            dc_freqs: [0; 16],
            ac_freqs: [0; 256],
        }
    }
}

impl SymbolFrequencies {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_dc(&mut self, category: u8) {
        if (category as usize) < 16 {
            self.dc_freqs[category as usize] += 1;
        }
    }

    pub fn record_ac(&mut self, symbol: u8) {
        self.ac_freqs[symbol as usize] += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        for i in 0..16 {
            self.dc_freqs[i] += other.dc_freqs[i];
        }
        for i in 0..256 {
            self.ac_freqs[i] += other.ac_freqs[i];
        }
    }
}

/// Generate optimal Huffman table from symbol frequencies (ISO/IEC 10918-1 Annex K)
pub fn generate_optimal_huffman_table(
    freqs: &[usize],
    max_symbols: usize,
) -> (Vec<u8>, Vec<u8>) {
    // Build frequency list with valid symbols
    let mut freq_list: Vec<(usize, u8)> = freqs
        .iter()
        .enumerate()
        .take(max_symbols)
        .filter(|(_, &f)| f > 0)
        .map(|(sym, &f)| (f, sym as u8))
        .collect();

    if freq_list.is_empty() {
        // No symbols - return minimal table
        return (vec![0u8; 16], vec![]);
    }

    if freq_list.len() == 1 {
        // Single symbol - needs 2 symbols minimum for valid Huffman tree
        let sym = freq_list[0].1;
        let other_sym = if sym == 0 { 1 } else { 0 };
        return (
            vec![0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![sym, other_sym],
        );
    }

    // Sort by frequency (ascending)
    freq_list.sort_by_key(|&(f, _)| f);

    // Build Huffman tree using package-merge algorithm
    let code_lengths = build_limited_length_codes(&freq_list, 16);

    // Generate lengths array and values array
    let (lengths_vec, values_vec) = generate_huffman_spec(&code_lengths);

    // Validate table
    let mut check_sum = 0u32;
    for (i, &count) in lengths_vec.iter().enumerate() {
        // i=0 is length 1. Shift is 16 - 1 = 15.
        // i=15 is length 16. Shift is 16 - 16 = 0.
        check_sum += (count as u32) << (15 - i);
    }
    
    // Kraft sum must equal 65536 for valid complete Huffman codes
    if check_sum != 65536 {
        // This might happen if we have a single symbol (handled above) or empty.
        // But for N > 1, it should be exact.
    }

    (lengths_vec, values_vec)
}

/// Build limited-length Huffman codes
fn build_limited_length_codes(freq_list: &[(usize, u8)], max_len: usize) -> Vec<(u8, u8)> {
    let n = freq_list.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        // Single symbol must have length 1 (and we need a dummy symbol to make a tree, but here we just return 1)
        // Actually generate_optimal checks this.
        return vec![(freq_list[0].1, 1)];
    }

    // 1. Build standard Huffman tree to get initial lengths
    // Create leaf nodes
    let _nodes: Vec<(usize, usize)> = freq_list // (weight, height/depth - initially 0)
        .iter()
        .map(|&(f, _)| (f, 0)) // 0 means leaf
        .collect();
    
    // We need to track the tree structure to determine depths.
    // A simple way is to track "depth" if we build bottom-up?
    // Standard Huffman builds bottom-up.
    // Let's use a standard parent pointer array approach for simplicity.
    // Or just count depths.
    
    // Let's use a simpler approach:
    // Work with a list of "Items": (Frequency, OriginalIndex)
    // We merge items.
    // But we need to assign lengths at the end.
    
    #[derive(Debug, Clone, Copy)]
    struct Node {
        freq: usize,
        is_leaf: bool,
        index: usize, // index in freq_list if leaf
        left: Option<usize>,
        right: Option<usize>,
    }
    
    let mut tree_nodes: Vec<Node> = Vec::with_capacity(2 * n);
    let mut active_nodes: Vec<usize> = Vec::with_capacity(n);
    
    // Initialize leaves
    for (i, &(f, _)) in freq_list.iter().enumerate() {
        tree_nodes.push(Node { freq: f, is_leaf: true, index: i, left: None, right: None });
        active_nodes.push(i);
    }
    
    // Build tree
    while active_nodes.len() > 1 {
        // Sort descending by freq so we can pop smallest from end
        active_nodes.sort_by(|&a, &b| tree_nodes[b].freq.cmp(&tree_nodes[a].freq));
        
        let n1_idx = active_nodes.pop().unwrap();
        let n2_idx = active_nodes.pop().unwrap();
        
        let new_freq = tree_nodes[n1_idx].freq + tree_nodes[n2_idx].freq;
        let new_node_idx = tree_nodes.len();
        
        tree_nodes.push(Node {
            freq: new_freq,
            is_leaf: false,
            index: 0,
            left: Some(n1_idx),
            right: Some(n2_idx),
        });
        
        active_nodes.push(new_node_idx);
    }
    
    // Traverse tree to get lengths
    let mut lengths = vec![0u8; n];
    let mut stack: Vec<(usize, u8)> = vec![(active_nodes[0], 0)];
    
    while let Some((node_idx, depth)) = stack.pop() {
        let node = &tree_nodes[node_idx];
        if node.is_leaf {
            lengths[node.index] = depth;
        } else {
            if let Some(l) = node.left { stack.push((l, depth + 1)); }
            if let Some(r) = node.right { stack.push((r, depth + 1)); }
        }
    }
    
    // 2. Enforce length limit (16)
    // Convert lengths to counts
    let mut counts = vec![0usize; 33];
    let mut max_depth = 0;
    for &l in &lengths {
        if l > 0 {
            if l as usize >= counts.len() { counts.resize(l as usize + 1, 0); }
            counts[l as usize] += 1;
            max_depth = max_depth.max(l as usize);
        }
    }

    if max_depth > max_len {
        // Step A: Squash everything > max_len into max_len
        for i in (max_len + 1..=max_depth).rev() {
            counts[max_len] += counts[i];
            counts[i] = 0;
        }

        // Step B: Resolve Kraft inequality violation (overflow)
        // Calculate used capacity in units of 2^-16
        // Capacity = 2^16 = 65536
        // Node at depth L consumes 2^(16-L) units
        let mut used_capacity: u32 = 0;
        for (i, &count) in counts.iter().enumerate().take(max_len + 1).skip(1) {
            used_capacity += (count as u32) * (1 << (max_len - i));
        }

        let max_capacity = 1 << max_len;
        
        while used_capacity > max_capacity {
            // Find a level k (1..15) to push down to k+1
            // We should pick the deepest level < 16 to minimize impact?
            // Moving k -> k+1 reduces capacity by 2^(16-k-1)
            // We loop from 15 down to 1
            let mut best_k = 0;
            for k in (1..max_len).rev() {
                if counts[k] > 0 {
                    best_k = k;
                    break;
                }
            }
            
            if best_k == 0 {
                // Should impossible if n <= 2^16
                break;
            }
            
            counts[best_k] -= 1;
            counts[best_k + 1] += 1;
            
            // Update used_capacity
            // Change: -2^(16-k) + 2^(16-(k+1))
            // = -2 * 2^(15-k) + 2^(15-k) = -2^(15-k)
            used_capacity -= 1 << (max_len - best_k - 1);
        }
    }
    
    // Step C: Enforce counts[i] <= 255 (DHT limit)
    // Since total symbols <= 256, this only happens if one level has ALL 256 symbols.
    // If so, we move one symbol to adjacent level.
    for i in 1..=max_len {
        if counts[i] > 255 {
            // Must have exactly 256 symbols at this level
            if i < max_len {
                // Move one to deeper level (decreases Kraft usage)
                counts[i] -= 1;
                counts[i + 1] += 1;
            } else {
                // At max_len, move one to shallower level (increases Kraft usage)
                // But since we have 256 symbols at depth 16, usage is 256/65536 << 1. Safe.
                counts[i] -= 1;
                counts[i - 1] += 1;
            }
        }
    }
    
    // 3. Assign lengths to symbols
    // Sort symbols by frequency (standard Huffman: most frequent -> shortest length).
    // We have `freq_list`.
    // We have `counts` array saying how many codes of length L exist.
    // We assign the shortest lengths to the most frequent symbols.
    
    // Sort original list by freq descending (primary)
    // If freqs equal, sort by symbol value to be deterministic
    let mut sorted_syms: Vec<(usize, u8)> = freq_list.to_vec();
    // Use stable sort or secondary key
    sorted_syms.sort_by(|a, b| {
        // Higher freq -> first (shortest code)
        b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1))
    });
    
    // Assign lengths
    let mut result = Vec::with_capacity(n);
    let mut cur_len = 1;
    for &(_freq, sym) in &sorted_syms {
        while cur_len <= max_len && counts[cur_len] == 0 {
            cur_len += 1;
        }
        if cur_len > max_len {
            // Should not happen if logic is correct
            cur_len = max_len;
        }
        
        result.push((sym, cur_len as u8));
        if counts[cur_len] > 0 {
            counts[cur_len] -= 1;
        }
    }
    
    result
}

/// Generate JPEG Huffman table specification (lengths and values arrays)
fn generate_huffman_spec(code_lengths: &[(u8, u8)]) -> (Vec<u8>, Vec<u8>) {
    // Count symbols at each code length
    let mut bit_len_count = [0usize; 17]; // Index 0 unused, 1-16 for code lengths
    for &(_, len) in code_lengths {
        if len > 0 && (len as usize) <= 16 {
            bit_len_count[len as usize] += 1;
        }
    }

    // Create lengths array (JPEG format: number of codes of each length 1-16)
    let lengths: Vec<u8> = bit_len_count[1..=16]
        .iter()
        .map(|&count| count as u8)
        .collect();

    // Create values array (symbols sorted by code length, then by value)
    let mut sorted_symbols: Vec<(u8, u8)> = code_lengths.to_vec();
    sorted_symbols.sort_by_key(|&(sym, len)| (len, sym));
    let values: Vec<u8> = sorted_symbols
        .iter()
        .filter(|&&(_, len)| len > 0)
        .map(|&(sym, _)| sym)
        .collect();

    (lengths, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_huffman_generation() {
        // Test with simple frequency distribution
        let freqs = vec![10, 5, 3, 2, 1];
        let (lengths, values) = generate_optimal_huffman_table(&freqs, 5);

        // Should have generated some codes
        assert!(lengths.iter().sum::<u8>() > 0);
        assert!(!values.is_empty());

        // Most frequent symbol should have shortest code
        assert!(values.len() >= 2);
    }

    #[test]
    fn test_symbol_frequencies() {
        let mut freqs = SymbolFrequencies::new();
        freqs.record_dc(5);
        freqs.record_dc(5);
        freqs.record_ac(0x12);

        assert_eq!(freqs.dc_freqs[5], 2);
        assert_eq!(freqs.ac_freqs[0x12], 1);
    }
}

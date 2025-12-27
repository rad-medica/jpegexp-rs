//! JPEG Lossless (Process 14) implementation (ISO/IEC 10918-1 / ITU-T T.81).

use crate::error::JpeglsError;
use crate::jpeg1::huffman::{HuffmanEncoder, HuffmanTable, JpegBitReader};

/// JPEG Lossless predictor functions.
/// Px is the predicted value for the current sample.
/// Ra is the sample to the left, Rb is the sample above, Rc is the sample to the upper-left.
pub struct LosslessPredictor;

impl LosslessPredictor {
    /// Predicts the value based on the selection value (SV/Predictor ID).
    /// SV 1-7 are standard predictors. SV 0 is for the first line/column or specific cases.
    pub fn predict(sv: u8, ra: i32, rb: i32, rc: i32) -> i32 {
        match sv {
            0 => 0,                     // No prediction
            1 => ra,                    // A
            2 => rb,                    // B
            3 => rc,                    // C
            4 => ra + rb - rc,          // A + B - C
            5 => ra + ((rb - rc) >> 1), // A + (B - C) / 2
            6 => rb + ((ra - rc) >> 1), // B + (A - C) / 2
            7 => (ra + rb) >> 1,        // (A + B) / 2
            _ => 0,
        }
    }
}

pub struct Jpeg1LosslessDecoder;

impl Jpeg1LosslessDecoder {
    /// Decodes a single component of a lossless scan.
    pub fn decode_component(
        predictor_id: u8,
        width: usize,
        height: usize,
        bit_depth: u8,
        reader: &mut JpegBitReader,
        huffman_table: &HuffmanTable,
    ) -> Result<Vec<i32>, JpeglsError> {
        let mut pixels = vec![0i32; width * height];

        for y in 0..height {
            for x in 0..width {
                // 1. Decode category (K) from Huffman table
                let cat = huffman_table.decode(reader)?;

                // 2. Read 'cat' additional bits
                let bits = reader.read_bits(cat)?;

                // 3. Convert bits to signed difference
                let diff = HuffmanEncoder::decode_value_bits(bits, cat) as i32;

                // 4. Calculate prediction
                // Handle boundaries as per T.81
                let ra = if x > 0 {
                    pixels[y * width + x - 1]
                } else if y > 0 {
                    pixels[(y - 1) * width + x]
                } else {
                    1 << (bit_depth - 1)
                };

                let rb = if y > 0 {
                    pixels[(y - 1) * width + x]
                } else {
                    ra
                };

                let rc = if x > 0 && y > 0 {
                    pixels[(y - 1) * width + x - 1]
                } else {
                    rb
                };

                let px = if x == 0 && y == 0 {
                    1 << (bit_depth - 1)
                } else if y == 0 {
                    ra // Special case for first row: use predictor 1
                } else if x == 0 {
                    rb // Special case for first column: use predictor 2
                } else {
                    LosslessPredictor::predict(predictor_id, ra, rb, rc)
                };

                pixels[y * width + x] = px + diff;
            }
        }
        Ok(pixels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg1::huffman::HuffmanTable;

    #[test]
    fn test_lossless_predictors() {
        // Test predictors with some values
        assert_eq!(LosslessPredictor::predict(1, 100, 200, 50), 100); // Ra
        assert_eq!(LosslessPredictor::predict(2, 100, 200, 50), 200); // Rb
        assert_eq!(LosslessPredictor::predict(3, 100, 200, 50), 50); // Rc
        assert_eq!(LosslessPredictor::predict(4, 100, 200, 50), 250); // Ra + Rb - Rc
        assert_eq!(LosslessPredictor::predict(7, 100, 200, 50), 150); // (Ra + Rb) / 2
    }

    #[test]
    fn test_decode_component_lossless() -> Result<(), JpeglsError> {
        // Small 2x2 image, bit depth 8, predictor 1
        // Pixels: [128, 130]
        //         [128, 132]
        // Predictor 1 (Ra):
        // (0,0): px=128, diff=0 -> 128
        // (1,0): px=128, diff=2 -> 130
        // (0,1): px=128, diff=0 -> 128
        // (1,1): px=128, diff=4 -> 132 (Wait, (1,1) Ra is (0,1)=128. px=128. diff=4 -> 132)

        // Huffman codes for diffs: 0, 2, 0, 4
        // Category for 0: 0
        // Category for 2: 2 (bits: 10)
        // Category for 4: 3 (bits: 100)

        let mut lengths = [0u8; 16];
        lengths[0] = 1; // code 0 for cat 0
        lengths[1] = 1; // code 10 for cat 2
        lengths[2] = 1; // code 110 for cat 3
        let values = vec![0, 2, 3];
        let table = HuffmanTable::build_from_dht(&lengths, &values);

        // Bitstream:
        // [0] (cat 0)
        // [10][10] (cat 2, bits 2)
        // [0] (cat 0)
        // [110][100] (cat 3, bits 4)
        // Bits: 0 10 10 0 110 100 -> 0101 0011 0100 -> 0x53 0x40
        let data = vec![0x53, 0x40];
        let mut reader = JpegBitReader::new(&data);

        let pixels = Jpeg1LosslessDecoder::decode_component(1, 2, 2, 8, &mut reader, &table)?;
        assert_eq!(pixels, vec![128, 130, 128, 132]);
        Ok(())
    }
}

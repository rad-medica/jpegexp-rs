//! Progressive JPEG Encoder Infrastructure
//!
//! Handles coefficient buffering and multi-scan encoding management.

/// Represents a single 8x8 block of quantized coefficients.
#[derive(Clone, Copy)]
pub struct QuantizedBlock {
    pub coeffs: [i16; 64],
}

impl Default for QuantizedBlock {
    fn default() -> Self {
        Self { coeffs: [0; 64] }
    }
}

/// Buffer for storing all quantized coefficients of an image component.
/// Required because progressive encoding needs to access coefficients multiple times
/// in different orders (spectral selection and successive approximation).
pub struct CoefficientBuffer {
    pub width: usize,
    pub height: usize,
    pub blocks: Vec<QuantizedBlock>,
    pub h_samp: u8,
    pub v_samp: u8,
}

impl CoefficientBuffer {
    pub fn new(width: usize, height: usize, h_samp: u8, v_samp: u8) -> Self {
        let mcu_width = 8 * h_samp as usize;
        let mcu_height = 8 * v_samp as usize;
        let mcu_cols = (width + mcu_width - 1) / mcu_width;
        let mcu_rows = (height + mcu_height - 1) / mcu_height;

        // Total blocks = MCUs * blocks_per_mcu
        let total_blocks = mcu_cols * mcu_rows * (h_samp as usize * v_samp as usize);

        Self {
            width,
            height,
            blocks: vec![QuantizedBlock::default(); total_blocks],
            h_samp,
            v_samp,
        }
    }

    pub fn get_block_mut(
        &mut self,
        mcu_index: usize,
        block_index_in_mcu: usize,
    ) -> &mut QuantizedBlock {
        // Blocks are stored in MCU order, then interleaved
        let blocks_per_mcu = self.h_samp as usize * self.v_samp as usize;
        &mut self.blocks[mcu_index * blocks_per_mcu + block_index_in_mcu]
    }
}

/// Defines the parameters for a single scan in a progressive sequence.
#[derive(Clone, Debug)]
pub struct ScanSpecification {
    /// Start of spectral selection (0-63)
    pub ss_start: u8,
    /// End of spectral selection (0-63)
    pub ss_end: u8,
    /// Successive approximation bit position high (0-13)
    pub ah: u8,
    /// Successive approximation bit position low (0-13)
    pub al: u8,
    /// Component indices included in this scan
    pub component_indices: Vec<u8>,
}

/// Defines the complete sequence of scans for progressive encoding.
pub struct ScanScript {
    pub scans: Vec<ScanSpecification>,
}

impl Default for ScanScript {
    /// Returns the standard default scan script (usually ~10 scans)
    /// Similar to the one used by jpegtran and cjpeg.
    fn default() -> Self {
        Self {
            scans: vec![
                // DC Scan (all components)
                ScanSpecification { ss_start: 0, ss_end: 0, ah: 0, al: 0, component_indices: vec![0, 1, 2] },
                
                // AC Scans (Spectral selection)
                // Luma AC
                ScanSpecification { ss_start: 1, ss_end: 5, ah: 0, al: 2, component_indices: vec![0] }, // First few AC
                ScanSpecification { ss_start: 6, ss_end: 63, ah: 0, al: 2, component_indices: vec![0] }, // Rest of AC
                
                // Chroma AC
                ScanSpecification { ss_start: 1, ss_end: 63, ah: 0, al: 1, component_indices: vec![1] }, // Cb
                ScanSpecification { ss_start: 1, ss_end: 63, ah: 0, al: 1, component_indices: vec![2] }, // Cr
                
                // Refinement Scans (Successive approximation)
                // Luma AC Refinement
                ScanSpecification { ss_start: 1, ss_end: 63, ah: 2, al: 1, component_indices: vec![0] },
                
                // DC Refinement (if needed, usually done in one pass if AL=0)
                // ...
                // This is a simplified script for now. The "standard" script is quite complex.
                // Let's start with a simpler "Spectal Selection Only" script for Phase 1.
            ]
        }
    }
}

impl ScanScript {
    /// Creates a simple spectral selection script.
    /// This is easier to implement first: just split AC coefficients into bands.
    /// No bit-shifting (successive approximation) involved yet (Ah=0, Al=0).
    pub fn simple_spectral() -> Self {
        Self {
            scans: vec![
                // Scan 1: DC (all components)
                ScanSpecification {
                    ss_start: 0,
                    ss_end: 0,
                    ah: 0,
                    al: 0,
                    component_indices: vec![0, 1, 2],
                },
                // Scan 2: AC Luma 1-5
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 5,
                    ah: 0,
                    al: 0,
                    component_indices: vec![0],
                },
                // Scan 3: AC Cb 1-5
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 5,
                    ah: 0,
                    al: 0,
                    component_indices: vec![1],
                },
                // Scan 4: AC Cr 1-5
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 5,
                    ah: 0,
                    al: 0,
                    component_indices: vec![2],
                },
                // Scan 5: AC Luma 6-63
                ScanSpecification {
                    ss_start: 6,
                    ss_end: 63,
                    ah: 0,
                    al: 0,
                    component_indices: vec![0],
                },
                // Scan 6: AC Cb 6-63
                ScanSpecification {
                    ss_start: 6,
                    ss_end: 63,
                    ah: 0,
                    al: 0,
                    component_indices: vec![1],
                },
                // Scan 7: AC Cr 6-63
                ScanSpecification {
                    ss_start: 6,
                    ss_end: 63,
                    ah: 0,
                    al: 0,
                    component_indices: vec![2],
                },
            ],
        }
    }

    /// Creates a standard progressive script using Successive Approximation.
    /// This provides the best progressive experience ("blurry to sharp").
    pub fn standard_successive_approximation() -> Self {
        Self {
            scans: vec![
                // Scan 1: DC (all components) - First approximation (High bits)
                ScanSpecification {
                    ss_start: 0,
                    ss_end: 0,
                    ah: 0,
                    al: 0,
                    component_indices: vec![0, 1, 2],
                },

                // Scan 2: AC Luma - First approximation (High bits)
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 63,
                    ah: 0,
                    al: 2,
                    component_indices: vec![0],
                },

                // Scan 3: AC Cb - First approximation (High bits)
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 63,
                    ah: 0,
                    al: 1,
                    component_indices: vec![1],
                },

                // Scan 4: AC Cr - First approximation (High bits)
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 63,
                    ah: 0,
                    al: 1,
                    component_indices: vec![2],
                },

                // Scan 5: AC Luma - Refinement (Al=1)
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 63,
                    ah: 2,
                    al: 1,
                    component_indices: vec![0],
                },

                // Scan 6: AC Cb - Refinement (Al=0 - Full precision)
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 63,
                    ah: 1,
                    al: 0,
                    component_indices: vec![1],
                },

                // Scan 7: AC Cr - Refinement (Al=0 - Full precision)
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 63,
                    ah: 1,
                    al: 0,
                    component_indices: vec![2],
                },

                // Scan 8: AC Luma - Final Refinement (Al=0 - Full precision)
                ScanSpecification {
                    ss_start: 1,
                    ss_end: 63,
                    ah: 1,
                    al: 0,
                    component_indices: vec![0],
                },
            ],
        }
    }
}

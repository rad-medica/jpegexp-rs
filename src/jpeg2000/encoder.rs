//! JPEG 2000 Encoder
//!
//! This module provides basic JPEG 2000 encoding functionality.
//! Currently implements a simplified encoder that produces valid J2K codestreams.

use super::bit_io::J2kBitWriter;
use super::dwt::{Dwt53, Dwt97};
use super::image::{J2kCod, J2kQcd};
use super::quantization;
use super::writer::J2kWriter;
use crate::FrameInfo;
use crate::JpeglsError;

/// JPEG 2000 Encoder
pub struct J2kEncoder {
    /// Number of DWT decomposition levels
    decomposition_levels: u8,
    /// Use 9-7 irreversible transform (false = 5-3 reversible)
    use_irreversible: bool,
    /// Quality parameter (0-100, maps to quantization step size)
    quality: u8,
}

impl J2kEncoder {
    /// Create a new J2K encoder with default settings
    pub fn new() -> Self {
        Self {
            decomposition_levels: 5,
            use_irreversible: true,
            quality: 85,
        }
    }

    /// Set the quality level (0-100)
    pub fn set_quality(&mut self, quality: u8) {
        self.quality = quality.min(100).max(1);
    }

    /// Set the number of decomposition levels
    pub fn set_decomposition_levels(&mut self, levels: u8) {
        self.decomposition_levels = levels.min(32);
    }

    /// Set whether to use irreversible (9-7) or reversible (5-3) transform
    pub fn set_irreversible(&mut self, irreversible: bool) {
        self.use_irreversible = irreversible;
    }

    /// Encode pixel data to JPEG 2000 codestream
    pub fn encode(
        &mut self,
        pixels: &[u8],
        frame_info: &FrameInfo,
        destination: &mut [u8],
    ) -> Result<usize, JpeglsError> {
        let width = frame_info.width;
        let height = frame_info.height;
        let components = frame_info.component_count as u32;

        // Initialize writer
        let mut writer = J2kWriter::new(destination);

        // Write SOC (Start of Codestream)
        writer.write_soc()?;

        // Write SIZ (Image and Tile Size)
        // Use image dimensions as tile size (single tile)
        writer.write_siz(
            width,
            height,
            width,  // tile_width = image width
            height, // tile_height = image height
            components as u16,
            frame_info.bits_per_sample as u8,
            1, // sub_x
            1, // sub_y
        )?;

        // Create COD marker
        let cod = J2kCod {
            coding_style: 0,
            progression_order: 0, // LRCP
            number_of_layers: 1,
            mct: 0,
            decomposition_levels: self.decomposition_levels,
            codeblock_width_exp: 4,  // 2^4 = 16
            codeblock_height_exp: 4, // 2^4 = 16
            transformation: 1,
            precinct_sizes: Vec::new(),
        };
        writer.write_cod(&cod)?;

        // Create QCD marker with quantization step sizes
        // Map quality (1-100) to step size
        // Higher quality = smaller step size
        let step_size = if self.quality >= 90 {
            1.0
        } else if self.quality >= 70 {
            2.0
        } else if self.quality >= 50 {
            4.0
        } else if self.quality >= 30 {
            8.0
        } else {
            16.0
        };

        // For each decomposition level + LL band, we need a step size
        // Simplified: use same step size for all subbands
        let num_subbands = (self.decomposition_levels + 1) * 3 + 1; // LL + 3 per level
        let step_sizes: Vec<u16> = (0..num_subbands)
            .map(|_| {
                // Convert step_size to u16 format (11-bit mantissa, 5-bit exponent)
                // Simplified: just use a fixed value
                (step_size * 256.0) as u16
            })
            .collect();

        let qcd = J2kQcd {
            quant_style: if self.use_irreversible { 0 } else { 1 }, // 0 = scalar expounded, 1 = scalar derived
            step_sizes,
        };
        writer.write_qcd(&qcd)?;

        // Write SOT (Start of Tile) for tile 0
        // We'll write the tile length later, for now use 0 (unknown)
        writer.write_sot(0, 0, 0, 1)?;

        // Write SOD (Start of Data)
        writer.write_sod()?;

        // Write empty packet headers for all expected packets
        // The decoder expects packets in progression order (LRCP by default)
        // For LRCP: Layer -> Resolution -> Component -> Precinct
        // We have: 1 layer, (decomposition_levels + 1) resolutions, components, 1 precinct
        let num_layers = cod.number_of_layers as usize;
        let num_resolutions = (self.decomposition_levels + 1) as usize;
        let num_components = components as usize;
        let grid_w = 1; // Single precinct
        let grid_h = 1;

        // Write empty packets in LRCP order (progression_order = 0)
        for _layer in 0..num_layers {
            for _res in 0..num_resolutions {
                for _comp in 0..num_components {
                    for _py in 0..grid_h {
                        for _px in 0..grid_w {
                            // Write empty packet header (single 0 bit)
                            let mut bit_writer = J2kBitWriter::new();
                            bit_writer.write_bit(0); // Empty packet indicator
                            let packet_header_bytes = bit_writer.finish();
                            writer.write_bytes(&packet_header_bytes)?;
                        }
                    }
                }
            }
        }

        // Write EOC (End of Codestream)
        writer.write_eoc()?;

        Ok(writer.len())
    }

    /// Encode a single component
    fn encode_component(
        &mut self,
        _writer: &mut J2kWriter,
        pixels: &[u8],
        width: usize,
        height: usize,
        step_size: f32,
    ) -> Result<(), JpeglsError> {
        // Convert pixels to f32 for DWT
        let mut image_data: Vec<f32> = pixels.iter().map(|&p| p as f32).collect();

        // Apply 2D DWT (simplified: single level for now)
        if self.use_irreversible {
            // Use 9-7 transform
            self.apply_dwt_2d_97(&mut image_data, width, height)?;
        } else {
            // Use 5-3 transform (convert to i32 first)
            let mut int_data: Vec<i32> = pixels.iter().map(|&p| p as i32).collect();
            self.apply_dwt_2d_53(&mut int_data, width, height)?;
            image_data = int_data.iter().map(|&x| x as f32).collect();
        }

        // Quantize coefficients
        let _quantized: Vec<i32> = image_data
            .iter()
            .map(|&c| quantization::quantize_scalar(c, step_size))
            .collect();

        // For now, write a minimal packet structure
        // A real implementation would:
        // 1. Organize coefficients into code-blocks
        // 2. Perform bit-plane coding
        // 3. Use MQ coder or HT block coder
        // 4. Form packets with proper headers

        // Simplified: write a basic packet header indicating no codeblocks
        // This creates a valid but minimal codestream
        // Packet header: 0 bits (empty packet for now)

        Ok(())
    }

    /// Apply 2D DWT using 9-7 filter
    fn apply_dwt_2d_97(
        &self,
        data: &mut [f32],
        width: usize,
        height: usize,
    ) -> Result<(), JpeglsError> {
        // Simplified 2D DWT: apply 1D transform row-wise, then column-wise
        let _row_buffer = vec![0.0f32; width.max(height)];
        let mut col_buffer_l = vec![0.0f32; height];
        let mut col_buffer_h = vec![0.0f32; height];

        // Transform rows
        for y in 0..height {
            let row_start = y * width;
            let row = &data[row_start..row_start + width];
            let (l_len, h_len) = ((width + 1) / 2, width / 2);
            col_buffer_l.resize(l_len, 0.0);
            col_buffer_h.resize(h_len, 0.0);
            Dwt97::forward(row, &mut col_buffer_l, &mut col_buffer_h);
            // For now, just store back (simplified)
            for (i, &val) in col_buffer_l.iter().enumerate() {
                if i < width {
                    data[row_start + i] = val;
                }
            }
        }

        // Note: Full 2D DWT would also transform columns and organize into subbands
        // This is a simplified version
        Ok(())
    }

    /// Apply 2D DWT using 5-3 filter
    fn apply_dwt_2d_53(
        &self,
        data: &mut [i32],
        width: usize,
        height: usize,
    ) -> Result<(), JpeglsError> {
        // Simplified 2D DWT: apply 1D transform row-wise
        let mut row_buffer_l = vec![0i32; (width + 1) / 2];
        let mut row_buffer_h = vec![0i32; width / 2];

        // Transform rows
        for y in 0..height {
            let row_start = y * width;
            let row = &data[row_start..row_start + width];
            Dwt53::forward(row, &mut row_buffer_l, &mut row_buffer_h);
            // For now, just store back (simplified)
            for (i, &val) in row_buffer_l.iter().enumerate() {
                if i < width {
                    data[row_start + i] = val;
                }
            }
        }

        // Note: Full 2D DWT would also transform columns and organize into subbands
        Ok(())
    }
}

impl Default for J2kEncoder {
    fn default() -> Self {
        Self::new()
    }
}

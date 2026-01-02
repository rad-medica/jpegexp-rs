//! JPEG 2000 Encoder
//!
//! This module provides basic JPEG 2000 encoding functionality.
//! Currently implements a simplified encoder that produces valid J2K codestreams.

use super::bit_io::J2kBitWriter;
use super::dwt::Dwt53;
use super::image::{J2kCod, J2kQcd};
use super::quantization;
use super::writer::J2kWriter;
use super::packet::{PacketHeader, CodeBlockInfo, PrecinctState};
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
            use_irreversible: false, // Default to Reversible 5-3 (Lossless)
            quality: 0, // Lossless
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
        let width = frame_info.width as usize;
        let height = frame_info.height as usize;
        let components = frame_info.component_count as usize;
        let depth = frame_info.bits_per_sample as u8;

        // Validate input
        let expected_size = width * height * components;
        if pixels.len() < expected_size {
            return Err(JpeglsError::InvalidData);
        }

        // Initialize writer
        let mut writer = J2kWriter::new(destination);

        // Write SOC (Start of Codestream)
        writer.write_soc()?;

        // Write SIZ (Image and Tile Size)
        writer.write_siz(
            width as u32,
            height as u32,
            width as u32, // single tile
            height as u32,
            components as u16,
            depth,
            1,
            1, // no subsampling
        )?;

        // Determine transform type
        let transformation = if self.use_irreversible { 0 } else { 1 }; // 0=9-7, 1=5-3

        // Create COD marker
        let cod = J2kCod {
            coding_style: 0,
            progression_order: 0, // LRCP
            number_of_layers: 1,
            mct: if components >= 3 { 1 } else { 0 },
            decomposition_levels: self.decomposition_levels,
            codeblock_width_exp: 4, // 64x64 code-blocks
            codeblock_height_exp: 4,
            transformation,
            precinct_sizes: Vec::new(),
        };
        writer.write_cod(&cod)?;

        // Create QCD marker
        let num_subbands = 1 + 3 * self.decomposition_levels as usize; // LL + 3 per level
        let guard_bits = 1;
        let quant_style = (guard_bits << 5) | if self.use_irreversible { 2 } else { 0 };
        
        let qcd = J2kQcd {
            quant_style, 
            step_sizes: if self.use_irreversible {
                 let base_step = self.calculate_step_size(depth);
                 (0..num_subbands).map(|i| self.encode_step_size(base_step, i)).collect()
            } else {
                 // Reversible: Exponent = depth + 1 (guard bit)
                 // Or rather: Exponent = bit depth of the subband range?
                 // Standard says for 5-3: exponent = dynamic range.
                 // Depth 8 -> Range 8. + 1 guard = 9?
                 // Let's use (depth + guard) << 3 ?
                 // No, Exponent field in QCD is 5 bits.
                 // Val = (exponent << 3).
                 vec![((depth as u16 + guard_bits as u16) << 3); num_subbands]
            },
        };
        writer.write_qcd(&qcd)?;

        // 1. De-interleave and Level Shift
        let mut planes = vec![vec![0i32; width * height]; components];
        for i in 0..(width * height) {
            for c in 0..components {
                // DC Level Shift: subtract 2^(depth-1) = 128 for 8-bit
                planes[c][i] = pixels[i * components + c] as i32 - 128;
            }
        }

        // 2. MCT (Forward)
        if cod.mct == 1 && components >= 3 {
            // Reversible Color Transform (RCT)
            for i in 0..(width * height) {
                let r = planes[0][i];
                let g = planes[1][i];
                let b = planes[2][i];
                
                let y = (r + 2 * g + b) >> 2;
                let cb = b - g;
                let cr = r - g;
                
                planes[0][i] = y;
                planes[1][i] = cb;
                planes[2][i] = cr;
            }
        }

        // 3. DWT and Encode Components
        let mut component_packets: Vec<Vec<Vec<u8>>> = Vec::new();

        for c in 0..components {
            let mut coeffs = planes[c].clone();
            
            // DWT (Reversible 5-3)
            coeffs = self.apply_forward_dwt_53(&coeffs, width, height)?;
            
            // Encode Codeblocks
            let res_packets = self.encode_component_codeblocks(
                &coeffs, 
                width, 
                height, 
                64, // cb_size
                self.decomposition_levels as usize, 
                depth
            )?;
            
            component_packets.push(res_packets);
        }

        // Write SOT (Start of Tile)
        writer.write_sot(0, 0, 0, 1)?;

        // Write SOD (Start of Data)
        writer.write_sod()?;

        // Write Packets (LRCP: Layer, Resolution, Component)
        let num_resolutions = (self.decomposition_levels + 1) as usize;
        for r in 0..num_resolutions {
            for c in 0..components {
                if r < component_packets[c].len() {
                    let packet_data = &component_packets[c][r];
                    writer.write_bytes(packet_data)?;
                } else {
                    // Empty packet if missing
                    let mut bw = J2kBitWriter::new();
                    bw.write_bit(0);
                    writer.write_bytes(&bw.finish())?;
                }
            }
        }

        // Write EOC (End of Codestream)
        writer.write_eoc()?;

        Ok(writer.len())
    }

    /// Calculate quantization step size based on quality
    fn calculate_step_size(&self, depth: u8) -> f32 {
        let base = 1.0 / (1 << depth) as f32;
        let quality_factor = (101 - self.quality.clamp(1, 100)) as f32 / 50.0;
        base * quality_factor.max(0.01)
    }

    /// Encode step size to JPEG2000 format (5-bit exponent, 11-bit mantissa)
    fn encode_step_size(&self, step: f32, subband_idx: usize) -> u16 {
        let gain = if subband_idx == 0 {
            1.0
        } else {
            2.0f32.powi((subband_idx as i32 - 1) / 3 + 1)
        };
        let adjusted = step * gain;

        let log2 = adjusted.log2();
        let exponent = ((-log2).floor() as i32).clamp(0, 31) as u16;
        let mantissa = ((adjusted * (1 << exponent) as f32 - 1.0) * 2048.0) as u16 & 0x7FF;

        (exponent << 11) | mantissa
    }

    fn apply_forward_dwt_53(
        &self,
        data: &[i32],
        width: usize,
        height: usize,
    ) -> Result<Vec<i32>, JpeglsError> {
        let mut result = data.to_vec();
        let mut current_w = width;
        let mut current_h = height;

        for _level in 0..self.decomposition_levels {
            if current_w < 2 || current_h < 2 {
                break;
            }

            let mut out_l = vec![0i32; (current_w + 1) / 2];
            let mut out_h = vec![0i32; current_w / 2];
            
            for y in 0..current_h {
                let row_start = y * width;
                let row = &result[row_start..row_start + current_w];
                Dwt53::forward(row, &mut out_l, &mut out_h);
                for (i, &v) in out_l.iter().enumerate() { result[row_start + i] = v; }
                for (i, &v) in out_h.iter().enumerate() { result[row_start + out_l.len() + i] = v; }
            }

            let mut out_l = vec![0i32; (current_h + 1) / 2];
            let mut out_h = vec![0i32; current_h / 2];
            
            for x in 0..current_w {
                let mut col = Vec::with_capacity(current_h);
                for y in 0..current_h { col.push(result[y * width + x]); }
                Dwt53::forward(&col, &mut out_l, &mut out_h);
                for (i, &v) in out_l.iter().enumerate() { result[i * width + x] = v; }
                for (i, &v) in out_h.iter().enumerate() { result[(out_l.len() + i) * width + x] = v; }
            }

            current_w = (current_w + 1) / 2;
            current_h = (current_h + 1) / 2;
        }

        Ok(result)
    }

    /// Encode component code-blocks
    fn encode_component_codeblocks(
        &self,
        coeffs: &[i32],
        width: usize,
        height: usize,
        cb_size: usize,
        num_levels: usize,
        depth: u8,
    ) -> Result<Vec<Vec<u8>>, JpeglsError> {
        let mut packets = Vec::new();
        let guard_bits = 1;
        
        let mut dims = vec![(width, height)];
        for _ in 0..num_levels {
            let (last_w, last_h) = dims.last().unwrap();
            let next_w = (last_w + 1) / 2;
            let next_h = (last_h + 1) / 2;
            dims.push((next_w, next_h));
        }
        
        for r in 0..=num_levels {
            let mut precinct_state = PrecinctState::new(0, 0);
            let num_subbands = if r == 0 { 1 } else { 3 };
            
            let subbands = if r == 0 {
                let (w, h) = dims[num_levels];
                vec![(0, 0, 0, w, h)]
            } else {
                let d = num_levels - r + 1;
                let (pw, ph) = dims[d-1];
                let (lw, lh) = dims[d];
                vec![
                    (0, lw, 0, pw-lw, lh), 
                    (1, 0, lh, lw, ph-lh), 
                    (2, lw, lh, pw-lw, ph-lh) 
                ]
            };
            
            let mut max_cb_w = 0;
            let mut max_cb_h = 0;
            let mut packet_body = Vec::new();
            let mut included_cblks = Vec::new();
            
            for &(s_idx, sx, sy, sw, sh) in &subbands {
                if sw == 0 || sh == 0 { continue; }
                let cb_cols = sw.div_ceil(cb_size);
                let cb_rows = sh.div_ceil(cb_size);
                max_cb_w = max_cb_w.max(cb_cols);
                max_cb_h = max_cb_h.max(cb_rows);
                
                for cby in 0..cb_rows {
                    for cbx in 0..cb_cols {
                        let x = sx + cbx * cb_size;
                        let y = sy + cby * cb_size;
                        let w = cb_size.min(sw - cbx * cb_size);
                        let h = cb_size.min(sh - cby * cb_size);
                        
                        // Extract coefficients
                        let mut cb_coeffs = vec![0i32; w * h];
                        for cy in 0..h {
                            for cx in 0..w {
                                let src_x = x + cx;
                                let src_y = y + cy;
                                if src_y < height && src_x < width {
                                    cb_coeffs[cy * w + cx] = coeffs[src_y * width + src_x];
                                }
                            }
                        }
                        
                        let max_val = cb_coeffs.iter().map(|&v| v.abs()).max().unwrap_or(0);
                        if max_val > 0 {
                            let mut bpc = super::bit_plane_coder::BitPlaneCoder::new(w as u32, h as u32, &cb_coeffs);
                        let msb = if max_val > 0 {
                            (max_val as f32).log2().floor() as u8
                        } else {
                            0
                        };
                        // Bitplane index is 0..msb. Count is msb+1.
                        // e.g. 127 -> log2=6.9 -> floor=6. 0..6 is 7 planes.
                        let num_bitplanes = if max_val > 0 { msb + 1 } else { 0 };
                        
                        // Guard=1. Depth=8. M_b = 8 + 1 - 1 = 8.
                        // Decoder max_bp = M_b - 1 - zero_bp = 7 - zero_bp.
                        // We want max_bp = msb.
                        // msb = 7 - zero_bp => zero_bp = 7 - msb.
                        // For 127: msb=6. zero_bp = 7 - 6 = 1.
                        // check: max_bp = 7 - 1 = 6. Correct.
                        // For 255: msb=7. zero_bp = 7 - 7 = 0.
                        // check: max_bp = 7 - 0 = 7. Correct.
                        // For 28: msb=4. zero_bp = 7 - 4 = 3.
                        // check: max_bp = 7 - 3 = 4. Correct.
                        // General formula: zero_bp = (depth + guard - 1 - 1) - msb = depth + guard - 2 - msb.
                        // Wait, M_b formula in decoder:
                        // m_b = guard_bits + depth - 1. (if reversible)
                        // max_bit_plane = m_b.saturating_sub(1).saturating_sub(cb_info.zero_bp)
                        // max_bp = guard + depth - 2 - zero_bp.
                        // msb = guard + depth - 2 - zero_bp
                        // zero_bp = guard + depth - 2 - msb.
                        // For Depth 8, Guard 1: zero_bp = 1 + 8 - 2 - msb = 7 - msb.
                        
                        let zero_bp = if max_val > 0 {
                            (depth + guard_bits - 2).saturating_sub(msb)
                        } else {
                            0 // Doesn't matter for empty
                        };

                            
                            // Determine orientation for context (LL=0, HL=1, LH=2, HH=3)
                            // s_idx maps: 0->?, 1->?, 2->?
                            // If r=0, s_idx=0 -> LL -> orientation 0
                            // If r>0: s_idx=0(HL)->1, s_idx=1(LH)->2, s_idx=2(HH)->3
                            let orientation = if r == 0 { 
                                0 
                            } else {
                                match s_idx {
                                    0 => 1,
                                    1 => 2,
                                    2 => 3,
                                    _ => 0
                                }
                            };

                            let mut passes = 0;
                            if num_bitplanes > 0 {
                                // MSB Plane: Only Cleanup
                                bpc.cleanup(num_bitplanes - 1, orientation);
                                passes += 1;
                                
                                for bp in (0..num_bitplanes - 1).rev() {
                                    bpc.significance_propagation(bp, orientation);
                                    bpc.magnitude_refinement(bp);
                                    bpc.cleanup(bp, orientation);
                                    passes += 3;
                                }
                            }
                            bpc.mq.flush();
                            let data = bpc.mq.get_buffer().to_vec();
                            
                            included_cblks.push(CodeBlockInfo {
                                x: cbx,
                                y: cby,
                                subband_index: s_idx as u8,
                                included: true,
                                num_passes: passes,
                                data_len: data.len() as u32,
                                zero_bp,
                            });
                            packet_body.extend(data);
                        } else {
                            // Not included or empty?
                            // If empty, not included in layer 0? 
                            // Or included with 0 len?
                            // Included=false usually for empty blocks in first layer
                            included_cblks.push(CodeBlockInfo {
                                x: cbx,
                                y: cby,
                                subband_index: s_idx as u8,
                                included: false,
                                num_passes: 0,
                                data_len: 0,
                                zero_bp: 0,
                            });
                        }
                    }
                }
            }
            
            let header = PacketHeader {
                packet_seq_num: 0,
                empty: included_cblks.iter().all(|cb| !cb.included),
                layer_index: 0,
                included_cblks,
            };
            
            let mut bit_writer = J2kBitWriter::new();
            header.write(&mut bit_writer, &mut precinct_state, max_cb_w, max_cb_h, num_subbands);
            
            let mut packet = bit_writer.finish();
            packet.extend(packet_body);
            packets.push(packet);
        }

        Ok(packets)
    }
}

impl Default for J2kEncoder {
    fn default() -> Self {
        Self::new()
    }
}

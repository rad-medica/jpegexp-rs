//! JPEG 2000 Encoder
//!
//! This module provides JPEG 2000 encoding functionality with proper DWT,
//! quantization, and EBCOT entropy coding.

use super::bit_io::J2kBitWriter;
use super::bit_plane_coder::BitPlaneCoder;
use super::dwt::Dwt53;
use super::image::{J2kCod, J2kQcd};
use super::packet::{PacketHeader, PrecinctState};
use super::writer::J2kWriter;
use crate::FrameInfo;
use crate::JpeglsError;

/// JPEG 2000 Encoder
pub struct J2kEncoder {
    /// Number of DWT decomposition levels
    decomposition_levels: u8,
    /// Use 9-7 irreversible transform (false = 5-3 reversible)
    use_irreversible: bool,
    /// Codeblock size exponent (4 = 64x64)
    codeblock_exp: u8,
    /// Quality parameter (unused for lossless, kept for API compatibility)
    #[allow(dead_code)]
    quality: u8,
}

/// Encoded packet data structure
struct Packet {
    resolution: u8,
    component: u8,
    layer: u8,
    header_data: Vec<u8>,
    body_data: Vec<u8>,
}

impl J2kEncoder {
    /// Create a new J2K encoder with default settings
    pub fn new() -> Self {
        Self {
            decomposition_levels: 5,
            use_irreversible: false, // Default to reversible for lossless
            codeblock_exp: 4,        // 64x64 codeblocks
            quality: 100,
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
        let bytes_per_sample = if depth > 8 { 2 } else { 1 };
        let expected_size = width * height * components * bytes_per_sample;
        if pixels.len() < expected_size {
            return Err(JpeglsError::InvalidData);
        }

        // Calculate max supported decomposition levels
        let max_levels = (width.min(height) as f32).log2().floor() as u8;
        let decomposition_levels = self.decomposition_levels.min(max_levels).min(5);

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
            decomposition_levels,
            codeblock_width_exp: self.codeblock_exp,
            codeblock_height_exp: self.codeblock_exp,
            transformation,
            precinct_sizes: Vec::new(),
        };
        writer.write_cod(&cod)?;

        // Create QCD marker with proper guard bits
        let num_subbands = 1 + 3 * decomposition_levels as usize;
        let guard_bits = 2u8;

        // For reversible transform, use no quantization (style 0)
        let step_sizes: Vec<u16> = (0..num_subbands)
            .map(|i| {
                let epsilon = if i == 0 {
                    depth + guard_bits
                } else {
                    depth + guard_bits + 1
                };
                (epsilon as u16) << 11
            })
            .collect();

        let qcd = J2kQcd {
            quant_style: guard_bits << 5,
            step_sizes,
        };
        writer.write_qcd(&qcd)?;

        // Calculate codeblock size
        let cb_size = 1usize << (self.codeblock_exp + 2);

        // Transform and encode each component
        let mut packets: Vec<Packet> = Vec::new();

        // Level shift
        let level_shift = (1i32 << (depth - 1)) as i32;
        let mut component_data: Vec<Vec<i32>> = (0..components)
            .map(|c| {
                (0..width * height)
                    .map(|i| {
                        let val = if depth > 8 {
                            let idx = (i * components + c) * 2;
                            // Assume Little Endian input for 16-bit
                            let b0 = pixels[idx] as i32;
                            let b1 = pixels[idx + 1] as i32;
                            (b1 << 8) | b0
                        } else {
                            pixels[i * components + c] as i32
                        };
                        val - level_shift
                    })
                    .collect()
            })
            .collect();

        // Apply RCT (Reversible Color Transform) if 3 components and not using irreversible transform
        if components == 3 && !self.use_irreversible {
            for i in 0..width * height {
                let r = component_data[0][i];
                let g = component_data[1][i];
                let b = component_data[2][i];

                let y = (r + 2 * g + b) >> 2;
                let u = b - g;
                let v = r - g;

                component_data[0][i] = y;
                component_data[1][i] = u;
                component_data[2][i] = v;
            }
        }
        // Apply ICT (Irreversible Color Transform) if 3 components and using irreversible transform
        else if components == 3 && self.use_irreversible {
            // ICT implementation (floating point usually, but let's stick to integer approx or skip for now)
            // For now, let's just stick to RGB or implement simplified ICT if needed.
            // Given the user wants Lossless, RCT is the priority.
        }

        for (comp_idx, mut comp_data) in component_data.into_iter().enumerate() {
            // Apply forward 2D DWT
            let coeffs = self.apply_forward_dwt_2d(&mut comp_data, width, height)?;

            // Encode component into packets
            let comp_packets = self.encode_component_packets(
                &coeffs,
                width,
                height,
                cb_size,
                decomposition_levels,
                depth,
                guard_bits,
                comp_idx as u8,
            )?;
            packets.extend(comp_packets);
        }

        // Sort packets by LRCP (Layer, Resolution, Component, Precinct)
        // Currently only 1 layer, 1 precinct.
        // So Sort Key: (layer, resolution, component)
        packets.sort_by(|a, b| {
            a.layer
                .cmp(&b.layer)
                .then(a.resolution.cmp(&b.resolution))
                .then(a.component.cmp(&b.component))
        });

        // Write SOT (Start of Tile)
        // Calculate total length first
        let total_packet_len: usize = packets
            .iter()
            .map(|p| p.header_data.len() + p.body_data.len())
            .sum();
        let tile_total_len = 12 + 2 + total_packet_len as u32; // SOT + SOD + Data

        writer.write_sot(0, tile_total_len, 0, 1)?;

        // Write SOD (Start of Data)
        writer.write_sod()?;

        // Write packet data
        for p in packets {
            writer.write_bytes(&p.header_data)?;
            writer.write_bytes(&p.body_data)?;
        }

        // Write EOC (End of Codestream)
        writer.write_eoc()?;

        Ok(writer.len())
    }

    /// Apply forward 2D DWT using 5-3 reversible transform
    fn apply_forward_dwt_2d(
        &self,
        data: &mut [i32],
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

            // Apply 1D DWT to rows
            for y in 0..current_h {
                let row_start = y * width;
                let row: Vec<i32> = result[row_start..row_start + current_w].to_vec();

                let l_len = (current_w + 1) / 2;
                let h_len = current_w / 2;
                let mut out_l = vec![0i32; l_len];
                let mut out_h = vec![0i32; h_len];

                Dwt53::forward(&row, &mut out_l, &mut out_h);

                for (i, &v) in out_l.iter().enumerate() {
                    result[row_start + i] = v;
                }
                for (i, &v) in out_h.iter().enumerate() {
                    result[row_start + l_len + i] = v;
                }
            }

            // Apply 1D DWT to columns
            for x in 0..current_w {
                let col: Vec<i32> = (0..current_h).map(|y| result[y * width + x]).collect();

                let l_len = (current_h + 1) / 2;
                let h_len = current_h / 2;
                let mut out_l = vec![0i32; l_len];
                let mut out_h = vec![0i32; h_len];

                Dwt53::forward(&col, &mut out_l, &mut out_h);

                for (i, &v) in out_l.iter().enumerate() {
                    result[i * width + x] = v;
                }
                for (i, &v) in out_h.iter().enumerate() {
                    result[(l_len + i) * width + x] = v;
                }
            }

            current_w = (current_w + 1) / 2;
            current_h = (current_h + 1) / 2;
        }

        Ok(result)
    }

    /// Encode a component's coefficients into packets (internal)
    fn encode_component_packets(
        &self,
        coeffs: &[i32],
        width: usize,
        height: usize,
        cb_size: usize,
        num_levels: u8,
        depth: u8,
        guard_bits: u8,
        comp_idx: u8,
    ) -> Result<Vec<Packet>, JpeglsError> {
        let mut packets = Vec::new();
        let num_resolutions = (num_levels + 1) as usize;

        // Iterate through resolutions (lowest to highest)
        for res in 0..num_resolutions {
            let (ll_w, ll_h) = self.get_ll_size(width, height, num_levels as usize, res);

            // For now, assume 1 precinct per resolution
            let cb_log2 = self.codeblock_exp;
            let cb_dim = 1 << (cb_log2 + 2); // 64

            // Calculate grid dimensions (in codeblocks) based on subband size
            // We use the largest subband size to size the precinct grid
            let (sb_w, sb_h) = if res == 0 {
                (ll_w, ll_h)
            } else {
                // HL/LH/HH are roughly same size as previous LL
                let (prev_w, prev_h) =
                    self.get_ll_size(width, height, num_levels as usize, res - 1);
                (ll_w - prev_w, ll_h - prev_h) // Approximate
            };

            let grid_w = (sb_w + cb_dim - 1) / cb_dim;
            let grid_h = (sb_h + cb_dim - 1) / cb_dim;

            let mut precinct_state = PrecinctState::new(grid_w, grid_h);
            let mut packet_header = PacketHeader {
                packet_seq_num: 0, // Ignored in structure
                empty: false,
                layer_index: 0,
                included_cblks: Vec::new(),
            };

            let mut packet_body = Vec::new();
            let num_bands = if res == 0 { 1 } else { 3 };

            for band in 0..num_bands {
                let sb_idx = if res == 0 { 0 } else { band }; // 0, 1, 2

                let (sb_coeffs, sb_w, sb_h) = self.extract_subband_coeffs(
                    coeffs,
                    width,
                    height,
                    num_levels as usize,
                    res,
                    sb_idx,
                );

                // Calculate epsilon for this subband (matches write_qcd)
                let is_ll = res == 0;
                let epsilon = if is_ll {
                    depth + guard_bits
                } else {
                    depth + guard_bits + 1
                };

                for cby in 0..grid_h {
                    for cbx in 0..grid_w {
                        let x0 = cbx * cb_dim;
                        let y0 = cby * cb_dim;

                        if x0 >= sb_w || y0 >= sb_h {
                            continue;
                        }

                        let x1 = (x0 + cb_dim).min(sb_w);
                        let y1 = (y0 + cb_dim).min(sb_h);
                        let bw = x1 - x0;
                        let bh = y1 - y0;

                        if bw == 0 || bh == 0 {
                            continue;
                        }

                        let mut block_data = Vec::with_capacity(bw * bh);
                        for y in 0..bh {
                            for x in 0..bw {
                                block_data.push(sb_coeffs[(y0 + y) * sb_w + (x0 + x)]);
                            }
                        }

                        let mut bpc = BitPlaneCoder::new(bw as u32, bh as u32, &block_data);
                        if let Some(max_bp) = bpc.calculate_max_bit_plane() {
                            // Map band 0..2 to orientation 1..3?
                            // encoder.rs loop: band 0..num_bands.
                            // if res=0, band=0 (LL -> orient 0).
                            // if res>0, band 0..2 (HL, LH, HH -> orient 1, 2, 3).
                            let orientation = if res == 0 { 0 } else { band as u8 + 1 };

                            // DEBUG:
                            if std::env::var("J2K_DEBUG").is_ok() {
                                let max_val = block_data.iter().map(|v| v.abs()).max().unwrap_or(0);
                                eprintln!(
                                    "ENC: CB[{},{}] res={} band={} orient={} max_val={} max_bp={}",
                                    cbx, cby, res, band, orientation, max_val, max_bp
                                );
                            }

                            let passes = bpc.encode_codeblock(max_bp, orientation);
                            bpc.mq.flush();
                            let encoded = bpc.mq.get_buffer();

                            if std::env::var("J2K_DEBUG").is_ok() {
                                eprintln!(
                                    "Enc CB[{},{}] band={} len={} max_bp={} passes={}",
                                    cbx,
                                    cby,
                                    band,
                                    encoded.len(),
                                    max_bp,
                                    passes
                                );
                            }

                            // zero_bp calculation
                            // M_b = G + epsilon - 1
                            let mb = (guard_bits + epsilon).saturating_sub(1);

                            let zero_bp = if max_bp < mb { mb - max_bp } else { 0 };

                            if std::env::var("J2K_DEBUG").is_ok() {
                                eprintln!("    -> zero_bp={}", zero_bp);
                            }

                            packet_header
                                .included_cblks
                                .push(super::packet::CodeBlockInfo {
                                    x: cbx,
                                    y: cby,
                                    subband_index: band as u8,
                                    included: true,
                                    num_passes: passes,
                                    data_len: encoded.len() as u32,
                                    zero_bp,
                                });

                            packet_body.extend_from_slice(encoded);
                        }
                    }
                }
            }

            // Write Packet
            if packet_header.included_cblks.is_empty() {
                packet_header.empty = true;
            }

            let mut header_writer = J2kBitWriter::new();
            packet_header.write(
                &mut header_writer,
                &mut precinct_state,
                grid_w,
                grid_h,
                num_bands,
            );

            packets.push(Packet {
                resolution: res as u8,
                component: comp_idx,
                layer: 0,
                header_data: header_writer.finish(),
                body_data: packet_body,
            });
        }

        Ok(packets)
    }

    /// Get LL subband size at a given resolution level
    /// This matches the iterative ceiling division used in the forward DWT
    fn get_ll_size(
        &self,
        width: usize,
        height: usize,
        num_levels: usize,
        res: usize,
    ) -> (usize, usize) {
        // Must use iterative ceiling division to match the actual DWT
        // The forward DWT uses: current_w = (current_w + 1) / 2 at each level
        let levels_remaining = num_levels - res;
        let mut w = width;
        let mut h = height;
        for _ in 0..levels_remaining {
            w = (w + 1) / 2; // Ceiling division, matching DWT
            h = (h + 1) / 2;
        }
        (w.max(1), h.max(1))
    }

    /// Extract subband coefficients from the full coefficient array
    fn extract_subband_coeffs(
        &self,
        coeffs: &[i32],
        width: usize,
        height: usize,
        num_levels: usize,
        res: usize,
        sb_idx: usize,
    ) -> (Vec<i32>, usize, usize) {
        // For resolution 0, return LL coefficients
        if res == 0 {
            let (ll_w, ll_h) = self.get_ll_size(width, height, num_levels, 0);
            let mut ll_coeffs = Vec::with_capacity(ll_w * ll_h);
            for y in 0..ll_h {
                for x in 0..ll_w {
                    if y * width + x < coeffs.len() {
                        ll_coeffs.push(coeffs[y * width + x]);
                    } else {
                        ll_coeffs.push(0);
                    }
                }
            }
            return (ll_coeffs, ll_w, ll_h);
        }

        // For higher resolutions, extract HL, LH, or HH
        let (ll_w, ll_h) = self.get_ll_size(width, height, num_levels, res);
        let (prev_ll_w, prev_ll_h) = self.get_ll_size(width, height, num_levels, res - 1);

        let (sb_w, sb_h, start_x, start_y) = match sb_idx {
            0 => {
                // HL (right of LL)
                (ll_w - prev_ll_w, prev_ll_h, prev_ll_w, 0)
            }
            1 => {
                // LH (below LL)
                (prev_ll_w, ll_h - prev_ll_h, 0, prev_ll_h)
            }
            2 => {
                // HH (diagonal)
                (ll_w - prev_ll_w, ll_h - prev_ll_h, prev_ll_w, prev_ll_h)
            }
            _ => (0, 0, 0, 0),
        };

        let mut sb_coeffs = Vec::with_capacity(sb_w * sb_h);
        for y in 0..sb_h {
            for x in 0..sb_w {
                let src_x = start_x + x;
                let src_y = start_y + y;
                if src_y * width + src_x < coeffs.len() {
                    sb_coeffs.push(coeffs[src_y * width + src_x]);
                } else {
                    sb_coeffs.push(0);
                }
            }
        }

        (sb_coeffs, sb_w, sb_h)
    }
}

impl Default for J2kEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_creates_valid_codestream() {
        let width = 8;
        let height = 8;
        let components = 1;

        let mut pixels = vec![0u8; width * height * components];
        for y in 0..height {
            for x in 0..width {
                pixels[y * width + x] = ((x + y) * 16).min(255) as u8;
            }
        }

        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: components as i32,
        };

        let mut encoded = vec![0u8; 4096];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);

        let result = encoder.encode(&pixels, &frame_info, &mut encoded);
        assert!(result.is_ok(), "Encoding should succeed");

        let len = result.unwrap();
        assert!(len > 0, "Should produce non-empty output");

        assert_eq!(encoded[0], 0xFF, "Should start with FF");
        assert_eq!(encoded[1], 0x4F, "Should have SOC marker");
        assert_eq!(encoded[len - 2], 0xFF, "Should end with FF");
        assert_eq!(encoded[len - 1], 0xD9, "Should have EOC marker");
    }

    #[test]
    fn test_forward_dwt_produces_output() {
        let width = 8;
        let height = 8;
        let mut data: Vec<i32> = (0..width * height).map(|i| (i % 256) as i32).collect();

        let encoder = J2kEncoder::new();
        let result = encoder.apply_forward_dwt_2d(&mut data, width, height);

        assert!(result.is_ok());
        let coeffs = result.unwrap();
        assert_eq!(coeffs.len(), width * height);
    }
}

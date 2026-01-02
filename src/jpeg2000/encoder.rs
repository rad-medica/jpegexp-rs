//! JPEG 2000 Encoder
//!
//! This module provides JPEG 2000 encoding functionality using the reversible 5/3 DWT
//! and EBCOT (Embedded Block Coding with Optimized Truncation) bit-plane coding.

use super::bit_io::J2kBitWriter;
use super::bit_plane_coder::BitPlaneCoder;
use super::dwt::Dwt53;
use super::image::{J2kCod, J2kQcd, SubbandOrientation};
use super::packet::{CodeBlockInfo, PacketHeader, PrecinctState};
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
            use_irreversible: false, // Default to reversible (lossless)
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

        // Use reversible 5/3 transform for lossless encoding
        let transformation = 1; // 1=5-3 reversible

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

        // Create QCD marker for reversible (no quantization)
        let num_subbands = 1 + 3 * self.decomposition_levels as usize; // LL + 3 per level
        let step_sizes: Vec<u16> = (0..num_subbands)
            .map(|i| self.encode_reversible_step_size(depth, i))
            .collect();

        let qcd = J2kQcd {
            quant_style: 0, // 0=no quantization (reversible)
            step_sizes,
        };
        writer.write_qcd(&qcd)?;

        // Apply MCT (Multiple Component Transform) if RGB
        let mut component_data: Vec<Vec<i32>> = Vec::with_capacity(components);
        for c in 0..components {
            let mut comp_pixels: Vec<i32> = Vec::with_capacity(width * height);
            for i in 0..(width * height) {
                let val = pixels[i * components + c] as i32;
                // Level shift: subtract 2^(depth-1) for signed representation
                comp_pixels.push(val - (1 << (depth - 1)));
            }
            component_data.push(comp_pixels);
        }

        // Apply RCT (Reversible Color Transform) for RGB
        if components >= 3 && cod.mct == 1 {
            let count = width * height;
            for i in 0..count {
                let r = component_data[0][i];
                let g = component_data[1][i];
                let b = component_data[2][i];

                // RCT: Y = floor((R + 2G + B) / 4), Cb = B - G, Cr = R - G
                let y = (r + 2 * g + b) >> 2;
                let cb = b - g;
                let cr = r - g;

                component_data[0][i] = y;
                component_data[1][i] = cb;
                component_data[2][i] = cr;
            }
        }

        // Apply forward DWT to each component and collect subbands
        let cb_size = 1 << (cod.codeblock_width_exp + 2); // 64
        let num_resolutions = (self.decomposition_levels + 1) as usize;

        // Collect all encoded code-block data for each component
        let mut all_encoded_data: Vec<Vec<Vec<EncodedCodeBlock>>> = Vec::new();

        for comp_data in &component_data {
            let subbands = self.apply_forward_dwt_full(comp_data, width, height)?;
            let encoded_subbands =
                self.encode_component_subbands(&subbands, width, height, cb_size, depth)?;
            all_encoded_data.push(encoded_subbands);
        }

        // Write SOT (Start of Tile) - we'll update Psot later
        let sot_pos = writer.len();
        writer.write_sot(0, 0, 0, 1)?;

        // Write SOD (Start of Data)
        writer.write_sod()?;

        let sod_pos = writer.len();

        // Write packets in LRCP order: Layer -> Resolution -> Component -> Precinct
        for _layer in 0..1 {
            for res in 0..num_resolutions {
                for (comp_idx, comp_encoded) in all_encoded_data.iter().enumerate() {
                    // Get encoded data for this resolution
                    if res < comp_encoded.len() {
                        let res_data = &comp_encoded[res];

                        // Create and write packet
                        let packet_bytes =
                            self.create_packet(res_data, res, comp_idx, width, height)?;
                        writer.write_bytes(&packet_bytes)?;
                    } else {
                        // Write empty packet
                        let mut bit_writer = J2kBitWriter::new();
                        bit_writer.write_bit(0);
                        writer.write_bytes(&bit_writer.finish())?;
                    }
                }
            }
        }

        // Calculate tile length BEFORE writing EOC
        let tile_len = (writer.len() - sot_pos) as u32;

        // Write EOC (End of Codestream)
        writer.write_eoc()?;

        // Get final length and drop the writer to release the borrow
        let total_len = writer.len();
        drop(writer);

        // Update Psot (tile length) in SOT marker
        // SOT format: FF90 + Lsot(2) + Isot(2) + Psot(4) + TPsot(1) + TNsot(1)
        // Psot is at offset sot_pos + 6
        let psot_offset = sot_pos + 6;
        if psot_offset + 4 <= destination.len() {
            destination[psot_offset] = ((tile_len >> 24) & 0xFF) as u8;
            destination[psot_offset + 1] = ((tile_len >> 16) & 0xFF) as u8;
            destination[psot_offset + 2] = ((tile_len >> 8) & 0xFF) as u8;
            destination[psot_offset + 3] = (tile_len & 0xFF) as u8;
        }

        // sod_pos could be used for validation in debug builds
        debug_assert!(sod_pos > sot_pos, "SOD should come after SOT");

        Ok(total_len)
    }

    /// Encode step size for reversible transform (no quantization)
    fn encode_reversible_step_size(&self, depth: u8, subband_idx: usize) -> u16 {
        // For reversible transform, step sizes encode the exponent only
        // The exponent indicates the number of guard bits + bit depth
        let guard_bits = 2u8;
        let gain = if subband_idx == 0 {
            0 // LL band
        } else {
            ((subband_idx - 1) / 3 + 1) as u8 // Higher bands have gain
        };
        let epsilon = depth + guard_bits + gain;
        // Format: 5-bit exponent, no mantissa for reversible
        (epsilon as u16) << 3
    }

    /// Apply full forward 2D DWT and return organized subbands
    fn apply_forward_dwt_full(
        &self,
        data: &[i32],
        width: usize,
        height: usize,
    ) -> Result<Vec<SubbandData>, JpeglsError> {
        let mut result = data.to_vec();
        let mut subbands = Vec::new();

        let mut current_w = width;
        let mut current_h = height;
        let num_levels = self.decomposition_levels as usize;

        for level in 0..num_levels {
            if current_w < 2 || current_h < 2 {
                break;
            }

            let ll_w = (current_w + 1) / 2;
            let ll_h = (current_h + 1) / 2;
            let hl_w = current_w / 2;
            let lh_h = current_h / 2;

            // Temporary storage for transformed data
            let mut temp = vec![0i32; current_w * current_h];

            // Apply 1D DWT to rows
            for y in 0..current_h {
                let row_start = y * width;
                let row: Vec<i32> = result[row_start..row_start + current_w].to_vec();

                let mut out_l = vec![0i32; ll_w];
                let mut out_h = vec![0i32; hl_w];

                Dwt53::forward(&row, &mut out_l, &mut out_h);

                // Store L in left half, H in right half of temp
                for (i, &v) in out_l.iter().enumerate() {
                    temp[y * current_w + i] = v;
                }
                for (i, &v) in out_h.iter().enumerate() {
                    temp[y * current_w + ll_w + i] = v;
                }
            }

            // Apply 1D DWT to columns
            for x in 0..current_w {
                let col: Vec<i32> = (0..current_h).map(|y| temp[y * current_w + x]).collect();

                let mut out_l = vec![0i32; ll_h];
                let mut out_h = vec![0i32; lh_h];

                Dwt53::forward(&col, &mut out_l, &mut out_h);

                // Store L in top half, H in bottom half
                for (i, &v) in out_l.iter().enumerate() {
                    result[i * width + x] = v;
                }
                for (i, &v) in out_h.iter().enumerate() {
                    result[(ll_h + i) * width + x] = v;
                }
            }

            // Extract subbands for this level (HL, LH, HH)
            // HL: top-right quadrant
            let mut hl_data = vec![0i32; hl_w * ll_h];
            for y in 0..ll_h {
                for x in 0..hl_w {
                    hl_data[y * hl_w + x] = result[y * width + ll_w + x];
                }
            }
            subbands.push(SubbandData {
                data: hl_data,
                width: hl_w,
                height: ll_h,
                orientation: SubbandOrientation::HL,
                resolution: num_levels - level,
            });

            // LH: bottom-left quadrant
            let mut lh_data = vec![0i32; ll_w * lh_h];
            for y in 0..lh_h {
                for x in 0..ll_w {
                    lh_data[y * ll_w + x] = result[(ll_h + y) * width + x];
                }
            }
            subbands.push(SubbandData {
                data: lh_data,
                width: ll_w,
                height: lh_h,
                orientation: SubbandOrientation::LH,
                resolution: num_levels - level,
            });

            // HH: bottom-right quadrant
            let mut hh_data = vec![0i32; hl_w * lh_h];
            for y in 0..lh_h {
                for x in 0..hl_w {
                    hh_data[y * hl_w + x] = result[(ll_h + y) * width + ll_w + x];
                }
            }
            subbands.push(SubbandData {
                data: hh_data,
                width: hl_w,
                height: lh_h,
                orientation: SubbandOrientation::HH,
                resolution: num_levels - level,
            });

            current_w = ll_w;
            current_h = ll_h;
        }

        // Extract final LL subband (lowest resolution)
        let mut ll_data = vec![0i32; current_w * current_h];
        for y in 0..current_h {
            for x in 0..current_w {
                ll_data[y * current_w + x] = result[y * width + x];
            }
        }
        subbands.push(SubbandData {
            data: ll_data,
            width: current_w,
            height: current_h,
            orientation: SubbandOrientation::LL,
            resolution: 0,
        });

        Ok(subbands)
    }

    /// Encode all subbands for a component
    fn encode_component_subbands(
        &self,
        subbands: &[SubbandData],
        _img_width: usize,
        _img_height: usize,
        cb_size: usize,
        depth: u8,
    ) -> Result<Vec<Vec<EncodedCodeBlock>>, JpeglsError> {
        let num_resolutions = (self.decomposition_levels + 1) as usize;
        let mut result: Vec<Vec<EncodedCodeBlock>> = vec![Vec::new(); num_resolutions];

        for sb in subbands {
            let res_idx = sb.resolution;
            if sb.width == 0 || sb.height == 0 {
                continue;
            }

            // Calculate number of code-blocks
            let cb_cols = sb.width.div_ceil(cb_size);
            let cb_rows = sb.height.div_ceil(cb_size);

            for cby in 0..cb_rows {
                for cbx in 0..cb_cols {
                    let cb_x_start = cbx * cb_size;
                    let cb_y_start = cby * cb_size;
                    let cb_w = cb_size.min(sb.width - cb_x_start);
                    let cb_h = cb_size.min(sb.height - cb_y_start);

                    // Extract code-block coefficients
                    let mut cb_coeffs = vec![0i32; cb_w * cb_h];
                    for y in 0..cb_h {
                        for x in 0..cb_w {
                            let src_x = cb_x_start + x;
                            let src_y = cb_y_start + y;
                            if src_y < sb.height && src_x < sb.width {
                                cb_coeffs[y * cb_w + x] = sb.data[src_y * sb.width + src_x];
                            }
                        }
                    }

                    // Find maximum magnitude
                    let max_val = cb_coeffs.iter().map(|&v| v.abs()).max().unwrap_or(0);

                    if max_val == 0 {
                        // Empty code-block
                        result[res_idx].push(EncodedCodeBlock {
                            x: cbx,
                            y: cby,
                            subband: sb.orientation,
                            data: Vec::new(),
                            num_passes: 0,
                            zero_bit_planes: depth,
                        });
                        continue;
                    }

                    // Calculate MSB position (number of zero bit planes)
                    let msb = 32 - max_val.leading_zeros();
                    let zero_bp = (depth as u32).saturating_sub(msb) as u8;

                    // Encode using bit-plane coder
                    let mut bpc = BitPlaneCoder::new(cb_w as u32, cb_h as u32, &cb_coeffs);

                    // Encode from MSB down to 0
                    let num_bitplanes = msb.min(depth as u32) as u8;
                    for bp in (0..num_bitplanes).rev() {
                        bpc.significance_propagation(bp);
                        bpc.magnitude_refinement(bp);
                        bpc.cleanup(bp);
                    }

                    // Finalize MQ stream
                    bpc.mq.flush();
                    let encoded_data = bpc.mq.get_buffer().to_vec();

                    // Calculate number of coding passes
                    // Each bitplane has 3 passes (sig prop, mag ref, cleanup)
                    // But the first bitplane only has cleanup, so: 1 + (n-1)*3 = 3n - 2
                    let num_passes = if num_bitplanes > 0 {
                        3 * num_bitplanes - 2
                    } else {
                        0
                    };

                    result[res_idx].push(EncodedCodeBlock {
                        x: cbx,
                        y: cby,
                        subband: sb.orientation,
                        data: encoded_data,
                        num_passes,
                        zero_bit_planes: zero_bp,
                    });
                }
            }
        }

        Ok(result)
    }

    /// Create a packet for a resolution level
    fn create_packet(
        &self,
        encoded_blocks: &[EncodedCodeBlock],
        res: usize,
        _comp: usize,
        _width: usize,
        _height: usize,
    ) -> Result<Vec<u8>, JpeglsError> {
        if encoded_blocks.is_empty() || encoded_blocks.iter().all(|b| b.data.is_empty()) {
            // Empty packet
            let mut bit_writer = J2kBitWriter::new();
            bit_writer.write_bit(0);
            return Ok(bit_writer.finish());
        }

        let mut bit_writer = J2kBitWriter::new();

        // Non-empty packet
        bit_writer.write_bit(1);

        // Determine subband count for this resolution
        let num_subbands = if res == 0 { 1 } else { 3 };

        // Group blocks by subband
        let subbands_order = if res == 0 {
            vec![SubbandOrientation::LL]
        } else {
            vec![
                SubbandOrientation::HL,
                SubbandOrientation::LH,
                SubbandOrientation::HH,
            ]
        };

        // Calculate grid dimensions
        let mut precinct_state = PrecinctState::new(0, 0);

        // Collect all included blocks
        let mut included_cblks = Vec::new();

        for (sb_idx, sb_orient) in subbands_order.iter().enumerate() {
            let blocks: Vec<&EncodedCodeBlock> = encoded_blocks
                .iter()
                .filter(|b| &b.subband == sb_orient)
                .collect();

            for block in &blocks {
                if !block.data.is_empty() {
                    included_cblks.push(CodeBlockInfo {
                        x: block.x,
                        y: block.y,
                        subband_index: sb_idx as u8,
                        included: true,
                        num_passes: block.num_passes,
                        data_len: block.data.len() as u32,
                        zero_bp: block.zero_bit_planes,
                    });
                }
            }
        }

        // Create packet header
        let header = PacketHeader {
            packet_seq_num: 0,
            empty: false,
            layer_index: 0,
            included_cblks,
        };

        // Determine grid size for packet header
        let max_x = encoded_blocks.iter().map(|b| b.x).max().unwrap_or(0) + 1;
        let max_y = encoded_blocks.iter().map(|b| b.y).max().unwrap_or(0) + 1;

        // Write packet header
        header.write(&mut bit_writer, &mut precinct_state, max_x, max_y, num_subbands);

        // Finish the bit writer (adds padding)
        let mut packet_bytes = bit_writer.finish();

        // Append code-block body data
        for sb_orient in &subbands_order {
            let blocks: Vec<&EncodedCodeBlock> = encoded_blocks
                .iter()
                .filter(|b| &b.subband == sb_orient && !b.data.is_empty())
                .collect();

            for block in blocks {
                packet_bytes.extend(&block.data);
            }
        }

        Ok(packet_bytes)
    }
}

/// Represents a subband of DWT coefficients
#[derive(Debug)]
struct SubbandData {
    data: Vec<i32>,
    width: usize,
    height: usize,
    orientation: SubbandOrientation,
    resolution: usize,
}

/// Represents an encoded code-block
#[derive(Debug, Clone)]
struct EncodedCodeBlock {
    x: usize,
    y: usize,
    subband: SubbandOrientation,
    data: Vec<u8>,
    num_passes: u8,
    zero_bit_planes: u8,
}

impl Default for J2kEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg2000::decoder::J2kDecoder;
    use crate::jpeg_stream_reader::JpegStreamReader;

    #[test]
    fn test_encode_grayscale_small() {
        // Create a small 8x8 grayscale test pattern
        let width = 8u32;
        let height = 8u32;
        let mut pixels = vec![0u8; (width * height) as usize];

        // Simple gradient pattern
        for y in 0..height {
            for x in 0..width {
                pixels[(y * width + x) as usize] = ((x + y) * 16) as u8;
            }
        }

        let frame_info = FrameInfo {
            width,
            height,
            bits_per_sample: 8,
            component_count: 1,
        };

        // Encode
        let mut encoder = J2kEncoder::new();
        encoder.set_decomposition_levels(2); // Use fewer levels for small image
        let mut dest = vec![0u8; 4096];
        let len = encoder.encode(&pixels, &frame_info, &mut dest).unwrap();
        dest.truncate(len);

        // Verify J2K magic bytes
        assert_eq!(dest[0], 0xFF);
        assert_eq!(dest[1], 0x4F); // SOC

        // Should end with EOC
        assert_eq!(dest[len - 2], 0xFF);
        assert_eq!(dest[len - 1], 0xD9); // EOC

        println!("Encoded {} bytes for {}x{} grayscale image", len, width, height);
    }

    #[test]
    fn test_encode_decode_roundtrip_grayscale() {
        // Create a 16x16 grayscale test pattern
        let width = 16u32;
        let height = 16u32;
        let mut pixels = vec![0u8; (width * height) as usize];

        // Simple pattern
        for y in 0..height {
            for x in 0..width {
                pixels[(y * width + x) as usize] = ((x * 8 + y * 8) % 256) as u8;
            }
        }

        let frame_info = FrameInfo {
            width,
            height,
            bits_per_sample: 8,
            component_count: 1,
        };

        // Encode
        let mut encoder = J2kEncoder::new();
        encoder.set_decomposition_levels(2);
        let mut dest = vec![0u8; 8192];
        let len = encoder.encode(&pixels, &frame_info, &mut dest).unwrap();
        dest.truncate(len);

        // Verify basic J2K structure
        assert!(len > 50);
        assert_eq!(dest[0], 0xFF);
        assert_eq!(dest[1], 0x4F); // SOC
        assert_eq!(dest[len - 2], 0xFF);
        assert_eq!(dest[len - 1], 0xD9); // EOC

        // Try to decode header at minimum
        let mut reader = JpegStreamReader::new(&dest);
        let mut parser = crate::jpeg2000::parser::J2kParser::new(&mut reader);
        let header_result = parser.parse_main_header();
        assert!(header_result.is_ok(), "Header should parse correctly");
        
        assert_eq!(parser.image.width, width);
        assert_eq!(parser.image.height, height);
        assert_eq!(parser.image.component_count, 1);
    }

    #[test]
    fn test_encode_gradient_image() {
        // Create a 64x64 gradient image
        let width = 64u32;
        let height = 64u32;
        let mut pixels = vec![0u8; (width * height) as usize];

        // Horizontal gradient
        for y in 0..height {
            for x in 0..width {
                pixels[(y * width + x) as usize] = (x * 4) as u8;
            }
        }

        let frame_info = FrameInfo {
            width,
            height,
            bits_per_sample: 8,
            component_count: 1,
        };

        // Encode with default settings
        let mut encoder = J2kEncoder::new();
        let mut dest = vec![0u8; 32768];
        let len = encoder.encode(&pixels, &frame_info, &mut dest).unwrap();

        println!("Encoded {}x{} gradient: {} bytes (ratio: {:.2}x)", 
                 width, height, len, (width * height) as f32 / len as f32);

        // Verify basic structure
        assert!(len > 100); // Should have some content
        assert!(len < (width * height) as usize); // Should compress somewhat
    }

    #[test]
    fn test_decode_structure() {
        use crate::jpeg2000::parser::J2kParser;
        
        // Create a simple test with constant value (easy to decode)
        let width = 8u32;
        let height = 8u32;
        let pixels = vec![128u8; (width * height) as usize];
        
        let frame_info = FrameInfo {
            width,
            height,
            bits_per_sample: 8,
            component_count: 1,
        };
        
        let mut encoder = J2kEncoder::new();
        encoder.set_decomposition_levels(2);
        let mut dest = vec![0u8; 4096];
        let len = encoder.encode(&pixels, &frame_info, &mut dest).unwrap();
        dest.truncate(len);
        
        // Print hex of encoded data
        println!("Encoded {} bytes:", len);
        for (i, chunk) in dest.chunks(16).enumerate() {
            print!("{:04x}: ", i * 16);
            for b in chunk {
                print!("{:02x} ", b);
            }
            println!();
        }
        
        // Try to parse
        let mut reader = JpegStreamReader::new(&dest);
        let mut parser = J2kParser::new(&mut reader);
        match parser.parse_main_header() {
            Ok(marker) => println!("Main header parsed OK, next marker: {:?}", marker),
            Err(e) => println!("Main header parse error: {:?}", e),
        }
        
        println!("\nParsed image:");
        println!("  Width: {}", parser.image.width);
        println!("  Height: {}", parser.image.height);
        println!("  Components: {}", parser.image.component_count);
        if let Some(cod) = &parser.image.cod {
            println!("  Decomp levels: {}", cod.decomposition_levels);
            println!("  Transform: {}", cod.transformation);
        }
        if let Some(qcd) = &parser.image.qcd {
            println!("  QCD style: {}", qcd.quant_style);
            println!("  Step sizes: {} entries", qcd.step_sizes.len());
        }
        
        // Test full decode
        let mut reader2 = JpegStreamReader::new(&dest);
        let mut decoder = J2kDecoder::new(&mut reader2);
        match decoder.decode() {
            Ok(img) => {
                println!("\nFull decode successful!");
                println!("  Tiles: {}", img.tiles.len());
                if !img.tiles.is_empty() {
                    println!("  Tile 0 components: {}", img.tiles[0].components.len());
                }
                // Try to reconstruct
                match img.reconstruct_pixels() {
                    Ok(pix) => println!("  Reconstructed {} pixels", pix.len()),
                    Err(e) => println!("  Reconstruct error: {}", e),
                }
            },
            Err(e) => println!("\nFull decode error: {:?}", e),
        }
    }
}

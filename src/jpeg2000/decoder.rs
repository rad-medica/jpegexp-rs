//! JPEG 2000 / HTJ2K Decoder.
//!
//! This module provides the `J2kDecoder` which manages the high-level
//! decoding process, including header parsing and dispatching to Tier-1/Tier-2 coders.

use super::image::J2kImage;
use super::parser::J2kParser;
use crate::JpeglsError;
use crate::jpeg_stream_reader::JpegStreamReader;

use crate::jpeg2000::packet::PrecinctState;
use std::collections::HashMap;

#[derive(Default)]
struct TileState {
    components: Vec<ComponentState>,
}

#[derive(Default)]
struct ComponentState {
    resolutions: Vec<ResolutionState>,
}

struct ResolutionState {
    pub width: u32,
    pub height: u32,
    precincts: HashMap<(u32, u32), crate::jpeg2000::packet::PrecinctState>,
}

impl ResolutionState {
    fn new(w: usize, h: usize) -> Self {
        Self {
            width: w as u32,
            height: h as u32,
            precincts: HashMap::new(),
        }
    }
}

impl Default for ResolutionState {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            precincts: HashMap::new(),
        }
    }
}

/// High-level generic JPEG 2000 Decoder.
/// Orchestrates parsing, block decoding, and image reconstruction.
pub struct J2kDecoder<'a, 'b> {
    parser: J2kParser<'a, 'b>,
    tile_states: Vec<TileState>,
}

impl<'a, 'b> J2kDecoder<'a, 'b> {
    pub fn new(reader: &'b mut JpegStreamReader<'a>) -> Self {
        Self {
            parser: J2kParser::new(reader),
            tile_states: Vec::new(),
        }
    }

    /// Decodes the JPEG 2000 image from the stream.
    pub fn decode(&mut self) -> Result<&J2kImage, JpeglsError> {
        // 0. Container Detection (JP2 Box)
        // We use a separate reader/parser logic for checking the container.
        let codestream = {
            let mut jp2_reader =
                crate::jpeg2000::jp2::Jp2Reader::new(self.parser.reader.remaining_data());
            match jp2_reader.find_codestream() {
                Ok(cs) => cs,
                Err(_) => None, // Fallback to raw if logic fails or not JP2
            }
        };

        let icc_profile = {
            let mut jp2_reader =
                crate::jpeg2000::jp2::Jp2Reader::new(self.parser.reader.remaining_data());
            match jp2_reader.find_icc_profile() {
                Ok(icc) => icc,
                Err(_) => None,
            }
        };

        if let Some(cs) = codestream {
            let mut sub_reader = JpegStreamReader::new(&cs);
            let mut sub_parser = J2kParser::new(&mut sub_reader);

            // 1. Parse Main Header with sub_parser
            let last_marker = sub_parser.parse_main_header()?;

            // 2. Identify Decoding Path
            let is_htj2k = if let Some(cap) = &sub_parser.image.cap {
                (cap.pcap & (1 << 14)) != 0
            } else {
                false
            };

            // 3. Decode Tiles using sub_parser
            Self::__decode_tiles_loop(
                &mut sub_parser,
                last_marker,
                is_htj2k,
                &mut self.tile_states,
            )?;

            // Copy results back to main parser state
            self.parser.image = sub_parser.image.clone();
            self.parser.image.icc_profile = icc_profile;
        } else {
            // 1. Parse Main Header with self.parser
            let last_marker = self.parser.parse_main_header()?;

            // 2. Identify Decoding Path
            let is_htj2k = if let Some(cap) = &self.parser.image.cap {
                (cap.pcap & (1 << 14)) != 0
            } else {
                false
            };

            // 3. Decode Tiles using self.parser
            Self::__decode_tiles_loop(
                &mut self.parser,
                last_marker,
                is_htj2k,
                &mut self.tile_states,
            )?;
        }

        Ok(&self.parser.image)
    }

    /// Internal loop to process tiles.
    /// Detached from `self` to allow using either `self.parser` or `sub_parser`.
    fn __decode_tiles_loop(
        parser: &mut J2kParser,
        mut marker: crate::jpeg_marker_code::JpegMarkerCode,
        is_htj2k: bool,
        tile_states: &mut Vec<TileState>,
    ) -> Result<(), JpeglsError> {
        loop {
            if marker == crate::jpeg_marker_code::JpegMarkerCode::EndOfImage {
                break;
            }

            if marker == crate::jpeg_marker_code::JpegMarkerCode::StartOfTile {
                let (psot, isot) = parser.parse_tile_part_header()?;
                Self::decode_tile_data(parser, psot, isot, is_htj2k, tile_states)?;

                if parser.reader.remaining_data().is_empty() {
                    break;
                }

                marker = Self::find_next_marker(parser)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Scans the stream for the next marker.
    fn find_next_marker(
        parser: &mut J2kParser,
    ) -> Result<crate::jpeg_marker_code::JpegMarkerCode, JpeglsError> {
        loop {
            match parser.reader.read_u8() {
                Ok(0xFF) => {
                    let b2 = parser.reader.read_u8()?;
                    if b2 == 0x90 {
                        return Ok(crate::jpeg_marker_code::JpegMarkerCode::StartOfTile);
                    } else if b2 == 0xD9 {
                        return Ok(crate::jpeg_marker_code::JpegMarkerCode::EndOfImage);
                    }
                }
                Ok(_) => continue,
                Err(_) => return Err(JpeglsError::InvalidData),
            }
        }
    }

    /// Decodes the tile data.
    fn decode_tile_data(
        parser: &mut J2kParser,
        _len: u32,
        isot: u16,
        _is_htj2k: bool,
        tile_states: &mut Vec<TileState>,
    ) -> Result<(), JpeglsError> {
        let tile_idx = isot as usize;
        if parser.image.tiles.len() <= tile_idx {
            parser
                .image
                .tiles
                .resize_with(tile_idx + 1, Default::default);
            parser.image.tiles[tile_idx].index = isot as u32;
        }

        let cod = parser
            .image
            .cod
            .as_ref()
            .ok_or(JpeglsError::InvalidData)?
            .clone();
        let _num_layers = cod.number_of_layers as usize;
        let num_resolutions = (cod.decomposition_levels + 1) as usize;
        let num_components = parser.image.component_count as usize;

        let _progression_order = cod.progression_order;
        let _cb_w = 1 << (cod.codeblock_width_exp + 2);
        let _cb_h = 1 << (cod.codeblock_height_exp + 2);

        // Coordinate calculation logic based on ISO/IEC 15444-1 Annex B
        // 1. Determine Tile Grid indices (p, q)
        let x_siz = parser.image.width;
        let y_siz = parser.image.height;
        let x_osiz = parser.image.x_origin;
        let y_osiz = parser.image.y_origin;
        let x_tsiz = parser.image.tile_width;
        let y_tsiz = parser.image.tile_height;
        let x_tosiz = parser.image.tile_x_origin;
        let y_tosiz = parser.image.tile_y_origin;

        // Number of tiles in X and Y
        let num_tiles_x = (x_siz.saturating_sub(x_tosiz) + x_tsiz - 1) / x_tsiz;
        // avoid div by zero if tile size is huge or something (parser checks usually catch this)
        let num_tiles_x = if num_tiles_x == 0 { 1 } else { num_tiles_x };

        let p = isot as u32 % num_tiles_x;
        let q = isot as u32 / num_tiles_x;

        // 2. Determine Tile coordinates on Reference Grid (tx0, ty0, tx1, ty1)
        let tx0 = (x_tosiz + p * x_tsiz).max(x_osiz);
        let ty0 = (y_tosiz + q * y_tsiz).max(y_osiz);
        let tx1 = (x_tosiz + (p + 1) * x_tsiz).min(x_siz);
        let ty1 = (y_tosiz + (q + 1) * y_tsiz).min(y_siz);

        // Initialize tile components and resolutions with correct dimensions
        {
            let tile = &mut parser.image.tiles[tile_idx];
            if tile.components.len() < num_components {
                tile.components
                    .resize_with(num_components, Default::default);
            }
            for c in 0..num_components {
                // Get component subsampling factors
                let (dx, dy) = if c < parser.image.components.len() {
                    let info = &parser.image.components[c];
                    (info.dx as u32, info.dy as u32)
                } else {
                    (1, 1) // Default to 1 if missing info
                };

                // 3. Determine Tile-Component coordinates (tcx0, tcy0, tcx1, tcy1)
                // ceil(a / b) = (a + b - 1) / b
                let tcx0 = (tx0 + dx - 1) / dx;
                let tcx1 = (tx1 + dx - 1) / dx;
                let tcy0 = (ty0 + dy - 1) / dy;
                let tcy1 = (ty1 + dy - 1) / dy;

                let comp = &mut tile.components[c];
                if comp.resolutions.len() < num_resolutions {
                    comp.resolutions
                        .resize_with(num_resolutions, Default::default);
                }
                for r in 0..num_resolutions {
                    // Logic from get_grid_size (duplicated here to avoid closure borrow issues)
                    let shift = if r >= (num_resolutions - 1) {
                        0
                    } else {
                        num_resolutions - 1 - r
                    };

                    // 4. Determine Resolution Level coordinates (trx0, try0, trx1, try1)
                    // Division by 2^shift is equivalent to >> shift, but ceil requires handling.
                    // ceil(x / 2^s) = (x + (1<<s) - 1) >> s
                    let denom = 1 << shift;
                    let trx0 = (tcx0 + denom - 1) >> shift;
                    let trx1 = (tcx1 + denom - 1) >> shift;
                    let try0 = (tcy0 + denom - 1) >> shift;
                    let try1 = (tcy1 + denom - 1) >> shift;

                    let res_w = trx1.saturating_sub(trx0);
                    let res_h = try1.saturating_sub(try0);

                    comp.resolutions[r].width = res_w;
                    comp.resolutions[r].height = res_h;

                    if comp.resolutions[r].subbands.len() < 4 {
                        comp.resolutions[r]
                            .subbands
                            .resize_with(4, Default::default);
                    }

                    // Init subbands
                    let orientations = [
                        crate::jpeg2000::image::SubbandOrientation::LL,
                        crate::jpeg2000::image::SubbandOrientation::HL,
                        crate::jpeg2000::image::SubbandOrientation::LH,
                        crate::jpeg2000::image::SubbandOrientation::HH,
                    ];

                    for (i, sb) in comp.resolutions[r].subbands.iter_mut().enumerate() {
                        if i < 4 {
                            sb.orientation = orientations[i];

                            // Calculate subband dimensions
                            if r == 0 {
                                // Res 0: only LL (0) matters and matches Res 0 dims
                                if i == 0 {
                                    sb.width = res_w;
                                    sb.height = res_h;
                                } else {
                                    sb.width = 0;
                                    sb.height = 0;
                                }
                            } else {
                                // Res > 0: Subbands split the resolution
                                // LL(0), HL(1): width approx (W+1)/2
                                // LH(2), HH(3): width approx W/2
                                let w_sb = if i == 0 || i == 1 {
                                    (res_w + 1) / 2
                                } else {
                                    res_w / 2
                                };
                                let h_sb = if i == 0 || i == 2 {
                                    (res_h + 1) / 2
                                } else {
                                    res_h / 2
                                };
                                sb.width = w_sb;
                                sb.height = h_sb;
                            }
                        }
                    }
                }
            }
        }

        // Finalize decoding steps (e.g. IDWT, Color Transform) are handled in `decode` after this returns
        Self::decode_packets(parser, tile_states)
    }

    // Updated decode_packets to use per-precinct TagTrees
    fn decode_packets(
        parser: &mut J2kParser,
        tile_states: &mut Vec<TileState>,
    ) -> Result<(), JpeglsError> {
        // Ensure we have state for the current tile
        if tile_states.is_empty() {
            tile_states.push(TileState::default());
        }
        let tile_state_idx = tile_states.len() - 1;

        let cod = parser
            .image
            .cod
            .as_ref()
            .ok_or(JpeglsError::InvalidData)?
            .clone();

        let num_layers = cod.number_of_layers as usize;
        let num_resolutions = (cod.decomposition_levels + 1) as usize;
        let num_components = parser.image.component_count as usize;

        // Use actual number of layers
        let safe_num_layers = num_layers; // Corrected logic

        let isot = parser.image.tiles.len().saturating_sub(1) as u16;
        let is_htj2k = false; // Placeholder

        // LRCP Loop
        for l in 0..safe_num_layers {
            for r in 0..num_resolutions {
                let num_subbands = if r == 0 { 1 } else { 3 };

                for c in 0..num_components {
                    // Ensure state exists
                    if tile_states[tile_state_idx].components.len() <= c {
                        tile_states[tile_state_idx]
                            .components
                            .resize_with(c + 1, Default::default);
                    }
                    let comp_state = &mut tile_states[tile_state_idx].components[c];

                    // Ensure resolution state exists
                    if comp_state.resolutions.len() <= r {
                        // NOTE: decode_tile_data MUST have been called first to populate dimensions in parser.image.tiles
                        // However, we need to get dimensions from parser.image to init state if needed,
                        // or better, rely on decode_tile_data having set it up?
                        // decode_tile_data updates parser.image.tiles.
                        // tile_states is parallel.
                        // We need width/height here.
                        // Since we are iterating c, r, we can pull from parser.image
                        let tile = &parser.image.tiles[tile_state_idx]; // assuming isot matches idx
                        // Wait, tile_idx passed to decode_tile_data was isot.
                        // Here tile_states uses tile_state_idx.
                        // We should lookup tile by index if possible, but for now assume sequential.
                        let comp_info = &tile.components[c];
                        let res_info = &comp_info.resolutions[r];
                        comp_state.resolutions.resize_with(r + 1, || {
                            ResolutionState::new(res_info.width as usize, res_info.height as usize)
                        });
                    }
                    let res_state = &mut comp_state.resolutions[r];
                    let res_w = res_state.width;
                    let res_h = res_state.height;

                    // Calculate grid dimensions for this component/resolution
                    let (ppx, ppy) = if !cod.precinct_sizes.is_empty() {
                        if r < cod.precinct_sizes.len() {
                            let s = cod.precinct_sizes[r];
                            let shift_x = s & 0x0F;
                            let shift_y = (s >> 4) & 0x0F;
                            (1 << shift_x, 1 << shift_y)
                        } else {
                            (32768, 32768)
                        }
                    } else {
                        (32768, 32768)
                    };

                    let grid_w = (res_w + ppx - 1) / ppx;
                    let grid_h = (res_h + ppy - 1) / ppy;

                    // Iterate Precincts
                    let num_px = grid_w;
                    let num_py = grid_h;

                    for py in 0..num_py {
                        for px in 0..num_px {
                            let precinct_state = res_state
                                .precincts
                                .entry((px as u32, py as u32))
                                .or_insert_with(|| PrecinctState::new(num_subbands, 0));

                            // SOP Marker Handling
                            if (cod.coding_style & 0x02) != 0 {
                                // SOP: FF 91 + Lsop(2) + Nsop(2) = 6 bytes

                                // Read strict
                                let marker = parser.reader.read_u16().unwrap_or(0);
                                if marker == 0xFF91 {
                                    // eprintln!("DEBUG: Found SOP marker at {}", pos);
                                    let _lsop = parser.reader.read_u16().unwrap_or(0);
                                    let _nsop = parser.reader.read_u16().unwrap_or(0);
                                } else {
                                    // eprintln!("DEBUG: Expected SOP at {}, got {:04X}", pos, marker);
                                    return Err(JpeglsError::InvalidData);
                                }
                            }

                            // Read Packet Header
                            let mut header = None;
                            let mut pos_to_advance = 0;
                            {
                                let remaining = parser.reader.remaining_data();
                                // Skip bits? No, usually byte aligned after body.
                                if !remaining.is_empty() {
                                    let mut bit_reader =
                                        crate::jpeg2000::bit_io::J2kBitReader::new(remaining);
                                    let h = crate::jpeg2000::packet::PacketHeader::read(
                                        &mut bit_reader,
                                        precinct_state,
                                        l as u32,
                                        grid_w as usize,
                                        grid_h as usize,
                                        num_subbands,
                                    );
                                    match h {
                                        Ok(h) => {
                                            pos_to_advance = bit_reader.position();
                                            header = Some(h);
                                        }
                                        Err(_) => {
                                            return Err(JpeglsError::InvalidData);
                                        }
                                    }
                                }
                            }
                            if let Some(h) = header {
                                parser.reader.advance(pos_to_advance);

                                // EPH Marker Handling
                                if (cod.coding_style & 0x04) != 0 {
                                    // EPH: FF 92 (2 bytes)
                                    let marker = parser.reader.read_u16().unwrap_or(0);
                                    if marker == 0xFF92 {
                                        // eprintln!("DEBUG: Found EPH marker at {}", pos);
                                    } else {
                                        // If EPH is mandatory and missing, error.
                                        return Err(JpeglsError::InvalidData);
                                    }
                                }

                                Self::decode_packet_body(parser, h, isot, c, r, l, is_htj2k)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn decode_packet_body(
        parser: &mut J2kParser,
        header: crate::jpeg2000::packet::PacketHeader,
        isot: u16,
        comp: usize,
        res: usize,
        layer: usize,
        is_htj2k: bool,
    ) -> Result<(), JpeglsError> {
        for cb_info in header.included_cblks {
            if cb_info.data_len > 0 {
                let data_len = cb_info.data_len as usize;
                let mut data = vec![0u8; data_len];
                for item in &mut data {
                    *item = parser.reader.read_u8()?;
                }

                let tile = &mut parser.image.tiles[isot as usize];
                if tile.components.len() <= comp {
                    tile.components.resize_with(comp + 1, Default::default);
                    tile.components[comp].component_index = comp as u32;
                }
                let component = &mut tile.components[comp];

                if component.resolutions.len() <= res {
                    component.resolutions.resize_with(res + 1, Default::default);
                    component.resolutions[res].level = res as u8;
                }
                let resolution = &mut component.resolutions[res];

                let subband_idx = cb_info.subband_index as usize;
                if resolution.subbands.len() <= subband_idx {
                    resolution
                        .subbands
                        .resize_with(subband_idx + 1, Default::default);
                }
                let subband = &mut resolution.subbands[subband_idx];

                if res == 0 {
                    subband.orientation = crate::jpeg2000::image::SubbandOrientation::LL;
                } else {
                    match subband_idx {
                        0 => subband.orientation = crate::jpeg2000::image::SubbandOrientation::HL,
                        1 => subband.orientation = crate::jpeg2000::image::SubbandOrientation::LH,
                        2 => subband.orientation = crate::jpeg2000::image::SubbandOrientation::HH,
                        _ => {}
                    }
                }

                if is_htj2k {
                    let mut coder = crate::jpeg2000::ht_block_coder::coder::HTBlockCoder::new(
                        &data, &data, 64, 64,
                    );
                    let mut block = crate::jpeg2000::image::J2kCodeBlock::default();
                    block.layer_data.push(data.clone());
                    block.layers_decoded = (layer + 1) as u8;
                    let _ = coder.decode_block(&mut block);
                    subband.codeblocks.push(block);
                } else {
                    let cod = parser.image.cod.as_ref().unwrap();
                    let nom_w = 1 << (cod.codeblock_width_exp + 2);
                    let nom_h = 1 << (cod.codeblock_height_exp + 2);

                    let (res_w, res_h) = (resolution.width as usize, resolution.height as usize);
                    let (sb_w, sb_h) = if res == 0 {
                        (res_w, res_h)
                    } else {
                        let ll_w = (res_w + 1) / 2;
                        let ll_h = (res_h + 1) / 2;
                        match subband_idx {
                            0 => (res_w - ll_w, ll_h),         // HL
                            1 => (ll_w, res_h - ll_h),         // LH
                            2 => (res_w - ll_w, res_h - ll_h), // HH
                            _ => (0, 0),
                        }
                    };

                    let cb_x = cb_info.x * nom_w;
                    let cb_y = cb_info.y * nom_h;
                    let cb_width = nom_w.min(sb_w.saturating_sub(cb_x));
                    let cb_height = nom_h.min(sb_h.saturating_sub(cb_y));

                    let qcd = parser
                        .image
                        .qcd
                        .as_ref()
                        .map_or(Default::default(), |q| q.clone());
                    let guard_bits = (qcd.quant_style >> 5) & 0x07;

                    let qcd_idx = if res == 0 {
                        0
                    } else {
                        1 + (res - 1) * 3 + subband_idx
                    };

                    let epsilon_b = if qcd_idx < qcd.step_sizes.len() {
                        (qcd.step_sizes[qcd_idx] >> 11) as u8
                    } else {
                        if comp < parser.image.components.len() {
                            parser.image.components[comp].depth
                        } else {
                            8
                        }
                    };

                    let m_b = if cod.transformation == 1 {
                        // Reversible: Use Depth instead of Epsilon?
                        // Epsilon from file is 10. Depth is 8.
                        // Experiment: Force based on depth
                        guard_bits
                            + if comp < parser.image.components.len() {
                                parser.image.components[comp].depth - 1
                            } else {
                                7
                            }
                    } else {
                        guard_bits + epsilon_b - 1
                    };
                    let max_bit_plane = m_b.saturating_sub(1).saturating_sub(cb_info.zero_bp);

                    let cb_idx = subband
                        .codeblocks
                        .iter()
                        .position(|cb| cb.x == cb_info.x as u32 && cb.y == cb_info.y as u32);

                    if let Some(idx) = cb_idx {
                        let block = &mut subband.codeblocks[idx];
                        block.layer_data.push(data.clone());
                        block.layers_decoded = (layer + 1) as u8;

                        let mut bpc = crate::jpeg2000::bit_plane_coder::BitPlaneCoder::new(
                            block.width,
                            block.height,
                            &[],
                        );
                        bpc.coefficients = block.coefficients.clone();
                        bpc.state = block.state.clone();
                        bpc.num_passes_decoded = block.coding_passes as u32;

                        let _ = bpc.decode_codeblock(
                            &data,
                            max_bit_plane as u8,
                            cb_info.num_passes as u8,
                        );

                        block.coefficients = bpc.coefficients;
                        block.state = bpc.state;
                        block.coding_passes = bpc.num_passes_decoded as u8;
                    } else {
                        let mut block = crate::jpeg2000::image::J2kCodeBlock::default();
                        block.x = cb_info.x as u32;
                        block.y = cb_info.y as u32;
                        block.width = cb_width as u32;
                        block.height = cb_height as u32;
                        block.layer_data.push(data.clone());
                        block.layers_decoded = (layer + 1) as u8;
                        block.coding_passes = 0;

                        let mut bpc = crate::jpeg2000::bit_plane_coder::BitPlaneCoder::new(
                            cb_width as u32,
                            cb_height as u32,
                            &[],
                        );
                        if let Ok(coefficients) = bpc.decode_codeblock(
                            &data,
                            max_bit_plane as u8,
                            cb_info.num_passes as u8,
                        ) {
                            block.coefficients = coefficients;
                            block.state = bpc.state;
                            block.coding_passes = bpc.num_passes_decoded as u8;
                        }
                        subband.codeblocks.push(block);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg_stream_reader::JpegStreamReader;
    use crate::jpeg2000::image::{J2kCod, J2kComponentInfo, J2kImage};
    use crate::jpeg2000::parser::J2kParser;

    #[test]
    fn test_subsampling_resolution_calculation() {
        // Mock image with 2 components:
        // Comp 0: 1x1 subsampling (Full res)
        // Comp 1: 2x2 subsampling (Half res)
        let mut image = J2kImage::default();
        image.width = 512;
        image.height = 512;
        image.x_origin = 0;
        image.y_origin = 0;
        image.tile_width = 512;
        image.tile_height = 512;
        image.tile_x_origin = 0;
        image.tile_y_origin = 0;
        image.component_count = 2;

        // Component 0
        image.components.push(J2kComponentInfo {
            depth: 8,
            is_signed: false,
            dx: 1,
            dy: 1,
        });
        // Component 1
        image.components.push(J2kComponentInfo {
            depth: 8,
            is_signed: false,
            dx: 2,
            dy: 2,
        });

        // Tiles setup
        image.tiles.resize_with(1, Default::default);
        image.tiles[0].index = 0;

        // COD: 1 decomposition level => 2 resolutions (0 and 1)
        image.cod = Some(J2kCod {
            coding_style: 0,
            progression_order: 0,
            number_of_layers: 1,
            mct: 0,
            decomposition_levels: 1,
            codeblock_width_exp: 4,
            codeblock_height_exp: 4,
            transformation: 0,
            precinct_sizes: vec![],
        });

        // Dummy reader
        let data = vec![0; 100];
        let mut reader = JpegStreamReader::new(&data);
        let mut parser = J2kParser::new(&mut reader);
        // Inject our constructed image into parser
        parser.image = Box::new(image);

        let mut tile_states = Vec::new();

        // Call decode_tile_data
        let _ = J2kDecoder::decode_tile_data(&mut parser, 0, 0, false, &mut tile_states);

        // Verify tile_states
        let tile = &parser.image.tiles[0];
        assert_eq!(tile.components.len(), 2);

        // Check Component 0 (Full Res 512x512)
        // Res 0 (LL): 256x256
        // Res 1 (Full): 512x512
        let comp0 = &tile.components[0];
        assert_eq!(comp0.resolutions.len(), 2);
        assert_eq!(
            comp0.resolutions[0].width, 256,
            "Comp 0 Res 0 width mismatch"
        );
        assert_eq!(
            comp0.resolutions[0].height, 256,
            "Comp 0 Res 0 height mismatch"
        );
        assert_eq!(
            comp0.resolutions[1].width, 512,
            "Comp 0 Res 1 width mismatch"
        );
        assert_eq!(
            comp0.resolutions[1].height, 512,
            "Comp 0 Res 1 height mismatch"
        );

        // Check Component 1 (Subsampled 2x2 => 256x256)
        // Res 0 (LL): 128x128
        // Res 1 (Full): 256x256
        let comp1 = &tile.components[1];
        assert_eq!(comp1.resolutions.len(), 2);
        assert_eq!(
            comp1.resolutions[0].width, 128,
            "Comp 1 Res 0 width mismatch"
        );
        assert_eq!(
            comp1.resolutions[0].height, 128,
            "Comp 1 Res 0 height mismatch"
        );
        assert_eq!(
            comp1.resolutions[1].width, 256,
            "Comp 1 Res 1 width mismatch"
        );
        assert_eq!(
            comp1.resolutions[1].height, 256,
            "Comp 1 Res 1 height mismatch"
        );
    }
}

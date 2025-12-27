//! JPEG 2000 / HTJ2K Decoder.
//!
//! This module provides the `J2kDecoder` which manages the high-level
//! decoding process, including header parsing and dispatching to Tier-1/Tier-2 coders.

use super::image::J2kImage;
use super::parser::J2kParser;
use crate::JpeglsError;
use crate::jpeg_stream_reader::JpegStreamReader;

/// High-level generic JPEG 2000 Decoder.
/// Orchestrates parsing, block decoding, and image reconstruction.
pub struct J2kDecoder<'a, 'b> {
    parser: J2kParser<'a, 'b>,
}

impl<'a, 'b> J2kDecoder<'a, 'b> {
    pub fn new(reader: &'b mut JpegStreamReader<'a>) -> Self {
        Self {
            parser: J2kParser::new(reader),
        }
    }

    /// Decodes the JPEG 2000 image from the stream.
    pub fn decode(&mut self) -> Result<&J2kImage, JpeglsError> {
        eprintln!("DEBUG: J2kDecoder::decode ENTRY");
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
            eprintln!("DEBUG: JP2 Codestream found");
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
            Self::__decode_tiles_loop(&mut sub_parser, last_marker, is_htj2k)?;

            // Copy results back to main parser state
            self.parser.image = sub_parser.image.clone();
            self.parser.image.icc_profile = icc_profile;
        } else {
            eprintln!("DEBUG: Raw J2K Stream assumed");
            // 1. Parse Main Header with self.parser
            let last_marker = self.parser.parse_main_header()?;

            // 2. Identify Decoding Path
            let is_htj2k = if let Some(cap) = &self.parser.image.cap {
                (cap.pcap & (1 << 14)) != 0
            } else {
                false
            };

            // 3. Decode Tiles using self.parser
            Self::__decode_tiles_loop(&mut self.parser, last_marker, is_htj2k)?;
        }

        Ok(&self.parser.image)
    }

    /// Internal loop to process tiles.
    /// Detached from `self` to allow using either `self.parser` or `sub_parser`.
    fn __decode_tiles_loop(
        parser: &mut J2kParser,
        mut marker: crate::jpeg_marker_code::JpegMarkerCode,
        is_htj2k: bool,
    ) -> Result<(), JpeglsError> {
        eprintln!("DEBUG: decode loop entry. Marker={:?}", marker);
        loop {
            if marker == crate::jpeg_marker_code::JpegMarkerCode::EndOfImage {
                eprintln!("DEBUG: decode loop EOI");
                break;
            }

            if marker == crate::jpeg_marker_code::JpegMarkerCode::StartOfTile {
                eprintln!("DEBUG: Decoder processing SOT");
                let (psot, isot) = parser.parse_tile_part_header()?;
                Self::decode_tile_data(parser, psot, isot, is_htj2k)?;
                marker = Self::find_next_marker(parser)?;
                eprintln!("DEBUG: Next marker={:?}", marker);
            } else {
                eprintln!("DEBUG: decode loop unknown marker {:?}", marker);
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
        is_htj2k: bool,
    ) -> Result<(), JpeglsError> {
        eprintln!("DEBUG: decode_tile_data isot={}", isot);
        let tile_idx = isot as usize;
        if parser.image.tiles.len() <= tile_idx {
            parser
                .image
                .tiles
                .resize_with(tile_idx + 1, Default::default);
            parser.image.tiles[tile_idx].index = isot as u32;
        }

        let cod = parser.image.cod.as_ref().ok_or(JpeglsError::InvalidData)?;
        let num_layers = cod.number_of_layers as usize;
        let num_resolutions = (cod.decomposition_levels + 1) as usize;
        let num_components = parser.image.component_count as usize;

        let progression_order = cod.progression_order;
        let cb_w = 1 << (cod.codeblock_width_exp + 2);
        let cb_h = 1 << (cod.codeblock_height_exp + 2);
        let tile_w = parser.image.tile_width;
        let tile_h = parser.image.tile_height;

        let get_grid_size = |r: usize| -> (usize, usize) {
            let shift = if r >= (num_resolutions - 1) {
                0
            } else {
                num_resolutions - 1 - r
            };
            let res_w = (tile_w + (1 << shift) - 1) >> shift;
            let res_h = (tile_h + (1 << shift) - 1) >> shift;
            let gw = (res_w as usize).div_ceil(cb_w);
            let gh = (res_h as usize).div_ceil(cb_h);
            (gw, gh)
        };

        match progression_order {
            0 => {
                // LRCP
                for l in 0..num_layers {
                    for r in 0..num_resolutions {
                        let (grid_w, grid_h) = get_grid_size(r);
                        for c in 0..num_components {
                            for py in 0..grid_h {
                                for px in 0..grid_w {
                                    if parser.reader.remaining_data().is_empty() {
                                        return Ok(());
                                    }
                                    Self::decode_packet(
                                        parser, l, r, c, px, py, grid_w, grid_h, isot, is_htj2k,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
            2 => {
                // RPCL
                for r in 0..num_resolutions {
                    let (grid_w, grid_h) = get_grid_size(r);
                    for py in 0..grid_h {
                        for px in 0..grid_w {
                            for c in 0..num_components {
                                for l in 0..num_layers {
                                    if parser.reader.remaining_data().is_empty() {
                                        return Ok(());
                                    }
                                    Self::decode_packet(
                                        parser, l, r, c, px, py, grid_w, grid_h, isot, is_htj2k,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                // Fallback
                for l in 0..num_layers {
                    for r in 0..num_resolutions {
                        let (grid_w, grid_h) = get_grid_size(r);
                        for c in 0..num_components {
                            for py in 0..grid_h {
                                for px in 0..grid_w {
                                    if parser.reader.remaining_data().is_empty() {
                                        return Ok(());
                                    }
                                    let _ = Self::decode_packet(
                                        parser, l, r, c, px, py, grid_w, grid_h, isot, is_htj2k,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn decode_packet(
        parser: &mut J2kParser,
        layer: usize,
        res: usize,
        comp: usize,
        _px: usize,
        _py: usize,
        grid_w: usize,
        grid_h: usize,
        isot: u16,
        is_htj2k: bool,
    ) -> Result<(), JpeglsError> {
        let num_subbands = if res == 0 { 1 } else { 3 };
        let mut state = crate::jpeg2000::packet::PrecinctState::new(grid_w, grid_h);
        let header;
        let consumed;
        {
            let remaining = parser.reader.remaining_data();
            if remaining.is_empty() {
                return Ok(());
            }
            let mut bit_reader = crate::jpeg2000::bit_io::J2kBitReader::new(remaining);
            header = crate::jpeg2000::packet::PacketHeader::read(
                &mut bit_reader,
                &mut state,
                layer as u32,
                grid_w,
                grid_h,
                num_subbands,
            )
            .map_err(|_| JpeglsError::InvalidData)?;
            consumed = bit_reader.position();
        }
        parser.reader.advance(consumed);

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

                let subband_idx = 0;
                if resolution.subbands.len() <= subband_idx {
                    resolution
                        .subbands
                        .resize_with(subband_idx + 1, Default::default);
                }
                let subband = &mut resolution.subbands[subband_idx];

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
                    let cb_width = 64;
                    let cb_height = 64;
                    let max_bit_plane = 30;
                    let mut bpc = crate::jpeg2000::bit_plane_coder::BitPlaneCoder::new(
                        cb_width,
                        cb_height,
                        &[],
                    );
                    if let Ok(coefficients) =
                        bpc.decode_codeblock(&data, cb_width, cb_height, max_bit_plane)
                    {
                        let mut block = crate::jpeg2000::image::J2kCodeBlock::default();
                        block.width = cb_width;
                        block.height = cb_height; // Corrected from cb_width in copy-paste
                        block.coefficients = coefficients;
                        block.layer_data.push(data.clone());
                        block.layers_decoded = (layer + 1) as u8;
                        subband.codeblocks.push(block);
                    }
                }
            }
        }
        let current = parser.image.decoded_layers;
        if (layer as u32 + 1) > current {
            parser.image.decoded_layers = (layer + 1) as u32;
        }
        Ok(())
    }
}

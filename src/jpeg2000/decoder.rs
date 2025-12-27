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
    /// Returns the active J2kImage structure (metadata only for now).
    pub fn decode(&mut self) -> Result<&J2kImage, JpeglsError> {
        // 0. Container Detection (JP2 Box)
        let codestream = {
            let mut jp2_reader =
                crate::jpeg2000::jp2::Jp2Reader::new(self.parser.reader.remaining_data());
            jp2_reader.find_codestream()?
        };

        // Extract ICC profile if JP2 container
        let icc_profile = {
            let mut jp2_reader =
                crate::jpeg2000::jp2::Jp2Reader::new(self.parser.reader.remaining_data());
            jp2_reader.find_icc_profile()?
        };

        if let Some(cs) = codestream {
            let mut sub_reader = JpegStreamReader::new(cs);
            let mut sub_parser = J2kParser::new(&mut sub_reader);
            sub_parser.parse_main_header()?;
            self.parser.image = sub_parser.image.clone();
            self.parser.image.icc_profile = icc_profile;
        }

        // 1. Parse Main Header
        let last_marker = self.parser.parse_main_header()?;

        // 2. Identify Decoding Path (HT vs Standard)
        let is_htj2k = if let Some(cap) = &self.parser.image.cap {
            (cap.pcap & (1 << 14)) != 0
        } else {
            false
        };

        // 3. Iterate Tiles
        let mut marker = last_marker;
        loop {
            if marker == crate::jpeg_marker_code::JpegMarkerCode::EndOfImage {
                break;
            }

            if marker == crate::jpeg_marker_code::JpegMarkerCode::StartOfTile {
                // Note: parse_main_header already consumed the FF 90 marker bytes
                let psot = self.parser.parse_tile_part_header()?;
                self.decode_tile_data(psot, is_htj2k)?;
                marker = self.find_next_marker()?;
            } else {
                break;
            }
        }

        Ok(&self.parser.image)
    }

    /// Scans the stream for the next marker (SOT or EOC).
    fn find_next_marker(&mut self) -> Result<crate::jpeg_marker_code::JpegMarkerCode, JpeglsError> {
        loop {
            match self.parser.reader.read_u8() {
                Ok(0xFF) => {
                    let b2 = self.parser.reader.read_u8()?;
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

    /// Decodes the tile data (packets -> codeblocks -> coefficients).
    fn decode_tile_data(&mut self, _len: u32, is_htj2k: bool) -> Result<(), JpeglsError> {
        let cod = self
            .parser
            .image
            .cod
            .as_ref()
            .ok_or(JpeglsError::InvalidData)?;
        let num_layers = cod.number_of_layers as usize;
        let num_resolutions = (cod.decomposition_levels + 1) as usize;
        let num_components = self.parser.image.component_count as usize;

        let progression_order = cod.progression_order;
        let grid_w = 1;
        let grid_h = 1;

        match progression_order {
            0 => {
                // LRCP
                for l in 0..num_layers {
                    for r in 0..num_resolutions {
                        for c in 0..num_components {
                            for py in 0..grid_h {
                                for px in 0..grid_w {
                                    if self.parser.reader.remaining_data().is_empty() {
                                        return Ok(());
                                    }
                                    self.decode_packet(l, r, c, px, py, is_htj2k)?;
                                }
                            }
                        }
                    }
                }
            }
            2 => {
                // RPCL
                for r in 0..num_resolutions {
                    for py in 0..grid_h {
                        for px in 0..grid_w {
                            for c in 0..num_components {
                                for l in 0..num_layers {
                                    if self.parser.reader.remaining_data().is_empty() {
                                        return Ok(());
                                    }
                                    self.decode_packet(l, r, c, px, py, is_htj2k)?;
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
                        for c in 0..num_components {
                            if self.parser.reader.remaining_data().is_empty() {
                                return Ok(());
                            }
                            let _ = self.decode_packet(l, r, c, 0, 0, is_htj2k);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn decode_packet(
        &mut self,
        layer: usize,
        _res: usize,
        _comp: usize,
        _px: usize,
        _py: usize,
        is_htj2k: bool,
    ) -> Result<(), JpeglsError> {
        let mut state = crate::jpeg2000::packet::PrecinctState::new(1, 1);
        let header;
        let consumed;
        {
            let remaining = self.parser.reader.remaining_data();
            if remaining.is_empty() {
                return Ok(());
            }
            let mut bit_reader = crate::jpeg2000::bit_io::J2kBitReader::new(remaining);
            header = crate::jpeg2000::packet::PacketHeader::read(
                &mut bit_reader,
                &mut state,
                layer as u32,
                1,
                1,
            )
            .map_err(|_| JpeglsError::InvalidData)?;
            consumed = bit_reader.position();
        }

        self.parser.reader.advance(consumed);

        // Track layer contributions for quality layer support
        for cb_info in header.included_cblks {
            if cb_info.data_len > 0 {
                let data_len = cb_info.data_len as usize;
                let mut data = vec![0u8; data_len];
                for item in &mut data {
                    *item = self.parser.reader.read_u8()?;
                }

                if is_htj2k {
                    let mut coder = crate::jpeg2000::ht_block_coder::coder::HTBlockCoder::new(
                        &data, &data, 64, 64,
                    );
                    let mut block = crate::jpeg2000::image::J2kCodeBlock::default();
                    // Store layer data for accumulation
                    block.layer_data.push(data.clone());
                    block.layers_decoded = (layer + 1) as u8;
                    let _ = coder.decode_block(&mut block);
                } else {
                    // Standard JPEG 2000: use bit plane coder
                    // For now, use a default codeblock size (should come from COD marker)
                    let cb_width = 64;
                    let cb_height = 64;
                    let max_bit_plane = 30; // Should be determined from data

                    let mut bpc = crate::jpeg2000::bit_plane_coder::BitPlaneCoder::new(
                        cb_width, cb_height, &[]
                    );

                    match bpc.decode_codeblock(&data, cb_width, cb_height, max_bit_plane) {
                        Ok(_coefficients) => {
                            // Store decoded coefficients in a codeblock structure
                            // This would need to be integrated with the tile/subband structure
                            // For now, we just decode successfully
                        }
                        Err(_) => {
                            // Decoding failed, but continue
                        }
                    }
                }
            }
        }

        // Update global decoded_layers counter
        let current = self.parser.image.decoded_layers;
        if (layer as u32 + 1) > current {
            self.parser.image.decoded_layers = (layer + 1) as u32;
        }

        Ok(())
    }
}

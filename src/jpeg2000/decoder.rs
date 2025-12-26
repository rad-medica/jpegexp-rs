use super::image::J2kImage;
use super::parser::J2kParser;
use crate::JpeglsError;
use crate::jpeg_stream_reader::JpegStreamReader;

/// High-level generic JPEG 2000 Decoder.
/// Orchestrates parsing, block decoding, and image reconstruction.
pub struct J2kDecoder<'a, 'b> {
    parser: J2kParser<'a, 'b>,
    // In the future, we will hold decoding state, DWT engine, etc.
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
        // 1. Parse Main Header
        let last_marker = self.parser.parse_main_header()?;

        // 2. Identify Decoding Path (HT vs Standard)
        let is_htj2k = if let Some(cap) = &self.parser.image.cap {
            // Check Pcap bit 14 (15th bit) for Part 15 (HTJ2K)
            (cap.pcap & (1 << 14)) != 0
        } else {
            false
        };

        println!("Decoding J2K Codestream. HTJ2K Mode: {}", is_htj2k);

        // 3. Iterate Tiles
        let mut marker = last_marker;
        loop {
            if marker == crate::jpeg_marker_code::JpegMarkerCode::EndOfImage {
                break;
            }

            if marker == crate::jpeg_marker_code::JpegMarkerCode::StartOfTile {
                // Parse Tile Header
                // This reads SOT, and any tile-part markers until SOD.
                let psot = self.parser.parse_tile_part_header()?;

                // NOW we are at Start of Data (SOD).
                // We need to read/decode the bitstream.
                // Psot tells us the length of the tile-part (header + data).
                // We need to calculate how much data is left.
                // But Psot includes headers we've already parsed.
                // This tracking is tricky if parser consumed bytes.
                // For now, let's assume we read until next marker if Psot=0, or rely on logic.

                // Decode Tile Data
                self.decode_tile_data(psot, is_htj2k)?;

                // Scan for next marker (SOT or EOC) to continue loop
                marker = self.find_next_marker()?;
            } else {
                // Should not happen if parser logic is correct
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
        // Placeholder for full Tier-2 parsing.
        // We will mock the grid for a single resolution to demonstrate integration.
        // Assume minimal SIZ: 1 component, full image in 1 tile.

        // 1. Setup Precinct State
        // Need grid size. Mocking 4x4 codeblocks for now.
        let grid_w = 4; // Mock
        let grid_h = 4; // Mock
        let mut state = crate::jpeg2000::packet::PrecinctState::new(grid_w, grid_h);

        // 2. Mock Packet Loop
        // Scope for bit_reader borrow
        let header;
        let consumed;
        {
            let remaining = self.parser.reader.remaining_data();
            let mut bit_reader = crate::jpeg2000::bit_io::J2kBitReader::new(remaining);
            header = crate::jpeg2000::packet::PacketHeader::read(
                &mut bit_reader,
                &mut state,
                0, // Layer 0
                grid_w,
                grid_h,
            )
            .map_err(|_| JpeglsError::InvalidData)?;
            consumed = bit_reader.position();
        }

        self.parser.reader.advance(consumed);

        if header.empty {
            return Ok(());
        }

        // 3. Process Codeblocks
        for cb_info in header.included_cblks {
            if cb_info.data_len > 0 {
                let data_len = cb_info.data_len as usize;
                let mut data = vec![0u8; data_len];
                for i in 0..data_len {
                    data[i] = self.parser.reader.read_u8()?;
                }

                if is_htj2k {
                    // 4. Dispatch to HTBlockCoder
                    // Assuming data contains both MEL and MagSgn for now.
                    let mut coder = crate::jpeg2000::ht_block_coder::coder::HTBlockCoder::new(
                        &data, // Mel data (mock: same buffer)
                        &data, // MagSgn data (mock: same buffer)
                        64,    // Block width (standard)
                        64,    // Block height (standard)
                    );

                    let mut block = crate::jpeg2000::image::J2kCodeBlock::default();
                    // Result ignored for now as DWT/IQ not ready to use output
                    let _ = coder.decode_block(&mut block);

                    println!(
                        "Decoded HT Codeblock at ({}, {}) len={}",
                        cb_info.x, cb_info.y, data_len
                    );
                } else {
                    // Tier-1 MQ Coder (Placeholder)
                    println!("Standard J2K Codeblock (Not Implemented)");
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

    #[test]
    fn test_decoder_htj2k_detection() {
        // Mock stream with SOC, CAP (HTJ2K), SIZ, COD, QCD, SOT, SOD, data, EOC
        let data = vec![
            0xFF, 0x4F, // SOC
            // CAP: 0xFF50, Len=6 (2 len + 4 Pcap), Pcap=0x00004000
            0xFF, 0x50, 0x00, 0x06, 0x00, 0x00, 0x40, 0x00, // Pcap (big endian 0x00004000)
            // SIZ
            0xFF, 0x51, 0x00, 0x29, 0x00, 0x00, // Caps
            0x00, 0x00, 0x01, 0x00, // W=256
            0x00, 0x00, 0x01, 0x00, // H=256
            0x00, 0x00, 0x00, 0x00, // offX
            0x00, 0x00, 0x00, 0x00, // offY
            0x00, 0x00, 0x01, 0x00, // tileW=256
            0x00, 0x00, 0x01, 0x00, // tileH=256
            0x00, 0x00, 0x00, 0x00, // tileOffX
            0x00, 0x00, 0x00, 0x00, // tileOffY
            0x00, 0x01, // 1 Comp
            0x07, 0x01, 0x01, // Depth 8
            // COD
            0xFF, 0x52, 0x00, 0x0A, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01,
            // QCD
            0xFF, 0x5C, 0x00, 0x05, 0x06, 0x00, 0x10, // SOT
            0xFF, 0x90, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // SOD
            0xFF, 0x93, 0x00, // Empty Packet (Packet header 0 bit)
            // EOC
            0xFF, 0xD9,
        ];

        let mut reader = JpegStreamReader::new(&data);
        let mut decoder = J2kDecoder::new(&mut reader);

        let res = decoder.decode();
        assert!(res.is_ok(), "Decode failed: {:?}", res.err());
        let image = res.unwrap();
        assert!(
            image.cap.is_some(),
            "CAP marker not present in parsed image"
        );
        let cap = image.cap.as_ref().unwrap();
        assert_eq!(
            cap.pcap & (1 << 14),
            1 << 14,
            "HTJ2K bit (14) not set in PCAP: {:08X}",
            cap.pcap
        );
    }

    #[test]
    fn test_decoder_htj2k_with_data() {
        // Mock stream with non-empty HTJ2K packet
        let data = vec![
            0xFF, 0x4F, // SOC
            // CAP: 0xFF50, Len=6, Pcap=0x00004000 (HTJ2K)
            0xFF, 0x50, 0x00, 0x06, 0x00, 0x00, 0x40, 0x00,
            // SIZ: 41 bytes total (2 len + 39 payload)
            0xFF, 0x51, 0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // Xsiz=16
            0x00, 0x00, 0x00, 0x10, // Ysiz=16
            0x00, 0x00, 0x00, 0x00, // XOsiz=0
            0x00, 0x00, 0x00, 0x00, // YOsiz=0
            0x00, 0x00, 0x00, 0x10, // XTsiz=16
            0x00, 0x00, 0x00, 0x10, // YTsiz=16
            0x00, 0x00, 0x00, 0x00, // XTOsiz=0
            0x00, 0x00, 0x00, 0x00, // YTOsiz=0
            0x00, 0x01, // Csiz=1
            0x07, 0x01, 0x01, // Comp0: 8-bit, 1x1 subsampling
            // COD: 12 bytes total (2 len + 10 payload)
            0xFF, 0x52, 0x00, 0x0C, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // QCD: 4 bytes total (2 len + 2 payload)
            0xFF, 0x5C, 0x00, 0x04, 0x00, 0x20,
            // SOT: 10 bytes total (2 len + 8 payload)
            0xFF, 0x90, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            // SOD
            0xFF, 0x93,
            // Packet Header: 1 (not empty), 0 (included), 00000000 00000100 (len 4)
            // Bytes: 10000000 00000001 00... -> 0x80, 0x01, 0x00
            0x80, 0x01, 0x00, // Packet Data: 4 bytes
            0xDE, 0xAD, 0xBE, 0xEF, // EOC
            0xFF, 0xD9,
        ];

        let mut reader = JpegStreamReader::new(&data);
        let mut decoder = J2kDecoder::new(&mut reader);

        let res = decoder.decode();
        // Since we have mocked HTBlockCoder to just return Ok(()),
        // and we haven't implemented full pixel reconstruction yet,
        // we just verify it doesn't crash and reaches EOC.
        assert!(res.is_ok(), "Decode failed with data: {:?}", res.err());
    }
}

//! JPEG 2000 Codestream Parser.
//!
//! Handles the parsing of Main Headers (SOC, SIZ, COD, QCD, CAP) and
//! Tile-Part Headers (SOT, SOD).

use super::image::{J2kCap, J2kCod, J2kComponentInfo, J2kImage, J2kQcd};
use crate::JpeglsError;
use crate::jpeg_marker_code::JpegMarkerCode;
use crate::jpeg_stream_reader::JpegStreamReader;

/// A parser that transforms raw J2K marker segments into structured metadata.
pub struct J2kParser<'a, 'b> {
    pub reader: &'b mut JpegStreamReader<'a>,
    pub image: Box<J2kImage>,
}

impl<'a, 'b> J2kParser<'a, 'b> {
    pub fn new(reader: &'b mut JpegStreamReader<'a>) -> Self {
        Self {
            reader,
            image: Box::new(J2kImage::default()),
        }
    }

    pub fn parse_main_header(&mut self) -> Result<JpegMarkerCode, JpeglsError> {
        // Expect SOC (0xFF4F)
        let soc = self.reader.read_u16()?;
        if soc != 0xFF4F {
            return Err(JpeglsError::InvalidData); // Expected SOC
        }

        loop {
            // Read next marker (FFxx)
            self.reader.align_to_byte();

            if self.reader.remaining_data().len() < 2 {
                if self.reader.remaining_data().is_empty() {
                    return Err(JpeglsError::InvalidData);
                }
            }
            let b1 = self.reader.read_u8()?;
            if b1 != 0xFF {
                return Err(JpeglsError::InvalidData);
            }
            let b2 = self.reader.read_u8()?;
            let marker = JpegMarkerCode::try_from(b2)?;

            match marker {
                JpegMarkerCode::ImageAndTileSize => self.parse_siz()?,
                JpegMarkerCode::CodingStyleDefault => self.parse_cod()?,
                JpegMarkerCode::QuantizationDefault => self.parse_qcd()?,
                JpegMarkerCode::StartOfTile => {
                    // SOT indicates end of main header
                    return Ok(JpegMarkerCode::StartOfTile);
                }
                JpegMarkerCode::Capability => self.parse_cap()?,
                JpegMarkerCode::RegionOfInterest => self.parse_rgn()?,
                JpegMarkerCode::J2kComment => {
                    let len = self.reader.read_u16()?;
                    if len < 2 {
                        return Err(JpeglsError::InvalidData);
                    }
                    self.reader.advance((len - 2) as usize);
                }
                _ => {
                    // Skip unknown segment
                    let len = self.reader.read_u16()?;
                    if len < 2 {
                        return Err(JpeglsError::InvalidData);
                    }
                    self.reader.advance((len - 2) as usize);
                }
            }
        }
    }

    pub fn parse_siz(&mut self) -> Result<(), JpeglsError> {
        let _len = self.reader.read_u16()?;
        let _caps = self.reader.read_u16()?; // RSiz
        self.image.width = self.reader.read_u32()?;
        self.image.height = self.reader.read_u32()?;
        self.image.x_origin = self.reader.read_u32()?;
        self.image.y_origin = self.reader.read_u32()?;

        self.image.tile_width = self.reader.read_u32()?;
        self.image.tile_height = self.reader.read_u32()?;
        self.image.tile_x_origin = self.reader.read_u32()?;
        self.image.tile_y_origin = self.reader.read_u32()?;

        let comps = self.reader.read_u16()?;
        self.image.component_count = comps as u32;

        // Components info follows... (Precision, Subsamp) - Skip for now or store
        for _c in 0..comps {
            let depth_byte = self.reader.read_u8()?;
            let depth = (depth_byte & 0x7F) + 1;
            let is_signed = (depth_byte & 0x80) != 0;
            let sub_x = self.reader.read_u8()?;
            let sub_y = self.reader.read_u8()?;
            self.image.components.push(J2kComponentInfo {
                depth,
                is_signed,
                dx: sub_x,
                dy: sub_y,
            });
        }
        Ok(())
    }

    pub fn parse_cod(&mut self) -> Result<(), JpeglsError> {
        // COD marker parsing
        // Length includes the length field itself
        let len = self.reader.read_u16()?;
        // We read 8 bytes of content (Scod=1, SGcod=4, SPcod partial=3)
        // plus 2 bytes for length = 10 bytes minimum.
        if len < 12 {
            return Err(JpeglsError::InvalidData);
        }
        let scod = self.reader.read_u8()?; // coding style flags
        let sprog = self.reader.read_u8()?; // progression order
        let nlayers = self.reader.read_u16()?; // number of layers
        let mct = self.reader.read_u8()?; // multi-component transform flag
        let decomposition_levels = self.reader.read_u8()?; // number of decomposition levels
        let codeblock_width_exp = self.reader.read_u8()?; // codeblock width exponent (log2)
        let codeblock_height_exp = self.reader.read_u8()?; // codeblock height exponent (log2)
        let _codeblock_style = self.reader.read_u8()?;
        let transformation = self.reader.read_u8()?;

        let mut precinct_sizes = Vec::new();
        // If Scod bit 0 (Precincts defined) is set, read precinct sizes.
        if (scod & 0x01) != 0 {
            let count = (decomposition_levels + 1) as usize;
            for _ in 0..count {
                precinct_sizes.push(self.reader.read_u8()?);
            }
        }

        // We consumed 2(len) + 5(Scod..Decomp) + 2(CB dim) + 2(CB style, Trans) + Prec = 11 + Prec
        // Previous calc was 8 (Scod..CBH) + ...
        // Total fields read:
        // Scod(1), Sprog(1), Nlayers(2), MCT(1), Decomp(1), CBW(1), CBH(1), Style(1), Trans(1) = 10 bytes.
        // + 2 bytes length = 12 bytes.
        let parsed_bytes = 12 + precinct_sizes.len();
        let remaining = (len as usize).saturating_sub(parsed_bytes);
        if remaining > 0 {
            self.reader.advance(remaining);
        }
        self.image.cod = Some(J2kCod {
            coding_style: scod,
            progression_order: sprog,
            number_of_layers: nlayers,
            mct,
            decomposition_levels,
            codeblock_width_exp,
            codeblock_height_exp,
            transformation,
            precinct_sizes,
        });
        Ok(())
    }

    pub fn parse_qcd(&mut self) -> Result<(), JpeglsError> {
        // QCD marker parsing
        let len = self.reader.read_u16()?;
        // eprintln!("DEBUG: parse_qcd len={}", len);
        if len < 3 {
            return Err(JpeglsError::InvalidData);
        }
        let sqcd = self.reader.read_u8()?; // quantization style flags
        // eprintln!("DEBUG: parse_qcd sqcd={:02X}", sqcd);

        // Remaining in the marker segment
        // len includes 2 bytes for len.
        // We read 1 byte (sqcd).
        // Total consumed from len budget = 3.
        let mut bytes_left = (len as usize).saturating_sub(3);

        let mut step_sizes = Vec::new();
        // Read step sizes based on quantization style
        let quant_type = sqcd & 0x1F;
        let is_16bit = quant_type == 0x02; // Scalar Expounded

        let step_size_len = if is_16bit { 2 } else { 1 };

        while bytes_left >= step_size_len {
            let step = if is_16bit {
                self.reader.read_u16()?
            } else {
                // Align 8-bit steps (Exp << 3) to 16-bit format (Exp << 11) by shifting left 8
                (self.reader.read_u8()? as u16) << 8
            };
            step_sizes.push(step);
            bytes_left -= step_size_len as usize;
        }
        // Skip any leftover bytes (e.g. if odd length, though unlikely for u16 steps)
        if bytes_left > 0 {
            self.reader.advance(bytes_left);
        }
        self.image.qcd = Some(J2kQcd {
            quant_style: sqcd,
            step_sizes,
        });
        Ok(())
    }

    pub fn parse_cap(&mut self) -> Result<(), JpeglsError> {
        // CAP marker (0xFF50)
        let len = self.reader.read_u16()?;
        if len < 6 {
            // Must have at least Pcap (4 bytes)
            return Err(JpeglsError::InvalidData);
        }

        let pcap = self.reader.read_u32()?;
        // Ccap parts
        // len includes 2 bytes length. We read 4 bytes Pcap.
        // Total 6 bytes.
        let mut bytes_left = (len as usize).saturating_sub(6);

        // Ccap is u16[]
        let mut ccap = Vec::new();
        while bytes_left >= 2 {
            let c = self.reader.read_u16()?;
            ccap.push(c);
            bytes_left -= 2;
        }

        // Advance remaining (align)
        if bytes_left > 0 {
            self.reader.advance(bytes_left);
        }

        self.image.cap = Some(J2kCap { pcap, ccap });

        Ok(())
    }

    pub fn parse_rgn(&mut self) -> Result<(), JpeglsError> {
        // RGN marker (0xFF5E) - Region of Interest
        let len = self.reader.read_u16()?;
        if len < 5 {
            return Err(JpeglsError::InvalidData);
        }
        let component_index = self.reader.read_u8()?;
        let roi_style = self.reader.read_u8()?;
        let shift_value = self.reader.read_u8()?;

        // Skip remaining bytes if any
        let remaining = (len as usize).saturating_sub(5);
        if remaining > 0 {
            self.reader.advance(remaining);
        }

        self.image.roi = Some(super::image::J2kRoi {
            component_index,
            roi_style,
            shift_value,
        });

        Ok(())
    }

    /// Parses a Tile-Part.
    /// Returns (Psot, Isot).
    /// - Psot: Length of the data.
    /// - Isot: Tile index.
    pub fn parse_tile_part_header(&mut self) -> Result<(u32, u16), JpeglsError> {
        // Assume SOT marker (FF90) has been consumed (or we are inside SOT segment).

        let _lsot = self.reader.read_u16()?;
        let isot = self.reader.read_u16()?;
        let psot = self.reader.read_u32()?;
        let _tpsot = self.reader.read_u8()?;
        let _tnsot = self.reader.read_u8()?;

        // eprintln!("DEBUG: SOT isot={} psot={}", isot, psot);

        // Loop for other markers until SOD
        loop {
            // Check for potential markers
            if self.reader.remaining_data().len() < 2 {
                // eprintln!("DEBUG: SOT Loop EOF");
                return Err(JpeglsError::InvalidData);
            }

            let b1 = self.reader.read_u8()?;
            if b1 != 0xFF {
                // eprintln!("DEBUG: SOT Loop expected FF, got {:02X}", b1);
                return Err(JpeglsError::InvalidData);
            }
            let b2 = self.reader.read_u8()?;
            if b2 == 0x93 {
                // SOD
                // eprintln!("DEBUG: Found SOD");
                break;
            }

            let marker = JpegMarkerCode::try_from(b2)?;
            // eprintln!("DEBUG: Tile Marker {:?} ({:02X})", marker, b2);

            match marker {
                JpegMarkerCode::CodingStyleDefault => self.parse_cod()?,
                JpegMarkerCode::QuantizationDefault => self.parse_qcd()?,
                // Add COC, QCC, etc. support as needed
                _ => {
                    // Skip unknown
                    let len = self.reader.read_u16()?;
                    if len < 2 {
                        return Err(JpeglsError::InvalidData);
                    }
                    self.reader.advance((len - 2) as usize);
                }
            }
        }

        // At this point we are at the start of bitstream.
        Ok((psot, isot))
    }

    /// Parses the entire codestream (Main Header + All Tiles).
    pub fn parse_codestream(&mut self) -> Result<(), JpeglsError> {
        let mut marker = self.parse_main_header()?;

        loop {
            if marker == JpegMarkerCode::EndOfImage {
                break;
            }

            if marker == JpegMarkerCode::StartOfTile {
                // Parse tile part
                let (psot, _isot) = self.parse_tile_part_header()?;

                // Read Tile Data (packets)
                if psot == 0 {
                    // Read until EOC.
                    break;
                } else {
                    // Logic to find next marker (skip data)
                    loop {
                        if self.reader.remaining_data().len() < 2 {
                            break;
                        }
                        let b = self.reader.read_u8()?;
                        if b == 0xFF {
                            let b2 = self.reader.read_u8()?;
                            if b2 == 0x90 {
                                // SOT
                                marker = JpegMarkerCode::StartOfTile;
                                break;
                            } else if b2 == 0xD9 {
                                // EOC
                                marker = JpegMarkerCode::EndOfImage;
                                break;
                            }
                        }
                    }
                }
            } else {
                // Unexpected marker or EOC
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_siz() {
        // Mock stream: SOC, SIZ
        // SIZ len = 38+3*comps.
        // 2(len) + 2(cap) + 4(w) + 4(h) + 4(ox) + 4(oy) + 4(tw) + 4(th) + 4(tox) + 4(toy) + 2(c) + 3*1(comp)
        // 2+2+16+16+2+3 = 41 bytes.

        let data = vec![
            0xFF, 0x4F, // SOC
            0xFF, 0x51, // SIZ
            0x00, 0x29, // Len = 41 (0x29)
            0x00, 0x00, // Caps
            0x00, 0x00, 0x01, 0x00, // W = 256
            0x00, 0x00, 0x01, 0x00, // H = 256
            0x00, 0x00, 0x00, 0x00, // offX
            0x00, 0x00, 0x00, 0x00, // offY
            0x00, 0x00, 0x01, 0x00, // tileW = 256
            0x00, 0x00, 0x01, 0x00, // tileH = 256
            0x00, 0x00, 0x00, 0x00, // tileOffX
            0x00, 0x00, 0x00, 0x00, // tileOffY
            0x00, 0x01, // 1 Component
            0x07, 0x01, 0x01, // Depth 8 (unsigned), 1x1 sub
            // Terminate so parser loop breaks or fails on EOF if we don't have SOT.
            // Let's add SOT.
            0xFF, 0x90, // SOT
            // Add fake SOT len
            0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let mut reader = JpegStreamReader::new(&data);
        let mut parser = J2kParser::new(&mut reader);

        parser.parse_main_header().unwrap();

        assert_eq!(parser.image.width, 256);
        assert_eq!(parser.image.height, 256);
        assert_eq!(parser.image.component_count, 1);
    }

    #[test]
    fn test_parse_codestream() {
        let data = vec![
            0xFF, 0x4F, // SOC
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
            // SOT 1
            0xFF, 0x90, 0x00, 0x0A, // Lsot=10
            0x00, 0x00, // Isot=0
            0x00, 0x00, 0x00, 0x14, // Psot=20 (10 header + 2 marker + 8 data)
            0x00, // TPsot=0
            0x01, // TNsot=1
            // SOD
            0xFF, 0x93, // Data (Start of bitstream)
            0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, // 8 bytes data
            // EOC
            0xFF, 0xD9,
        ];

        // Actually Psot=20 includes: SOT header (14 bytes?? Markers are 2 bytes, Segment is len. SOT marker is 2 bytes. Segment len=10. So SOT total = 12 bytes.)
        // Plus SOD marker (2 bytes) = 14 bytes.
        // Data = 6 bytes.
        // Let's adjust Psot to 25 to be safe/explicit?
        // Standard: Psot "Length of the tile-part ... starting from the first byte of the SOT marker segment."
        // "SOT marker segment" includes the marker header?
        // "The length ... includes ... the marker segment itself."
        // Yes, includes the 2 bytes of marker?
        // Wait, "marker segment" usually refers to the parameters (including length).
        // The marker code (FF90) is NOT part of the segment length field usually.
        // But Psot says "length of THIS tile-part".
        // A.4.2: "Psot: Length of this tile-part. ... Value: Total number of bytes ... including the SOT marker segment and the tile-part bit stream."
        // "SOT Marker Segment" usually means FF90 + Lsot + ...
        // If Psot=0, length is unknown.

        // Let's test blindly.

        let mut reader = JpegStreamReader::new(&data);
        let mut parser = J2kParser::new(&mut reader);

        // parser.parse_codestream().unwrap();
        // Since we didn't implement robust skipping based on Psot yet (we scan for FF90/FFD9),
        // the Psot value in our mock might be ignored by the scanner loop, but parsed.

        let res = parser.parse_codestream();
        assert!(res.is_ok(), "Codestream parsing failed: {:?}", res.err());
    }

    #[test]
    fn test_parse_cod_qcd() {
        // Build a mock stream with SOC, SIZ, COD, QCD, and SOT markers.
        let data = vec![
            0xFF, 0x4F, // SOC
            // SIZ marker
            0xFF, 0x51, // SIZ
            0x00, 0x29, // Len = 41
            0x00, 0x00, // Caps
            0x00, 0x00, 0x01, 0x00, // width 256
            0x00, 0x00, 0x01, 0x00, // height 256
            0x00, 0x00, 0x00, 0x00, // offX
            0x00, 0x00, 0x00, 0x00, // offY
            0x00, 0x00, 0x01, 0x00, // tileW 256
            0x00, 0x00, 0x01, 0x00, // tileH 256
            0x00, 0x00, 0x00, 0x00, // tileOffX
            0x00, 0x00, 0x00, 0x00, // tileOffY
            0x00, 0x01, // 1 component
            0x07, 0x01, 0x01, // depth/subsampling
            // COD marker
            0xFF, 0x52, // COD
            0x00, 0x0C, // length 12 (2 len + 10 payload)
            0x00, // scod (User defined precincts = 0)
            0x02, // sprog
            0x00, 0x01, // nlayers = 1
            0x00, // mct (unused)
            0x03, // decomposition levels
            0x04, // codeblock width exponent
            0x05, // codeblock height exponent
            0x00, // codeblock style
            0x00, // transformation
            // QCD marker
            0xFF, 0x5C, // QCD
            0x00, 0x04, // length 4
            0x06, // sqcd
            0x10, // step size
            // SOT marker
            0xFF, 0x90, // SOT
            0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = JpegStreamReader::new(&data);
        let mut parser = J2kParser::new(&mut reader);
        parser.parse_main_header().unwrap();
        let cod = parser.image.cod.as_ref().expect("COD should be parsed");
        assert_eq!(cod.coding_style, 0x00);
        assert_eq!(cod.progression_order, 0x02);
        assert_eq!(cod.number_of_layers, 1);
        let qcd = parser.image.qcd.as_ref().expect("QCD should be parsed");
        assert_eq!(qcd.quant_style, 0x06);
        assert_eq!(qcd.step_sizes, vec![0x1000]);
    }
}

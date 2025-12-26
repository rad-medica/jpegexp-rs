use super::image::{J2kCod, J2kImage, J2kQcd};
use crate::JpeglsError;
use crate::jpeg_marker_code::JpegMarkerCode;
use crate::jpeg_stream_reader::JpegStreamReader;

pub struct J2kParser<'a, 'b> {
    reader: &'b mut JpegStreamReader<'a>,
    image: J2kImage,
}

impl<'a, 'b> J2kParser<'a, 'b> {
    pub fn new(reader: &'b mut JpegStreamReader<'a>) -> Self {
        Self {
            reader,
            image: J2kImage::default(),
        }
    }

    pub fn parse_main_header(&mut self) -> Result<(), JpeglsError> {
        // Expect SOC (0xFF4F)
        let soc = self.reader.read_u16()?;
        if soc != 0xFF4F {
            return Err(JpeglsError::InvalidData); // Expected SOC
        }

        loop {
            // Read next marker (FFxx)
            // JpegStreamReader might need a generic `read_marker` that handles 0xFF stuffing or just aligned reads.
            // J2K doesn't use 0xFF stuffing in headers same way as J1 entropy, but code stream does.

            // Allow reader to skip padding
            self.reader.align_to_byte();

            // Peek or read marker
            let b1 = self.reader.read_u8()?;
            if b1 != 0xFF {
                // Should be marker start
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
                    // Backtrack? Or just stop.
                    // Ideally we push back the marker or leave stream position.
                    // For now, let's assume we consume SOT and return, to let tile parser take over.
                    // But JpegStreamReader doesn't support unread easily.
                    break;
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

        Ok(())
    }

    fn parse_siz(&mut self) -> Result<(), JpeglsError> {
        let _len = self.reader.read_u16()?;
        let _caps = self.reader.read_u16()?; // RSiz
        self.image.width = self.reader.read_u32()?;
        self.image.height = self.reader.read_u32()?;
        let _off_x = self.reader.read_u32()?;
        let _off_y = self.reader.read_u32()?;

        self.image.tile_width = self.reader.read_u32()?;
        self.image.tile_height = self.reader.read_u32()?;
        let _tile_off_x = self.reader.read_u32()?;
        let _tile_off_y = self.reader.read_u32()?;

        let comps = self.reader.read_u16()?;
        self.image.component_count = comps as u32;

        // Components info follows... (Precision, Subsamp) - Skip for now or store
        for _ in 0..comps {
            let _depth = self.reader.read_u8()?;
            let _sub_x = self.reader.read_u8()?;
            let _sub_y = self.reader.read_u8()?;
        }
        Ok(())
    }

    fn parse_cod(&mut self) -> Result<(), JpeglsError> {
        // COD marker parsing (simplified)
        // Length includes the length field itself
        let len = self.reader.read_u16()?;
        // Ensure we have enough bytes for minimal fields
        if len < 7 {
            return Err(JpeglsError::InvalidData);
        }
        // Read fields according to JPEG2000 spec (subset)
        let scod = self.reader.read_u8()?; // coding style flags
        let sprog = self.reader.read_u8()?; // progression order
        let nlayers = self.reader.read_u16()?; // number of layers
        let mct = self.reader.read_u8()?; // multi-component transform flag (unused here)
        let decomposition_levels = self.reader.read_u8()?; // number of decomposition levels
        let codeblock_width_exp = self.reader.read_u8()?; // codeblock width exponent (log2)
        let codeblock_height_exp = self.reader.read_u8()?; // codeblock height exponent (log2)
        // Skip any remaining bytes in the COD segment
        let remaining = (len as usize).saturating_sub(7);
        if remaining > 0 {
            self.reader.advance(remaining);
        }
        self.image.cod = Some(J2kCod {
            coding_style: scod,
            progression_order: sprog,
            number_of_layers: nlayers,
            // Additional fields can be added later
        });
        Ok(())
    }

    fn parse_qcd(&mut self) -> Result<(), JpeglsError> {
        // QCD marker parsing (simplified)
        let len = self.reader.read_u16()?;
        if len < 3 {
            return Err(JpeglsError::InvalidData);
        }
        let sqcd = self.reader.read_u8()?; // quantization style flags
        // Remaining bytes are step sizes (2 bytes each)
        let mut step_sizes = Vec::new();
        let mut bytes_left = (len as usize).saturating_sub(3);
        while bytes_left >= 2 {
            let step = self.reader.read_u16()?;
            step_sizes.push(step);
            bytes_left -= 2;
        }
        // Skip any leftover bytes
        if bytes_left > 0 {
            self.reader.advance(bytes_left);
        }
        self.image.qcd = Some(J2kQcd {
            quant_style: sqcd,
            step_sizes,
        });
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

        let mut data = vec![
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
        ];

        // Terminate so parser loop breaks or fails on EOF if we don't have SOT.
        // Let's add SOT.
        data.extend_from_slice(&[0xFF, 0x90]); // SOT

        // Add fake SOT len
        data.extend_from_slice(&[0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let mut reader = JpegStreamReader::new(&data);
        let mut parser = J2kParser::new(&mut reader);

        parser.parse_main_header().unwrap();

        assert_eq!(parser.image.width, 256);
        assert_eq!(parser.image.height, 256);
        assert_eq!(parser.image.component_count, 1);
    }

    #[test]
    fn test_parse_cod_qcd() {
        // Build a mock stream with SOC, SIZ, COD, QCD, and SOT markers.
        let mut data = vec![
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
            0x00, 0x09, // length 9
            0x01, // scod
            0x02, // sprog
            0x00, 0x01, // nlayers = 1
            0x00, // mct (unused)
            0x03, // decomposition levels
            0x04, // codeblock width exponent
            0x05, // codeblock height exponent
            // QCD marker
            0xFF, 0x5C, // QCD
            0x00, 0x05, // length 5
            0x06, // sqcd
            0x00, 0x10, // step size
            // SOT marker
            0xFF, 0x90, // SOT
            0x00, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let mut reader = JpegStreamReader::new(&data);
        let mut parser = J2kParser::new(&mut reader);
        match parser.parse_main_header() {
            Ok(_) => {}
            Err(e) => panic!("Parse failed: {:?}", e),
        }
        let cod = parser.image.cod.as_ref().expect("COD should be parsed");
        assert_eq!(cod.coding_style, 0x01);
        assert_eq!(cod.progression_order, 0x02);
        assert_eq!(cod.number_of_layers, 1);
        let qcd = parser.image.qcd.as_ref().expect("QCD should be parsed");
        assert_eq!(qcd.quant_style, 0x06);
        assert_eq!(qcd.step_sizes, vec![0x0010]);
    }
}

use super::image::{J2kCod, J2kQcd};
use crate::JpeglsError;
use crate::jpeg_marker_code::JpegMarkerCode;
use crate::jpeg_stream_writer::JpegStreamWriter;

pub struct J2kWriter<'a> {
    writer: JpegStreamWriter<'a>,
}

impl<'a> J2kWriter<'a> {
    pub fn new(destination: &'a mut [u8]) -> Self {
        Self {
            writer: JpegStreamWriter::new(destination),
        }
    }

    pub fn len(&self) -> usize {
        self.writer.len()
    }

    pub fn write_soc(&mut self) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::StartOfCodestream)
    }

    pub fn write_eoc(&mut self) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::EndOfImage)
    }

    pub fn write_siz(
        &mut self,
        width: u32,
        height: u32,
        tile_width: u32,
        tile_height: u32,
        component_count: u16,
        bit_depth: u8, // Assuming same for all components for now
        sub_x: u8,
        sub_y: u8,
    ) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::ImageAndTileSize)?;

        // Length: 2 (Rsiz) + 4(W) + 4(H) + 4(OX) + 4(OY) + 4(TW) + 4(TH) + 4(TOX) + 4(TOY) + 2(C) + 3*C
        let length = 38 + 3 * (component_count as u16);
        self.writer.write_u16(length)?;
        self.writer.write_u16(0)?; // Rsiz (Capabilities) - 0 for default
        self.writer.write_u32(width)?;
        self.writer.write_u32(height)?;
        self.writer.write_u32(0)?; // OffX
        self.writer.write_u32(0)?; // OffY
        self.writer.write_u32(tile_width)?;
        self.writer.write_u32(tile_height)?;
        self.writer.write_u32(0)?; // TileOffX
        self.writer.write_u32(0)?; // TileOffY
        self.writer.write_u16(component_count)?;

        for _ in 0..component_count {
            self.writer.write_byte(bit_depth - 1)?; // Precision (bit depth - 1) | Sign bit (0 for unsigned)
            self.writer.write_byte(sub_x)?;
            self.writer.write_byte(sub_y)?;
        }
        Ok(())
    }

    pub fn write_cod(&mut self, cod: &J2kCod) -> Result<(), JpeglsError> {
        self.writer
            .write_marker(JpegMarkerCode::CodingStyleDefault)?;

        // Length: 10 + headers?
        // Lcod + Scod + SGcod + SPcod
        // Scod (1) + SGcod (4) + SPcod (5??) = 10?
        // SGcod: Progression (1), Layers (2), MCT (1) = 4 bytes.
        // SPcod: Decomp (1), Codeblock W (1), Codeblock H (1), Codeblock Style (1), Transform (1)?
        // Total 1 byte Scod + 4 bytes SG + 5 bytes SP = 10 bytes payload.
        // So len = 12.

        let length = 12; // Minimal COD length
        self.writer.write_u16(length)?;

        self.writer.write_byte(cod.coding_style)?;

        // SGcod
        self.writer.write_byte(cod.progression_order)?;
        self.writer.write_u16(cod.number_of_layers)?;
        self.writer.write_byte(0)?; // MCT enabled? 

        // SPcod
        self.writer.write_byte(5)?; // Number of decomposition levels (default 5 or struct field?)
        self.writer.write_byte(4)?; // Code-block width (xcb) - 4 -> 16
        self.writer.write_byte(4)?; // Code-block height (ycb) - 4 -> 16
        self.writer.write_byte(0)?; // Code-block style
        self.writer.write_byte(0)?; // Wavelet transform (0=9-7, 1=5-3)

        Ok(())
    }

    pub fn write_qcd(&mut self, qcd: &J2kQcd) -> Result<(), JpeglsError> {
        self.writer
            .write_marker(JpegMarkerCode::QuantizationDefault)?;

        // Length: 3 (Sqcd) + 2 * step_sizes.len() + 2 (len field) = 5?
        // Lqcd (2) + Sqcd (1) + SPqcd (n)
        let payload_len = 1 + qcd.step_sizes.len() * 2;
        self.writer.write_u16((payload_len + 2) as u16)?;

        self.writer.write_byte(qcd.quant_style)?;

        for &step in &qcd.step_sizes {
            self.writer.write_u16(step)?;
        }
        Ok(())
    }

    pub fn write_sot(
        &mut self,
        tile_index: u16,
        tile_len: u32,
        tile_part_index: u8,
        num_tile_parts: u8,
    ) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::StartOfTile)?;
        self.writer.write_u16(10)?; // Length of marker segment (10 bytes)
        self.writer.write_u16(tile_index)?;
        self.writer.write_u32(tile_len)?; // Psot
        self.writer.write_byte(tile_part_index)?; // TPsot
        self.writer.write_byte(num_tile_parts)?; // TNsot
        Ok(())
    }

    pub fn write_sod(&mut self) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::StartOfData) // 0xFF93
    }

    // Helper to access internal buffer to write raw data (packets)
    pub fn write_bytes(&mut self, data: &[u8]) -> Result<(), JpeglsError> {
        for &b in data {
            self.writer.write_byte(b)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_header_sequence() {
        let mut buffer = vec![0u8; 1024];
        let mut j2k_writer = J2kWriter::new(&mut buffer);

        j2k_writer.write_soc().unwrap();

        j2k_writer
            .write_siz(
                512, 512, // W, H
                128, 128, // TW, TH
                3,   // Comps
                8,   // Depth
                1, 1, // Sub x, y
            )
            .unwrap();

        let cod = J2kCod {
            coding_style: 0,
            progression_order: 0,
            number_of_layers: 1,
        };
        j2k_writer.write_cod(&cod).unwrap();

        let qcd = J2kQcd {
            quant_style: 0,
            step_sizes: vec![12345], // Example step
        };
        j2k_writer.write_qcd(&qcd).unwrap();

        j2k_writer.write_sot(0, 0, 0, 1).unwrap(); // Tile 0
        j2k_writer.write_sod().unwrap();

        // Write mock data
        j2k_writer.write_bytes(&[0x11, 0x22]).unwrap();

        j2k_writer.write_eoc().unwrap();

        let len = j2k_writer.len();
        let written = &buffer[..len];

        // Verify markers present
        assert_eq!(written[0], 0xFF);
        assert_eq!(written[1], 0x4F); // SOC
        // SIZ
        assert_eq!(written[2], 0xFF);
        assert_eq!(written[3], 0x51);
        // COD
        // Search for COD
        // QCD
        // SOT
        assert!(written.windows(2).any(|w| w == [0xFF, 0x90])); // SOT
        assert!(written.windows(2).any(|w| w == [0xFF, 0x93])); // SOD
        assert!(written.windows(2).any(|w| w == [0xFF, 0xD9])); // EOC
    }
}

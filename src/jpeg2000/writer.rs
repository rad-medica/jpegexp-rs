use super::image::{J2kCod, J2kQcd};
use crate::jpeg_marker_code::JpegMarkerCode;
use crate::jpeg_stream_writer::JpegStreamWriter;
use crate::JpeglsError;

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

    pub fn is_empty(&self) -> bool {
        self.writer.len() == 0
    }

    pub fn write_soc(&mut self) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::StartOfCodestream)
    }

    pub fn write_eoc(&mut self) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::EndOfImage)
    }

    /// Write CAP (Capability) marker for HTJ2K
    /// Pcap bit 14 set indicates HTJ2K support (MSB-first bit numbering)
    /// component_count: number of image components (needed for Ccap array)
    pub fn write_cap(&mut self, use_htj2k: bool, component_count: u16) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::Capability)?;

        // Length: 2 (length field) + 4 (Pcap) + 2 * component_count (Ccap array)
        let length = 2 + 4 + 2 * component_count;
        self.writer.write_u16(length)?;

        // Pcap: bit 14 = HTJ2K support (MSB-first bit numbering)
        // Bit 14 (MSB-first in 32-bit word) = 0x00020000
        // This is equivalent to bit 17 in LSB-first notation
        let pcap = if use_htj2k {
            0x0002_0000_u32 // Bit 14 MSB-first
        } else {
            0
        };

        self.writer.write_u32(pcap)?;

        // Ccap: component capabilities (one u16 per component)
        // For basic HTJ2K, set all Ccap values to 0
        for _ in 0..component_count {
            self.writer.write_u16(0)?;
        }

        Ok(())
    }

    pub fn write_siz(
        &mut self,
        width: u32,
        height: u32,
        tile_width: u32,
        tile_height: u32,
        component_count: u16,
        bit_depth: u8,
        is_signed: bool,
        sub_x: u8,
        sub_y: u8,
    ) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::ImageAndTileSize)?;

        // Length: 2 (Rsiz) + 4(W) + 4(H) + 4(OX) + 4(OY) + 4(TW) + 4(TH) + 4(TOX) + 4(TOY) + 2(C) + 3*C
        let length = 38 + 3 * component_count;
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

        let ssiz = (bit_depth - 1) | (if is_signed { 0x80 } else { 0 });
        for _ in 0..component_count {
            self.writer.write_byte(ssiz)?;
            self.writer.write_byte(sub_x)?;
            self.writer.write_byte(sub_y)?;
        }
        Ok(())
    }

    pub fn write_cod(&mut self, cod: &J2kCod) -> Result<(), JpeglsError> {
        self.writer
            .write_marker(JpegMarkerCode::CodingStyleDefault)?;

        // Length: Lcod (2 included) + Scod (1) + SGcod (4) + SPcod (5) = 12 bytes
        let length = 12;
        self.writer.write_u16(length)?;

        self.writer.write_byte(cod.coding_style)?;

        // SGcod
        self.writer.write_byte(cod.progression_order)?;
        self.writer.write_u16(cod.number_of_layers)?;
        self.writer.write_byte(cod.mct)?;

        // SPcod
        self.writer.write_byte(cod.decomposition_levels)?;
        self.writer.write_byte(cod.codeblock_width_exp)?;
        self.writer.write_byte(cod.codeblock_height_exp)?;
        self.writer.write_byte(cod.code_block_style)?; // SPcod_Scoc byte 9: code-block style
        self.writer.write_byte(cod.transformation)?;

        Ok(())
    }

    pub fn write_qcd(&mut self, qcd: &J2kQcd) -> Result<(), JpeglsError> {
        self.writer
            .write_marker(JpegMarkerCode::QuantizationDefault)?;

        // Determine step size format based on quantization type
        let quant_type = qcd.quant_style & 0x1F;
        let is_16bit = quant_type == 0x02; // Scalar Expounded uses 16-bit steps

        // Lqcd (2) + Sqcd (1) + SPqcd (n bytes)
        let step_size_bytes = if is_16bit { 2 } else { 1 };
        let payload_len = 1 + qcd.step_sizes.len() * step_size_bytes;
        self.writer.write_u16((payload_len + 2) as u16)?;

        self.writer.write_byte(qcd.quant_style)?;

        for &step in &qcd.step_sizes {
            if is_16bit {
                // OpenJPEG and standard expect Big Endian u16
                // (Exponent << 11) | Mantissa
                self.writer.write_u16(step)?;
            } else {
                // For reversible mode (quant_type 0 or 1), step_sizes store
                // epsilon in the high 5 bits of the stored byte value.
                // The encoder stores (epsilon << 11) in u16, so we extract
                // the high byte which contains (epsilon << 3).
                self.writer.write_byte((step >> 8) as u8)?;
            }
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

    /// Write TLM (Tile-part Lengths) marker
    /// Usually placed in the main header to help decoders with random access
    pub fn write_tlm(
        &mut self,
        tile_index: u16,
        tile_len: u32,
        num_tiles: u32,
    ) -> Result<(), JpeglsError> {
        self.writer.write_marker(JpegMarkerCode::TilePartLengths)?;

        // Stlm byte structure:
        // Bits 0-1 (ST): Size of Ttlm (tile index)
        //   0b00 = 0 bytes, 0b01 = 1 byte, 0b10 = 2 bytes
        // Bit 6 (SP): Size of Ptlm (tile-part length)
        //   0 = 2 bytes, 1 = 4 bytes
        let st = if num_tiles <= 1 {
            0
        } else if num_tiles <= 256 {
            1
        } else {
            2
        };
        let sp = if tile_len <= 65535 { 0 } else { 1 };
        let stlm = (sp << 6) | st;

        let st_bytes = match st {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 0,
        };
        let sp_bytes = if sp == 0 { 2 } else { 4 };

        // Length: Ltlm (2) + Ztlm (1) + Stlm (1) + (ST + SP) * n entries
        // Here we write 1 entry
        let payload_len = 2 + (st_bytes + sp_bytes);
        self.writer.write_u16(payload_len as u16 + 2)?;
        self.writer.write_byte(0)?; // Ztlm: index of this marker
        self.writer.write_byte(stlm)?;

        if st == 1 {
            self.writer.write_byte(tile_index as u8)?;
        } else if st == 2 {
            self.writer.write_u16(tile_index)?;
        }

        if sp == 0 {
            self.writer.write_u16(tile_len as u16)?;
        } else {
            self.writer.write_u32(tile_len)?;
        }

        Ok(())
    }

    /// Write PLT (Packet Lengths) marker
    /// Usually placed in the tile-part header (after SOT)
    pub fn write_plt(&mut self, packet_lengths: &[u32]) -> Result<(), JpeglsError> {
        if packet_lengths.is_empty() {
            return Ok(());
        }

        self.writer.write_marker(JpegMarkerCode::PacketLengths)?;

        let mut encoded_lengths = Vec::new();
        for &len in packet_lengths {
            let mut bytes = Vec::new();
            let mut remaining = len;

            // Variable length encoding (7 bits per byte, MSB is continuation bit)
            // LSB-first calculation
            bytes.push((remaining & 0x7F) as u8);
            remaining >>= 7;

            while remaining > 0 {
                bytes.push(0x80 | (remaining & 0x7F) as u8);
                remaining >>= 7;
            }

            // Write in reverse (MSB first in stream)
            for &b in bytes.iter().rev() {
                encoded_lengths.push(b);
            }
        }

        // Length: Lplt (2) + Zplt (1) + encoded lengths
        let payload_len = 1 + encoded_lengths.len();
        self.writer.write_u16(payload_len as u16 + 2)?;
        self.writer.write_byte(0)?; // Zplt: index of this marker
        self.write_bytes(&encoded_lengths)?;

        Ok(())
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
                false, // Signed
                1, 1, // Sub x, y
            )
            .unwrap();

        let cod = J2kCod {
            coding_style: 0,
            progression_order: 0,
            number_of_layers: 1,
            ..Default::default()
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

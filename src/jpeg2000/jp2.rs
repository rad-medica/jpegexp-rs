//! JP2 Box structure implementation (ISO/IEC 15444-1 Annex I).

use crate::error::JpeglsError;

pub struct Jp2Box {
    pub length: u64,
    pub box_type: [u8; 4],
    pub data_range: std::ops::Range<usize>,
}

pub struct Jp2Reader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Jp2Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn find_codestream(&mut self) -> Result<Option<&'a [u8]>, JpeglsError> {
        // JP2 signature box is exactly 12 bytes: 00 00 00 0C 6A 50 20 20 0D 0A 87 0A
        if self.data.len() < 12 {
            return Ok(None);
        }

        if &self.data[0..12] != b"\x00\x00\x00\x0CjP  \r\n\x87\n" {
            // Not a JP2 container
            return Ok(None);
        }

        // It is a JP2 container, proceed to find jp2c
        while let Some(b) = self.read_box()? {
            if b.box_type == *b"jp2c" {
                return Ok(Some(&self.data[b.data_range]));
            }
        }
        Ok(None)
    }

    /// Finds and extracts the ICC profile from the JP2 colr box.
    /// Returns None if no ICC profile is present or the file is not a JP2 container.
    pub fn find_icc_profile(&mut self) -> Result<Option<Vec<u8>>, JpeglsError> {
        // Reset position
        self.position = 0;

        // JP2 signature box check
        if self.data.len() < 12 {
            return Ok(None);
        }
        if &self.data[0..12] != b"\x00\x00\x00\x0CjP  \r\n\x87\n" {
            return Ok(None);
        }

        // Look for the colr box
        while let Some(b) = self.read_box()? {
            if b.box_type == *b"colr" {
                let box_data = &self.data[b.data_range.clone()];
                if box_data.len() < 3 {
                    continue;
                }
                let method = box_data[0];
                // Method 2 = ICC profile
                if method == 2 && box_data.len() > 3 {
                    let icc_data = box_data[3..].to_vec();
                    return Ok(Some(icc_data));
                }
            }
        }
        Ok(None)
    }

    pub fn read_box(&mut self) -> Result<Option<Jp2Box>, JpeglsError> {
        if self.position + 8 > self.data.len() {
            return Ok(None);
        }

        let start_pos = self.position;
        let mut length = u32::from_be_bytes([
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
        ]) as u64;

        let box_type = [
            self.data[self.position + 4],
            self.data[self.position + 5],
            self.data[self.position + 6],
            self.data[self.position + 7],
        ];

        self.position += 8;
        let mut header_size = 8;

        if length == 1 {
            if self.position + 8 > self.data.len() {
                return Err(JpeglsError::InvalidData);
            }
            length = u64::from_be_bytes([
                self.data[self.position],
                self.data[self.position + 1],
                self.data[self.position + 2],
                self.data[self.position + 3],
                self.data[self.position + 4],
                self.data[self.position + 5],
                self.data[self.position + 6],
                self.data[self.position + 7],
            ]);
            self.position += 8;
            header_size += 8;
        } else if length == 0 {
            length = self.data.len() as u64 - start_pos as u64;
        }

        let data_start = start_pos + header_size;
        let data_end = start_pos + length as usize;

        if data_end > self.data.len() {
            return Err(JpeglsError::InvalidData);
        }

        self.position = data_end;

        Ok(Some(Jp2Box {
            length,
            box_type,
            data_range: data_start..data_end,
        }))
    }
}

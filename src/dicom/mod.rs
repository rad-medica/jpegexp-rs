/// DICOM Encapsulation for JPEG 2000
/// 
/// Implements DICOM PS3.5 Section 8.2.4 and Annex A.4 for encapsulating
/// JPEG 2000 codestreams in DICOM format.
/// 
/// # DICOM Fragment Encapsulation Format
/// 
/// ```text
/// Item Tag         (FFFE,E000)  4 bytes
/// Item Length                    4 bytes (JPEG 2000 codestream length)
/// JPEG 2000 Data                 N bytes
/// ```
/// 
/// # Basic Offset Table Format
/// 
/// ```text
/// Item Tag         (FFFE,E000)  4 bytes
/// Item Length                    4 bytes (table length, 0 or 4*num_frames)
/// Frame Offsets                  4*num_frames bytes
/// ```
/// 
/// # Multi-Frame Encapsulation
/// 
/// ```text
/// Basic Offset Table (optional)
/// Frame 1 Fragment (Item Tag + Length + Data)
/// Frame 2 Fragment (Item Tag + Length + Data)
/// ...
/// Sequence Delimiter (FFFE,E0DD) + Length(0)
/// ```

use std::io::{self, Write};

/// DICOM Item Tag for encapsulated data fragments
pub const ITEM_TAG: u16 = 0xFFFE;
pub const ITEM_ELEMENT: u16 = 0xE000;

/// DICOM Sequence Delimiter Tag
pub const SEQ_DELIMITER_TAG: u16 = 0xFFFE;
pub const SEQ_DELIMITER_ELEMENT: u16 = 0xE0DD;

/// DICOM Encapsulation Error
#[derive(Debug)]
pub enum DicomError {
    IoError(io::Error),
    InvalidFrameCount,
    InvalidOffset,
}

impl From<io::Error> for DicomError {
    fn from(e: io::Error) -> Self {
        DicomError::IoError(e)
    }
}

impl std::fmt::Display for DicomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DicomError::IoError(e) => write!(f, "I/O error: {}", e),
            DicomError::InvalidFrameCount => write!(f, "Invalid frame count"),
            DicomError::InvalidOffset => write!(f, "Invalid offset value"),
        }
    }
}

impl std::error::Error for DicomError {}

/// DICOM Encapsulator for JPEG 2000 codestreams
pub struct DicomEncapsulator {
    frames: Vec<Vec<u8>>,
    include_offset_table: bool,
}

impl DicomEncapsulator {
    /// Create a new DICOM encapsulator
    pub fn new() -> Self {
        Self {
            frames: Vec::new(),
            include_offset_table: true,
        }
    }

    /// Set whether to include the Basic Offset Table
    /// 
    /// The Basic Offset Table is optional but recommended for multi-frame images.
    /// It allows random access to frames without parsing the entire encapsulated data.
    pub fn set_include_offset_table(&mut self, include: bool) {
        self.include_offset_table = include;
    }

    /// Add a frame (JPEG 2000 codestream) to the encapsulation
    /// 
    /// # Arguments
    /// * `codestream` - Complete JPEG 2000 codestream (SOC...EOC)
    pub fn add_frame(&mut self, codestream: Vec<u8>) -> Result<(), DicomError> {
        if codestream.len() > u32::MAX as usize {
            return Err(DicomError::InvalidOffset);
        }
        self.frames.push(codestream);
        Ok(())
    }

    /// Get the number of frames
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Calculate the Basic Offset Table
    /// 
    /// Returns a vector of byte offsets for each frame, relative to the first
    /// byte following the Basic Offset Table.
    fn calculate_offsets(&self) -> Vec<u32> {
        let mut offsets = Vec::with_capacity(self.frames.len());
        let mut current_offset = 0u32;

        for frame in &self.frames {
            offsets.push(current_offset);
            // Each frame has: Item Tag (4) + Length (4) + Data (frame.len())
            current_offset += 8 + frame.len() as u32;
        }

        offsets
    }

    /// Write the encapsulated data to a writer
    /// 
    /// # Format
    /// 
    /// For single-frame images:
    /// ```text
    /// Empty Basic Offset Table (8 bytes)
    /// Frame Fragment
    /// Sequence Delimiter (8 bytes)
    /// ```
    /// 
    /// For multi-frame images with offset table:
    /// ```text
    /// Basic Offset Table with frame offsets
    /// Frame 1 Fragment
    /// Frame 2 Fragment
    /// ...
    /// Sequence Delimiter (8 bytes)
    /// ```
    pub fn write<W: Write>(&self, writer: &mut W) -> Result<(), DicomError> {
        if self.frames.is_empty() {
            return Err(DicomError::InvalidFrameCount);
        }

        // Write Basic Offset Table
        if self.include_offset_table && self.frames.len() > 1 {
            self.write_offset_table(writer)?;
        } else {
            // Empty offset table (required even if no offsets)
            self.write_empty_offset_table(writer)?;
        }

        // Write each frame as a fragment
        for frame in &self.frames {
            self.write_fragment(writer, frame)?;
        }

        // Write sequence delimiter
        self.write_sequence_delimiter(writer)?;

        Ok(())
    }

    /// Write Basic Offset Table with frame offsets
    fn write_offset_table<W: Write>(&self, writer: &mut W) -> Result<(), DicomError> {
        let offsets = self.calculate_offsets();
        let table_length = (offsets.len() * 4) as u32;

        // Item Tag (FFFE,E000)
        writer.write_all(&ITEM_TAG.to_le_bytes())?;
        writer.write_all(&ITEM_ELEMENT.to_le_bytes())?;

        // Item Length (number of bytes in offset table)
        writer.write_all(&table_length.to_le_bytes())?;

        // Write each offset as 32-bit little-endian
        for offset in offsets {
            writer.write_all(&offset.to_le_bytes())?;
        }

        Ok(())
    }

    /// Write empty Basic Offset Table
    fn write_empty_offset_table<W: Write>(&self, writer: &mut W) -> Result<(), DicomError> {
        // Item Tag (FFFE,E000)
        writer.write_all(&ITEM_TAG.to_le_bytes())?;
        writer.write_all(&ITEM_ELEMENT.to_le_bytes())?;

        // Item Length (0 = no offsets)
        writer.write_all(&0u32.to_le_bytes())?;

        Ok(())
    }

    /// Write a single frame fragment
    fn write_fragment<W: Write>(&self, writer: &mut W, data: &[u8]) -> Result<(), DicomError> {
        let length = data.len() as u32;

        // Item Tag (FFFE,E000)
        writer.write_all(&ITEM_TAG.to_le_bytes())?;
        writer.write_all(&ITEM_ELEMENT.to_le_bytes())?;

        // Item Length
        writer.write_all(&length.to_le_bytes())?;

        // Frame data
        writer.write_all(data)?;

        Ok(())
    }

    /// Write sequence delimiter
    fn write_sequence_delimiter<W: Write>(&self, writer: &mut W) -> Result<(), DicomError> {
        // Sequence Delimiter Tag (FFFE,E0DD)
        writer.write_all(&SEQ_DELIMITER_TAG.to_le_bytes())?;
        writer.write_all(&SEQ_DELIMITER_ELEMENT.to_le_bytes())?;

        // Length (always 0)
        writer.write_all(&0u32.to_le_bytes())?;

        Ok(())
    }

    /// Calculate the total size of the encapsulated data
    /// 
    /// Useful for pre-allocating buffers.
    pub fn calculate_size(&self) -> usize {
        if self.frames.is_empty() {
            return 0;
        }

        let mut size = 0;

        // Basic Offset Table
        if self.include_offset_table && self.frames.len() > 1 {
            size += 8; // Item tag + length
            size += self.frames.len() * 4; // Offsets
        } else {
            size += 8; // Empty offset table
        }

        // Fragments
        for frame in &self.frames {
            size += 8; // Item tag + length
            size += frame.len(); // Data
        }

        // Sequence delimiter
        size += 8;

        size
    }
}

impl Default for DicomEncapsulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse encapsulated DICOM JPEG 2000 data
pub struct DicomParser<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> DicomParser<'a> {
    /// Create a new DICOM parser
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// Parse and extract all frames
    /// 
    /// Returns a vector of JPEG 2000 codestreams (one per frame)
    pub fn parse_frames(&mut self) -> Result<Vec<Vec<u8>>, DicomError> {
        let mut frames = Vec::new();

        // Skip Basic Offset Table
        self.skip_offset_table()?;

        // Parse fragments until sequence delimiter
        loop {
            if self.is_sequence_delimiter()? {
                break;
            }

            let frame = self.parse_fragment()?;
            frames.push(frame);
        }

        Ok(frames)
    }

    /// Skip the Basic Offset Table
    fn skip_offset_table(&mut self) -> Result<(), DicomError> {
        // Read Item Tag
        let tag = self.read_u16()?;
        let element = self.read_u16()?;

        if tag != ITEM_TAG || element != ITEM_ELEMENT {
            return Err(DicomError::IoError(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected Item Tag for Basic Offset Table",
            )));
        }

        // Read length and skip the table
        let length = self.read_u32()?;
        self.position += length as usize;

        Ok(())
    }

    /// Parse a single fragment
    fn parse_fragment(&mut self) -> Result<Vec<u8>, DicomError> {
        // Read Item Tag
        let tag = self.read_u16()?;
        let element = self.read_u16()?;

        if tag != ITEM_TAG || element != ITEM_ELEMENT {
            return Err(DicomError::IoError(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected Item Tag for fragment",
            )));
        }

        // Read length
        let length = self.read_u32()? as usize;

        // Read data
        if self.position + length > self.data.len() {
            return Err(DicomError::IoError(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Fragment data exceeds buffer",
            )));
        }

        let data = self.data[self.position..self.position + length].to_vec();
        self.position += length;

        Ok(data)
    }

    /// Check if current position is a sequence delimiter
    fn is_sequence_delimiter(&mut self) -> Result<bool, DicomError> {
        if self.position + 4 > self.data.len() {
            return Ok(false);
        }

        let tag = u16::from_le_bytes([self.data[self.position], self.data[self.position + 1]]);
        let element = u16::from_le_bytes([self.data[self.position + 2], self.data[self.position + 3]]);

        if tag == SEQ_DELIMITER_TAG && element == SEQ_DELIMITER_ELEMENT {
            self.position += 8; // Skip delimiter tag + length
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Read a u16 (little-endian)
    fn read_u16(&mut self) -> Result<u16, DicomError> {
        if self.position + 2 > self.data.len() {
            return Err(DicomError::IoError(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Cannot read u16",
            )));
        }

        let value = u16::from_le_bytes([self.data[self.position], self.data[self.position + 1]]);
        self.position += 2;
        Ok(value)
    }

    /// Read a u32 (little-endian)
    fn read_u32(&mut self) -> Result<u32, DicomError> {
        if self.position + 4 > self.data.len() {
            return Err(DicomError::IoError(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Cannot read u32",
            )));
        }

        let value = u32::from_le_bytes([
            self.data[self.position],
            self.data[self.position + 1],
            self.data[self.position + 2],
            self.data[self.position + 3],
        ]);
        self.position += 4;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_frame_encapsulation() {
        let mut encapsulator = DicomEncapsulator::new();
        
        // Create a minimal JPEG 2000 codestream
        let frame = vec![0xFF, 0x4F, 0x00, 0x01, 0x02, 0x03, 0xFF, 0xD9]; // SOC ... EOC
        encapsulator.add_frame(frame.clone()).unwrap();

        let mut output = Vec::new();
        encapsulator.write(&mut output).unwrap();

        // Verify structure:
        // Empty offset table (8 bytes) + Fragment (8 + data) + Delimiter (8)
        let expected_size = 8 + 8 + frame.len() + 8;
        assert_eq!(output.len(), expected_size);

        // Verify tags
        assert_eq!(&output[0..2], &ITEM_TAG.to_le_bytes());
        assert_eq!(&output[2..4], &ITEM_ELEMENT.to_le_bytes());
    }

    #[test]
    fn test_multi_frame_encapsulation() {
        let mut encapsulator = DicomEncapsulator::new();
        
        let frame1 = vec![0xFF, 0x4F, 0x00, 0x01, 0xFF, 0xD9];
        let frame2 = vec![0xFF, 0x4F, 0x00, 0x02, 0x03, 0xFF, 0xD9];
        
        encapsulator.add_frame(frame1.clone()).unwrap();
        encapsulator.add_frame(frame2.clone()).unwrap();

        let mut output = Vec::new();
        encapsulator.write(&mut output).unwrap();

        // Verify offset table is included
        let offset_table_length = u32::from_le_bytes([output[4], output[5], output[6], output[7]]);
        assert_eq!(offset_table_length, 8); // 2 frames * 4 bytes per offset
    }

    #[test]
    fn test_roundtrip() {
        let mut encapsulator = DicomEncapsulator::new();
        
        let frame1 = vec![0xFF, 0x4F, 0x01, 0x02, 0x03, 0xFF, 0xD9];
        let frame2 = vec![0xFF, 0x4F, 0x04, 0x05, 0x06, 0x07, 0xFF, 0xD9];
        
        encapsulator.add_frame(frame1.clone()).unwrap();
        encapsulator.add_frame(frame2.clone()).unwrap();

        let mut output = Vec::new();
        encapsulator.write(&mut output).unwrap();

        // Parse back
        let mut parser = DicomParser::new(&output);
        let frames = parser.parse_frames().unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], frame1);
        assert_eq!(frames[1], frame2);
    }

    #[test]
    fn test_size_calculation() {
        let mut encapsulator = DicomEncapsulator::new();
        
        let frame = vec![0xFF; 100];
        encapsulator.add_frame(frame.clone()).unwrap();

        let calculated_size = encapsulator.calculate_size();
        
        let mut output = Vec::new();
        encapsulator.write(&mut output).unwrap();

        assert_eq!(output.len(), calculated_size);
    }
}

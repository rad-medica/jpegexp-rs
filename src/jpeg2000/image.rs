/// Top-level J2K/HTJ2K Image structure containing metadata and tile data.
#[derive(Debug, Clone, Default)]
pub struct J2kImage {
    /// Total width of the image in pixels.
    pub width: u32,
    /// Total height of the image in pixels.
    pub height: u32,
    /// Width of an individual tile. If equal to `width`, the image has only one tile.
    pub tile_width: u32,
    /// Height of an individual tile.
    pub tile_height: u32,
    /// Number of color or data components.
    pub component_count: u32,
    /// Optional Coding Style Default (COD) parameters.
    pub cod: Option<J2kCod>,
    /// Optional Quantization Default (QCD) parameters.
    pub qcd: Option<J2kQcd>,
    /// Optional Capability (CAP) marker information.
    pub cap: Option<J2kCap>,
    /// List of tiles that make up the image.
    pub tiles: Vec<J2kTile>,
}

/// A single tile-part or tile within a J2K codestream.
#[derive(Debug, Clone, Default)]
pub struct J2kTile {
    /// Index of the tile (0-indexed).
    pub index: u32,
    /// Components belonging to this tile.
    pub components: Vec<J2kTileComponent>,
}

/// Component data specific to a single tile.
#[derive(Debug, Clone, Default)]
pub struct J2kTileComponent {
    /// Index of the component.
    pub component_index: u32,
    /// Resolution levels (multi-resolution decomposition).
    pub resolutions: Vec<J2kResolution>,
    /// Intermediate buffer for DWT/Quantization coefficients.
    pub data: Vec<f32>,
}

/// A specific resolution level in the DWT decomposition.
#[derive(Debug, Clone, Default)]
pub struct J2kResolution {
    /// Decomposition level (0 is the lowest resolution/LL).
    pub level: u8,
    /// Width of the resolution level.
    pub width: u32,
    /// Height of the resolution level.
    pub height: u32,
    /// Subbands (LL, HL, LH, HH) for this level.
    pub subbands: Vec<J2kSubband>,
}

/// A frequency subband within a resolution level.
#[derive(Debug, Clone, Default)]
pub struct J2kSubband {
    /// Orientation of the subband (LL, HL, LH, HH).
    pub orientation: SubbandOrientation,
    /// Width of the subband in pixels/coefficients.
    pub width: u32,
    /// Height of the subband.
    pub height: u32,
    /// Code-blocks that make up the subband data.
    pub codeblocks: Vec<J2kCodeBlock>,
    /// Raw floating-point coefficients after inverse quantization/DWT.
    pub data: Vec<f32>,
}

/// Orientation of a wavelet subband.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubbandOrientation {
    #[default]
    /// Low-Low (base image)
    LL,
    /// High-Low (horizontal details)
    HL,
    /// Low-High (vertical details)
    LH,
    /// High-High (diagonal details)
    HH,
}

/// A core unit of compressed data in J2K (typically 32x32 or 64x64).
#[derive(Debug, Clone, Default)]
pub struct J2kCodeBlock {
    /// Compressed bitstream for the code-block.
    pub compressed_data: Vec<u8>,
    /// Width of the code-block.
    pub width: u32,
    /// Height of the code-block.
    pub height: u32,
    /// Number of zero bit-planes at the start of the block.
    pub zero_bit_planes: u8,
    /// Number of coding passes included in the packet.
    pub coding_passes: u8,
}
/// Coding Style Default (COD) marker information
#[derive(Debug, Clone, Default)]
pub struct J2kCod {
    pub coding_style: u8,
    pub progression_order: u8,
    pub number_of_layers: u16,
    pub mct: u8,
    pub decomposition_levels: u8,
    pub codeblock_width_exp: u8,
    pub codeblock_height_exp: u8,
}

/// Quantization Default (QCD) marker information
#[derive(Debug, Clone, Default)]
pub struct J2kQcd {
    pub quant_style: u8,
    pub step_sizes: Vec<u16>,
}

/// Capability (CAP) marker information (Part 15)
#[derive(Debug, Clone, Default)]
pub struct J2kCap {
    pub pcap: u32,
    pub ccap: Vec<u16>,
}

// Extend J2kImage with optional COD and QCD information
impl J2kImage {
    // Existing fields remain unchanged; we add optional fields via struct definition below
}

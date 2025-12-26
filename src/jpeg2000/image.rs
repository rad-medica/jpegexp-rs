#[derive(Debug, Clone, Default)]
pub struct J2kImage {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub component_count: u32,
    pub cod: Option<J2kCod>,
    pub qcd: Option<J2kQcd>,
    pub cap: Option<J2kCap>,
    pub tiles: Vec<J2kTile>,
}

#[derive(Debug, Clone, Default)]
pub struct J2kTile {
    pub index: u32,
    pub components: Vec<J2kTileComponent>,
}

#[derive(Debug, Clone, Default)]
pub struct J2kTileComponent {
    pub component_index: u32,
    pub resolutions: Vec<J2kResolution>,
    pub data: Vec<f32>, // Intermediate buffer for DWT/Quantization
}

#[derive(Debug, Clone, Default)]
pub struct J2kResolution {
    pub level: u8,
    pub width: u32,
    pub height: u32,
    pub subbands: Vec<J2kSubband>,
}

#[derive(Debug, Clone, Default)]
pub struct J2kSubband {
    pub orientation: SubbandOrientation,
    pub width: u32,
    pub height: u32,
    pub codeblocks: Vec<J2kCodeBlock>,
    pub data: Vec<f32>, // Coefficients
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubbandOrientation {
    #[default]
    LL,
    HL,
    LH,
    HH,
}

#[derive(Debug, Clone, Default)]
pub struct J2kCodeBlock {
    pub compressed_data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub zero_bit_planes: u8,
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

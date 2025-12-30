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
    /// Horizontal offset of the image area on the reference grid.
    pub x_origin: u32,
    /// Vertical offset of the image area on the reference grid.
    pub y_origin: u32,
    /// Horizontal offset of the first tile on the reference grid.
    pub tile_x_origin: u32,
    /// Vertical offset of the first tile on the reference grid.
    pub tile_y_origin: u32,
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
    /// Optional Region of Interest information.
    pub roi: Option<J2kRoi>,
    /// Optional ICC color profile extracted from JP2 container.
    pub icc_profile: Option<Vec<u8>>,
    /// Number of quality layers decoded (for progressive quality).
    pub decoded_layers: u32,
    /// Component information (depth, signedness, subsampling) from SIZ marker.
    pub components: Vec<J2kComponentInfo>,
}

/// Metadata for a single component from the SIZ marker.
#[derive(Debug, Clone, Default)]
pub struct J2kComponentInfo {
    /// bit depth (e.g. 8, 12, 16)
    pub depth: u8,
    /// true if signed, false if unsigned
    pub is_signed: bool,
    /// Horizontal subsampling factor
    pub dx: u8,
    /// Vertical subsampling factor
    pub dy: u8,
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
    /// Horizontal index of the code-block in the subband.
    pub x: u32,
    /// Vertical index of the code-block in the subband.
    pub y: u32,
    /// Width of the code-block.
    pub width: u32,
    /// Height of the code-block.
    pub height: u32,
    /// Number of zero bit-planes at the start of the block.
    pub zero_bit_planes: u8,
    /// Number of coding passes included in the packet.
    pub coding_passes: u8,
    /// Layer contributions: each entry contains data for a specific quality layer.
    pub layer_data: Vec<Vec<u8>>,
    /// Number of layers that have contributed to this codeblock.
    pub layers_decoded: u8,
    /// Decoded coefficient values (accumulated across layers).
    pub coefficients: Vec<i32>,
    /// Internal state of the entropy coder (significance, visited, etc.)
    pub state: Vec<u8>,
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
    pub transformation: u8,
    /// Precinct sizes if defined (Scod bit 0 set).
    /// One byte per resolution level (PPx + PPy<<4).
    pub precinct_sizes: Vec<u8>,
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
    /// Reconstruct pixels from DWT coefficients using IDWT
    /// Returns a vector of pixel values (u8) for the image
    pub fn reconstruct_pixels(&self) -> Result<Vec<u8>, String> {
        if self.tiles.is_empty() {
            eprintln!("DEBUG: reconstruct_pixels - No tiles!");
            return Err("No tiles in image".to_string());
        }
        eprintln!("DEBUG: reconstruct_pixels - Tiles: {}", self.tiles.len());

        let mut pixels = vec![0u8; (self.width * self.height * self.component_count) as usize];
        let pixels_per_component = (self.width * self.height) as usize;

        // For now, handle single tile case
        let tile = &self.tiles[0];
        eprintln!("DEBUG: Tile 0 Components: {}", tile.components.len());

        let cod = self.cod.as_ref().ok_or("No COD marker")?;
        let nom_w = 1 << (cod.codeblock_width_exp + 2);
        let nom_h = 1 << (cod.codeblock_height_exp + 2);

        let get_subband_data = |res: &J2kResolution, orientation: SubbandOrientation| -> Vec<f32> {
            let mut found = None;
            for sb in &res.subbands {
                if sb.orientation == orientation {
                    found = Some(sb);
                    break;
                }
            }
            if let Some(sb) = found {
                let sb_w = sb.width;
                let sb_h = sb.height;
                let cap = (sb_w * sb_h) as usize;
                let mut sb_data = vec![0.0f32; cap]; // Zero initialized

                eprintln!(
                    "DEBUG: Retrieving subband Res=? Ori={:?} Blocks={}",
                    orientation,
                    sb.codeblocks.len()
                );
                for cb in &sb.codeblocks {
                    let start_x = cb.x * nom_w as u32;
                    let start_y = cb.y * nom_h as u32;

                    let nz = cb.coefficients.iter().filter(|&&c| c != 0).count();
                    eprintln!(
                        "DEBUG: CB ({}, {}) w={} h={} coeffs={} (Non-zero: {})",
                        cb.x,
                        cb.y,
                        cb.width,
                        cb.height,
                        cb.coefficients.len(),
                        nz
                    );

                    for cy in 0..cb.height {
                        for cx in 0..cb.width {
                            let src_idx = (cy * cb.width + cx) as usize;
                            if src_idx < cb.coefficients.len() {
                                let val = cb.coefficients[src_idx];
                                let dest_x = start_x + cx;
                                let dest_y = start_y + cy;

                                if dest_x < sb_w && dest_y < sb_h {
                                    let dest_idx = (dest_y * sb_w + dest_x) as usize;
                                    sb_data[dest_idx] = val as f32;
                                }
                            }
                        }
                    }
                }
                sb_data
            } else {
                let w = if orientation == SubbandOrientation::LL
                    || orientation == SubbandOrientation::HL
                {
                    (res.width as usize + 1) / 2
                } else {
                    res.width as usize / 2
                };
                let h = if orientation == SubbandOrientation::LL
                    || orientation == SubbandOrientation::LH
                {
                    (res.height as usize + 1) / 2
                } else {
                    res.height as usize / 2
                };
                vec![0.0f32; w * h]
            }
        };

        for (comp_idx, component) in tile.components.iter().enumerate() {
            eprintln!("DEBUG: Reconstructing Component {}", comp_idx);
            if component.resolutions.is_empty() {
                continue;
            }

            // Start with LL from Resolution 0
            let mut current_ll =
                get_subband_data(&component.resolutions[0], SubbandOrientation::LL);
            eprintln!(
                "DEBUG: Res 0 LL size: {} (Non-zero: {})",
                current_ll.len(),
                current_ll.iter().filter(|&&v| v != 0.0).count()
            );

            if current_ll.is_empty() {
                let r0 = &component.resolutions[0];
                current_ll = vec![0.0f32; (r0.width * r0.height) as usize];
            }

            let cod = self.cod.as_ref().ok_or("No COD marker")?;
            eprintln!(
                "DEBUG: reconstruct_pixels cod.transformation = {}",
                cod.transformation
            );
            let is_reversible = cod.transformation == 1;
            eprintln!("DEBUG: Reversible: {}", is_reversible);

            if !is_reversible {
                let qcd = self.qcd.as_ref().ok_or("No QCD for Irreversible")?;
                let guard_bits = (qcd.quant_style >> 5) & 0x07;
                let decode_step = |val: u16| -> f32 {
                    let exp = (val >> 11) & 0x1F;
                    let mant = val & 0x7FF;
                    let rb = 8 + guard_bits;
                    let s = (1.0 + (mant as f32 / 2048.0)) * 2.0f32.powi(exp as i32 - rb as i32);
                    eprintln!(
                        "DEBUG: Dequant step: val={} exp={} mant={} rb={} s={}",
                        val, exp, mant, rb, s
                    );
                    s
                };
                let step_ll = if !qcd.step_sizes.is_empty() {
                    decode_step(qcd.step_sizes[0])
                } else {
                    1.0
                };
                eprintln!("DEBUG: Res 0 LL Dequant Step: {:.4}", step_ll);
                for v in &mut current_ll {
                    *v *= step_ll;
                }
                eprintln!(
                    "DEBUG: Res 0 LL Samples: {:?}",
                    &current_ll.iter().take(5).collect::<Vec<_>>()
                );
            }

            // Iterate through higher resolutions (1..N) to apply IDWT
            for r in 1..component.resolutions.len() {
                let res = &component.resolutions[r];
                let hl = get_subband_data(res, SubbandOrientation::HL);
                let lh = get_subband_data(res, SubbandOrientation::LH);
                let hh = get_subband_data(res, SubbandOrientation::HH);

                eprintln!(
                    "DEBUG: Res {} IDWT. HL nz: {}, LH nz: {}, HH nz: {}",
                    r,
                    hl.iter().filter(|&&v| v != 0.0).count(),
                    lh.iter().filter(|&&v| v != 0.0).count(),
                    hh.iter().filter(|&&v| v != 0.0).count()
                );

                if !hl.is_empty() {
                    eprintln!(
                        "DEBUG: HL[0..5]: {:?}",
                        hl.iter().take(5).collect::<Vec<_>>()
                    );
                }
                if !lh.is_empty() {
                    eprintln!(
                        "DEBUG: LH[0..5]: {:?}",
                        lh.iter().take(5).collect::<Vec<_>>()
                    );
                }
                if !hh.is_empty() {
                    eprintln!(
                        "DEBUG: HH[0..5]: {:?}",
                        hh.iter().take(5).collect::<Vec<_>>()
                    );
                }

                let mut output = vec![0.0f32; (res.width * res.height) as usize];

                if is_reversible {
                    // Reversible 5-3 (Integers)
                    let ll_i32: Vec<i32> = current_ll.iter().map(|&f| f as i32).collect();
                    let hl_i32: Vec<i32> = hl.iter().map(|&f| f as i32).collect();
                    let lh_i32: Vec<i32> = lh.iter().map(|&f| f as i32).collect();
                    let hh_i32: Vec<i32> = hh.iter().map(|&f| f as i32).collect();
                    let mut output_i32 = vec![0i32; output.len()];

                    crate::jpeg2000::dwt::Dwt53::inverse_2d(
                        &ll_i32,
                        &hl_i32,
                        &lh_i32,
                        &hh_i32,
                        res.width,
                        res.height,
                        &mut output_i32,
                    );
                    for i in 0..output.len() {
                        output[i] = output_i32[i] as f32;
                    }
                } else {
                    // Irreversible 9-7 (Floats)
                    // Dequantization required.
                    let qcd = self.qcd.as_ref().ok_or("No QCD for Irreversible")?;
                    let guard_bits = (qcd.quant_style >> 5) & 0x07;
                    let quant_style = qcd.quant_style & 0x1F; // 0=No, 1=Derived, 2=Expounded

                    // Helper to decode step size
                    let decode_step = |val: u16| -> f32 {
                        let exp = (val >> 11) & 0x1F;
                        let mant = val & 0x7FF;
                        // Rb = component depth + guard bits. assume depth 8.
                        let rb = 8 + guard_bits;
                        // Delta = 2^(Rb - exp) * (1 + mant / 2048.0)
                        let delta =
                            (1.0 + (mant as f32 / 2048.0)) * 2.0f32.powi(rb as i32 - exp as i32);
                        delta
                    };

                    // Determine step sizes for HL, LH, HH
                    let (step_hl, step_lh, step_hh) = if quant_style == 1 {
                        // Derived
                        let base = qcd.step_sizes[0];
                        // TODO: accurate derivation formula. For now, decode base.
                        let s = decode_step(base);
                        // Approximation for derived? Standard says:
                        // epsilon_b = epsilon_0 + (N_L - n_b)
                        // mu_b = mu_0
                        // So we should recalculate using modified exponent.
                        // But for now, let's just use the indexed values if available, or just use base if ONLY base available.
                        if qcd.step_sizes.len() > 1 {
                            // Use Expounded indexing if avail
                            let idx_hl = 1 + (r - 1) * 3;
                            let idx_lh = idx_hl + 1;
                            let idx_hh = idx_hl + 2;
                            (
                                decode_step(qcd.step_sizes[idx_hl.min(qcd.step_sizes.len() - 1)]),
                                decode_step(qcd.step_sizes[idx_lh.min(qcd.step_sizes.len() - 1)]),
                                decode_step(qcd.step_sizes[idx_hh.min(qcd.step_sizes.len() - 1)]),
                            )
                        } else {
                            (s, s, s) // Fallback
                        }
                    } else {
                        // Expounded or Fallback
                        let idx_hl = 1 + (r - 1) * 3;
                        let idx_lh = idx_hl + 1;
                        let idx_hh = idx_hl + 2;
                        (
                            decode_step(qcd.step_sizes[idx_hl.min(qcd.step_sizes.len() - 1)]),
                            decode_step(qcd.step_sizes[idx_lh.min(qcd.step_sizes.len() - 1)]),
                            decode_step(qcd.step_sizes[idx_hh.min(qcd.step_sizes.len() - 1)]),
                        )
                    };

                    eprintln!(
                        "DEBUG: Res {} Dequant Steps: HL={:.4} LH={:.4} HH={:.4}",
                        r, step_hl, step_lh, step_hh
                    );

                    // Apply step sizes
                    let hl_fq: Vec<f32> = hl.iter().map(|&v| v * step_hl).collect();
                    let lh_fq: Vec<f32> = lh.iter().map(|&v| v * step_lh).collect();
                    let hh_fq: Vec<f32> = hh.iter().map(|&v| v * step_hh).collect();

                    crate::jpeg2000::dwt::Dwt97::inverse_2d(
                        &current_ll,
                        &hl_fq,
                        &lh_fq,
                        &hh_fq,
                        res.width,
                        res.height,
                        &mut output,
                    );
                }
                current_ll = output;
            }

            let final_data = current_ll;
            // Convert to u8 pixels and store
            let offset = comp_idx * pixels_per_component;

            eprintln!(
                "DEBUG: Final Pixels Comp {} (showing first 10, offset 128 applied):",
                comp_idx
            );
            for i in 0..10.min(final_data.len()) {
                let val_u8 = (final_data[i] + 128.0).round().clamp(0.0, 255.0) as u8;
                eprintln!("  [{}] {:.2} -> {}", i, final_data[i], val_u8);
            }
            for i in 0..pixels_per_component.min(final_data.len()) {
                let depth = if self.components.len() > comp_idx {
                    self.components[comp_idx].depth
                } else {
                    8
                };
                let shift = depth.saturating_sub(8);
                let level_offset = (1 << (depth - 1)) as f32;
                let scale_div = (1 << shift) as f32;

                // For unsigned data, we subtract level shift in decoder, so here we add it back.
                // If is_signed is true, we might handled it differently, but usually J2K works on signed samples relative to mid-grey.

                let val = ((final_data[i] + level_offset) / scale_div)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                if offset + i < pixels.len() {
                    pixels[offset + i] = val;
                }
            }
        }
        Ok(pixels)
    }
}

/// Region of Interest (ROI) marker information.
#[derive(Debug, Clone, Default)]
pub struct J2kRoi {
    /// Component index affected by ROI.
    pub component_index: u8,
    /// ROI style (0 = implicit, 1 = explicit).
    pub roi_style: u8,
    /// Shift value for ROI coefficients (SPrgn).
    pub shift_value: u8,
}

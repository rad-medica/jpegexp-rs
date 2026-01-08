/// Top-level J2K/HTJ2K Image structure containing metadata and tile data.
#[derive(Debug, Clone, Default)]
pub struct J2kImage {
    pub width: u32,
    pub height: u32,
    pub x_origin: u32,
    pub y_origin: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tile_x_origin: u32,
    pub tile_y_origin: u32,
    pub component_count: u32,
    pub cod: Option<J2kCod>,
    pub qcd: Option<J2kQcd>,
    pub cap: Option<J2kCap>,
    pub roi: Option<J2kRoi>,
    pub icc_profile: Option<Vec<u8>>,
    pub tiles: Vec<J2kTile>,
    pub components: Vec<J2kComponentInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct J2kRoi {
    pub component_index: u16,
    pub shift: u8,
}

#[derive(Debug, Clone, Default)]
pub struct J2kCap {
    pub pcap: u32,
    pub ccap: Vec<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct J2kComponentInfo {
    pub depth: u8,
    pub is_signed: bool,
    pub dx: u8,
    pub dy: u8,
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
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub zero_bit_planes: u8,
    pub coding_passes: u8,
    pub coefficients: Vec<i32>,
    pub layer_data: Vec<Vec<u8>>,
    pub layers_decoded: u8,
    pub state: Vec<u8>,
    pub mq_contexts: Vec<u8>,
    pub mq_a: u32,
    pub mq_c: u32,
    pub mq_ct: u32,
}

#[derive(Debug, Clone, Default)]
pub struct J2kCod {
    pub coding_style: u8,
    pub progression_order: u8,
    pub number_of_layers: u16,
    pub mct: u8,
    pub decomposition_levels: u8,
    pub transformation: u8, // 1: 5-3, 0: 9-7
    pub codeblock_width_exp: u8,
    pub codeblock_height_exp: u8,
    pub precinct_sizes: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct J2kQcd {
    pub quant_style: u8,
    pub step_sizes: Vec<u16>,
}

impl J2kImage {
    pub fn reconstruct_pixels(&self) -> Result<Vec<u8>, String> {
        if self.tiles.is_empty() { return Err("No tiles".to_string()); }
        let tile = &self.tiles[0];
        let cod = self.cod.as_ref().ok_or("No COD")?;
        let nom_w = 1 << (cod.codeblock_width_exp + 2);
        let nom_h = 1 << (cod.codeblock_height_exp + 2);
        let is_reversible = cod.transformation == 1;

        let get_subband_coeffs = |res: &J2kResolution, orientation: SubbandOrientation| -> Vec<i32> {
            for sb in &res.subbands {
                if sb.orientation == orientation {
                    let mut sb_data = vec![0i32; (sb.width * sb.height) as usize];
                    for cb in &sb.codeblocks {
                        let start_x = cb.x * nom_w;
                        let start_y = cb.y * nom_h;
                        for cy in 0..cb.height {
                            for cx in 0..cb.width {
                                let src_idx = (cy * cb.width + cx) as usize;
                                if src_idx < cb.coefficients.len() {
                                    let dest_x = start_x + cx;
                                    let dest_y = start_y + cy;
                                    if dest_x < sb.width && dest_y < sb.height {
                                        sb_data[(dest_y * sb.width + dest_x) as usize] = cb.coefficients[src_idx];
                                    }
                                }
                            }
                        }
                    }
                    return sb_data;
                }
            }
            vec![0; (res.width * res.height) as usize]
        };

        let mut component_buffers: Vec<Vec<i32>> = Vec::new();
        for (comp_idx, component) in tile.components.iter().enumerate() {
            if component.resolutions.is_empty() {
                component_buffers.push(vec![0; (self.width * self.height) as usize]);
                continue;
            }
            let mut current_ll = get_subband_coeffs(&component.resolutions[0], SubbandOrientation::LL);

            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!("COMP {} RES 0 LL: len={} first_few={:?}", 
                         comp_idx, current_ll.len(), &current_ll[..current_ll.len().min(10)]);
            }

            for r in 1..component.resolutions.len() {
                let res = &component.resolutions[r];
                let hl = get_subband_coeffs(res, SubbandOrientation::HL);
                let lh = get_subband_coeffs(res, SubbandOrientation::LH);
                let hh = get_subband_coeffs(res, SubbandOrientation::HH);
                
                // DEBUG: Log subband statistics
                if std::env::var("J2K_DEBUG").is_ok() {
                    let check_subband = |name: &str, data: &[i32]| {
                        if data.len() > 0 {
                            let min = data.iter().min().unwrap();
                            let max = data.iter().max().unwrap();
                            let unique = {
                                let mut v = data.to_vec();
                                v.sort_unstable();
                                v.dedup();
                                v.len()
                            };
                            eprintln!("RECON res={} {}: len={} range=[{},{}] unique={} first_few={:?}", 
                                     r, name, data.len(), min, max, unique, &data[..data.len().min(5)]);
                        }
                    };
                    check_subband("LL", &current_ll);
                    check_subband("HL", &hl);
                    check_subband("LH", &lh);
                    check_subband("HH", &hh);
                }
                
                let mut output = vec![0i32; (res.width * res.height) as usize];
                
                if is_reversible {
                    crate::jpeg2000::dwt::Dwt53::inverse_2d(&current_ll, &hl, &lh, &hh, res.width, res.height, &mut output);
                } else {
                    // Irreversible: use float path
                    let mut cur_ll_f32: Vec<f32> = current_ll.iter().map(|&v| v as f32).collect();
                    let hl_f32: Vec<f32> = hl.iter().map(|&v| v as f32).collect();
                    let lh_f32: Vec<f32> = lh.iter().map(|&v| v as f32).collect();
                    let hh_f32: Vec<f32> = hh.iter().map(|&v| v as f32).collect();
                    
                    // Simple dequantization (approx)
                    let qcd = self.qcd.as_ref().unwrap();
                    let depth = self.components[comp_idx].depth;
                    let guard_bits = (qcd.quant_style >> 5) & 0x07;
                    let step = |idx: usize| -> f32 {
                        let val = qcd.step_sizes[idx];
                        (1.0 + (val & 0x7FF) as f32 / 2048.0) * 2.0f32.powi((depth + guard_bits) as i32 - ((val >> 11) & 0x1F) as i32)
                    };
                    
                    let s_ll = step(0);
                    for v in &mut cur_ll_f32 { *v *= s_ll; }
                    
                    let idx_base = 1 + (r - 1) * 3;
                    let s_hl = step(idx_base);
                    let s_lh = step(idx_base + 1);
                    let s_hh = step(idx_base + 2);
                    let mut hl_f = hl_f32; for v in &mut hl_f { *v *= s_hl; }
                    let mut lh_f = lh_f32; for v in &mut lh_f { *v *= s_lh; }
                    let mut hh_f = hh_f32; for v in &mut hh_f { *v *= s_hh; }

                    let mut out_f = vec![0.0f32; output.len()];
                    crate::jpeg2000::dwt::Dwt97::inverse_2d(&cur_ll_f32, &hl_f, &lh_f, &hh_f, res.width, res.height, &mut out_f);
                    for i in 0..output.len() { output[i] = out_f[i].round() as i32; }
                }
                current_ll = output;
            }
            component_buffers.push(current_ll);
        }

        if cod.mct == 1 && component_buffers.len() >= 3 {
            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!("DEC RCT: Applying inverse RCT to {} pixels", component_buffers[0].len());
                eprintln!("DEC RCT BEFORE: Y[0]={}, U[0]={}, V[0]={}", 
                         component_buffers[0][0], component_buffers[1][0], component_buffers[2][0]);
            }
            for i in 0..component_buffers[0].len() {
                let y = component_buffers[0][i];
                let u = component_buffers[1][i];
                let v = component_buffers[2][i];
                if is_reversible {
                    let g = y - ((u + v) >> 2);
                    component_buffers[0][i] = v + g;
                    component_buffers[1][i] = g;
                    component_buffers[2][i] = u + g;
                } else {
                    let yf = y as f32; let uf = u as f32; let vf = v as f32;
                    component_buffers[0][i] = (yf + 1.402 * vf).round() as i32;
                    component_buffers[1][i] = (yf - 0.34413 * uf - 0.71414 * vf).round() as i32;
                    component_buffers[2][i] = (yf + 1.772 * uf).round() as i32;
                }
            }
            if std::env::var("J2K_DEBUG").is_ok() {
                eprintln!("DEC RCT AFTER: R[0]={}, G[0]={}, B[0]={}", 
                         component_buffers[0][0], component_buffers[1][0], component_buffers[2][0]);
            }
        }

        let max_depth = self.components.iter().map(|c| c.depth).max().unwrap_or(8);
        let bytes_per_sample = if max_depth > 8 { 2 } else { 1 };
        let mut out = vec![0u8; (self.width * self.height * self.component_count) as usize * bytes_per_sample];
        for i in 0..(self.width * self.height) as usize {
            for c in 0..self.component_count as usize {
                let depth = self.components.get(c).map_or(8, |info| info.depth);
                let level_offset = (1i32 << (depth - 1)) as i32;
                let val = component_buffers[c][i] + level_offset;
                let clamped = val.clamp(0, (1i32 << depth) - 1) as u32;
                if max_depth > 8 {
                    out[(i * self.component_count as usize + c) * 2] = clamped as u8;
                    out[(i * self.component_count as usize + c) * 2 + 1] = (clamped >> 8) as u8;
                } else {
                    out[i * self.component_count as usize + c] = clamped as u8;
                }
            }
        }
        Ok(out)
    }
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;

#[test]
fn compare_j2k_headers() {
    let width = 4;
    let height = 4;
    let pixels = vec![128u8; (width * height) as usize];
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; pixels.len() * 20];
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    output.truncate(bytes_written);
    
    let pgm_header = format!("P5\n{} {}\n255\n", width, height);
    let mut pgm_data = Vec::new();
    pgm_data.extend_from_slice(pgm_header.as_bytes());
    pgm_data.extend_from_slice(&pixels);
    fs::write("header_test.pgm", &pgm_data).unwrap();
    
    let opj_output = std::process::Command::new("libs/bin/opj_compress.exe")
        .args(&["-i", "header_test.pgm", "-o", "header_test_opj.j2k", "-n", "2"])
        .output()
        .expect("Failed to run opj_compress");
    
    if !opj_output.status.success() {
        println!("OpenJPEG failed: {}", String::from_utf8_lossy(&opj_output.stderr));
        return;
    }
    
    let opj_data = fs::read("header_test_opj.j2k").unwrap();
    
    println!("=== Our Headers ===");
    parse_markers(&output);
    
    println!("\n=== OpenJPEG Headers ===");
    parse_markers(&opj_data);
    
    let _ = fs::remove_file("header_test.pgm");
    let _ = fs::remove_file("header_test_opj.j2k");
}

fn parse_markers(data: &[u8]) {
    let mut i = 0;
    while i < data.len() - 1 {
        if data[i] != 0xFF {
            i += 1;
            continue;
        }
        
        let marker = (data[i] as u16) << 8 | data[i + 1] as u16;
        
        match marker {
            0xFF4F => {
                println!("SOC at {}", i);
                i += 2;
            }
            0xFF51 => {
                let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                println!("SIZ at {}, len={}", i, len);
                let siz_data = &data[i + 4..i + 2 + len];
                println!("  Rsiz: {:04X}", u16::from_be_bytes([siz_data[0], siz_data[1]]));
                println!("  Xsiz: {}", u32::from_be_bytes([siz_data[2], siz_data[3], siz_data[4], siz_data[5]]));
                println!("  Ysiz: {}", u32::from_be_bytes([siz_data[6], siz_data[7], siz_data[8], siz_data[9]]));
                println!("  XOsiz: {}", u32::from_be_bytes([siz_data[10], siz_data[11], siz_data[12], siz_data[13]]));
                println!("  YOsiz: {}", u32::from_be_bytes([siz_data[14], siz_data[15], siz_data[16], siz_data[17]]));
                println!("  XTsiz: {}", u32::from_be_bytes([siz_data[18], siz_data[19], siz_data[20], siz_data[21]]));
                println!("  YTsiz: {}", u32::from_be_bytes([siz_data[22], siz_data[23], siz_data[24], siz_data[25]]));
                println!("  XTOsiz: {}", u32::from_be_bytes([siz_data[26], siz_data[27], siz_data[28], siz_data[29]]));
                println!("  YTOsiz: {}", u32::from_be_bytes([siz_data[30], siz_data[31], siz_data[32], siz_data[33]]));
                println!("  Csiz: {}", u16::from_be_bytes([siz_data[34], siz_data[35]]));
                if len > 38 {
                    println!("  Ssiz[0]: {} (bits={}, signed={})", siz_data[36], (siz_data[36] & 0x7F) + 1, siz_data[36] >> 7);
                    println!("  XRsiz[0]: {}", siz_data[37]);
                    println!("  YRsiz[0]: {}", siz_data[38]);
                }
                i += 2 + len;
            }
            0xFF52 => {
                let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                println!("COD at {}, len={}", i, len);
                let cod_data = &data[i + 4..i + 2 + len];
                println!("  Scod: {:02X}", cod_data[0]);
                println!("  SGcod_ProgOrder: {}", cod_data[1]);
                println!("  SGcod_NumLayers: {}", u16::from_be_bytes([cod_data[2], cod_data[3]]));
                println!("  SGcod_MCT: {}", cod_data[4]);
                println!("  SPcod_NumDecompLevels: {}", cod_data[5]);
                println!("  SPcod_CblkWidthExp: {}", cod_data[6]);
                println!("  SPcod_CblkHeightExp: {}", cod_data[7]);
                println!("  SPcod_CblkStyle: {:02X}", cod_data[8]);
                println!("  SPcod_Transform: {} ({})", cod_data[9], if cod_data[9] == 0 { "9-7" } else { "5-3" });
                i += 2 + len;
            }
            0xFF5C => {
                let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                println!("QCD at {}, len={}", i, len);
                let qcd_data = &data[i + 4..i + 2 + len];
                let sqcd = qcd_data[0];
                let quant_style = sqcd & 0x1F;
                let guard_bits = sqcd >> 5;
                println!("  Sqcd: {:02X} (style={}, guard_bits={})", sqcd, quant_style, guard_bits);
                
                if quant_style == 0 {
                    println!("  Reversible (no quantization)");
                    for (idx, &byte) in qcd_data[1..].iter().enumerate() {
                        let epsilon = byte >> 3;
                        println!("    Subband {}: epsilon={}", idx, epsilon);
                    }
                } else {
                    println!("  Derived or expounded quantization");
                    let step_data = &qcd_data[1..];
                    for idx in (0..step_data.len()).step_by(2) {
                        if idx + 1 < step_data.len() {
                            let step = u16::from_be_bytes([step_data[idx], step_data[idx + 1]]);
                            let epsilon = step >> 11;
                            let mantissa = step & 0x7FF;
                            println!("    Subband {}: epsilon={}, mantissa={}", idx / 2, epsilon, mantissa);
                        }
                    }
                }
                i += 2 + len;
            }
            0xFF90 => {
                let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                println!("SOT at {}, len={}", i, len);
                let sot_data = &data[i + 4..i + 2 + len];
                println!("  Isot: {}", u16::from_be_bytes([sot_data[0], sot_data[1]]));
                println!("  Psot: {}", u32::from_be_bytes([sot_data[2], sot_data[3], sot_data[4], sot_data[5]]));
                println!("  TPsot: {}", sot_data[6]);
                println!("  TNsot: {}", sot_data[7]);
                i += 2 + len;
            }
            0xFF93 => {
                println!("SOD at {}", i);
                return;
            }
            0xFFD9 => {
                println!("EOC at {}", i);
                return;
            }
            _ => {
                if marker >= 0xFF00 && marker <= 0xFFFE {
                    if marker >= 0xFF30 && marker <= 0xFF3F {
                        i += 2;
                    } else if i + 3 < data.len() {
                        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                        println!("Marker {:04X} at {}, len={}", marker, i, len);
                        i += 2 + len;
                    } else {
                        i += 2;
                    }
                } else {
                    i += 1;
                }
            }
        }
    }
}

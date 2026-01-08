use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;

const OPJ_COMPRESS: &str = "openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_compress.exe";
const OPJ_DECOMPRESS: &str = "openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_decompress.exe";

fn calculate_mae(original: &[u16], decoded: &[u16]) -> f64 {
    if original.len() != decoded.len() { return f64::MAX; }
    let sum: u64 = original.iter().zip(decoded.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs() as u64).sum();
    sum as f64 / original.len() as f64
}

fn write_pgm_ppm(path: &str, pixels: &[u16], w: u32, h: u32, c: u32, depth: u8) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    let magic = if c == 1 { "P5" } else { "P6" };
    let maxval = (1 << depth) - 1;
    writeln!(f, "{}\n{} {}\n{}", magic, w, h, maxval)?;
    if depth > 8 {
        for &v in pixels { f.write_all(&[(v >> 8) as u8, (v & 0xFF) as u8])?; }
    } else {
        for &v in pixels { f.write_all(&[v as u8])?; }
    }
    Ok(())
}

fn read_pgm_ppm(path: &str) -> std::io::Result<(Vec<u16>, u32, u32, u32)> {
    let data = std::fs::read(path)?;
    let mut pos = 0;
    if data[0] != b'P' { return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not PGM/PPM")); }
    let magic = data[1];
    pos += 2;
    let mut in_comment = false;
    let mut next_token = |p: &mut usize| -> String {
        let mut s = String::new();
        while *p < data.len() {
            let b = data[*p];
            if in_comment { if b == b'\n' { in_comment = false; } *p += 1; continue; }
            if (b as char).is_whitespace() { if !s.is_empty() { *p += 1; break; } *p += 1; continue; }
            if b == b'#' { in_comment = true; *p += 1; continue; }
            s.push(b as char); *p += 1;
        }
        s
    };
    let w: u32 = next_token(&mut pos).parse().unwrap();
    let h: u32 = next_token(&mut pos).parse().unwrap();
    let maxval: u32 = next_token(&mut pos).parse().unwrap();
    if pos < data.len() && (data[pos] as char).is_whitespace() { pos += 1; }
    let pixel_data = &data[pos..];
    let mut pixels = Vec::new();
    if maxval > 255 {
        for chunk in pixel_data.chunks(2) { if chunk.len() == 2 { pixels.push(((chunk[0] as u16) << 8) | (chunk[1] as u16)); } }
    } else {
        for &b in pixel_data { pixels.push(b as u16); }
    }
    let components = if magic == b'5' { 1 } else { 3 };
    Ok((pixels, w, h, components))
}

#[test]
fn test_final_interop() {
    let w = 1; let h = 1; let c = 1; let depth = 8;
    let original_u16: Vec<u16> = vec![100; (w * h * c) as usize];
    let pixels_u8 = vec![100u8; (w * h * c) as usize];
    let frame_info = FrameInfo { width: w, height: h, bits_per_sample: depth, component_count: c as i32 };

    // 1. jpegexp -> OpenJPEG (Lossless)
    let mut encoded = vec![0u8; 4096];
    let mut enc = J2kEncoder::new();
    enc.set_irreversible(false);
    enc.set_decomposition_levels(0);
    let len = enc.encode(&pixels_u8, &frame_info, &mut encoded).unwrap();
    let j2k_path = "tests/final_interop_A.j2k";
    std::fs::write(j2k_path, &encoded[..len]).unwrap();
    let out_pgm = "tests/final_interop_A.pgm";
    let _ = Command::new(OPJ_DECOMPRESS).args(["-i", j2k_path, "-o", out_pgm]).output().unwrap();
    let (dec, _, _, _) = read_pgm_ppm(out_pgm).unwrap();
    let mae_a = calculate_mae(&original_u16, &dec);
    println!("jpegexp -> OpenJPEG Lossless MAE: {:.4}", mae_a);
    if mae_a > 0.0 {
        println!("Value: Exp {} -> Got {}", original_u16[0], dec[0]);
    }

    // 2. OpenJPEG -> jpegexp (Lossless)
    let src_pgm = "tests/final_interop_B_src.pgm";
    write_pgm_ppm(src_pgm, &original_u16, w, h, c, depth as u8).unwrap();
    let j2k_path_b = "tests/final_interop_B.j2k";
    let _ = Command::new(OPJ_COMPRESS).args(["-i", src_pgm, "-o", j2k_path_b, "-r", "1"]).output().unwrap();
    let data_b = std::fs::read(j2k_path_b).unwrap();
    let mut reader = JpegStreamReader::new(&data_b);
    let mut dec_obj = J2kDecoder::new(&mut reader);
    let img = dec_obj.decode().unwrap();
    let dec_u8 = img.reconstruct_pixels().unwrap();
    let dec_u16_b: Vec<u16> = dec_u8.iter().map(|&b| b as u16).collect();
    let mae_b = calculate_mae(&original_u16, &dec_u16_b);
    println!("OpenJPEG -> jpegexp Lossless MAE: {:.4}", mae_b);
    if mae_b > 0.0 {
        println!("Value: Exp {} -> Got {}", original_u16[0], dec_u16_b[0]);
    }

    assert_eq!(mae_a, 0.0);
    assert_eq!(mae_b, 0.0);
}

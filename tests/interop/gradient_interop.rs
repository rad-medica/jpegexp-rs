use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;

fn calculate_mae(original: &[u16], decoded: &[u16]) -> f64 {
    if original.len() != decoded.len() { return f64::MAX; }
    let sum: u64 = original.iter().zip(decoded.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).unsigned_abs() as u64).sum();
    sum as f64 / original.len() as f64
}

#[test]
#[ignore = "OpenJPEG path not configured - incomplete test"]
fn test_gradient_interop() {
    let w = 256; let h = 256; let c = 1; let depth = 8;
    let mut original_u16 = Vec::new();
    let mut pixels_u8 = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let val = (x % 256) as u8;
            original_u16.push(val as u16);
            pixels_u8.push(val);
        }
    }
    let frame_info = FrameInfo { width: w, height: h, bits_per_sample: depth, component_count: c as i32 };

    // 1. jpegexp -> OpenJPEG (Lossless)
    let mut encoded = vec![0u8; (w*h*2) as usize];
    let mut enc = J2kEncoder::new();
    enc.set_irreversible(false);
    let len = enc.encode(&pixels_u8, &frame_info, &mut encoded).unwrap();
    let j2k_path = "tests/fixtures/grad_interop_A.j2k";
    std::fs::write(j2k_path, &encoded[..len]).unwrap();
    let out_pgm = "tests/grad_interop_A.pgm";
    let _ = Command::new("openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_decompress.exe")
        .args(["-i", j2k_path, "-o", out_pgm]).output().unwrap();
    
    // We need a helper to read PGM properly in this test or just rely on MAE failing
    // Let's assume the previous read_pgm works if I copy it.
}

/// Large RGB Image OpenJPEG Interoperability Tests
/// 
/// Tests that our encoder produces files that OpenJPEG can decode correctly
/// and that our decoder can read OpenJPEG-encoded files correctly.
/// 
/// Requirements: opj_compress.exe and opj_decompress.exe in PATH or tests/ directory

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;
use std::path::Path;

fn mae_u8(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        panic!("Length mismatch: {} vs {}", a.len(), b.len());
    }
    let sum: u64 = a.iter().zip(b.iter())
        .map(|(&x, &y)| (x as i32 - y as i32).abs() as u64)
        .sum();
    sum as f64 / a.len() as f64
}

fn find_openjpeg_binary(name: &str) -> Option<String> {
    // Try local tests directory first
    let local = format!("tests/{}.exe", name);
    if Path::new(&local).exists() {
        return Some(local);
    }
    
    // Try without .exe (Linux/Mac)
    let local_unix = format!("tests/{}", name);
    if Path::new(&local_unix).exists() {
        return Some(local_unix);
    }
    
    // Try PATH
    if Command::new(name).arg("--help").output().is_ok() {
        return Some(name.to_string());
    }
    
    None
}

fn write_ppm(path: &str, pixels: &[u8], width: usize, height: usize) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "P6")?;
    writeln!(file, "{} {}", width, height)?;
    writeln!(file, "255")?;
    file.write_all(pixels)?;
    Ok(())
}

fn read_ppm(path: &str) -> std::io::Result<(Vec<u8>, usize, usize)> {
    use std::io::{BufRead, BufReader, Read};
    let file = std::fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !line.trim().starts_with("P6") {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a PPM P6 file"));
    }
    
    // Skip comments
    loop {
        line.clear();
        reader.read_line(&mut line)?;
        if !line.trim().starts_with('#') {
            break;
        }
    }
    
    let dims: Vec<usize> = line.trim().split_whitespace()
        .map(|s| s.parse().unwrap())
        .collect();
    let (width, height) = (dims[0], dims[1]);
    
    line.clear();
    reader.read_line(&mut line)?; // maxval
    
    let mut pixels = vec![0u8; width * height * 3];
    reader.read_exact(&mut pixels)?;
    
    Ok((pixels, width, height))
}

fn create_gradient_rgb(width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let r = ((x * 255) / width) as u8;
            let g = ((y * 255) / height) as u8;
            let b = (((x + y) * 255) / (width + height)) as u8;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }
    }
    pixels
}

fn create_checkerboard_rgb(width: usize, height: usize, block_size: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / block_size) + (y / block_size)) % 2 == 0;
            let val = if is_white { 255 } else { 0 };
            pixels.push(val);
            pixels.push(val);
            pixels.push(val);
        }
    }
    pixels
}

fn test_jpegexp_to_openjpeg(
    name: &str,
    pixels: &[u8],
    width: usize,
    height: usize,
    dwt_levels: u8,
    compress_bin: &str,
    decompress_bin: &str,
) -> Result<f64, String> {
    let temp_dir = std::env::temp_dir();
    let j2k_path = temp_dir.join(format!("test_interop_{}x{}_{}_L{}.j2k", width, height, name, dwt_levels));
    let ppm_path = temp_dir.join(format!("test_interop_{}x{}_{}_L{}.ppm", width, height, name, dwt_levels));
    
    // Encode with jpegexp
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 3,
    };
    
    let mut encoded = vec![0u8; pixels.len() * 4];
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(dwt_levels);
    
    let encoded_len = encoder.encode(pixels, &frame_info, &mut encoded)
        .map_err(|e| format!("Encode failed: {:?}", e))?;
    encoded.truncate(encoded_len);
    
    fs::write(&j2k_path, &encoded)
        .map_err(|e| format!("Write J2K failed: {:?}", e))?;
    
    // Decode with OpenJPEG
    let output = Command::new(decompress_bin)
        .args(["-i", j2k_path.to_str().unwrap(), "-o", ppm_path.to_str().unwrap()])
        .output()
        .map_err(|e| format!("opj_decompress failed: {:?}", e))?;
    
    if !output.status.success() {
        return Err(format!("opj_decompress failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    let (decoded, dec_w, dec_h) = read_ppm(ppm_path.to_str().unwrap())
        .map_err(|e| format!("Read PPM failed: {:?}", e))?;
    
    if dec_w != width || dec_h != height {
        return Err(format!("Size mismatch: expected {}x{}, got {}x{}", width, height, dec_w, dec_h));
    }
    
    let mae = mae_u8(pixels, &decoded);
    
    // Cleanup
    let _ = fs::remove_file(j2k_path);
    let _ = fs::remove_file(ppm_path);
    
    Ok(mae)
}

fn test_openjpeg_to_jpegexp(
    name: &str,
    pixels: &[u8],
    width: usize,
    height: usize,
    dwt_levels: u8,
    compress_bin: &str,
) -> Result<f64, String> {
    let temp_dir = std::env::temp_dir();
    let ppm_path = temp_dir.join(format!("test_interop_opj_{}x{}_{}_L{}.ppm", width, height, name, dwt_levels));
    let j2k_path = temp_dir.join(format!("test_interop_opj_{}x{}_{}_L{}.j2k", width, height, name, dwt_levels));
    
    // Write PPM
    write_ppm(ppm_path.to_str().unwrap(), pixels, width, height)
        .map_err(|e| format!("Write PPM failed: {:?}", e))?;
    
    // Encode with OpenJPEG (lossless: -r 1 means compression ratio 1:1)
    let output = Command::new(compress_bin)
        .args([
            "-i", ppm_path.to_str().unwrap(),
            "-o", j2k_path.to_str().unwrap(),
            "-r", "1",  // Lossless
            "-n", &dwt_levels.to_string(),  // DWT levels
        ])
        .output()
        .map_err(|e| format!("opj_compress failed: {:?}", e))?;
    
    if !output.status.success() {
        return Err(format!("opj_compress failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    // Decode with jpegexp
    let encoded = fs::read(&j2k_path)
        .map_err(|e| format!("Read J2K failed: {:?}", e))?;
    
    let mut reader = JpegStreamReader::new(&encoded);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode()
        .map_err(|e| format!("Decode failed: {:?}", e))?;
    let decoded = image.reconstruct_pixels()
        .map_err(|e| format!("Reconstruct failed: {:?}", e))?;
    
    let mae = mae_u8(pixels, &decoded);
    
    // Cleanup
    let _ = fs::remove_file(ppm_path);
    let _ = fs::remove_file(j2k_path);
    
    Ok(mae)
}

#[test]
fn test_large_gradient_interop() {
    let compress_bin = match find_openjpeg_binary("opj_compress") {
        Some(bin) => bin,
        None => {
            println!("⚠️  Skipping test: opj_compress not found in PATH or tests/");
            println!("   To run this test, install OpenJPEG and ensure opj_compress is in PATH");
            return;
        }
    };
    
    let decompress_bin = match find_openjpeg_binary("opj_decompress") {
        Some(bin) => bin,
        None => {
            println!("⚠️  Skipping test: opj_decompress not found in PATH or tests/");
            return;
        }
    };
    
    println!("\n===================================================================");
    println!("Large RGB Gradient - OpenJPEG Interoperability Test");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (256, 256, 4),
        (512, 512, 5),
        (1024, 1024, 5),
        (2048, 2048, 5),
    ];
    
    for (width, height, max_dwt) in test_cases {
        println!("Testing {}x{} gradient:", width, height);
        let pixels = create_gradient_rgb(width, height);
        
        for dwt in [0, 3, max_dwt] {
            // Test jpegexp -> OpenJPEG
            match test_jpegexp_to_openjpeg("gradient", &pixels, width, height, dwt, &compress_bin, &decompress_bin) {
                Ok(mae) => {
                    if mae < 0.01 {
                        println!("  DWT L{}: jpegexp→OpenJPEG MAE={:.6} ✅", dwt, mae);
                    } else {
                        println!("  DWT L{}: jpegexp→OpenJPEG MAE={:.6} ❌ FAIL", dwt, mae);
                        panic!("Interop failed: MAE={}", mae);
                    }
                }
                Err(e) => {
                    println!("  DWT L{}: jpegexp→OpenJPEG FAILED: {}", dwt, e);
                    panic!("{}", e);
                }
            }
            
            // Test OpenJPEG -> jpegexp
            match test_openjpeg_to_jpegexp("gradient", &pixels, width, height, dwt, &compress_bin) {
                Ok(mae) => {
                    if mae < 0.01 {
                        println!("  DWT L{}: OpenJPEG→jpegexp MAE={:.6} ✅", dwt, mae);
                    } else {
                        println!("  DWT L{}: OpenJPEG→jpegexp MAE={:.6} ❌ FAIL", dwt, mae);
                        panic!("Interop failed: MAE={}", mae);
                    }
                }
                Err(e) => {
                    println!("  DWT L{}: OpenJPEG→jpegexp FAILED: {}", dwt, e);
                    panic!("{}", e);
                }
            }
        }
        println!();
    }
    
    println!("✅ ALL GRADIENT INTEROP TESTS PASSED!");
}

#[test]
fn test_large_checkerboard_interop() {
    let compress_bin = match find_openjpeg_binary("opj_compress") {
        Some(bin) => bin,
        None => {
            println!("⚠️  Skipping test: opj_compress not found");
            return;
        }
    };
    
    let decompress_bin = match find_openjpeg_binary("opj_decompress") {
        Some(bin) => bin,
        None => {
            println!("⚠️  Skipping test: opj_decompress not found");
            return;
        }
    };
    
    println!("\n===================================================================");
    println!("Large RGB Checkerboard - OpenJPEG Interoperability Test");
    println!("===================================================================\n");
    
    let test_cases = vec![
        (256, 256, 8, 4),
        (512, 512, 16, 5),
        (1024, 1024, 32, 5),
    ];
    
    for (width, height, block_size, max_dwt) in test_cases {
        println!("Testing {}x{} checkerboard ({}x{} blocks):", width, height, block_size, block_size);
        let pixels = create_checkerboard_rgb(width, height, block_size);
        
        for dwt in [0, 3, max_dwt] {
            // Test both directions
            match test_jpegexp_to_openjpeg("checkerboard", &pixels, width, height, dwt, &compress_bin, &decompress_bin) {
                Ok(mae) => {
                    if mae < 0.01 {
                        println!("  DWT L{}: jpegexp→OpenJPEG MAE={:.6} ✅", dwt, mae);
                    } else {
                        panic!("Interop failed: MAE={}", mae);
                    }
                }
                Err(e) => panic!("{}", e),
            }
            
            match test_openjpeg_to_jpegexp("checkerboard", &pixels, width, height, dwt, &compress_bin) {
                Ok(mae) => {
                    if mae < 0.01 {
                        println!("  DWT L{}: OpenJPEG→jpegexp MAE={:.6} ✅", dwt, mae);
                    } else {
                        panic!("Interop failed: MAE={}", mae);
                    }
                }
                Err(e) => panic!("{}", e),
            }
        }
        println!();
    }
    
    println!("✅ ALL CHECKERBOARD INTEROP TESTS PASSED!");
}

#[test]
#[ignore] // Very slow - run with: cargo test test_4k_interop -- --ignored --nocapture
fn test_4k_interop() {
    let compress_bin = match find_openjpeg_binary("opj_compress") {
        Some(bin) => bin,
        None => {
            println!("⚠️  Skipping test: opj_compress not found");
            return;
        }
    };
    
    let decompress_bin = match find_openjpeg_binary("opj_decompress") {
        Some(bin) => bin,
        None => {
            println!("⚠️  Skipping test: opj_decompress not found");
            return;
        }
    };
    
    println!("\n===================================================================");
    println!("4K RGB Image - OpenJPEG Interoperability Test");
    println!("===================================================================\n");
    
    let width = 3840;
    let height = 2160;
    println!("Testing 4K (3840x2160) gradient - this will take several minutes...");
    let pixels = create_gradient_rgb(width, height);
    
    for dwt in [0, 5] {
        let start = std::time::Instant::now();
        
        // Test jpegexp -> OpenJPEG
        match test_jpegexp_to_openjpeg("4k_gradient", &pixels, width, height, dwt, &compress_bin, &decompress_bin) {
            Ok(mae) => {
                if mae < 0.01 {
                    println!("  DWT L{}: jpegexp→OpenJPEG MAE={:.6} time={:.1}s ✅", 
                             dwt, mae, start.elapsed().as_secs_f64());
                } else {
                    panic!("4K Interop failed: MAE={}", mae);
                }
            }
            Err(e) => panic!("4K test failed: {}", e),
        }
        
        // Test OpenJPEG -> jpegexp
        let start2 = std::time::Instant::now();
        match test_openjpeg_to_jpegexp("4k_gradient", &pixels, width, height, dwt, &compress_bin) {
            Ok(mae) => {
                if mae < 0.01 {
                    println!("  DWT L{}: OpenJPEG→jpegexp MAE={:.6} time={:.1}s ✅", 
                             dwt, mae, start2.elapsed().as_secs_f64());
                } else {
                    panic!("4K Interop failed: MAE={}", mae);
                }
            }
            Err(e) => panic!("4K test failed: {}", e),
        }
    }
    
    println!("\n✅ 4K INTEROP TEST PASSED!");
}

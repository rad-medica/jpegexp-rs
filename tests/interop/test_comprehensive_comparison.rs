use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::path::Path;
use std::time::Instant;

fn find_openjpeg_binary(name: &str) -> Option<String> {
    // Try centralized libs/bin directory first
    let bin_dir = "libs/bin";
    let bin_path = format!("{}/{}.exe", bin_dir, name);
    if Path::new(&bin_path).exists() {
        return Some(bin_path);
    }
    
    // Try without .exe (Linux/Mac)
    let bin_path_unix = format!("{}/{}", bin_dir, name);
    if Path::new(&bin_path_unix).exists() {
        return Some(bin_path_unix);
    }

    // Try hardcoded Windows path for legacy support
    let hardcoded = if name == "opj_compress" {
        "openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_compress.exe"
    } else {
        "openjpeg/openjpeg-v2.5.2-windows-x64/bin/opj_decompress.exe"
    };
    
    if Path::new(hardcoded).exists() {
        return Some(hardcoded.to_string());
    }
    
    if Command::new(name).arg("-h").output().is_ok() {
        return Some(name.to_string());
    }
    
    None
}

/// Generate test images with various patterns
mod test_patterns {
    pub fn gradient_horizontal(width: usize, height: usize) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height];
        for y in 0..height {
            for x in 0..width {
                pixels[y * width + x] = ((x * 255) / width.saturating_sub(1)) as u8;
            }
        }
        pixels
    }

    pub fn gradient_vertical(width: usize, height: usize) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height];
        for y in 0..height {
            for x in 0..width {
                pixels[y * width + x] = ((y * 255) / height.saturating_sub(1)) as u8;
            }
        }
        pixels
    }

    pub fn gradient_diagonal(width: usize, height: usize) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height];
        for y in 0..height {
            for x in 0..width {
                let dist = ((x + y) * 255) / (width + height - 2);
                pixels[y * width + x] = dist.min(255) as u8;
            }
        }
        pixels
    }

    pub fn checkerboard(width: usize, height: usize, square_size: usize) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height];
        for y in 0..height {
            for x in 0..width {
                let checker = ((x / square_size) + (y / square_size)) % 2;
                pixels[y * width + x] = if checker == 0 { 0 } else { 255 };
            }
        }
        pixels
    }

    pub fn concentric_circles(width: usize, height: usize) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height];
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let max_dist = ((cx * cx + cy * cy).sqrt()) as usize;
        
        for y in 0..height {
            for x in 0..width {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let dist = (dx * dx + dy * dy).sqrt() as usize;
                pixels[y * width + x] = ((dist * 255) / max_dist).min(255) as u8;
            }
        }
        pixels
    }

    pub fn sine_wave(width: usize, height: usize, frequency: f32) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height];
        for y in 0..height {
            for x in 0..width {
                let val = ((x as f32 * frequency * 2.0 * std::f32::consts::PI / width as f32).sin() + 1.0) * 127.5;
                pixels[y * width + x] = val as u8;
            }
        }
        pixels
    }

    pub fn noise(width: usize, height: usize, seed: u64) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height];
        let mut rng = seed;
        for pixel in &mut pixels {
            // Simple LCG random number generator
            rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
            *pixel = (rng >> 24) as u8;
        }
        pixels
    }

    pub fn rgb_gradient(width: usize, height: usize) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height * 3];
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 3;
                pixels[idx] = ((x * 255) / width.saturating_sub(1)) as u8;     // R
                pixels[idx + 1] = ((y * 255) / height.saturating_sub(1)) as u8; // G
                pixels[idx + 2] = 128;                                          // B
            }
        }
        pixels
    }

    pub fn rgb_color_bars(width: usize, height: usize) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height * 3];
        let bar_width = width / 7;
        let colors = [
            (255, 255, 255), // White
            (255, 255, 0),   // Yellow
            (0, 255, 255),   // Cyan
            (0, 255, 0),     // Green
            (255, 0, 255),   // Magenta
            (255, 0, 0),     // Red
            (0, 0, 255),     // Blue
        ];
        
        for y in 0..height {
            for x in 0..width {
                let bar = (x / bar_width).min(6);
                let idx = (y * width + x) * 3;
                pixels[idx] = colors[bar].0;
                pixels[idx + 1] = colors[bar].1;
                pixels[idx + 2] = colors[bar].2;
            }
        }
        pixels
    }
}

/// Calculate Mean Absolute Error
fn calculate_mae(original: &[u8], decoded: &[u8]) -> f64 {
    if original.len() != decoded.len() {
        panic!("Image sizes don't match: {} vs {}", original.len(), decoded.len());
    }
    
    let sum: u64 = original.iter()
        .zip(decoded.iter())
        .map(|(a, b)| (*a as i32 - *b as i32).abs() as u64)
        .sum();
    
    sum as f64 / original.len() as f64
}

/// Calculate Peak Signal-to-Noise Ratio
fn calculate_psnr(original: &[u8], decoded: &[u8]) -> f64 {
    let mae = calculate_mae(original, decoded);
    if mae < 0.0001 {
        return 100.0; // Essentially perfect
    }
    
    let mse = original.iter()
        .zip(decoded.iter())
        .map(|(a, b)| {
            let diff = *a as f64 - *b as f64;
            diff * diff
        })
        .sum::<f64>() / original.len() as f64;
    
    if mse < 0.0001 {
        return 100.0;
    }
    
    20.0 * (255.0_f64).log10() - 10.0 * mse.log10()
}

/// Test configuration
struct TestConfig {
    name: &'static str,
    width: usize,
    height: usize,
    components: u8,
    pattern_fn: fn(usize, usize) -> Vec<u8>,
    qualities: Vec<u8>,
    dwt_levels: Vec<u8>,
    test_lossless: bool,
}

impl TestConfig {
    fn grayscale(name: &'static str, width: usize, height: usize, 
                 pattern_fn: fn(usize, usize) -> Vec<u8>) -> Self {
        Self {
            name,
            width,
            height,
            components: 1,
            pattern_fn,
            qualities: vec![100, 95, 85, 75, 50],
            dwt_levels: vec![3, 5],
            test_lossless: true,
        }
    }

    fn rgb(name: &'static str, width: usize, height: usize,
           pattern_fn: fn(usize, usize) -> Vec<u8>) -> Self {
        Self {
            name,
            width,
            height,
            components: 3,
            pattern_fn,
            qualities: vec![100, 95, 85, 75, 50],
            dwt_levels: vec![3, 5],
            test_lossless: true,
        }
    }
}

struct TestResult {
    name: String,
    width: usize,
    height: usize,
    components: u8,
    mode: String,  // "lossless" or "lossy_Q##"
    dwt_level: u8,
    
    // jpegexp-rs results
    jpegexp_encode_time_us: u64,
    jpegexp_decode_time_us: u64,
    jpegexp_file_size: usize,
    jpegexp_self_mae: f64,
    jpegexp_self_psnr: f64,
    
    // OpenJPEG results
    openjpeg_encode_time_us: Option<u64>,
    openjpeg_decode_time_us: Option<u64>,
    openjpeg_file_size: Option<usize>,
    
    // Cross-compatibility
    jpegexp_to_openjpeg_mae: Option<f64>,
    jpegexp_to_openjpeg_psnr: Option<f64>,
    openjpeg_to_jpegexp_mae: Option<f64>,
    openjpeg_to_openjpeg_psnr: Option<f64>,
}

impl TestResult {
    fn print_summary(&self) {
        println!("\n{:=<80}", "");
        println!("Test: {} ({}x{}, {} components)", self.name, self.width, self.height, self.components);
        println!("Mode: {}, DWT Level: {}", self.mode, self.dwt_level);
        println!("{:-<80}", "");
        
        // jpegexp-rs results
        println!("jpegexp-rs:");
        println!("  Encode: {:>8} μs", self.jpegexp_encode_time_us);
        println!("  Decode: {:>8} μs", self.jpegexp_decode_time_us);
        println!("  Size:   {:>8} bytes", self.jpegexp_file_size);
        println!("  MAE:    {:>8.4}", self.jpegexp_self_mae);
        println!("  PSNR:   {:>8.2} dB", self.jpegexp_self_psnr);
        
        // OpenJPEG results
        if let (Some(enc), Some(dec), Some(size)) = (
            self.openjpeg_encode_time_us,
            self.openjpeg_decode_time_us,
            self.openjpeg_file_size
        ) {
            println!("\nOpenJPEG:");
            println!("  Encode: {:>8} μs  ({:>6.2}x {})", 
                     enc, 
                     enc as f64 / self.jpegexp_encode_time_us as f64,
                     if enc > self.jpegexp_encode_time_us { "slower" } else { "faster" });
            println!("  Decode: {:>8} μs  ({:>6.2}x {})", 
                     dec,
                     dec as f64 / self.jpegexp_decode_time_us as f64,
                     if dec > self.jpegexp_decode_time_us { "slower" } else { "faster" });
            println!("  Size:   {:>8} bytes ({:>6.2}% size)", 
                     size,
                     (size as f64 / self.jpegexp_file_size as f64) * 100.0);
        }
        
        // Cross-compatibility
        if let (Some(mae), Some(psnr)) = (self.jpegexp_to_openjpeg_mae, self.jpegexp_to_openjpeg_psnr) {
            println!("\nCross-Compatibility (jpegexp-rs encode → OpenJPEG decode):");
            println!("  MAE:    {:>8.4}", mae);
            println!("  PSNR:   {:>8.2} dB", psnr);
        }
    }
}

#[test]
#[ignore]  // Run with: cargo test --test comprehensive_comparison -- --ignored --nocapture
fn comprehensive_jpeg2000_comparison() {
    // Check if OpenJPEG is available
    let openjpeg_available = find_openjpeg_binary("opj_compress").is_some();
    if !openjpeg_available {
        println!("⚠️  OpenJPEG not found, running jpegexp-rs tests only");
    }
    
    // Define test configurations
    let configs = vec![
        // Small images (64x64)
        TestConfig::grayscale("gradient_h_64", 64, 64, test_patterns::gradient_horizontal),
        TestConfig::grayscale("checkerboard_64", 64, 64, |w, h| test_patterns::checkerboard(w, h, 8)),
        TestConfig::rgb("rgb_gradient_64", 64, 64, test_patterns::rgb_gradient),
        
        // Medium images (256x256)
        TestConfig::grayscale("gradient_v_256", 256, 256, test_patterns::gradient_vertical),
        TestConfig::grayscale("circles_256", 256, 256, test_patterns::concentric_circles),
        TestConfig::grayscale("sine_256", 256, 256, |w, h| test_patterns::sine_wave(w, h, 8.0)),
        TestConfig::rgb("color_bars_256", 256, 256, test_patterns::rgb_color_bars),
        
        // Large images (512x512)
        TestConfig::grayscale("gradient_d_512", 512, 512, test_patterns::gradient_diagonal),
        TestConfig::grayscale("noise_512", 512, 512, |w, h| test_patterns::noise(w, h, 12345)),
        TestConfig::rgb("rgb_gradient_512", 512, 512, test_patterns::rgb_gradient),
        
        // Medical imaging typical sizes
        TestConfig::grayscale("checkerboard_512", 512, 512, |w, h| test_patterns::checkerboard(w, h, 16)),
        
        // Large for stress testing (1024x1024)
        TestConfig::grayscale("gradient_h_1024", 1024, 1024, test_patterns::gradient_horizontal),
    ];
    
    let mut results = Vec::new();
    
    println!("\n");
    println!("{:=<80}", "");
    println!("COMPREHENSIVE JPEG 2000 COMPARISON TEST SUITE");
    println!("{:=<80}", "");
    println!("jpegexp-rs vs OpenJPEG 2.5.2");
    println!("Total configurations: {}", configs.len());
    println!("");
    
    for config in &configs {
        println!("\nTesting: {}", config.name);
        let pixels = (config.pattern_fn)(config.width, config.height);
        
        for &dwt_level in &config.dwt_levels {
            // Test lossless
            if config.test_lossless {
                let result = run_test(
                    config, &pixels, None, dwt_level, openjpeg_available
                );
                result.print_summary();
                results.push(result);
            }
            
            // Test lossy at various qualities
            for &quality in &config.qualities {
                let result = run_test(
                    config, &pixels, Some(quality), dwt_level, openjpeg_available
                );
                result.print_summary();
                results.push(result);
            }
        }
    }
    
    // Generate summary report
    println!("\n\n");
    println!("{:=<80}", "");
    println!("SUMMARY REPORT");
    println!("{:=<80}", "");
    
    generate_summary_report(&results);
}

/// Write grayscale image in PGM (Portable GrayMap) format
fn write_pgm(path: &Path, pixels: &[u8], width: usize, height: usize) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create(path)?;
    writeln!(file, "P5")?;
    writeln!(file, "{} {}", width, height)?;
    writeln!(file, "255")?;
    file.write_all(pixels)?;
    Ok(())
}

/// Write RGB image in PPM (Portable PixMap) format
fn write_ppm(path: &Path, pixels: &[u8], width: usize, height: usize) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Write;
    
    let mut file = File::create(path)?;
    writeln!(file, "P6")?;
    writeln!(file, "{} {}", width, height)?;
    writeln!(file, "255")?;
    file.write_all(pixels)?;
    Ok(())
}

/// Read PGM file
fn read_pgm(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Read};
    
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    
    // Read magic number
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !line.trim().starts_with("P5") {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a PGM file"));
    }
    
    // Skip comments and read width/height
    line.clear();
    loop {
        reader.read_line(&mut line)?;
        if !line.trim().starts_with('#') {
            break;
        }
        line.clear();
    }
    
    // Read max value
    line.clear();
    reader.read_line(&mut line)?;
    
    // Read pixel data
    let mut pixels = Vec::new();
    reader.read_to_end(&mut pixels)?;
    
    Ok(pixels)
}

/// Read PPM file
fn read_ppm(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Read};
    
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    
    // Read magic number
    let mut line = String::new();
    reader.read_line(&mut line)?;
    if !line.trim().starts_with("P6") {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a PPM file"));
    }
    
    // Skip comments and read width/height
    line.clear();
    loop {
        reader.read_line(&mut line)?;
        if !line.trim().starts_with('#') {
            break;
        }
        line.clear();
    }
    
    // Read max value
    line.clear();
    reader.read_line(&mut line)?;
    
    // Read pixel data
    let mut pixels = Vec::new();
    reader.read_to_end(&mut pixels)?;
    
    Ok(pixels)
}

/// Run OpenJPEG compression and return (encode_time_us, file_size)
fn run_openjpeg_encode(
    input_path: &Path,
    output_path: &Path,
    quality: Option<u8>,
    dwt_level: u8,
) -> Result<(u64, usize), String> {
    let compress_bin = find_openjpeg_binary("opj_compress")
        .ok_or_else(|| "opj_compress not found".to_string())?;
    
    let mut cmd = Command::new(&compress_bin);
    cmd.arg("-i").arg(input_path);
    cmd.arg("-o").arg(output_path);
    cmd.arg("-n").arg(dwt_level.to_string()); // Number of resolutions = dwt_level + 1
    
    if let Some(q) = quality {
        // OpenJPEG uses -r for rate (compression ratio)
        // For lossy, we'll use quality-based approach
        // Quality 100 ≈ rate 1 (nearly lossless), Quality 50 ≈ rate 10
        let rate = if q >= 95 {
            1.0
        } else if q >= 75 {
            2.0 + (95 - q) as f64 * 0.15
        } else {
            5.0 + (75 - q) as f64 * 0.2
        };
        cmd.arg("-r").arg(rate.to_string());
        cmd.arg("-I"); // Use irreversible 9-7 wavelet
    } else {
        // Lossless mode (default)
    }
    
    let start = Instant::now();
    let output = cmd.output().map_err(|e| format!("Failed to run opj_compress: {}", e))?;
    let encode_time = start.elapsed().as_micros() as u64;
    
    if !output.status.success() {
        return Err(format!(
            "opj_compress failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    let file_size = std::fs::metadata(output_path)
        .map_err(|e| format!("Failed to read output file: {}", e))?
        .len() as usize;
    
    Ok((encode_time, file_size))
}

/// Run OpenJPEG decompression and return (decode_time_us, pixels)
fn run_openjpeg_decode(
    input_path: &Path,
    output_path: &Path,
) -> Result<(u64, Vec<u8>), String> {
    let decompress_bin = find_openjpeg_binary("opj_decompress")
        .ok_or_else(|| "opj_decompress not found".to_string())?;
    
    let mut cmd = Command::new(&decompress_bin);
    cmd.arg("-i").arg(input_path);
    cmd.arg("-o").arg(output_path);
    
    let start = Instant::now();
    let output = cmd.output().map_err(|e| format!("Failed to run opj_decompress: {}", e))?;
    let decode_time = start.elapsed().as_micros() as u64;
    
    if !output.status.success() {
        return Err(format!(
            "opj_decompress failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    // Read the output file
    let pixels = if output_path.extension().and_then(|s| s.to_str()) == Some("ppm") {
        read_ppm(output_path)
    } else {
        read_pgm(output_path)
    }.map_err(|e| format!("Failed to read decoded image: {}", e))?;
    
    Ok((decode_time, pixels))
}

/// Run complete OpenJPEG comparison
/// Returns: (encode_time, decode_time, file_size, jpegexp_to_opj_mae, jpegexp_to_opj_psnr, opj_to_jpegexp_mae)
fn run_openjpeg_comparison(
    config: &TestConfig,
    pixels: &[u8],
    jpegexp_data: &[u8],
    quality: Option<u8>,
    dwt_level: u8,
) -> Result<(u64, u64, usize, f64, f64, f64), String> {
    use std::fs;
    
    // Create temp directory for OpenJPEG files
    let temp_dir = std::env::temp_dir().join(format!("jpegexp_openjpeg_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;
    
    // Write input image
    let input_ext = if config.components == 3 { "ppm" } else { "pgm" };
    let input_path = temp_dir.join(format!("input.{}", input_ext));
    
    if config.components == 3 {
        write_ppm(&input_path, pixels, config.width, config.height)
    } else {
        write_pgm(&input_path, pixels, config.width, config.height)
    }.map_err(|e| format!("Failed to write input file: {}", e))?;
    
    // OpenJPEG encode
    let opj_j2k_path = temp_dir.join("openjpeg.j2k");
    let (opj_encode_time, opj_file_size) = run_openjpeg_encode(
        &input_path,
        &opj_j2k_path,
        quality,
        dwt_level,
    )?;
    
    // OpenJPEG decode (from OpenJPEG-encoded file)
    let opj_output_path = temp_dir.join(format!("openjpeg_decoded.{}", input_ext));
    let (opj_decode_time, _opj_decoded_pixels) = run_openjpeg_decode(
        &opj_j2k_path,
        &opj_output_path,
    )?;
    
    // Cross-compatibility: jpegexp-rs encode → OpenJPEG decode
    let jpegexp_j2k_path = temp_dir.join("jpegexp.j2k");
    fs::write(&jpegexp_j2k_path, jpegexp_data)
        .map_err(|e| format!("Failed to write jpegexp data: {}", e))?;
    
    let jpegexp_output_path = temp_dir.join(format!("jpegexp_decoded.{}", input_ext));
    let (_, jpegexp_to_opj_pixels) = run_openjpeg_decode(
        &jpegexp_j2k_path,
        &jpegexp_output_path,
    )?;
    
    // Calculate cross-compatibility metrics
    let jpegexp_to_opj_mae = calculate_mae(pixels, &jpegexp_to_opj_pixels);
    let jpegexp_to_opj_psnr = calculate_psnr(pixels, &jpegexp_to_opj_pixels);
    
    // Cross-compatibility: OpenJPEG encode → jpegexp-rs decode
    let opj_data = fs::read(&opj_j2k_path)
        .map_err(|e| format!("Failed to read OpenJPEG output: {}", e))?;
    
    let opj_to_jpegexp_mae = match decode_with_jpegexp(&opj_data) {
        Ok(decoded) => calculate_mae(pixels, &decoded),
        Err(e) => {
            eprintln!("Warning: jpegexp-rs failed to decode OpenJPEG output: {}", e);
            f64::NAN
        }
    };
    
    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
    
    Ok((
        opj_encode_time,
        opj_decode_time,
        opj_file_size,
        jpegexp_to_opj_mae,
        jpegexp_to_opj_psnr,
        opj_to_jpegexp_mae,
    ))
}

/// Decode JPEG 2000 data with jpegexp-rs
fn decode_with_jpegexp(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = JpegStreamReader::new(data);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().map_err(|e| format!("Decode failed: {:?}", e))?;
    image.reconstruct_pixels().map_err(|e| format!("Reconstruct failed: {:?}", e))
}

fn run_test(
    config: &TestConfig,
    pixels: &[u8],
    quality: Option<u8>,
    dwt_level: u8,
    test_openjpeg: bool,
) -> TestResult {
    let mode = if let Some(q) = quality {
        format!("lossy_Q{}", q)
    } else {
        "lossless".to_string()
    };
    
    // jpegexp-rs encode
    let start = Instant::now();
    let frame_info = FrameInfo {
        width: config.width as u32,
        height: config.height as u32,
        bits_per_sample: 8,
        component_count: config.components as i32,
    };
    
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(dwt_level);
    
    if let Some(q) = quality {
        encoder.set_irreversible(true);
        encoder.set_quality(q);
    }
    
    let mut dest = vec![0u8; config.width * config.height * config.components as usize * 4]; // Generous buffer
    let size = encoder.encode(pixels, &frame_info, &mut dest).expect("jpegexp-rs encode failed");
    let jpegexp_data = &dest[..size];
    let jpegexp_encode_time = start.elapsed().as_micros() as u64;
    let jpegexp_file_size = jpegexp_data.len();
    
    // jpegexp-rs decode (self-roundtrip)
    let start = Instant::now();
    let mut reader = JpegStreamReader::new(jpegexp_data);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode().expect("jpegexp-rs decode failed");
    let jpegexp_decoded = image.reconstruct_pixels().expect("jpegexp-rs reconstruct failed");
    let jpegexp_decode_time = start.elapsed().as_micros() as u64;
    
    let jpegexp_self_mae = calculate_mae(pixels, &jpegexp_decoded);
    let jpegexp_self_psnr = calculate_psnr(pixels, &jpegexp_decoded);
    
    // OpenJPEG tests (if available)
    let (openjpeg_encode_time, openjpeg_decode_time, openjpeg_file_size, jpegexp_to_openjpeg_mae, jpegexp_to_openjpeg_psnr, openjpeg_to_jpegexp_mae) = if test_openjpeg {
        match run_openjpeg_comparison(config, pixels, jpegexp_data, quality, dwt_level) {
            Ok((enc_time, dec_time, file_size, to_opj_mae, to_opj_psnr, from_opj_mae)) => {
                (Some(enc_time), Some(dec_time), Some(file_size), Some(to_opj_mae), Some(to_opj_psnr), Some(from_opj_mae))
            }
            Err(e) => {
                eprintln!("OpenJPEG comparison failed: {}", e);
                (None, None, None, None, None, None)
            }
        }
    } else {
        (None, None, None, None, None, None)
    };
    
    TestResult {
        name: format!("{}_{}", config.name, mode),
        width: config.width,
        height: config.height,
        components: config.components,
        mode,
        dwt_level,
        jpegexp_encode_time_us: jpegexp_encode_time,
        jpegexp_decode_time_us: jpegexp_decode_time,
        jpegexp_file_size,
        jpegexp_self_mae,
        jpegexp_self_psnr,
        openjpeg_encode_time_us: openjpeg_encode_time,
        openjpeg_decode_time_us: openjpeg_decode_time,
        openjpeg_file_size,
        jpegexp_to_openjpeg_mae,
        jpegexp_to_openjpeg_psnr,
        openjpeg_to_jpegexp_mae,
        openjpeg_to_openjpeg_psnr: None, // This would be OpenJPEG self-test PSNR (not currently calculated)
    }
}

fn generate_summary_report(results: &[TestResult]) {
    // Group by mode
    let lossless: Vec<_> = results.iter().filter(|r| r.mode == "lossless").collect();
    let lossy: Vec<_> = results.iter().filter(|r| r.mode != "lossless").collect();
    
    println!("\nLossless Results (MAE should be 0.0):");
    println!("{:-<80}", "");
    println!("{:<30} {:>10} {:>10} {:>12} {:>10}", "Test", "Size", "Time(μs)", "MAE", "PSNR(dB)");
    for r in &lossless {
        println!("{:<30} {:>10} {:>10} {:>12.4} {:>10.2}", 
                 format!("{}x{}_L{}", r.width, r.height, r.dwt_level),
                 r.jpegexp_file_size,
                 r.jpegexp_encode_time_us,
                 r.jpegexp_self_mae,
                 r.jpegexp_self_psnr);
    }
    
    println!("\n\nLossy Results by Quality:");
    println!("{:-<80}", "");
    for quality in &[100, 95, 85, 75, 50] {
        let mode = format!("lossy_Q{}", quality);
        let q_results: Vec<_> = lossy.iter().filter(|r| r.mode == mode).collect();
        if q_results.is_empty() {
            continue;
        }
        
        println!("\nQuality {}:", quality);
        println!("{:<30} {:>10} {:>10} {:>12} {:>10}", "Test", "Size", "Time(μs)", "MAE", "PSNR(dB)");
        for r in q_results {
            println!("{:<30} {:>10} {:>10} {:>12.4} {:>10.2}", 
                     format!("{}x{}_L{}", r.width, r.height, r.dwt_level),
                     r.jpegexp_file_size,
                     r.jpegexp_encode_time_us,
                     r.jpegexp_self_mae,
                     r.jpegexp_self_psnr);
        }
    }
    
    // Statistics
    println!("\n\nOverall Statistics:");
    println!("{:-<80}", "");
    
    let avg_lossless_mae: f64 = lossless.iter().map(|r| r.jpegexp_self_mae).sum::<f64>() / lossless.len() as f64;
    let max_lossless_mae = lossless.iter().map(|r| r.jpegexp_self_mae).fold(0.0f64, f64::max);
    println!("Lossless MAE - Average: {:.4}, Max: {:.4}", avg_lossless_mae, max_lossless_mae);
    
    if !lossy.is_empty() {
        let avg_lossy_psnr: f64 = lossy.iter().map(|r| r.jpegexp_self_psnr).sum::<f64>() / lossy.len() as f64;
        let min_lossy_psnr = lossy.iter().map(|r| r.jpegexp_self_psnr).fold(f64::INFINITY, f64::min);
        println!("Lossy PSNR - Average: {:.2} dB, Min: {:.2} dB", avg_lossy_psnr, min_lossy_psnr);
    }
    
    println!("\n✅ All tests completed successfully!");
}

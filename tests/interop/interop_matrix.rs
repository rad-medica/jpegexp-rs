/// Interoperability Test Matrix Orchestrator
///
/// This module provides a unified framework for testing codec interoperability
/// with reference implementations:
/// - JPEG 1 vs libjpeg-turbo
/// - JPEG-LS vs CharLS
/// - JPEG 2000 vs OpenJPEG
/// - HTJ2K vs OpenHTJ2K
///
/// Each test validates bidirectional compatibility (Rust→Ref and Ref→Rust)
/// and reports metrics (MAE, PSNR, file size, timing).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use std::fs;
use std::io::Write;

/// Test result for a single codec/direction combination
#[derive(Debug, Clone)]
pub struct InteropTestResult {
    pub test_name: String,
    pub codec: String,
    pub direction: Direction,
    pub width: u32,
    pub height: u32,
    pub components: u8,
    pub bit_depth: u8,
    pub encode_time: Duration,
    pub decode_time: Duration,
    pub compressed_size: usize,
    pub mae: f64,
    pub psnr: f64,
    pub status: TestStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Encode with jpegexp-rs, decode with reference
    RustToReference,
    /// Encode with reference, decode with jpegexp-rs
    ReferenceToRust,
    /// Full roundtrip: jpegexp-rs → jpegexp-rs
    RustRoundtrip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Pass,
    Fail,
    Skip,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::RustToReference => write!(f, "Rust→Ref"),
            Direction::ReferenceToRust => write!(f, "Ref→Rust"),
            Direction::RustRoundtrip => write!(f, "Rust↔Rust"),
        }
    }
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Pass => write!(f, "✅ PASS"),
            TestStatus::Fail => write!(f, "❌ FAIL"),
            TestStatus::Skip => write!(f, "⏭️  SKIP"),
        }
    }
}

/// Binary locator for external reference tools
pub struct BinaryLocator {
    bin_dir: PathBuf,
}

impl BinaryLocator {
    pub fn new() -> Self {
        Self {
            bin_dir: PathBuf::from("libs/bin"),
        }
    }

    /// Find a binary in the libs/bin directory or system PATH
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        // Try libs/bin first
        let exe_name = if cfg!(windows) {
            format!("{}.exe", name)
        } else {
            name.to_string()
        };

        let path = self.bin_dir.join(&exe_name);
        if path.exists() {
            return Some(path);
        }

        // Try without .exe extension in libs/bin
        if name.ends_with(".exe") {
            let path = self.bin_dir.join(name);
            if path.exists() {
                return Some(path);
            }
        }

        // Try system PATH
        let check_cmd = if cfg!(windows) { "where" } else { "which" };
        if let Ok(output) = Command::new(check_cmd).arg(name).output() {
            if output.status.success() {
                return Some(PathBuf::from(name));
            }
        }

        None
    }

    /// Check if all required binaries for a codec are available
    pub fn check_codec_binaries(&self, codec: &str) -> bool {
        match codec {
            "jpeg1" => {
                self.find("cjpeg").is_some() && self.find("djpeg").is_some()
            }
            "jpegls" => {
                self.find("charls-encoder").is_some() 
                    && self.find("charls-decoder").is_some()
            }
            "jpeg2000" => {
                self.find("opj_compress").is_some() 
                    && self.find("opj_decompress").is_some()
            }
            "htj2k" => {
                self.find("oj_compress").is_some() 
                    && self.find("oj_decompress").is_some()
            }
            _ => false,
        }
    }
}

impl Default for BinaryLocator {
    fn default() -> Self {
        Self::new()
    }
}

/// PNM (PGM/PPM) file utilities for test data exchange
pub mod pnm {
    use super::*;

    /// Write pixels to PNM format (PGM for grayscale, PPM for RGB)
    pub fn write(
        path: &Path,
        pixels: &[u8],
        width: u32,
        height: u32,
        components: u8,
        bit_depth: u8,
    ) -> std::io::Result<()> {
        let magic = if components == 3 { "P6" } else { "P5" };
        let max_val = (1u32 << bit_depth) - 1;

        let mut file = fs::File::create(path)?;
        writeln!(file, "{}", magic)?;
        writeln!(file, "{} {}", width, height)?;
        writeln!(file, "{}", max_val)?;

        if bit_depth <= 8 {
            file.write_all(pixels)?;
        } else {
            // Convert from native endian to big-endian (PNM standard)
            let pixel_count = (width * height * components as u32) as usize;
            for i in 0..pixel_count {
                let val = u16::from_ne_bytes([pixels[i * 2], pixels[i * 2 + 1]]);
                file.write_all(&val.to_be_bytes())?;
            }
        }

        Ok(())
    }

    /// Read pixels from PNM format
    pub fn read(
        path: &Path,
        expected_pixel_count: usize,
        bit_depth: u8,
        components: u8,
    ) -> std::io::Result<Vec<u8>> {
        let data = fs::read(path)?;
        parse_pixels(&data, expected_pixel_count, bit_depth, components)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Failed to parse PNM file"
                )
            })
    }

    fn parse_pixels(
        data: &[u8],
        expected_pixel_count: usize,
        bit_depth: u8,
        components: u8,
    ) -> Option<Vec<u8>> {
        let mut pos = 0;

        // Parse header
        let magic = if components == 3 { b"P6" } else { b"P5" };
        if data.get(pos..pos + 2) != Some(magic) {
            return None;
        }
        pos += 2;

        // Skip comments and whitespace
        let mut count = 0;
        while count < 3 && pos < data.len() {
            while pos < data.len() && data[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if pos < data.len() && data[pos] == b'#' {
                while pos < data.len() && data[pos] != b'\n' {
                    pos += 1;
                }
            } else {
                while pos < data.len() && !data[pos].is_ascii_whitespace() {
                    pos += 1;
                }
                count += 1;
            }
        }

        if pos < data.len() && data[pos].is_ascii_whitespace() {
            pos += 1;
        }

        let pixel_data = &data[pos..];
        let bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };
        let expected_bytes = expected_pixel_count * components as usize * bytes_per_sample;

        if pixel_data.len() < expected_bytes {
            return None;
        }

        let raw = &pixel_data[..expected_bytes];

        if bit_depth <= 8 {
            Some(raw.to_vec())
        } else {
            // Convert from big-endian (PNM standard) to native endian
            let mut native = Vec::with_capacity(expected_bytes);
            for i in 0..(expected_bytes / 2) {
                let val = u16::from_be_bytes([raw[i * 2], raw[i * 2 + 1]]);
                native.extend_from_slice(&val.to_ne_bytes());
            }
            Some(native)
        }
    }
}

/// Metrics calculation utilities
pub mod metrics {
    /// Calculate Mean Absolute Error between two pixel buffers
    pub fn calculate_mae(a: &[u8], b: &[u8], bit_depth: u8) -> f64 {
        if a.len() != b.len() {
            return f64::MAX;
        }

        if bit_depth <= 8 {
            let sum: u64 = a
                .iter()
                .zip(b.iter())
                .map(|(&x, &y)| (x as i32 - y as i32).abs() as u64)
                .sum();
            sum as f64 / a.len() as f64
        } else {
            let count = a.len() / 2;
            let mut total_diff = 0u64;
            for i in 0..count {
                let val_a = u16::from_ne_bytes([a[i * 2], a[i * 2 + 1]]) as i32;
                let val_b = u16::from_ne_bytes([b[i * 2], b[i * 2 + 1]]) as i32;
                total_diff += (val_a - val_b).abs() as u64;
            }
            total_diff as f64 / count as f64
        }
    }

    /// Calculate Peak Signal-to-Noise Ratio
    pub fn calculate_psnr(mae: f64, bit_depth: u8) -> f64 {
        if mae == 0.0 {
            return f64::INFINITY;
        }

        let max_value = (1u64 << bit_depth) - 1;
        let mse = mae * mae;
        20.0 * (max_value as f64).log10() - 10.0 * mse.log10()
    }
}

/// Test report generator
pub struct TestReportGenerator {
    results: Vec<InteropTestResult>,
}

impl TestReportGenerator {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: InteropTestResult) {
        self.results.push(result);
    }

    /// Print summary to console
    pub fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║         INTEROPERABILITY TEST RESULTS SUMMARY                  ║");
        println!("╚════════════════════════════════════════════════════════════════╝\n");

        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.status == TestStatus::Pass).count();
        let failed = self.results.iter().filter(|r| r.status == TestStatus::Fail).count();
        let skipped = self.results.iter().filter(|r| r.status == TestStatus::Skip).count();

        println!("Total Tests: {}", total);
        println!("✅ Passed:   {} ({:.1}%)", passed, passed as f64 / total as f64 * 100.0);
        println!("❌ Failed:   {} ({:.1}%)", failed, failed as f64 / total as f64 * 100.0);
        println!("⏭️  Skipped:  {} ({:.1}%)\n", skipped, skipped as f64 / total as f64 * 100.0);

        // Group by codec
        let mut by_codec: std::collections::HashMap<String, Vec<&InteropTestResult>> = 
            std::collections::HashMap::new();
        
        for result in &self.results {
            by_codec.entry(result.codec.clone())
                .or_insert_with(Vec::new)
                .push(result);
        }

        for (codec, results) in by_codec.iter() {
            let passed = results.iter().filter(|r| r.status == TestStatus::Pass).count();
            let total = results.len();
            
            println!("┌─ {} ({}/{} passed)", codec, passed, total);
            
            for result in results {
                println!("│  {} {} - MAE: {:.4}, Size: {} bytes",
                    result.status,
                    result.test_name,
                    result.mae,
                    result.compressed_size
                );
                
                if let Some(ref err) = result.error_message {
                    println!("│    Error: {}", err);
                }
            }
            println!("└─");
        }
    }

    /// Save results to CSV file
    pub fn save_csv(&self, path: &Path) -> std::io::Result<()> {
        let mut csv = String::from(
            "TestName,Codec,Direction,Width,Height,Components,BitDepth,\
             EncTime_ms,DecTime_ms,Size,MAE,PSNR,Status,Error\n"
        );

        for r in &self.results {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{:.4},{:.2},{},{}\n",
                r.test_name,
                r.codec,
                r.direction,
                r.width,
                r.height,
                r.components,
                r.bit_depth,
                r.encode_time.as_millis(),
                r.decode_time.as_millis(),
                r.compressed_size,
                r.mae,
                r.psnr,
                match r.status {
                    TestStatus::Pass => "PASS",
                    TestStatus::Fail => "FAIL",
                    TestStatus::Skip => "SKIP",
                },
                r.error_message.as_deref().unwrap_or("")
            ));
        }

        fs::write(path, csv)
    }
}

impl Default for TestReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_locator() {
        let locator = BinaryLocator::new();
        
        // Just test that it doesn't panic
        let _ = locator.find("opj_compress");
        let _ = locator.check_codec_binaries("jpeg2000");
    }

    #[test]
    fn test_pnm_roundtrip_8bit() {
        use std::env;
        
        let pixels = vec![0, 64, 128, 192, 255, 128, 64, 0];
        let temp_dir = env::temp_dir();
        let path = temp_dir.join("test_pnm_8bit.pgm");

        pnm::write(&path, &pixels, 2, 4, 1, 8).unwrap();
        let read_pixels = pnm::read(&path, 8, 8, 1).unwrap();

        assert_eq!(pixels, read_pixels);
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_metrics_mae() {
        let a = vec![0u8, 10, 20, 30];
        let b = vec![0u8, 12, 18, 30];
        
        let mae = metrics::calculate_mae(&a, &b, 8);
        assert_eq!(mae, 1.0); // (0 + 2 + 2 + 0) / 4 = 1.0
    }

    #[test]
    fn test_metrics_psnr() {
        let psnr_perfect = metrics::calculate_psnr(0.0, 8);
        assert!(psnr_perfect.is_infinite());

        let psnr_typical = metrics::calculate_psnr(1.0, 8);
        assert!(psnr_typical > 40.0); // Should be reasonable PSNR
    }
}

// Integration test to validate the framework itself
#[test]
fn test_interop_framework_integration() {
    use std::env;
    
    println!("\n=== Testing Interoperability Framework ===\n");
    
    // Test binary locator
    let locator = BinaryLocator::new();
    println!("Checking for reference binaries:");
    println!("  OpenJPEG:  {}", if locator.check_codec_binaries("jpeg2000") { "✅" } else { "❌" });
    println!("  CharLS:    {}", if locator.check_codec_binaries("jpegls") { "✅" } else { "❌" });
    println!("  libjpeg:   {}", if locator.check_codec_binaries("jpeg1") { "✅" } else { "❌" });
    println!("  OpenHTJ2K: {}", if locator.check_codec_binaries("htj2k") { "✅" } else { "❌" });
    
    // Test PNM roundtrip
    let temp_dir = env::temp_dir();
    let test_path = temp_dir.join("interop_test.pgm");
    
    let pixels = vec![0u8, 50, 100, 150, 200, 250];
    pnm::write(&test_path, &pixels, 3, 2, 1, 8).unwrap();
    let read_back = pnm::read(&test_path, 6, 8, 1).unwrap();
    
    assert_eq!(pixels, read_back, "PNM roundtrip failed");
    fs::remove_file(test_path).ok();
    
    // Test report generator
    let mut reporter = TestReportGenerator::new();
    
    reporter.add_result(InteropTestResult {
        test_name: "test_example_pass".to_string(),
        codec: "JPEG2000".to_string(),
        direction: Direction::RustRoundtrip,
        width: 256,
        height: 256,
        components: 1,
        bit_depth: 8,
        encode_time: Duration::from_millis(10),
        decode_time: Duration::from_millis(5),
        compressed_size: 1024,
        mae: 0.0,
        psnr: f64::INFINITY,
        status: TestStatus::Pass,
        error_message: None,
    });
    
    reporter.add_result(InteropTestResult {
        test_name: "test_example_fail".to_string(),
        codec: "JPEG2000".to_string(),
        direction: Direction::RustToReference,
        width: 256,
        height: 256,
        components: 1,
        bit_depth: 16,
        encode_time: Duration::from_millis(15),
        decode_time: Duration::from_millis(8),
        compressed_size: 2048,
        mae: 19491.0,
        psnr: 10.5,
        status: TestStatus::Fail,
        error_message: Some("Endianness mismatch".to_string()),
    });
    
    reporter.print_summary();
    
    // Save CSV report
    let csv_path = temp_dir.join("interop_test_report.csv");
    reporter.save_csv(&csv_path).unwrap();
    println!("\nTest report saved to: {}", csv_path.display());
    
    println!("\n✅ Interoperability framework validated successfully\n");
}

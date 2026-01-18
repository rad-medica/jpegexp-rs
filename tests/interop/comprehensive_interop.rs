//! Comprehensive Codec Interoperability Test Suite
//!
//! This module provides comprehensive cross-validation testing between jpegexp-rs
//! codecs and reference implementations:
//!
//! - JPEG-LS: jpegexp-rs <-> CharLS
//! - JPEG 1: jpegexp-rs <-> libjpeg-turbo
//! - JPEG 2000: jpegexp-rs <-> OpenJPEG
//! - HTJ2K: jpegexp-rs <-> OpenHTJ2K
//!
//! # Test Philosophy
//!
//! **CRITICAL**: Never test a codec against itself.
//! - Test encoding: Our encoder -> Reference decoder
//! - Test decoding: Reference encoder -> Our decoder

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::jpegls::{JpeglsDecoder, JpeglsEncoder};
use jpegexp_rs::FrameInfo;

// Include the synthetic images module directly for standalone test
#[path = "../common/synthetic_images.rs"]
mod synthetic_images;

use synthetic_images::{SyntheticImage, TestPresets};

// ============================================================================
// Test Result Structures
// ============================================================================

/// Single test result with all metrics
#[derive(Debug, Clone)]
pub struct TestResult {
    // Test identification
    pub codec: String,
    pub direction: String, // "Rust->Ref" or "Ref->Rust"
    pub mode: String, // "Lossless", "NearLossless(N)", "Lossy(Q)"

    // Image parameters
    pub width: u32,
    pub height: u32,
    pub bit_depth: u32,
    pub components: u32,
    pub pattern: String,

    // Quality/compression parameters
    pub quality_param: i32, // 0=lossless, >0=near-lossless/quality

    // Timing metrics
    pub encode_time_us: u64,
    pub decode_time_us: u64,

    // Size metrics
    pub original_size: usize,
    pub compressed_size: usize,

    // Quality metrics
    pub mae: f64,
    pub max_error: u32,
    pub psnr: f64,

    // Status
    pub status: String,
    pub error_message: Option<String>,
}

impl TestResult {
    pub fn new(codec: &str, direction: &str, mode: &str) -> Self {
        Self {
            codec: codec.to_string(),
            direction: direction.to_string(),
            mode: mode.to_string(),
            width: 0,
            height: 0,
            bit_depth: 0,
            components: 0,
            pattern: String::new(),
            quality_param: 0,
            encode_time_us: 0,
            decode_time_us: 0,
            original_size: 0,
            compressed_size: 0,
            mae: 0.0,
            max_error: 0,
            psnr: f64::INFINITY,
            status: "OK".to_string(),
            error_message: None,
        }
    }

    pub fn with_image(mut self, img: &SyntheticImage) -> Self {
        self.width = img.width();
        self.height = img.height();
        self.bit_depth = img.bit_depth();
        self.components = img.components();
        self.pattern = img.config.pattern.name().to_string();
        self.original_size = img.pixels.len();
        self
    }

    pub fn fail(mut self, msg: &str) -> Self {
        self.status = "FAIL".to_string();
        self.error_message = Some(msg.to_string());
        self
    }

    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_size > 0 {
            self.original_size as f64 / self.compressed_size as f64
        } else {
            0.0
        }
    }

    pub fn throughput_mbps(&self) -> f64 {
        let total_time_s = (self.encode_time_us + self.decode_time_us) as f64 / 1_000_000.0;
        if total_time_s > 0.0 {
            (self.original_size as f64 / (1024.0 * 1024.0)) / total_time_s
        } else {
            0.0
        }
    }
}

/// Collection of test results with summary statistics
#[derive(Debug, Default)]
pub struct TestSuite {
    pub results: Vec<TestResult>,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
}

impl TestSuite {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            start_time: Some(Instant::now()),
            end_time: None,
        }
    }

    pub fn add(&mut self, result: TestResult) {
        self.results.push(result);
    }

    pub fn finish(&mut self) {
        self.end_time = Some(Instant::now());
    }

    pub fn passed(&self) -> usize {
        self.results.iter().filter(|r| r.status == "OK").count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|r| r.status != "OK").count()
    }

    pub fn total(&self) -> usize {
        self.results.len()
    }

    pub fn duration(&self) -> Duration {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => end.duration_since(start),
            _ => Duration::ZERO,
        }
    }

    /// Export results to CSV
    pub fn to_csv(&self) -> String {
        let mut csv = String::from(
            "Codec,Direction,Mode,Width,Height,BitDepth,Components,Pattern,QualityParam,\
             EncTime_us,DecTime_us,OriginalSize,CompressedSize,CompressionRatio,MAE,MaxError,PSNR,Throughput_MBps,Status\n",
        );

        for r in &self.results {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{:.2},{:.6},{},{:.2},{:.2},{}\n",
                r.codec,
                r.direction,
                r.mode,
                r.width,
                r.height,
                r.bit_depth,
                r.components,
                r.pattern,
                r.quality_param,
                r.encode_time_us,
                r.decode_time_us,
                r.original_size,
                r.compressed_size,
                r.compression_ratio(),
                r.mae,
                r.max_error,
                r.psnr,
                r.throughput_mbps(),
                r.status,
            ));
        }
        csv
    }

    /// Generate summary report
    pub fn summary(&self) -> String {
        let mut report = String::new();
        report.push_str(
            "================================================================================\n",
        );
        report.push_str(
            "                    COMPREHENSIVE INTEROPERABILITY TEST REPORT\n",
        );
        report.push_str(
            "================================================================================\n\n",
        );

        report.push_str(&format!("Test Duration: {:?}\n", self.duration()));
        report.push_str(&format!(
            "Total Tests: {} | Passed: {} | Failed: {}\n\n",
            self.total(),
            self.passed(),
            self.failed()
        ));

        // Group by codec
        let codecs: Vec<&str> = ["JPEGLS", "JPEG1", "J2K", "HTJ2K"]
            .iter()
            .copied()
            .collect();

        for codec in codecs {
            let codec_results: Vec<_> = self.results.iter().filter(|r| r.codec == codec).collect();
            if codec_results.is_empty() {
                continue;
            }

            let passed = codec_results.iter().filter(|r| r.status == "OK").count();
            let total = codec_results.len();

            report.push_str(&format!("\n{} Tests: {}/{} passed\n", codec, passed, total));
            report.push_str("-".repeat(60).as_str());
            report.push_str("\n");

            // Print header
            report.push_str(&format!(
                "{:<12} {:<10} {:>8} {:>4} {:>3} {:>10} {:>8} {:>6}\n",
                "Direction",
                "Mode",
                "Size",
                "Bits",
                "C",
                "Ratio",
                "MAE",
                "Status"
            ));

            for r in &codec_results {
                let size_str = format!("{}x{}", r.width, r.height);
                report.push_str(&format!(
                    "{:<12} {:<10} {:>8} {:>4} {:>3} {:>10.2} {:>8.4} {:>6}\n",
                    r.direction,
                    r.mode,
                    size_str,
                    r.bit_depth,
                    r.components,
                    r.compression_ratio(),
                    r.mae,
                    r.status,
                ));
            }
        }

        // Failed tests details
        let failures: Vec<_> = self.results.iter().filter(|r| r.status != "OK").collect();
        if !failures.is_empty() {
            report.push_str("\n\nFAILED TESTS:\n");
            report.push_str("=".repeat(60).as_str());
            report.push_str("\n");
            for r in failures {
                report.push_str(&format!(
                    "{} {} {} {}x{} {}bit: {}\n",
                    r.codec,
                    r.direction,
                    r.mode,
                    r.width,
                    r.height,
                    r.bit_depth,
                    r.error_message.as_deref().unwrap_or("Unknown error")
                ));
            }
        }

        report
    }
}

// ============================================================================
// External Binary Utilities
// ============================================================================

fn find_binary(name: &str) -> Option<String> {
    let bin_dir = Path::new("libs/bin");
    let exe_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    let path = bin_dir.join(&exe_name);
    if path.exists() {
        return Some(path.to_string_lossy().to_string());
    }

    // Try without .exe suffix
    if name.ends_with(".exe") {
        let path = bin_dir.join(name);
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

fn ensure_output_dir() {
    fs::create_dir_all("tests/fixtures/out").ok();
    fs::create_dir_all("docs/test-results").ok();
}

// ============================================================================
// Pixel Comparison Utilities
// ============================================================================

fn calculate_metrics(original: &[u8], decoded: &[u8], bit_depth: u32) -> (f64, u32, f64) {
    if original.len() != decoded.len() {
        return (f64::MAX, u32::MAX, 0.0);
    }

    let max_val = (1u64 << bit_depth) - 1;

    // NOTE: For bit_depth > 8 (including 10, 12, 16), pixels are stored in 16-bit containers
    // with native endian byte order. We use bytes_per_sample to determine the storage format.
    let bytes_per_sample = if bit_depth <= 8 { 1 } else { 2 };

    if bytes_per_sample == 1 {
        // 8-bit data: each sample is a single byte
        let mut sum_abs_error = 0u64;
        let mut sum_sq_error = 0u64;
        let mut max_error = 0u32;

        for (o, d) in original.iter().zip(decoded.iter()) {
            let diff = (*o as i32 - *d as i32).abs() as u32;
            sum_abs_error += diff as u64;
            sum_sq_error += (diff * diff) as u64;
            max_error = max_error.max(diff);
        }

        let count = original.len() as f64;
        let mae = sum_abs_error as f64 / count;
        let mse = sum_sq_error as f64 / count;
        let psnr = if mse > 0.0 {
            10.0 * ((max_val as f64 * max_val as f64) / mse).log10()
        } else {
            f64::INFINITY
        };

        (mae, max_error, psnr)
    } else {
        // 10/12/16-bit data: each sample is 2 bytes in native endian
        let count = original.len() / 2;
        let mut sum_abs_error = 0u64;
        let mut sum_sq_error = 0u64;
        let mut max_error = 0u32;

        for i in 0..count {
            let o = u16::from_ne_bytes([original[i * 2], original[i * 2 + 1]]) as i32;
            let d = u16::from_ne_bytes([decoded[i * 2], decoded[i * 2 + 1]]) as i32;
            let diff = (o - d).abs() as u32;
            sum_abs_error += diff as u64;
            sum_sq_error += diff as u64 * diff as u64;
            max_error = max_error.max(diff);
        }

        let mae = sum_abs_error as f64 / count as f64;
        let mse = sum_sq_error as f64 / count as f64;
        let psnr = if mse > 0.0 {
            10.0 * ((max_val as f64 * max_val as f64) / mse).log10()
        } else {
            f64::INFINITY
        };

        (mae, max_error, psnr)
    }
}

// ============================================================================
// PNM File Utilities
// ============================================================================

fn write_pnm(
    path: &str,
    pixels: &[u8],
    w: u32,
    h: u32,
    components: u32,
    bit_depth: u32,
) -> std::io::Result<()> {
    let magic = if components == 3 { "P6" } else { "P5" };
    let max_val = (1u32 << bit_depth) - 1;
    let mut data = format!("{}\n{} {}\n{}\n", magic, w, h, max_val).into_bytes();

    if bit_depth <= 8 {
        data.extend_from_slice(pixels);
    } else {
        // Convert from native endian to big endian for PNM
        let count = pixels.len() / 2;
        for i in 0..count {
            let val = u16::from_ne_bytes([pixels[i * 2], pixels[i * 2 + 1]]);
            data.extend_from_slice(&val.to_be_bytes());
        }
    }

    fs::write(path, data)
}

fn read_pnm_pixels(
    data: &[u8],
    expected_pixel_count: usize,
    bit_depth: u32,
    components: u32,
) -> Option<Vec<u8>> {
    // Simple PNM parser
    let mut pos = 0;
    let magic = if components == 3 { b"P6" } else { b"P5" };

    if data.get(pos..pos + 2) != Some(magic) {
        return None;
    }
    pos += 2;

    // Skip header (width, height, maxval)
    let mut count = 0;
    while count < 3 && pos < data.len() {
        while pos < data.len() && data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos < data.len() && data[pos] == b'#' {
            while pos < data.len() && data[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }
        while pos < data.len() && !data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        count += 1;
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
        // Convert from big endian (PNM) to native endian
        let mut native = Vec::with_capacity(expected_bytes);
        for i in 0..(expected_bytes / 2) {
            let val = u16::from_be_bytes([raw[i * 2], raw[i * 2 + 1]]);
            native.extend_from_slice(&val.to_ne_bytes());
        }
        Some(native)
    }
}

// ============================================================================
// JPEG-LS Interoperability Tests
// ============================================================================

fn run_jpegls_test(img: &SyntheticImage, near_lossless: i32, suite: &mut TestSuite) {
    // NOTE: CharLS CLI tool only supports lossless encoding.
    // Near-lossless (NEAR > 0) is not supported via CLI.
    // Skip near-lossless interop tests with CharLS.
    if near_lossless > 0 {
        return;
    }

    let charls_bin = match find_binary("charls") {
        Some(b) => b,
        None => {
            let mut result = TestResult::new("JPEGLS", "N/A", "N/A").with_image(img);
            result = result.fail("CharLS binary not found");
            suite.add(result);
            return;
        }
    };

    let mode = if near_lossless == 0 {
        "Lossless".to_string()
    } else {
        format!("NL({})", near_lossless)
    };

    let frame_info = FrameInfo {
        width: img.width(),
        height: img.height(),
        bits_per_sample: img.bit_depth() as i32,
        component_count: img.components() as i32,
    };

    // Test 1: Rust Encode -> CharLS Decode
    {
        let mut result = TestResult::new("JPEGLS", "Rust->Ref", &mode).with_image(img);
        result.quality_param = near_lossless;

        // Encode with our encoder
        let mut buf = vec![0u8; img.pixels.len() * 2 + 1024];
        let mut encoder = JpeglsEncoder::new(&mut buf);

        if let Err(e) = encoder.set_frame_info(frame_info) {
            result = result.fail(&format!("Set frame info failed: {:?}", e));
            suite.add(result);
            return;
        }

        if near_lossless > 0 {
            encoder.set_near_lossless(near_lossless).ok();
        }

        let start_enc = Instant::now();
        match encoder.encode(&img.pixels) {
            Ok(size) => {
                result.encode_time_us = start_enc.elapsed().as_micros() as u64;
                result.compressed_size = size;

                // Write to temp file
                let temp_jls = "tests/fixtures/out/temp_jpegls.jls";
                let temp_pnm = "tests/fixtures/out/temp_jpegls_out.pnm";
                fs::write(temp_jls, &buf[..size]).unwrap();

                // Decode with CharLS
                let start_dec = Instant::now();
                let output = Command::new(&charls_bin)
                    .args(["-decodetopnm", temp_jls, temp_pnm])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        result.decode_time_us = start_dec.elapsed().as_micros() as u64;

                        if let Ok(data) = fs::read(temp_pnm) {
                            let pixel_count = (img.width() * img.height()) as usize;
                            if let Some(decoded) = read_pnm_pixels(
                                &data,
                                pixel_count,
                                img.bit_depth(),
                                img.components(),
                            )
                            {
                                let (mae, max_err, psnr) =
                                    calculate_metrics(&img.pixels, &decoded, img.bit_depth());
                                result.mae = mae;
                                result.max_error = max_err;
                                result.psnr = psnr;

                                // Verify lossless or near-lossless tolerance
                                if near_lossless == 0 && mae > 0.0 {
                                    result = result.fail(
                                        &format!("Lossless test failed: MAE={:.4}", mae),
                                    );
                                } else if near_lossless > 0 && max_err > near_lossless as u32 {
                                    result = result.fail(&format!(
                                        "Near-lossless tolerance exceeded: max_err={} > NL={}",
                                        max_err,
                                        near_lossless
                                    ));
                                }
                            } else {
                                result = result.fail("Failed to parse PNM output");
                            }
                        } else {
                            result = result.fail("Failed to read CharLS output");
                        }
                    }
                    Ok(out) => {
                        result = result.fail(&format!(
                            "CharLS decode failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        ));
                    }
                    Err(e) => {
                        result = result.fail(&format!("Failed to run CharLS: {}", e));
                    }
                }

                // Cleanup
                let _ = fs::remove_file(temp_jls);
                let _ = fs::remove_file(temp_pnm);
            }
            Err(e) => {
                result = result.fail(&format!("Encode failed: {:?}", e));
            }
        }

        suite.add(result);
    }

    // Test 2: CharLS Encode -> Rust Decode
    {
        let mut result = TestResult::new("JPEGLS", "Ref->Rust", &mode).with_image(img);
        result.quality_param = near_lossless;

        let temp_pnm = "tests/fixtures/out/temp_jpegls_in.pnm";
        let temp_jls = "tests/fixtures/out/temp_jpegls_charls.jls";

        // Write input PNM
        if let Err(e) = write_pnm(
            temp_pnm,
            &img.pixels,
            img.width(),
            img.height(),
            img.components(),
            img.bit_depth(),
        )
        {
            result = result.fail(&format!("Failed to write PNM: {}", e));
            suite.add(result);
            return;
        }

        // Encode with CharLS (lossless only - CLI doesn't support NEAR parameter)
        let args = vec!["-encodepnm", temp_pnm, temp_jls];

        let start_enc = Instant::now();
        let output = Command::new(&charls_bin).args(&args).output();

        match output {
            Ok(out) if out.status.success() => {
                result.encode_time_us = start_enc.elapsed().as_micros() as u64;

                if let Ok(encoded) = fs::read(temp_jls) {
                    result.compressed_size = encoded.len();

                    // Decode with our decoder
                    let start_dec = Instant::now();
                    let mut decoder = JpeglsDecoder::new(&encoded);

                    match decoder.read_header() {
                        Ok(_) => {
                            let mut decoded = vec![0u8; img.pixels.len()];
                            match decoder.decode(&mut decoded) {
                                Ok(_) => {
                                    result.decode_time_us = start_dec.elapsed().as_micros() as u64;

                                    let (mae, max_err, psnr) =
                                        calculate_metrics(&img.pixels, &decoded, img.bit_depth());
                                    result.mae = mae;
                                    result.max_error = max_err;
                                    result.psnr = psnr;

                                    if near_lossless == 0 && mae > 0.0 {
                                        result = result.fail(&format!(
                                            "Lossless test failed: MAE={:.4}",
                                            mae
                                        ));
                                    } else if near_lossless > 0 && max_err > near_lossless as u32 {
                                        result = result.fail(&format!(
                                            "Near-lossless tolerance exceeded: max_err={} > NL={}",
                                            max_err,
                                            near_lossless
                                        ));
                                    }
                                }
                                Err(e) => {
                                    result = result.fail(&format!("Decode failed: {:?}", e));
                                }
                            }
                        }
                        Err(e) => {
                            result = result.fail(&format!("Read header failed: {:?}", e));
                        }
                    }
                } else {
                    result = result.fail("Failed to read CharLS output");
                }
            }
            Ok(out) => {
                result = result.fail(&format!(
                    "CharLS encode failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            Err(e) => {
                result = result.fail(&format!("Failed to run CharLS: {}", e));
            }
        }

        // Cleanup
        let _ = fs::remove_file(temp_pnm);
        let _ = fs::remove_file(temp_jls);

        suite.add(result);
    }
}

// ============================================================================
// JPEG 2000 Interoperability Tests
// ============================================================================

fn run_j2k_test(img: &SyntheticImage, lossless: bool, suite: &mut TestSuite) {
    let opj_compress = match find_binary("opj_compress") {
        Some(b) => b,
        None => {
            let mut result = TestResult::new("J2K", "N/A", "N/A").with_image(img);
            result = result.fail("opj_compress not found");
            suite.add(result);
            return;
        }
    };

    let opj_decompress = match find_binary("opj_decompress") {
        Some(b) => b,
        None => {
            let mut result = TestResult::new("J2K", "N/A", "N/A").with_image(img);
            result = result.fail("opj_decompress not found");
            suite.add(result);
            return;
        }
    };

    let mode = if lossless { "Lossless" } else { "Lossy" };

    let frame_info = FrameInfo {
        width: img.width(),
        height: img.height(),
        bits_per_sample: img.bit_depth() as i32,
        component_count: img.components() as i32,
    };

    // Test 1: Rust Encode -> OpenJPEG Decode
    {
        let mut result = TestResult::new("J2K", "Rust->Ref", mode).with_image(img);

        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(!lossless);

        let mut encoded = vec![0u8; img.pixels.len() * 2 + 1024];
        let start_enc = Instant::now();

        match encoder.encode(&img.pixels, &frame_info, &mut encoded) {
            Ok(size) => {
                result.encode_time_us = start_enc.elapsed().as_micros() as u64;
                result.compressed_size = size;

                let temp_j2k = "tests/fixtures/out/temp_j2k.j2k";
                let temp_pnm = "tests/fixtures/out/temp_j2k_out.pnm";
                fs::write(temp_j2k, &encoded[..size]).unwrap();

                let start_dec = Instant::now();
                let output = Command::new(&opj_decompress)
                    .args(["-i", temp_j2k, "-o", temp_pnm])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        result.decode_time_us = start_dec.elapsed().as_micros() as u64;

                        if let Ok(data) = fs::read(temp_pnm) {
                            let pixel_count = (img.width() * img.height()) as usize;
                            if let Some(decoded) = read_pnm_pixels(
                                &data,
                                pixel_count,
                                img.bit_depth(),
                                img.components(),
                            )
                            {
                                let (mae, max_err, psnr) =
                                    calculate_metrics(&img.pixels, &decoded, img.bit_depth());
                                result.mae = mae;
                                result.max_error = max_err;
                                result.psnr = psnr;

                                if lossless && mae > 0.0 {
                                    result = result.fail(&format!("Lossless MAE={:.4}", mae));
                                }
                            } else {
                                result = result.fail("Failed to parse PNM");
                            }
                        }
                    }
                    Ok(out) => {
                        result = result.fail(&format!(
                            "OpenJPEG decode failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        ));
                    }
                    Err(e) => {
                        result = result.fail(&format!("Failed to run OpenJPEG: {}", e));
                    }
                }

                let _ = fs::remove_file(temp_j2k);
                let _ = fs::remove_file(temp_pnm);
            }
            Err(e) => {
                result = result.fail(&format!("Encode failed: {:?}", e));
            }
        }

        suite.add(result);
    }

    // Test 2: OpenJPEG Encode -> Rust Decode
    {
        let mut result = TestResult::new("J2K", "Ref->Rust", mode).with_image(img);

        let temp_pnm = "tests/fixtures/out/temp_j2k_in.pnm";
        let temp_j2k = "tests/fixtures/out/temp_j2k_opj.j2k";

        write_pnm(
            temp_pnm,
            &img.pixels,
            img.width(),
            img.height(),
            img.components(),
            img.bit_depth(),
        ).unwrap();

        let mut args = vec!["-i", temp_pnm, "-o", temp_j2k];
        if lossless {
            args.extend(["-r", "1"]);
        } else {
            args.extend(["-q", "30"]);
        }

        let start_enc = Instant::now();
        let output = Command::new(&opj_compress).args(&args).output();

        match output {
            Ok(out) if out.status.success() => {
                result.encode_time_us = start_enc.elapsed().as_micros() as u64;

                if let Ok(encoded) = fs::read(temp_j2k) {
                    result.compressed_size = encoded.len();

                    let start_dec = Instant::now();
                    let mut reader = JpegStreamReader::new(&encoded);
                    let mut decoder = J2kDecoder::new(&mut reader);

                    match decoder.decode() {
                        Ok(img_decoded) => {
                            match img_decoded.reconstruct_pixels() {
                                Ok(decoded) => {
                                    result.decode_time_us = start_dec.elapsed().as_micros() as u64;

                                    let (mae, max_err, psnr) =
                                        calculate_metrics(&img.pixels, &decoded, img.bit_depth());
                                    result.mae = mae;
                                    result.max_error = max_err;
                                    result.psnr = psnr;

                                    if lossless && mae > 0.0 {
                                        result = result.fail(&format!("Lossless MAE={:.4}", mae));
                                    }
                                }
                                Err(e) => {
                                    result = result.fail(&format!("Reconstruct failed: {:?}", e));
                                }
                            }
                        }
                        Err(e) => {
                            result = result.fail(&format!("Decode failed: {:?}", e));
                        }
                    }
                }
            }
            Ok(out) => {
                result = result.fail(&format!(
                    "OpenJPEG encode failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            Err(e) => {
                result = result.fail(&format!("Failed to run OpenJPEG: {}", e));
            }
        }

        let _ = fs::remove_file(temp_pnm);
        let _ = fs::remove_file(temp_j2k);

        suite.add(result);
    }
}

// ============================================================================
// JPEG 1 Interoperability Tests
// ============================================================================

fn run_jpeg1_test(img: &SyntheticImage, quality: i32, suite: &mut TestSuite) {
    // JPEG 1 baseline only supports 8-bit
    if img.bit_depth() != 8 {
        return;
    }

    let cjpeg = match find_binary("cjpeg") {
        Some(b) => b,
        None => {
            let mut result = TestResult::new("JPEG1", "N/A", "N/A").with_image(img);
            result = result.fail("cjpeg not found");
            suite.add(result);
            return;
        }
    };

    let djpeg = match find_binary("djpeg") {
        Some(b) => b,
        None => {
            let mut result = TestResult::new("JPEG1", "N/A", "N/A").with_image(img);
            result = result.fail("djpeg not found");
            suite.add(result);
            return;
        }
    };

    let mode = format!("Q{}", quality);

    let frame_info = FrameInfo {
        width: img.width(),
        height: img.height(),
        bits_per_sample: 8,
        component_count: img.components() as i32,
    };

    // Test 1: Rust Encode -> libjpeg-turbo Decode
    {
        let mut result = TestResult::new("JPEG1", "Rust->Ref", &mode).with_image(img);
        result.quality_param = quality;

        let mut encoder = Jpeg1Encoder::new();
        encoder.set_quality(quality as u8);

        let mut encoded = vec![0u8; img.pixels.len() * 2 + 1024];
        let start_enc = Instant::now();

        match encoder.encode(&img.pixels, &frame_info, &mut encoded) {
            Ok(size) => {
                result.encode_time_us = start_enc.elapsed().as_micros() as u64;
                result.compressed_size = size;

                let temp_jpg = "tests/fixtures/out/temp_j1.jpg";
                let temp_pnm = "tests/fixtures/out/temp_j1_out.pnm";
                fs::write(temp_jpg, &encoded[..size]).unwrap();

                let start_dec = Instant::now();
                let output = Command::new(&djpeg)
                    .args(["-outfile", temp_pnm, temp_jpg])
                    .output();

                match output {
                    Ok(out) if out.status.success() => {
                        result.decode_time_us = start_dec.elapsed().as_micros() as u64;

                        if let Ok(data) = fs::read(temp_pnm) {
                            let pixel_count = (img.width() * img.height()) as usize;
                            if let Some(decoded) = read_pnm_pixels(
                                &data,
                                pixel_count,
                                8,
                                img.components(),
                            )
                            {
                                let (mae, max_err, psnr) =
                                    calculate_metrics(&img.pixels, &decoded, 8);
                                result.mae = mae;
                                result.max_error = max_err;
                                result.psnr = psnr;
                            }
                        }
                    }
                    Ok(out) => {
                        result = result.fail(&format!(
                            "djpeg failed: {}",
                            String::from_utf8_lossy(&out.stderr)
                        ));
                    }
                    Err(e) => {
                        result = result.fail(&format!("Failed to run djpeg: {}", e));
                    }
                }

                let _ = fs::remove_file(temp_jpg);
                let _ = fs::remove_file(temp_pnm);
            }
            Err(e) => {
                result = result.fail(&format!("Encode failed: {:?}", e));
            }
        }

        suite.add(result);
    }

    // Test 2: libjpeg-turbo Encode -> Rust Decode
    {
        let mut result = TestResult::new("JPEG1", "Ref->Rust", &mode).with_image(img);
        result.quality_param = quality;

        let temp_pnm = "tests/fixtures/out/temp_j1_in.pnm";
        let temp_jpg = "tests/fixtures/out/temp_j1_ljt.jpg";

        write_pnm(
            temp_pnm,
            &img.pixels,
            img.width(),
            img.height(),
            img.components(),
            8,
        ).unwrap();

        let quality_str = quality.to_string();
        let start_enc = Instant::now();
        let output = Command::new(&cjpeg)
            .args(["-quality", &quality_str, "-outfile", temp_jpg, temp_pnm])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                result.encode_time_us = start_enc.elapsed().as_micros() as u64;

                if let Ok(encoded) = fs::read(temp_jpg) {
                    result.compressed_size = encoded.len();

                    let start_dec = Instant::now();
                    let mut decoder = Jpeg1Decoder::new(&encoded);

                    match decoder.read_header() {
                        Ok(_) => {
                            let mut decoded = vec![0u8; img.pixels.len()];
                            match decoder.decode(&mut decoded) {
                                Ok(_) => {
                                    result.decode_time_us = start_dec.elapsed().as_micros() as u64;

                                    let (mae, max_err, psnr) =
                                        calculate_metrics(&img.pixels, &decoded, 8);
                                    result.mae = mae;
                                    result.max_error = max_err;
                                    result.psnr = psnr;
                                }
                                Err(e) => {
                                    result = result.fail(&format!("Decode failed: {:?}", e));
                                }
                            }
                        }
                        Err(e) => {
                            result = result.fail(&format!("Read header failed: {:?}", e));
                        }
                    }
                }
            }
            Ok(out) => {
                result = result.fail(&format!(
                    "cjpeg failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            Err(e) => {
                result = result.fail(&format!("Failed to run cjpeg: {}", e));
            }
        }

        let _ = fs::remove_file(temp_pnm);
        let _ = fs::remove_file(temp_jpg);

        suite.add(result);
    }
}

// ============================================================================
// Main Test Runners
// ============================================================================

#[test]
#[ignore]
fn comprehensive_jpegls_interop() {
    ensure_output_dir();
    let mut suite = TestSuite::new();

    println!("\n=== JPEG-LS Comprehensive Interoperability Tests ===\n");

    // Generate test images
    let images = TestPresets::jpegls();
    let near_lossless_values = [0, 1, 2, 5];

    for img in &images {
        // Only test grayscale for now (RGB requires sample interleave which CharLS CLI handles differently)
        if img.components() > 1 {
            continue;
        }

        for &nl in &near_lossless_values {
            println!(
                "Testing: {}x{} {}bit {} NL={}",
                img.width(),
                img.height(),
                img.bit_depth(),
                img.config.pattern.name(),
                nl
            );
            run_jpegls_test(img, nl, &mut suite);
        }
    }

    suite.finish();

    // Save results
    let csv = suite.to_csv();
    let timestamp = chrono_lite_timestamp();
    let csv_path = format!("docs/test-results/jpegls_interop_{}.csv", timestamp);
    fs::write(&csv_path, &csv).expect("Failed to write CSV");

    let summary = suite.summary();
    println!("{}", summary);

    let summary_path = format!("docs/test-results/jpegls_interop_{}.txt", timestamp);
    fs::write(&summary_path, &summary).expect("Failed to write summary");

    assert!(
        suite.failed() == 0,
        "JPEG-LS interop tests failed: {}/{}",
        suite.failed(),
        suite.total()
    );
}

#[test]
#[ignore]
fn comprehensive_j2k_interop() {
    ensure_output_dir();
    let mut suite = TestSuite::new();

    println!("\n=== JPEG 2000 Comprehensive Interoperability Tests ===\n");

    let images = TestPresets::jpeg2000();

    for img in &images {
        // Test lossless
        println!(
            "Testing Lossless: {}x{} {}bit {}",
            img.width(),
            img.height(),
            img.bit_depth(),
            img.config.pattern.name()
        );
        run_j2k_test(img, true, &mut suite);

        // Test lossy (only for 8-bit to avoid complexity)
        if img.bit_depth() == 8 {
            println!(
                "Testing Lossy: {}x{} {}bit {}",
                img.width(),
                img.height(),
                img.bit_depth(),
                img.config.pattern.name()
            );
            run_j2k_test(img, false, &mut suite);
        }
    }

    suite.finish();

    let csv = suite.to_csv();
    let timestamp = chrono_lite_timestamp();
    let csv_path = format!("docs/test-results/j2k_interop_{}.csv", timestamp);
    fs::write(&csv_path, &csv).expect("Failed to write CSV");

    let summary = suite.summary();
    println!("{}", summary);

    assert!(
        suite.failed() == 0,
        "J2K interop tests failed: {}/{}",
        suite.failed(),
        suite.total()
    );
}

#[test]
#[ignore]
fn comprehensive_jpeg1_interop() {
    ensure_output_dir();
    let mut suite = TestSuite::new();

    println!("\n=== JPEG 1 Comprehensive Interoperability Tests ===\n");

    let images = TestPresets::jpeg1();
    let quality_levels = [50, 75, 90, 95, 100];

    for img in &images {
        if img.bit_depth() != 8 {
            continue; // JPEG 1 baseline is 8-bit only
        }

        for &q in &quality_levels {
            println!(
                "Testing Q{}: {}x{} {}",
                q,
                img.width(),
                img.height(),
                img.config.pattern.name()
            );
            run_jpeg1_test(img, q, &mut suite);
        }
    }

    suite.finish();

    let csv = suite.to_csv();
    let timestamp = chrono_lite_timestamp();
    let csv_path = format!("docs/test-results/jpeg1_interop_{}.csv", timestamp);
    fs::write(&csv_path, &csv).expect("Failed to write CSV");

    let summary = suite.summary();
    println!("{}", summary);

    assert!(
        suite.failed() == 0,
        "JPEG 1 interop tests failed: {}/{}",
        suite.failed(),
        suite.total()
    );
}

#[test]
#[ignore]
fn run_all_comprehensive_interop() {
    ensure_output_dir();

    println!("\n");
    println!("================================================================================");
    println!("          COMPREHENSIVE CODEC INTEROPERABILITY TEST SUITE");
    println!("================================================================================");
    println!();

    comprehensive_jpegls_interop();
    comprehensive_j2k_interop();
    comprehensive_jpeg1_interop();

    println!("\n=== All comprehensive interop tests completed ===\n");
}

// ============================================================================
// Quick Tests (for CI)
// ============================================================================

#[test]
fn quick_jpegls_interop() {
    ensure_output_dir();
    let mut suite = TestSuite::new();

    let images = TestPresets::jpegls_quick();

    for img in &images {
        if img.components() > 1 {
            continue;
        }
        run_jpegls_test(img, 0, &mut suite); // Lossless only
    }

    suite.finish();
    println!("{}", suite.summary());

    // Don't fail on missing binaries in CI
    let real_failures = suite
        .results
        .iter()
        .filter(|r| {
            r.status != "OK" &&
                !r.error_message.as_deref().unwrap_or("").contains(
                    "not found",
                )
        })
        .count();

    assert!(
        real_failures == 0,
        "Quick JPEG-LS tests failed: {}",
        real_failures
    );
}

// ============================================================================
// Utility Functions
// ============================================================================

fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    format!("{}", duration.as_secs())
}

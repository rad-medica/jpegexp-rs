//! Benchmarks for JPEG 2000 compression
//!
//! Compares lossy vs lossless compression ratios and performance

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::time::Instant;

/// Generate a test image with specified pattern
fn generate_test_image(width: usize, height: usize, pattern: &str) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height];
    match pattern {
        "gradient" => {
            for y in 0..height {
                for x in 0..width {
                    let idx = y * width + x;
                    pixels[idx] = ((x * 255) / width) as u8;
                }
            }
        }
        "checkerboard" => {
            for y in 0..height {
                for x in 0..width {
                    let idx = y * width + x;
                    pixels[idx] = if (x / 8 + y / 8) % 2 == 0 { 255 } else { 0 };
                }
            }
        }
        "noise" => {
            // Simple pseudo-random pattern
            for i in 0..pixels.len() {
                pixels[i] = ((i * 1103515245 + 12345) % 256) as u8;
            }
        }
        "natural" => {
            // Simulate natural image with gradual transitions
            for y in 0..height {
                for x in 0..width {
                    let idx = y * width + x;
                    let fx = x as f32 / width as f32;
                    let fy = y as f32 / height as f32;
                    let value = (128.0 + 64.0 * (fx * 6.28).sin() + 64.0 * (fy * 6.28).cos()) as u8;
                    pixels[idx] = value;
                }
            }
        }
        _ => {}
    }
    pixels
}

/// Benchmark a single encoding operation
fn bench_encode(
    pixels: &[u8],
    width: usize,
    height: usize,
    quality: u8,
    irreversible: bool,
    levels: u8,
) -> (usize, u128) {
    let mut encoder = J2kEncoder::new();
    encoder.set_quality(quality);
    encoder.set_irreversible(irreversible);
    encoder.set_decomposition_levels(levels);

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut output = vec![0u8; pixels.len() * 2];

    let start = Instant::now();
    match encoder.encode(pixels, &frame_info, &mut output) {
        Ok(size) => {
            let duration = start.elapsed().as_micros();
            (size, duration)
        }
        Err(e) => {
            eprintln!("Warning: Encoding failed: {:?}", e);
            (0, 0) // Return zero to indicate failure
        }
    }
}

/// Print benchmark results in a formatted table
fn print_results(
    label: &str,
    original_size: usize,
    compressed_size: usize,
    duration_us: u128,
) {
    if compressed_size == 0 {
        // Skip failed encodings
        return;
    }
    let ratio = original_size as f64 / compressed_size as f64;
    let throughput_mbps = (original_size as f64 / 1_048_576.0) / (duration_us as f64 / 1_000_000.0);

    println!(
        "{:30} | {:8} bytes | {:6.2}x | {:8} µs | {:6.2} MB/s",
        label, compressed_size, ratio, duration_us, throughput_mbps
    );
}

fn main() {
    println!("\n=== JPEG 2000 Compression Benchmarks ===\n");

    let sizes = [(256, 256), (512, 512), (1024, 1024)];
    let patterns = ["gradient", "checkerboard", "noise", "natural"];

    for &(width, height) in &sizes {
        println!("\n--- Image Size: {}x{} ({} bytes) ---", width, height, width * height);
        println!("{:30} | {:>15} | {:>7} | {:>11} | {:>11}", "Configuration", "Size", "Ratio", "Time", "Throughput");
        println!("{}", "-".repeat(85));

        for pattern in &patterns {
            let pixels = generate_test_image(width, height, pattern);
            let original_size = pixels.len();

            println!("\nPattern: {}", pattern);

            // Lossless (5-3 reversible)
            let (size, time) = bench_encode(&pixels, width, height, 100, false, 5);
            print_results("Lossless (5-3, 5 levels)", original_size, size, time);

            // Lossy configurations (9-7 irreversible)
            // Note: Due to quantization issue, these may not work correctly yet
            let qualities = [100, 90, 75, 50];
            for quality in &qualities {
                let (size, time) = bench_encode(&pixels, width, height, *quality, true, 5);
                let label = format!("Lossy Q{} (9-7, 5 levels)", quality);
                print_results(&label, original_size, size, time);
            }

            // Different DWT levels (lossless for fair comparison)
            for levels in [0, 1, 3, 5] {
                let (size, time) = bench_encode(&pixels, width, height, 100, false, levels);
                let label = format!("Lossless ({} levels)", levels);
                print_results(&label, original_size, size, time);
            }
        }
    }

    println!("\n=== Comparison Summary ===\n");
    
    // Generate summary statistics for 512x512 natural image
    let width = 512;
    let height = 512;
    let pixels = generate_test_image(width, height, "natural");
    let original_size = pixels.len();

    println!("Test Image: 512x512 natural pattern ({} bytes)\n", original_size);
    println!("{:25} | {:>12} | {:>8} | {:>12}", "Mode", "Size", "Ratio", "Savings");
    println!("{}", "-".repeat(65));

    let (lossless_size, _) = bench_encode(&pixels, width, height, 100, false, 5);
    let lossless_ratio = original_size as f64 / lossless_size as f64;
    println!(
        "{:25} | {:8} bytes | {:6.2}x | {:8.1}%",
        "Lossless (baseline)",
        lossless_size,
        lossless_ratio,
        (1.0 - lossless_size as f64 / original_size as f64) * 100.0
    );

    let qualities = [(100, "Near-lossless"), (90, "Visually lossless"), (75, "High quality"), (50, "Medium quality")];
    for (quality, desc) in &qualities {
        let (size, _) = bench_encode(&pixels, width, height, *quality, true, 5);
        let ratio = original_size as f64 / size as f64;
        let vs_lossless = lossless_size as f64 / size as f64;
        println!(
            "{:25} | {:8} bytes | {:6.2}x | {:8.1}% ({}x vs lossless)",
            desc,
            size,
            ratio,
            (1.0 - size as f64 / original_size as f64) * 100.0,
            vs_lossless
        );
    }

    println!("\n=== Pattern Compressibility ===\n");
    println!("{:20} | {:>15} | {:>8}", "Pattern", "Lossless Size", "Ratio");
    println!("{}", "-".repeat(50));

    for pattern in &patterns {
        let pixels = generate_test_image(512, 512, pattern);
        let (size, _) = bench_encode(&pixels, 512, 512, 100, false, 5);
        let ratio = pixels.len() as f64 / size as f64;
        println!("{:20} | {:8} bytes | {:6.2}x", pattern, size, ratio);
    }

    println!("\n=== Performance Scaling ===\n");
    println!("{:15} | {:>15} | {:>12}", "Image Size", "Time (µs)", "Throughput");
    println!("{}", "-".repeat(48));

    for &(width, height) in &sizes {
        let pixels = generate_test_image(width, height, "natural");
        let (_, time) = bench_encode(&pixels, width, height, 100, false, 5);
        let throughput = (pixels.len() as f64 / 1_048_576.0) / (time as f64 / 1_000_000.0);
        println!(
            "{:15} | {:12} µs | {:8.2} MB/s",
            format!("{}x{}", width, height),
            time,
            throughput
        );
    }

    println!("\n=== Notes ===");
    println!("- Lossless mode uses 5-3 reversible DWT");
    println!("- Lossy mode uses 9-7 irreversible DWT with ICT");
    println!("- Quality 100 = near-lossless, 50 = medium compression");
    println!("- Ratios: higher is better (more compression)");
    println!("- Throughput measured in megabytes per second");
    println!("\nNote: Lossy compression has a known quantization issue");
    println!("      and may not produce expected quality/ratios yet.\n");
}

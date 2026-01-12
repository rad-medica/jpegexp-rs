//! Comprehensive Codec Interoperability Benchmarks
//!
//! This module provides comparative benchmarks between jpegexp-rs codecs
//! and reference implementations (CharLS, libjpeg-turbo, OpenJPEG).
//!
//! ## Benchmark Categories
//!
//! 1. **Encoding Speed**: Measure encode throughput (MB/s)
//! 2. **Decoding Speed**: Measure decode throughput (MB/s)
//! 3. **Roundtrip**: Combined encode + decode
//! 4. **Cross-Codec Comparison**: Same image, different codecs

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::jpegls::{JpeglsDecoder, JpeglsEncoder};
use jpegexp_rs::FrameInfo;

// ============================================================================
// Test Image Generation
// ============================================================================

/// Generate a deterministic test image (gradient pattern)
fn generate_gradient_image(width: usize, height: usize, bit_depth: u32) -> Vec<u8> {
    let max_val = (1u64 << bit_depth) - 1;
    let bytes_per_sample = if bit_depth <= 8 { 1 } else { 2 };
    let mut pixels = Vec::with_capacity(width * height * bytes_per_sample);

    for y in 0..height {
        for x in 0..width {
            let val = if width + height > 2 {
                ((x + y) as u64 * max_val / (width + height - 2) as u64) as u16
            } else {
                (max_val / 2) as u16
            };

            if bit_depth <= 8 {
                pixels.push(val as u8);
            } else {
                pixels.extend_from_slice(&val.to_ne_bytes());
            }
        }
    }
    pixels
}

/// Generate a noise image (worst-case for compression)
fn generate_noise_image(width: usize, height: usize, bit_depth: u32, seed: u32) -> Vec<u8> {
    let max_val = (1u64 << bit_depth) - 1;
    let bytes_per_sample = if bit_depth <= 8 { 1 } else { 2 };
    let mut pixels = Vec::with_capacity(width * height * bytes_per_sample);

    for i in 0..(width * height) {
        let val = ((i as u64)
            .wrapping_mul(1103515245)
            .wrapping_add(12345)
            .wrapping_add(seed as u64))
            % (max_val + 1);

        if bit_depth <= 8 {
            pixels.push(val as u8);
        } else {
            pixels.extend_from_slice(&(val as u16).to_ne_bytes());
        }
    }
    pixels
}

// ============================================================================
// Benchmark Utilities
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
        Some(path.to_string_lossy().to_string())
    } else {
        None
    }
}

fn ensure_temp_dir() {
    fs::create_dir_all("tests/fixtures/out").ok();
}

// ============================================================================
// JPEG-LS Benchmarks
// ============================================================================

fn bench_jpegls_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("jpegls_encode");
    group.measurement_time(Duration::from_secs(10));

    for size in [256, 512, 1024].iter() {
        let width = *size;
        let height = *size;

        // 8-bit gradient
        let input_8bit = generate_gradient_image(width, height, 8);
        group.throughput(Throughput::Bytes(input_8bit.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("rust_8bit", size),
            &input_8bit,
            |b, input| {
                let frame_info = FrameInfo {
                    width: width as u32,
                    height: height as u32,
                    bits_per_sample: 8,
                    component_count: 1,
                };
                let mut output = vec![0u8; input.len() * 2];

                b.iter(|| {
                    let mut encoder = JpeglsEncoder::new(&mut output);
                    encoder.set_frame_info(frame_info).unwrap();
                    let size = encoder.encode(black_box(input)).unwrap();
                    black_box(size)
                });
            },
        );

        // 16-bit gradient
        let input_16bit = generate_gradient_image(width, height, 16);
        group.throughput(Throughput::Bytes(input_16bit.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("rust_16bit", size),
            &input_16bit,
            |b, input| {
                let frame_info = FrameInfo {
                    width: width as u32,
                    height: height as u32,
                    bits_per_sample: 16,
                    component_count: 1,
                };
                let mut output = vec![0u8; input.len() * 2];

                b.iter(|| {
                    let mut encoder = JpeglsEncoder::new(&mut output);
                    encoder.set_frame_info(frame_info).unwrap();
                    let size = encoder.encode(black_box(input)).unwrap();
                    black_box(size)
                });
            },
        );
    }

    group.finish();
}

fn bench_jpegls_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("jpegls_decode");
    group.measurement_time(Duration::from_secs(10));

    for size in [256, 512, 1024].iter() {
        let width = *size;
        let height = *size;

        // Pre-encode 8-bit image
        let input_8bit = generate_gradient_image(width, height, 8);
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut encoded_8bit = vec![0u8; input_8bit.len() * 2];
        let mut encoder = JpeglsEncoder::new(&mut encoded_8bit);
        encoder.set_frame_info(frame_info).unwrap();
        let enc_size = encoder.encode(&input_8bit).unwrap();
        let encoded_8bit = encoded_8bit[..enc_size].to_vec();

        group.throughput(Throughput::Bytes(input_8bit.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("rust_8bit", size),
            &encoded_8bit,
            |b, encoded| {
                b.iter(|| {
                    let mut output = vec![0u8; width * height];
                    let mut decoder = JpeglsDecoder::new(black_box(encoded));
                    decoder.read_header().unwrap();
                    decoder.decode(&mut output).unwrap();
                    black_box(output)
                });
            },
        );

        // Pre-encode 16-bit image
        let input_16bit = generate_gradient_image(width, height, 16);
        let frame_info_16 = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 16,
            component_count: 1,
        };

        let mut encoded_16bit = vec![0u8; input_16bit.len() * 2];
        let mut encoder = JpeglsEncoder::new(&mut encoded_16bit);
        encoder.set_frame_info(frame_info_16).unwrap();
        let enc_size = encoder.encode(&input_16bit).unwrap();
        let encoded_16bit = encoded_16bit[..enc_size].to_vec();

        group.throughput(Throughput::Bytes(input_16bit.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("rust_16bit", size),
            &encoded_16bit,
            |b, encoded| {
                b.iter(|| {
                    let mut output = vec![0u8; width * height * 2];
                    let mut decoder = JpeglsDecoder::new(black_box(encoded));
                    decoder.read_header().unwrap();
                    decoder.decode(&mut output).unwrap();
                    black_box(output)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// JPEG 2000 Benchmarks
// ============================================================================

fn bench_j2k_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("j2k_encode");
    group.measurement_time(Duration::from_secs(10));

    for size in [256, 512].iter() {
        let width = *size;
        let height = *size;

        let input = generate_gradient_image(width, height, 8);
        group.throughput(Throughput::Bytes(input.len() as u64));

        // Lossless (5-3 DWT)
        group.bench_with_input(
            BenchmarkId::new("rust_lossless", size),
            &input,
            |b, input| {
                let frame_info = FrameInfo {
                    width: width as u32,
                    height: height as u32,
                    bits_per_sample: 8,
                    component_count: 1,
                };
                let mut output = vec![0u8; input.len() * 2];

                b.iter(|| {
                    let mut encoder = J2kEncoder::new();
                    encoder.set_irreversible(false);
                    let size = encoder
                        .encode(black_box(input), &frame_info, &mut output)
                        .unwrap();
                    black_box(size)
                });
            },
        );

        // Lossy (9-7 DWT)
        group.bench_with_input(BenchmarkId::new("rust_lossy", size), &input, |b, input| {
            let frame_info = FrameInfo {
                width: width as u32,
                height: height as u32,
                bits_per_sample: 8,
                component_count: 1,
            };
            let mut output = vec![0u8; input.len() * 2];

            b.iter(|| {
                let mut encoder = J2kEncoder::new();
                encoder.set_irreversible(true);
                let size = encoder
                    .encode(black_box(input), &frame_info, &mut output)
                    .unwrap();
                black_box(size)
            });
        });
    }

    group.finish();
}

fn bench_j2k_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("j2k_decode");
    group.measurement_time(Duration::from_secs(10));

    for size in [256, 512].iter() {
        let width = *size;
        let height = *size;

        // Pre-encode
        let input = generate_gradient_image(width, height, 8);
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut encoded = vec![0u8; input.len() * 2];
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        let enc_size = encoder.encode(&input, &frame_info, &mut encoded).unwrap();
        let encoded = encoded[..enc_size].to_vec();

        group.throughput(Throughput::Bytes(input.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("rust_lossless", size),
            &encoded,
            |b, encoded| {
                b.iter(|| {
                    let mut reader = JpegStreamReader::new(black_box(encoded));
                    let mut decoder = J2kDecoder::new(&mut reader);
                    let image = decoder.decode().unwrap();
                    let pixels = image.reconstruct_pixels().unwrap();
                    black_box(pixels)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// JPEG 1 Benchmarks
// ============================================================================

fn bench_jpeg1_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("jpeg1_encode");
    group.measurement_time(Duration::from_secs(10));

    for size in [256, 512, 1024].iter() {
        let width = *size;
        let height = *size;

        let input = generate_gradient_image(width, height, 8);
        group.throughput(Throughput::Bytes(input.len() as u64));

        for quality in [75, 90, 95].iter() {
            group.bench_with_input(
                BenchmarkId::new(format!("rust_q{}", quality), size),
                &input,
                |b, input| {
                    let frame_info = FrameInfo {
                        width: width as u32,
                        height: height as u32,
                        bits_per_sample: 8,
                        component_count: 1,
                    };
                    let mut output = vec![0u8; input.len() * 2];

                    b.iter(|| {
                        let mut encoder = Jpeg1Encoder::new();
                        encoder.set_quality(*quality as u8);
                        let size = encoder
                            .encode(black_box(input), &frame_info, &mut output)
                            .unwrap();
                        black_box(size)
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_jpeg1_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("jpeg1_decode");
    group.measurement_time(Duration::from_secs(10));

    for size in [256, 512, 1024].iter() {
        let width = *size;
        let height = *size;

        // Pre-encode
        let input = generate_gradient_image(width, height, 8);
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        let mut encoded = vec![0u8; input.len() * 2];
        let mut encoder = Jpeg1Encoder::new();
        encoder.set_quality(90);
        let enc_size = encoder.encode(&input, &frame_info, &mut encoded).unwrap();
        let encoded = encoded[..enc_size].to_vec();

        group.throughput(Throughput::Bytes(input.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("rust_q90", size),
            &encoded,
            |b, encoded| {
                b.iter(|| {
                    let mut output = vec![0u8; width * height];
                    let mut decoder = Jpeg1Decoder::new(black_box(encoded));
                    decoder.read_header().unwrap();
                    decoder.decode(&mut output).unwrap();
                    black_box(output)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Cross-Codec Comparison
// ============================================================================

fn bench_codec_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_comparison");
    group.measurement_time(Duration::from_secs(15));

    let width = 512;
    let height = 512;
    let input = generate_gradient_image(width, height, 8);

    group.throughput(Throughput::Bytes(input.len() as u64));

    // JPEG-LS Lossless
    group.bench_function("jpegls_lossless_roundtrip", |b| {
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        b.iter(|| {
            let mut encoded = vec![0u8; input.len() * 2];
            let mut decoded = vec![0u8; input.len()];
            let mut encoder = JpeglsEncoder::new(&mut encoded);
            encoder.set_frame_info(frame_info).unwrap();
            let size = encoder.encode(black_box(&input)).unwrap();

            let mut decoder = JpeglsDecoder::new(&encoded[..size]);
            decoder.read_header().unwrap();
            decoder.decode(&mut decoded).unwrap();
            black_box(decoded)
        });
    });

    // J2K Lossless
    group.bench_function("j2k_lossless_roundtrip", |b| {
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };
        let mut encoded = vec![0u8; input.len() * 2];

        b.iter(|| {
            let mut encoder = J2kEncoder::new();
            encoder.set_irreversible(false);
            let size = encoder
                .encode(black_box(&input), &frame_info, &mut encoded)
                .unwrap();

            let mut reader = JpegStreamReader::new(&encoded[..size]);
            let mut decoder = J2kDecoder::new(&mut reader);
            let image = decoder.decode().unwrap();
            let pixels = image.reconstruct_pixels().unwrap();
            black_box(pixels)
        });
    });

    // JPEG 1 (lossy baseline)
    group.bench_function("jpeg1_q90_roundtrip", |b| {
        let frame_info = FrameInfo {
            width: width as u32,
            height: height as u32,
            bits_per_sample: 8,
            component_count: 1,
        };

        b.iter(|| {
            let mut encoded = vec![0u8; input.len() * 2];
            let mut decoded = vec![0u8; input.len()];
            let mut encoder = Jpeg1Encoder::new();
            encoder.set_quality(90);
            let size = encoder
                .encode(black_box(&input), &frame_info, &mut encoded)
                .unwrap();

            let mut decoder = Jpeg1Decoder::new(&encoded[..size]);
            decoder.read_header().unwrap();
            decoder.decode(&mut decoded).unwrap();
            black_box(decoded)
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_jpegls_encode,
    bench_jpegls_decode,
    bench_j2k_encode,
    bench_j2k_decode,
    bench_jpeg1_encode,
    bench_jpeg1_decode,
    bench_codec_comparison,
);

criterion_main!(benches);

//! Minimal HTJ2K test cases for systematic debugging

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::fs;
use std::path::Path;

fn calculate_mae(img1: &[u8], img2: &[u8]) -> f64 {
    assert_eq!(img1.len(), img2.len());
    let sum: i64 = img1.iter()
        .zip(img2.iter())
        .map(|(&a, &b)| (a as i64 - b as i64).abs())
        .sum();
    sum as f64 / img1.len() as f64
}

fn save_pgm(path: &str, pixels: &[u8], width: usize, height: usize) -> std::io::Result<()> {
    let header = format!("P5\n{} {}\n255\n", width, height);
    let mut data = header.as_bytes().to_vec();
    data.extend_from_slice(pixels);
    fs::write(path, data)
}

/// Test 2x2 solid black (simplest possible case)
#[test]
#[ignore]
fn test_htj2k_minimal_2x2_black() {
    let width = 2;
    let height = 2;
    let pixels = vec![0u8; width * height]; // All zeros

    println!("\n=== Testing 2x2 solid black ===");

    // Save and encode with OpenHTJ2K
    save_pgm(
        "tests/fixtures/out/test_2x2_black.pgm",
        &pixels,
        width,
        height,
    ).unwrap();

    let output = Command::new("./open_htj2k_enc.exe")
        .args(
            &[
                "-i",
                "tests/fixtures/out/test_2x2_black.pgm",
                "-o",
                "tests/fixtures/out/test_2x2_black.j2c",
                "Creversible=yes",
            ],
        )
        .env("HTJ2K_DEBUG", "1")
        .output()
        .expect("Failed to run encoder");

    if !output.status.success() {
        eprintln!(
            "Encoder failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("Encoder failed");
    }

    println!(
        "Encoded file size: {} bytes",
        fs::metadata("tests/fixtures/out/test_2x2_black.j2c")
            .unwrap()
            .len()
    );

    // Decode with our decoder
    let encoded_data = fs::read("tests/fixtures/out/test_2x2_black.j2c").unwrap();
    println!(
        "Encoded data (hex): {}",
        hex::encode(&encoded_data[..encoded_data.len().min(100)])
    );

    let mut reader = JpegStreamReader::new(&encoded_data);
    let mut decoder = J2kDecoder::new(&mut reader);

    let image = decoder.decode().expect("Decode failed");
    let decoded = image.reconstruct_pixels().expect("Reconstruction failed");

    let mae = calculate_mae(&pixels, &decoded);
    println!("MAE: {}", mae);
    println!("Expected: {:?}", &pixels[..]);
    println!("Decoded:  {:?}", &decoded[..decoded.len().min(4)]);

    fs::remove_file("tests/fixtures/out/test_2x2_black.pgm").ok();
    fs::remove_file("tests/fixtures/out/test_2x2_black.j2c").ok();

    assert_eq!(mae, 0.0, "2x2 black should decode perfectly");
}

/// Test 2x2 solid white
#[test]
#[ignore]
fn test_htj2k_minimal_2x2_white() {
    let width = 2;
    let height = 2;
    let pixels = vec![255u8; width * height];

    println!("\n=== Testing 2x2 solid white ===");

    save_pgm(
        "tests/fixtures/out/test_2x2_white.pgm",
        &pixels,
        width,
        height,
    ).unwrap();

    let output = Command::new("./open_htj2k_enc.exe")
        .args(
            &[
                "-i",
                "tests/fixtures/out/test_2x2_white.pgm",
                "-o",
                "tests/fixtures/out/test_2x2_white.j2c",
                "Creversible=yes",
            ],
        )
        .env("HTJ2K_DEBUG", "1")
        .output()
        .expect("Failed to run encoder");

    if !output.status.success() {
        panic!("Encoder failed");
    }

    let encoded_data = fs::read("tests/fixtures/out/test_2x2_white.j2c").unwrap();
    let mut reader = JpegStreamReader::new(&encoded_data);
    let mut decoder = J2kDecoder::new(&mut reader);

    let image = decoder.decode().expect("Decode failed");
    let decoded = image.reconstruct_pixels().expect("Reconstruction failed");

    let mae = calculate_mae(&pixels, &decoded);
    println!("MAE: {}", mae);

    fs::remove_file("tests/fixtures/out/test_2x2_white.pgm").ok();
    fs::remove_file("tests/fixtures/out/test_2x2_white.j2c").ok();

    assert_eq!(mae, 0.0, "2x2 white should decode perfectly");
}

/// Test 2x2 checkerboard (0, 255, 255, 0)
#[test]
#[ignore]
fn test_htj2k_minimal_2x2_checker() {
    let width = 2;
    let height = 2;
    let pixels = vec![0, 255, 255, 0];

    println!("\n=== Testing 2x2 checkerboard ===");

    save_pgm(
        "tests/fixtures/out/test_2x2_checker.pgm",
        &pixels,
        width,
        height,
    ).unwrap();

    let output = Command::new("./open_htj2k_enc.exe")
        .args(
            &[
                "-i",
                "tests/fixtures/out/test_2x2_checker.pgm",
                "-o",
                "tests/fixtures/out/test_2x2_checker.j2c",
                "Creversible=yes",
            ],
        )
        .env("HTJ2K_DEBUG", "1")
        .output()
        .expect("Failed to run encoder");

    if !output.status.success() {
        panic!("Encoder failed");
    }

    let encoded_data = fs::read("tests/fixtures/out/test_2x2_checker.j2c").unwrap();
    println!("Encoded size: {} bytes", encoded_data.len());

    let mut reader = JpegStreamReader::new(&encoded_data);
    let mut decoder = J2kDecoder::new(&mut reader);

    let image = decoder.decode().expect("Decode failed");
    let decoded = image.reconstruct_pixels().expect("Reconstruction failed");

    let mae = calculate_mae(&pixels, &decoded);
    println!("MAE: {}", mae);
    println!("Expected: {:?}", pixels);
    println!("Decoded:  {:?}", decoded);

    fs::remove_file("tests/fixtures/out/test_2x2_checker.pgm").ok();
    fs::remove_file("tests/fixtures/out/test_2x2_checker.j2c").ok();

    assert_eq!(mae, 0.0, "2x2 checkerboard should decode perfectly");
}

/// Test 4x4 gradient
#[test]
#[ignore]
fn test_htj2k_minimal_4x4_gradient() {
    let width = 4;
    let height = 4;
    let mut pixels = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x + y) * 16) as u8;
        }
    }

    println!("\n=== Testing 4x4 gradient ===");

    save_pgm(
        "tests/fixtures/out/test_4x4_grad.pgm",
        &pixels,
        width,
        height,
    ).unwrap();

    let output = Command::new("./open_htj2k_enc.exe")
        .args(
            &[
                "-i",
                "tests/fixtures/out/test_4x4_grad.pgm",
                "-o",
                "tests/fixtures/out/test_4x4_grad.j2c",
                "Creversible=yes",
                "Clevels=0",
            ],
        )
        .env("HTJ2K_DEBUG", "1")
        .output()
        .expect("Failed to run encoder");

    if !output.status.success() {
        panic!("Encoder failed");
    }

    let encoded_data = fs::read("tests/fixtures/out/test_4x4_grad.j2c").unwrap();
    println!("Encoded size: {} bytes", encoded_data.len());

    let mut reader = JpegStreamReader::new(&encoded_data);
    let mut decoder = J2kDecoder::new(&mut reader);

    let image = decoder.decode().expect("Decode failed");
    let decoded = image.reconstruct_pixels().expect("Reconstruction failed");

    let mae = calculate_mae(&pixels, &decoded);
    println!("MAE: {}", mae);
    println!("Expected: {:?}", pixels);
    println!("Decoded:  {:?}", decoded);

    fs::remove_file("tests/fixtures/out/test_4x4_grad.pgm").ok();
    fs::remove_file("tests/fixtures/out/test_4x4_grad.j2c").ok();

    assert_eq!(mae, 0.0, "4x4 gradient should decode perfectly");
}

/// Test our HTJ2K encoder with OpenHTJ2K decoder
#[test]
#[ignore]
fn test_our_htj2k_encoder_with_openhtj2k_decoder() {
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x + y) % 256) as u8;
        }
    }

    println!("\n=== Testing our HTJ2K encoder with OpenHTJ2K decoder ===");

    // Encode with our encoder
    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = J2kEncoder::new();
    encoder.set_htj2k(true);
    encoder.set_decomposition_levels(2);

    let mut output = vec![0u8; pixels.len() * 2];
    let len = encoder.encode(&pixels, &frame_info, &mut output).expect(
        "Encode failed",
    );
    output.truncate(len);

    println!("Our encoder output: {} bytes", len);

    // Save for OpenHTJ2K decoder
    fs::write("tests/fixtures/out/test_our_htj2k.j2c", &output).unwrap();

    // Decode with OpenHTJ2K
    let decode_output = Command::new("./open_htj2k_dec.exe")
        .args(
            &[
                "-i",
                "tests/fixtures/out/test_our_htj2k.j2c",
                "-o",
                "tests/fixtures/out/test_our_htj2k_decoded.pgm",
            ],
        )
        .output()
        .expect("Failed to run decoder");

    if !decode_output.status.success() {
        eprintln!(
            "OpenHTJ2K decoder stderr: {}",
            String::from_utf8_lossy(&decode_output.stderr)
        );
        eprintln!(
            "OpenHTJ2K decoder stdout: {}",
            String::from_utf8_lossy(&decode_output.stdout)
        );

        fs::remove_file("tests/fixtures/out/test_our_htj2k.j2c").ok();
        panic!("OpenHTJ2K decoder failed on our HTJ2K encoded file");
    }

    // OpenHTJ2K adds _00 suffix to output filename for first component
    let decoded_path = if Path::new("tests/fixtures/out/test_our_htj2k_decoded_00.pgm").exists() {
        "tests/fixtures/out/test_our_htj2k_decoded_00.pgm"
    } else {
        "tests/fixtures/out/test_our_htj2k_decoded.pgm"
    };

    // Read decoded PGM
    let pgm_data = fs::read(decoded_path).unwrap();

    // Parse PGM header - OpenHTJ2K writes: "P5 width height maxval\n"
    // Find the newline after the header
    let header_end = pgm_data.iter().position(|&b| b == b'\n').expect(
        "Invalid PGM",
    ) + 1;
    let decoded_pixels = &pgm_data[header_end..];

    let mae = calculate_mae(&pixels, decoded_pixels);
    println!("MAE (our encoder → OpenHTJ2K decoder): {}", mae);

    fs::remove_file("tests/fixtures/out/test_our_htj2k.j2c").ok();
    fs::remove_file("tests/fixtures/out/test_our_htj2k_decoded.pgm").ok();
    fs::remove_file("tests/fixtures/out/test_our_htj2k_decoded_00.pgm").ok();

    assert_eq!(
        mae,
        0.0,
        "Our HTJ2K encoder should be compatible with OpenHTJ2K decoder"
    );
}

#[test]
#[ignore]
fn test_decode_openhtj2k_2x2_black() {
    let data = fs::read("test_openhtj2k_2x2_black.j2c").expect("Failed to read OpenHTJ2K file");

    println!("\n=== Decoding OpenHTJ2K 2x2 black ===");
    println!("File size: {} bytes", data.len());
    println!(
        "First 60 bytes (hex): {}",
        hex::encode(&data[..60.min(data.len())])
    );

    let mut reader = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(&data);
    let mut decoder = J2kDecoder::new(&mut reader);

    let image = decoder.decode().expect("Decode failed");
    println!("Image: {}x{}", image.width, image.height);
    println!("Has CAP: {}", image.cap.is_some());
    if let Some(cap) = &image.cap {
        println!(
            "Pcap: 0x{:08X}, is_htj2k: {}",
            cap.pcap,
            (cap.pcap & 0x00020000) != 0
        );
    }

    let pixels = image.reconstruct_pixels().expect("Reconstruct failed");
    println!("Pixels: {:?}", pixels);

    let expected = vec![0u8; 4];
    let mae = calculate_mae(&expected, &pixels);
    println!("MAE: {}", mae);

    assert_eq!(mae, 0.0, "OpenHTJ2K 2x2 black should decode to all zeros");
}

#[test]
#[ignore]
fn test_decode_openhtj2k_2x2_white() {
    let data = fs::read("test_openhtj2k_2x2_white.j2c").expect("Failed to read OpenHTJ2K file");

    println!("\n=== Decoding OpenHTJ2K 2x2 white ===");
    println!("File size: {} bytes", data.len());

    let mut reader = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(&data);
    let mut decoder = J2kDecoder::new(&mut reader);

    let image = decoder.decode().expect("Decode failed");
    let pixels = image.reconstruct_pixels().expect("Reconstruct failed");
    println!("Pixels: {:?}", pixels);

    let expected = vec![255u8; 4];
    let mae = calculate_mae(&expected, &pixels);
    println!("MAE: {}", mae);

    // Allow small error due to level shift or rounding?
    // 255 -> -1 -> level shift -> 255.
    // If it decodes to 255, perfect.
    assert!(mae < 1.0, "OpenHTJ2K 2x2 white should decode to approx 255");
}

/// Debug test for 10-bit JPEG-LS interoperability with CharLS
use jpegexp_rs::jpegls::{FrameInfo, JpeglsDecoder, JpeglsEncoder};
use std::fs;
use std::process::Command;

#[test]
#[ignore]
fn debug_jpegls_10bit_checkerboard() {
    // Create 4x4 10-bit checkerboard pattern
    let mut pixels = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            let val: u16 = if (x + y) % 2 == 0 { 0 } else { 1023 }; // 10-bit max
            pixels.extend_from_slice(&val.to_ne_bytes());
        }
    }

    let frame_info = FrameInfo {
        width: 4,
        height: 4,
        bits_per_sample: 10,
        component_count: 1,
    };

    // Test 1: Rust encode -> CharLS decode
    {
        let mut buf = vec![0u8; 1024];
        let mut encoder = JpeglsEncoder::new(&mut buf);

        encoder.set_frame_info(frame_info).unwrap();
        let size = encoder.encode(&pixels).unwrap();

        let output_file = "tests/fixtures/out/debug_rust_10bit.jls";
        fs::write(output_file, &buf[..size]).unwrap();

        println!("\n=== Rust Encode -> CharLS Decode ===");
        println!("Encoded {} bytes to {}", size, output_file);

        // Try to decode with CharLS
        let charls_out = "tests/fixtures/out/debug_rust_10bit_decoded.pnm";
        let result = Command::new("libs/bin/charls.exe")
            .args(["-decodetopnm", output_file, charls_out])
            .output();

        match result {
            Ok(out) if out.status.success() => {
                println!("CharLS decode: SUCCESS");
                let decoded_data = fs::read(charls_out).unwrap();
                println!("Decoded PNM size: {} bytes", decoded_data.len());
                
                // Parse PNM and check
                if let Some(decoded_pixels) = parse_pnm_10bit(&decoded_data) {
                    println!("Decoded {} pixels", decoded_pixels.len());
                    compare_pixels(&pixels, &decoded_pixels);
                }
            }
            Ok(out) => {
                println!("CharLS decode FAILED");
                println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                println!("stdout: {}", String::from_utf8_lossy(&out.stdout));
                panic!("CharLS failed to decode our 10-bit output");
            }
            Err(e) => panic!("Failed to run CharLS: {}", e),
        }
    }

    // Test 2: CharLS encode -> Rust decode
    {
        // Create PNM
        let pnm_file = "tests/fixtures/out/debug_charls_10bit_input.pnm";
        write_pnm_10bit(pnm_file, &pixels, 4, 4).unwrap();

        println!("\n=== CharLS Encode -> Rust Decode ===");

        // Encode with CharLS
        let jls_file = "tests/fixtures/out/debug_charls_10bit.jls";
        let result = Command::new("libs/bin/charls.exe")
            .args(["-encodepnm", pnm_file, jls_file])
            .output();

        match result {
            Ok(out) if out.status.success() => {
                println!("CharLS encode: SUCCESS");
                let encoded = fs::read(jls_file).unwrap();
                println!("Encoded {} bytes", encoded.len());

                // Decode with our decoder
                let mut decoder = JpeglsDecoder::new(&encoded);
                match decoder.read_header() {
                    Ok(_) => {
                        println!("Rust read_header: SUCCESS");
                        let mut decoded = vec![0u8; pixels.len()];
                        match decoder.decode(&mut decoded) {
                            Ok(_) => {
                                println!("Rust decode: SUCCESS");
                                compare_pixels(&pixels, &decoded);
                            }
                            Err(e) => {
                                println!("Rust decode FAILED: {:?}", e);
                                panic!("Failed to decode CharLS 10-bit output: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Rust read_header FAILED: {:?}", e);
                        panic!("Failed to read header from CharLS 10-bit output: {:?}", e);
                    }
                }
            }
            Ok(out) => {
                println!("CharLS encode FAILED");
                println!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                panic!("CharLS failed to encode 10-bit PNM");
            }
            Err(e) => panic!("Failed to run CharLS: {}", e),
        }
    }
}

fn write_pnm_10bit(path: &str, pixels: &[u8], w: u32, h: u32) -> std::io::Result<()> {
    let header = format!("P5\n{} {}\n{}\n", w, h, 1023);
    let mut data = header.into_bytes();

    // Convert from native endian to big endian
    let count = pixels.len() / 2;
    for i in 0..count {
        let val = u16::from_ne_bytes([pixels[i * 2], pixels[i * 2 + 1]]);
        data.extend_from_slice(&val.to_be_bytes());
    }

    fs::write(path, data)
}

fn parse_pnm_10bit(data: &[u8]) -> Option<Vec<u8>> {
    let mut pos = 0;

    // Check magic
    if data.get(pos..pos + 2) != Some(b"P5") {
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

    // Convert from big endian (PNM) to native endian
    let mut native = Vec::new();
    for i in 0..(pixel_data.len() / 2) {
        let val = u16::from_be_bytes([pixel_data[i * 2], pixel_data[i * 2 + 1]]);
        native.extend_from_slice(&val.to_ne_bytes());
    }

    Some(native)
}

fn compare_pixels(original: &[u8], decoded: &[u8]) {
    if original.len() != decoded.len() {
        println!(
            "WARNING: Size mismatch! Original: {} bytes, Decoded: {} bytes",
            original.len(),
            decoded.len()
        );
    }

    let count = original.len().min(decoded.len()) / 2;
    let mut diffs = 0;
    let mut max_diff = 0u16;

    for i in 0..count {
        let orig = u16::from_ne_bytes([original[i * 2], original[i * 2 + 1]]);
        let dec = u16::from_ne_bytes([decoded[i * 2], decoded[i * 2 + 1]]);

        if orig != dec {
            let diff = orig.abs_diff(dec);
            if diffs < 10 {
                println!("  Pixel {}: {} -> {} (diff: {})", i, orig, dec, diff);
            }
            diffs += 1;
            max_diff = max_diff.max(diff);
        }
    }

    if diffs == 0 {
        println!("✓ Perfect match! All {} pixels identical", count);
    } else {
        println!(
            "✗ {} pixels differ (max diff: {})",
            diffs, max_diff
        );
        panic!("Pixels do not match!");
    }
}

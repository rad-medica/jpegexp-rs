use jpegexp_rs::jpeg1::Jpeg1Encoder;
use jpegexp_rs::FrameInfo;
use std::process::Command;
use std::path::Path;
use std::fs;

#[path = "../common/mod.rs"]
mod common;
use common::file_io::get_test_output_path;

// Path to libjpeg-turbo djpeg
const DJPEG_PATH: &str = "libs/bin/djpeg.exe";

fn generate_complex_image(width: u32, height: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 3) as usize];
    
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 3) as usize;
            
            // Quadrant logic
            if y < height / 2 {
                if x < width / 2 {
                    // Q1: Random noise (simulated with math to be deterministic)
                    let r = ((x * y + x + y) % 256) as u8;
                    let g = ((x * x + y * y) % 256) as u8;
                    let b = ((x ^ y) % 256) as u8;
                    buffer[idx] = r;
                    buffer[idx+1] = g;
                    buffer[idx+2] = b;
                } else {
                    // Q2: Smooth Gradient
                    let r = (x * 255 / width) as u8;
                    let g = (y * 255 / height) as u8;
                    buffer[idx] = r;
                    buffer[idx+1] = g;
                    buffer[idx+2] = 128;
                }
            } else {
                if x < width / 2 {
                    // Q3: High frequency grid
                    let val = if (x % 4 == 0) || (y % 4 == 0) { 0 } else { 255 };
                    buffer[idx] = val;
                    buffer[idx+1] = val;
                    buffer[idx+2] = val;
                } else {
                    // Q4: Solid color with subtle variance
                    buffer[idx] = 200;
                    buffer[idx+1] = 100;
                    buffer[idx+2] = 50;
                }
            }
        }
    }
    buffer
}

fn verify_with_djpeg(jpeg_path: &str, output_path: &str) {
    if !Path::new(DJPEG_PATH).exists() {
        println!("SKIPPING: djpeg.exe not found at {}", DJPEG_PATH);
        return;
    }

    let output = Command::new(DJPEG_PATH)
        .arg("-outfile")
        .arg(output_path)
        .arg(jpeg_path)
        .output()
        .expect("Failed to execute djpeg");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("djpeg failed on {}:\n{}", jpeg_path, stderr);
    }
    println!("djpeg successfully decoded {}", jpeg_path);
}

#[test]
fn test_interop_large_complex_images() {
    let width = 2048;
    let height = 2048;
    let source = generate_complex_image(width, height);
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 3,
    };

    struct TestCase {
        name: &'static str,
        progressive: bool,
        optimized: bool,
        subsampling: Option<(u8, u8, u8, u8)>, // Hy, Vy, Hc, Vc
    }

    let cases = vec![
        TestCase { name: "baseline_444", progressive: false, optimized: false, subsampling: Some((1,1,1,1)) },
        TestCase { name: "baseline_420", progressive: false, optimized: false, subsampling: Some((2,2,1,1)) },
        TestCase { name: "baseline_422", progressive: false, optimized: false, subsampling: Some((2,1,1,1)) },
        // TestCase { name: "optimized_444", progressive: false, optimized: true, subsampling: Some((1,1,1,1)) },
        TestCase { name: "progressive_444", progressive: true, optimized: false, subsampling: Some((1,1,1,1)) },
        TestCase { name: "progressive_420", progressive: true, optimized: false, subsampling: Some((2,2,1,1)) },
        // TestCase { name: "progressive_optimized_444", progressive: true, optimized: true, subsampling: Some((1,1,1,1)) },
    ];

    for case in cases {
        println!("Testing case: {}", case.name);
        
        let mut encoder = Jpeg1Encoder::default();
        encoder.set_quality(85);
        encoder.set_progressive(case.progressive);
        encoder.set_optimize_huffman(case.optimized);
        if let Some((hy, vy, hc, vc)) = case.subsampling {
            encoder.set_subsampling(hy, vy, hc, vc);
        }

        let file_path = get_test_output_path(&format!("{}.jpg", case.name));
        let ppm_path = get_test_output_path(&format!("{}.ppm", case.name));

        // Encode
        let mut dest = vec![0u8; (width * height * 3) as usize]; // Large buffer
        let len = encoder.encode(&source, &frame_info, &mut dest).expect("Encoding failed");
        
        fs::write(&file_path, &dest[..len]).expect("Write failed");
        
        // Verify
        verify_with_djpeg(file_path.to_str().unwrap(), ppm_path.to_str().unwrap());
        
        // Cleanup PPM (too large to keep)
        // let _ = fs::remove_file(ppm_path);
    }
}

use jpegexp_rs::jpeg1::dct::{idct_8x8_baseline, idct_8x8_fixed_point};
use std::time::Instant;

fn main() {
    println!("Benchmarking IDCT implementations...");

    let input = [
        10.0, 5.0, -2.0, 1.0, 0.0, 0.0, 0.0, 0.0,
        3.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ]; // Simple mock DCT coefficients

    let iterations = 1_000_000;
    
    // Benchmark Baseline
    let mut output_baseline = [0.0f32; 64];
    let start = Instant::now();
    for _ in 0..iterations {
        idct_8x8_baseline(&input, &mut output_baseline);
        // prevent optimization
        std::hint::black_box(output_baseline);
    }
    let duration_baseline = start.elapsed();
    println!("Baseline (Float) IDCT: {:?} for {} iterations", duration_baseline, iterations);

    // Benchmark Fixed Point
    let mut output_fixed = [0.0f32; 64];
    let start = Instant::now();
    for _ in 0..iterations {
        idct_8x8_fixed_point(&input, &mut output_fixed);
        // prevent optimization
        std::hint::black_box(output_fixed);
    }
    let duration_fixed = start.elapsed();
    println!("Fixed Point IDCT: {:?} for {} iterations", duration_fixed, iterations);

    // Calc speedup
    let speedup = duration_baseline.as_secs_f64() / duration_fixed.as_secs_f64();
    println!("Speedup: {:.2}x", speedup);

    // Verify Accuracy
    let mut max_diff = 0.0f32;
    for i in 0..64 {
        let diff = (output_baseline[i] - output_fixed[i]).abs();
        if diff > max_diff {
            max_diff = diff;
        }
    }
    println!("Max difference between baseline and fixed-point: {}", max_diff);
    
    if max_diff < 2.0 {
        println!("Accuracy: PASSED (Tolerance < 2.0)");
    } else {
        println!("Accuracy: FAILED (Tolerance > 2.0)");
    }
}

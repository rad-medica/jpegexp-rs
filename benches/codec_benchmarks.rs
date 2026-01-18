use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpegls::encoder::JpeglsEncoder;
use jpegexp_rs::FrameInfo;

fn generate_image(width: usize, height: usize, seed: u32) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height];
    for i in 0..pixels.len() {
        // Simple LCG for deterministic "random" noise
        let val = (i as u32 * 1103515245 + 12345 + seed) % 256;
        pixels[i] = val as u8;
    }
    pixels
}

fn bench_jpeg2000_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("jpeg2000_encode");

    for size in [256, 512, 1024].iter() {
        let width = *size;
        let height = *size;
        let input = generate_image(width, height, 42);

        group.throughput(Throughput::Bytes((width * height) as u64));
        group.bench_with_input(BenchmarkId::new("lossless_5_3", size), size, |b, &_s| {
            let mut output = vec![0u8; width * height * 2]; // Pre-allocate output buffer
            let frame_info = FrameInfo {
                width: width as u32,
                height: height as u32,
                bits_per_sample: 8,
                component_count: 1,
            };

            b.iter(|| {
                let mut encoder = J2kEncoder::new();
                encoder.set_quality(100);
                encoder.set_irreversible(false);
                let _ = encoder.encode(&input, &frame_info, &mut output).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_jpegls_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("jpegls_encode");

    for size in [256, 512, 1024].iter() {
        let width = *size;
        let height = *size;
        let input = generate_image(width, height, 42);

        group.throughput(Throughput::Bytes((width * height) as u64));
        group.bench_with_input(BenchmarkId::new("lossless_default", size), size, |b, &_s| {
            let mut output = vec![0u8; width * height * 2];
            let frame_info = FrameInfo {
                width: width as u32,
                height: height as u32,
                bits_per_sample: 8,
                component_count: 1,
            };

            b.iter(|| {
                let mut encoder = JpeglsEncoder::new(&mut output);
                encoder.set_frame_info(frame_info).unwrap();
                let _ = encoder.encode(&input).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_jpeg2000_encode, bench_jpegls_encode);
criterion_main!(benches);

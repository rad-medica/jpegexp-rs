use jpegexp_rs::jpeg1::{Jpeg1Decoder, Jpeg1Encoder};
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::jpegls::{InterleaveMode, JpeglsDecoder, JpeglsEncoder};
use jpegexp_rs::FrameInfo;

#[derive(Debug, Clone, Copy)]
enum CodecType {
    Jpeg1,
    Jpegls,
    Jpeg2000,
    Htj2k,
}

fn calculate_mae(original: &[u16], reconstructed: &[u16]) -> f64 {
    if original.is_empty() {
        return 0.0;
    }
    let total_diff: u64 = original
        .iter()
        .zip(reconstructed.iter())
        .map(|(&a, &b)| (a as i32 - b as i32).abs() as u64)
        .sum();
    total_diff as f64 / original.len() as f64
}

fn run_universal_roundtrip(
    codec_type: CodecType,
    width: u32,
    height: u32,
    components: i32,
    bits: i32,
    is_signed: bool,
    quality: u8,
) {
    let frame_info = FrameInfo {
        width,
        height,
        component_count: components,
        bits_per_sample: bits,
    };

    let pixel_count = (width * height * components as u32) as usize;
    let mut source_u16 = vec![0u16; pixel_count];
    let max_val = (1 << bits) - 1;
    let level_shift = if is_signed { 1 << (bits - 1) } else { 0 };

    for i in 0..pixel_count {
        let val = (i % (max_val as usize + 1)) as i32;
        source_u16[i] = if is_signed {
            ((val - level_shift as i32) & 0xFFFF) as u16
        } else {
            val as u16
        };
    }

    let mut encoded = vec![0u8; pixel_count * 4 + 1024];
    let encoded_len: usize;

    match codec_type {
        CodecType::Jpeg1 => {
            let mut encoder = Jpeg1Encoder::new();
            encoder.set_bits_per_sample(bits as u8);
            encoder.set_quality(quality);
            if bits <= 8 {
                let source_u8: Vec<u8> = source_u16.iter().map(|&v| v as u8).collect();
                encoded_len = encoder
                    .encode(&source_u8, &frame_info, &mut encoded)
                    .expect("JPEG 1 encode failed");
            } else {
                encoded_len = encoder
                    .encode_u16(&source_u16, &frame_info, &mut encoded)
                    .expect("JPEG 1 u16 encode failed");
            }
        }
        CodecType::Jpegls => {
            let mut encoder = JpeglsEncoder::new(&mut encoded);
            encoder.set_frame_info(frame_info).unwrap();
            if components > 1 {
                encoder.set_interleave_mode(InterleaveMode::Sample).unwrap();
            }
            if bits <= 8 {
                let source_u8: Vec<u8> = source_u16.iter().map(|&v| v as u8).collect();
                encoded_len = encoder.encode(&source_u8).expect("JLS encode failed");
            } else {
                let mut source_bytes = vec![0u8; source_u16.len() * 2];
                for (i, &v) in source_u16.iter().enumerate() {
                    let b = v.to_ne_bytes();
                    source_bytes[i * 2] = b[0];
                    source_bytes[i * 2 + 1] = b[1];
                }
                encoded_len = encoder.encode(&source_bytes).expect(
                    "JLS u16 encode failed",
                );
            }
        }
        CodecType::Jpeg2000 |
        CodecType::Htj2k => {
            let mut encoder = J2kEncoder::new();
            encoder.set_htj2k(matches!(codec_type, CodecType::Htj2k));
            encoder.set_irreversible(quality < 100);
            if quality < 100 {
                encoder.set_quality(quality);
            }

            let bytes = if bits <= 8 {
                source_u16.iter().map(|&v| v as u8).collect()
            } else {
                let mut b = Vec::with_capacity(source_u16.len() * 2);
                for &v in &source_u16 {
                    b.extend_from_slice(&v.to_le_bytes());
                }
                b
            };

            encoded_len = encoder.encode(&bytes, &frame_info, &mut encoded).expect(
                "J2K encode failed",
            );
        }
    }

    let mut reconstructed_u16 = vec![0u16; pixel_count];
    match codec_type {
        CodecType::Jpeg1 => {
            let mut decoder = Jpeg1Decoder::new(&encoded[..encoded_len]);
            decoder.read_header().expect("JPEG 1 header read failed");
            if bits <= 8 {
                let mut recon_u8 = vec![0u8; pixel_count];
                decoder.decode(&mut recon_u8).expect("JPEG 1 decode failed");
                for i in 0..pixel_count {
                    reconstructed_u16[i] = recon_u8[i] as u16;
                }
            } else {
                decoder.decode_u16(&mut reconstructed_u16).expect(
                    "JPEG 1 u16 decode failed",
                );
            }
        }
        CodecType::Jpegls => {
            let mut decoder = JpeglsDecoder::new(&encoded[..encoded_len]);
            decoder.read_header().expect("JLS header read failed");
            if bits <= 8 {
                let mut recon_u8 = vec![0u8; pixel_count];
                decoder.decode(&mut recon_u8).expect("JLS decode failed");
                for i in 0..pixel_count {
                    reconstructed_u16[i] = recon_u8[i] as u16;
                }
            } else {
                let mut recon_bytes = vec![0u8; pixel_count * 2];
                decoder.decode(&mut recon_bytes).expect(
                    "JLS u16 decode failed",
                );
                for i in 0..pixel_count {
                    reconstructed_u16[i] =
                        u16::from_ne_bytes([recon_bytes[i * 2], recon_bytes[i * 2 + 1]]);
                }
            }
        }
        CodecType::Jpeg2000 |
        CodecType::Htj2k => {
            let mut reader = JpegStreamReader::new(&encoded[..encoded_len]);
            let mut decoder = J2kDecoder::new(&mut reader);
            let image = decoder.decode().expect("J2K decode failed");
            let pixels = image.reconstruct_pixels().expect(
                "J2K reconstruction failed",
            );
            for i in 0..pixel_count {
                reconstructed_u16[i] = pixels[i] as u16;
            }
        }
    }

    let mae = calculate_mae(&source_u16, &reconstructed_u16);
    println!(
        "{:?} ({}x{}x{}, {}b): MAE={:.4}",
        codec_type,
        width,
        height,
        components,
        bits,
        mae
    );

    let threshold = if quality == 100 { 0.1 } else { 50.0 };
    assert!(
        mae < threshold,
        "MAE too high for {:?}: {}",
        codec_type,
        mae
    );
}

#[test]
fn test_matrix_all_codecs() {
    let configs = [
        (CodecType::Jpeg1, 16, 16, 1, 8, false, 75),
        (CodecType::Jpeg1, 16, 16, 1, 12, false, 90),
        (CodecType::Jpegls, 16, 16, 1, 8, false, 100),
        (CodecType::Jpegls, 16, 16, 1, 16, false, 100),
        (CodecType::Jpegls, 16, 16, 3, 8, false, 100),
        (CodecType::Jpeg2000, 16, 16, 1, 8, false, 100),
        (CodecType::Jpeg2000, 16, 16, 3, 8, false, 100),
        (CodecType::Jpeg2000, 16, 16, 1, 16, true, 100),
        (CodecType::Htj2k, 16, 16, 1, 8, false, 100),
        (CodecType::Htj2k, 16, 16, 3, 8, false, 100),
    ];

    for (codec, w, h, comp, bits, signed, q) in configs {
        run_universal_roundtrip(codec, w, h, comp, bits, signed, q);
    }
}

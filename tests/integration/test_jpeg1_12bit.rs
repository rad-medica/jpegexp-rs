use jpegexp_rs::jpeg1::Jpeg1Decoder;
use jpegexp_rs::jpeg1::Jpeg1Encoder;
use jpegexp_rs::FrameInfo;

#[test]
fn test_jpeg1_12bit_roundtrip() {
    let width = 16;
    let height = 16;
    let mut source = vec![0u16; width * height];
    for (i, val) in source.iter_mut().enumerate() {
        *val = (i * 4) as u16; // 0 to 1020
    }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 12,
        component_count: 1,
    };

    let mut encoder = Jpeg1Encoder::new();
    encoder.set_bits_per_sample(12);
    encoder.set_quality(90);

    let mut encoded = vec![0u8; 10000];
    let enc_len = encoder
        .encode_u16(&source, &frame_info, &mut encoded)
        .expect("Encode failed");

    // Verify SOF marker
    let mut found_sof = false;
    let expected_sof = if frame_info.bits_per_sample > 8 {
        0xC1
    } else {
        0xC0
    };
    for (i, &byte) in encoded.iter().enumerate().take(enc_len - 1) {
        if byte == 0xFF && encoded[i + 1] == expected_sof {
            found_sof = true;
            break;
        }
    }
    assert!(
        found_sof,
        "Encoded stream should contain expected SOF marker"
    );

    let mut decoder = Jpeg1Decoder::new(&encoded[..enc_len]);
    decoder.read_header().expect("Read header failed");
    assert_eq!(
        decoder.frame_info().bits_per_sample,
        frame_info.bits_per_sample
    );

    let mut decoded = vec![0u16; width * height];
    decoder.decode_u16(&mut decoded).expect("Decode failed");

    println!("First 8 source: {:?}", &source[0..8]);
    println!("First 8 decoded: {:?}", &decoded[0..8]);

    for (i, (&src, &dec)) in source.iter().zip(decoded.iter()).enumerate() {
        let diff = src.abs_diff(dec);
        // 12-bit DCT is lossy, allow some error
        assert!(
            diff < 100,
            "Mismatch at index {}: src={} dec={} diff={}",
            i,
            src,
            dec,
            diff
        );
    }
}

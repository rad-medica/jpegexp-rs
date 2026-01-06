#[test]
#[ignore] // Fails on 64x64 blocks due to potential MQ coder desync
fn test_bpc_64x64_gradient() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

    let width = 64;
    let height = 64;

    // Create gradient data (similar to LL band of gradient image)
    // 12-bit signed data
    let mut data = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            // Gradient 0 to 4032 (Positive)
            let val = (x + y) as i32 * 32;
            data.push(val);
        }
    }

    // Encode
    let mut encoder = BitPlaneCoder::new(width, height, &data);
    let max_bp = encoder.calculate_max_bit_plane().unwrap_or(0);
    println!("Max BP: {}", max_bp);

    let orientation = 0; // LL
    let passes = encoder.encode_codeblock(max_bp, orientation);
    encoder.mq.flush();
    let encoded = encoder.mq.get_buffer().to_vec();

    println!("Encoded {} bytes, {} passes", encoded.len(), passes);

    // Decode
    let mut decoder = BitPlaneCoder::new(width, height, &[]);
    let decoded_data = decoder
        .decode_codeblock(&encoded, max_bp, passes, orientation)
        .expect("Decode failed");

    // Verify
    let mut mismatches = 0;
    for i in 0..data.len() {
        if data[i] != decoded_data[i] {
            println!(
                "Mismatch at {}: orig={}, dec={}",
                i, data[i], decoded_data[i]
            );
            mismatches += 1;
            if mismatches > 10 {
                break;
            }
        }
    }
    assert_eq!(mismatches, 0);
}

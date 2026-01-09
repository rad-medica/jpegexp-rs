
#[cfg(test)]
mod tests {
    use jpegexp_rs::jpeg2000::image::J2kCodeBlock;
    use jpegexp_rs::jpeg2000::ht_block_coder::encoder::HTBlockEncoder;
    use jpegexp_rs::jpeg2000::ht_block_coder::coder::HTBlockCoder;

    #[test]
    fn test_ht_coder_roundtrip_basic() {
        let w = 4;
        let h = 4;
        let mut block = J2kCodeBlock::default();
        block.width = w as u32;
        block.height = h as u32;
        block.coefficients = vec![0; (w * h) as usize];
        
        // Set a single pixel to 1
        block.coefficients[0] = 1; 
        
        // Encode
        let mut encoder = HTBlockEncoder::new(w, h);
        let encoded = encoder.encode_block(&block).unwrap();
        
        println!("Encoded {} bytes", encoded.len());
        
        // Decode
        // For HTJ2K, we need to split the buffer?
        // My HTBlockEncoder interleaves them into a single buffer.
        // HTBlockCoder expects separate buffers? 
        // No, HTBlockCoder::new takes mel_data and magsgn_data.
        // BUT my Encoder emits a SINGLE buffer (MagSgn ... MEL/VLC).
        // I need to split them?
        // HTJ2K packet format: [MagSgn] [MEL/VLC].
        // But they grow towards each other?
        // Or they are concatenated?
        // In the single buffer returned by encoder:
        // [MagSgn (forward) ...... MEL/VLC (backward)]
        // So I can pass the SAME buffer to both?
        // MelDecoder reads from end. MagSgnDecoder reads from start.
        // Yes, pass the same buffer.
        
        let mut decoder_block = J2kCodeBlock::default();
        let mut decoder = HTBlockCoder::new(&encoded, &encoded, w, h);
        
        decoder.decode_block(&mut decoder_block).unwrap();
        
        // Verify
        assert_eq!(decoder_block.coefficients.len(), block.coefficients.len());
        assert_eq!(decoder_block.coefficients[0], 1, "Pixel 0 mismatch");
        assert_eq!(decoder_block.coefficients[1], 0, "Pixel 1 mismatch");
        
        // Try other values
        let values = [1, -1, 127, -128, 255, 1024];
        for &val in &values {
            block.coefficients[0] = val;
            let mut enc = HTBlockEncoder::new(w, h);
            let data = enc.encode_block(&block).unwrap();
            
            let mut dec_block = J2kCodeBlock::default();
            let mut dec = HTBlockCoder::new(&data, &data, w, h);
            dec.decode_block(&mut dec_block).unwrap();
            
            assert_eq!(dec_block.coefficients[0], val, "Value {} mismatch. Got {}", val, dec_block.coefficients[0]);
        }
    }
}

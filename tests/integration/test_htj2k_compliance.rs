use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::FrameInfo;
use jpegexp_rs::jpeg_marker_code::JpegMarkerCode;

#[test]
fn test_htj2k_dicom_compliance_markers() {
    // 1. Setup Encoder
    let width = 64;
    let height = 64;
    let mut pixels = vec![0u8; width * height];
    // Fill with pattern
    for i in 0..pixels.len() { pixels[i] = (i % 255) as u8; }

    let frame_info = FrameInfo {
        width: width as u32,
        height: height as u32,
        bits_per_sample: 8,
        component_count: 1,
    };

    let mut encoder = J2kEncoder::new();
    encoder.set_htj2k(true); // Enable HTJ2K
    encoder.set_decomposition_levels(5);
    encoder.set_irreversible(false); // Lossless (5-3)

    let mut output = vec![0u8; width * height * 2 + 1024];
    let len = encoder.encode(&pixels, &frame_info, &mut output).expect("Encoding failed");
    output.truncate(len);

    // 2. Validate Markers manually
    let mut reader = JpegStreamReader::new(&output);
    
    // SOC
    let soc = reader.read_u16().unwrap();
    assert_eq!(soc, 0xFF4F, "Codestream must start with SOC (0xFF4F)");

    // Search for CAP marker (0xFF50)
    let mut found_cap = false;
    let mut found_siz = false;
    let mut found_cod = false;
    let mut pcap_val = 0u32;

    loop {
        // Find next marker (FFxx)
        // Simple scan for markers (not robust parser, but good for validation)
        // Real parser would skip segments.
        // We can use J2kParser logic or just look for tags.
        // Since we generated it, structure is predictable: SOC, CAP, SIZ, COD...
        
        let b1 = reader.read_u8().unwrap();
        if b1 != 0xFF { continue; }
        let b2 = reader.read_u8().unwrap();
        
        if b2 == 0x50 { // CAP
            found_cap = true;
            let len = reader.read_u16().unwrap();
            // Lcap should be >= 8 (2 + 4 + 2*C) for 1 component
            assert!(len >= 8, "CAP marker length too short");
            pcap_val = reader.read_u32().unwrap();
            // Don't consume more, let loop continue (or skip)
            reader.advance((len - 6) as usize); // 6 bytes read (2 len + 4 Pcap)
        } else if b2 == 0x51 { // SIZ
            found_siz = true;
            let len = reader.read_u16().unwrap();
            reader.advance((len - 2) as usize);
        } else if b2 == 0x52 { // COD
            found_cod = true;
            let len = reader.read_u16().unwrap();
            reader.advance((len - 2) as usize);
        } else if b2 == 0x90 { // SOT
            break; // Stop at tile part
        }
        
        if reader.remaining_data().is_empty() { break; }
    }

    assert!(found_cap, "CAP marker (0xFF50) missing - Required for HTJ2K");
    
    // Check Pcap bit 14 (or 17 depending on endianness/counting)
    // We expect 0x00020000 based on OpenHTJ2K compatibility
    let ht_bit = 0x00020000; 
    assert_eq!(pcap_val & ht_bit, ht_bit, "CAP marker Pcap must have HTJ2K bit set (0x{:08X})", pcap_val);

    println!("DICOM HTJ2K Compliance Check Passed: CAP present, Pcap=0x{:08X}", pcap_val);
}

#[test]
fn test_htj2k_lossless_transfer_syntax_requirements() {
    // DICOM 1.2.840.10008.1.2.4.201 requires 5-3 Reversible Transform
    let width = 32;
    let height = 32;
    let pixels = vec![0u8; width * height];
    let frame_info = FrameInfo { width: width as u32, height: height as u32, bits_per_sample: 8, component_count: 1 };

    let mut encoder = J2kEncoder::new();
    encoder.set_htj2k(true);
    encoder.set_irreversible(false); // 5-3

    let mut output = vec![0u8; 1024];
    let len = encoder.encode(&pixels, &frame_info, &mut output).expect("Encode failed");
    output.truncate(len);

    // Verify COD marker says Reversible
    let mut reader = JpegStreamReader::new(&output);
    loop {
        if reader.read_u8().unwrap() == 0xFF && reader.read_u8().unwrap() == 0x52 {
            // Found COD
            let _len = reader.read_u16().unwrap();
            let _scod = reader.read_u8().unwrap();
            let _sgcod = reader.read_u32().unwrap(); // 4 bytes (Prog, Layers, MCT)
            // SPcod: Decomp(1), CodeWidth(1), CodeHeight(1), Style(1), Trans(1)
            reader.read_u8().unwrap(); // Decomp
            reader.read_u8().unwrap(); // Width
            reader.read_u8().unwrap(); // Height
            reader.read_u8().unwrap(); // Style
            let trans = reader.read_u8().unwrap(); // Transformation
            
            assert_eq!(trans, 1, "Transformation must be 1 (5-3 Reversible) for Lossless HTJ2K");
            break;
        }
        if reader.remaining_data().is_empty() { break; }
    }
}

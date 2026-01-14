/// Manual bit-level analysis of Resolution 1 packet header
#[test]
fn analyze_res1_header_bits() {
    println!("=== Resolution 1 Packet Header Bit Analysis ===\n");

    println!("Known from trace tests:");
    println!("  Bytes 0-12: Match perfectly between ours and OpenJPEG");
    println!("  Byte 12: 0x8F (last matching byte)");
    println!("  Bytes 13-14 (ours):    0x9C 0x20");
    println!("  Bytes 13-14 (OpenJPEG): 0xB4 0x0C");
    println!("  Byte 15+: HL codeblock data starts (0x0D 0x02 0xFF 0x7F)");
    println!();

    let ours_b13: u8 = 0x9C;
    let ours_b14: u8 = 0x20;
    let opj_b13: u8 = 0xB4;
    let opj_b14: u8 = 0x0C;

    println!("Ours (bytes 13-14):    {:08b} {:08b}", ours_b13, ours_b14);
    println!("OpenJPEG (bytes 13-14): {:08b} {:08b}", opj_b13, opj_b14);
    println!();

    println!("Structure of Resolution 1 packet header:");
    println!("  For each subband (HL, LH, HH):");
    println!("    For each codeblock in 1x1 grid (just 1 CB per subband):");
    println!("      1. Inclusion tag tree encode");
    println!("      2. Zero bitplanes tag tree encode");
    println!("      3. Num passes (variable-length)");
    println!("      4. Lblock increment (comma code, variable-length)");
    println!("      5. Data length (lblock + log2(passes) bits)");
    println!();

    println!("From trace_full_pipeline:");
    println!("  HL: 13 passes, 4 bytes, zero_bp likely 7 or 8");
    println!("  LH: 13 passes, 4 bytes, zero_bp likely 7 or 8");
    println!("  HH: all zeros (excluded)");
    println!();

    println!("For 13 passes:");
    println!("  log2(13) = floor(3.7) = 3");
    println!("  Encoding: need 4 bits minimum to represent 13");
    println!("  JPEG 2000 pass count encoding is variable-length");
    println!();

    println!("For 4 bytes length:");
    println!("  bits_needed = floor(log2(4)) + 1 = 3");
    println!("  lblock = 3 + max(0, 3 - 3 - 3) = 3");
    println!("  lbits = 3 + 3 = 6 bits");
    println!("  Data length: {:06b} = 4", 4);
    println!();

    println!("Key question: What's the difference in the encoded bits?");
    println!("Likely candidates:");
    println!("  1. Zero bitplane values differ");
    println!("  2. Num passes encoding differs");
    println!("  3. Lblock calculation differs");
    println!("  4. HH subband handling (excluded vs empty-included)");
    println!();

    println!("Byte 13-14 combined as 16 bits:");
    let ours_combined = ((ours_b13 as u16) << 8) | (ours_b14 as u16);
    let opj_combined = ((opj_b13 as u16) << 8) | (opj_b14 as u16);
    println!(
        "  Ours:     {:016b} = 0x{:04X}",
        ours_combined,
        ours_combined
    );
    println!("  OpenJPEG: {:016b} = 0x{:04X}", opj_combined, opj_combined);

    let xor = ours_combined ^ opj_combined;
    println!(
        "  XOR:      {:016b} ({} bits differ)",
        xor,
        xor.count_ones()
    );
    println!();

    println!(
        "Byte 12 (last matching byte): 0x{:02X} = {:08b}",
        0x8F,
        0x8F
    );
    println!("If byte 12 is not byte-aligned, some bits of byte 13 belong to previous field.");
}

use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;

#[test]
fn analyze_openjpeg_numbps() {
    println!("=== Analyzing OpenJPEG cblk->numbps calculation ===\n");

    println!("From OpenJPEG source:");
    println!("  cblk->numbps = max ? (floor(log2(max)) + 1 - T1_NMSEDEC_FRACBITS) : 0");
    println!("  where T1_NMSEDEC_FRACBITS = 6");
    println!();

    println!("For HL subband with coefficients [0, 17, 0, 17]:");
    let hl_data = vec![0i32, 17, 0, 17];
    let max_val = hl_data.iter().map(|v| v.abs()).max().unwrap();
    println!("  max_val = {}", max_val);
    println!(
        "  floor(log2({})) = {}",
        max_val,
        (max_val as f32).log2().floor() as u32
    );
    println!(
        "  cblk->numbps = {} + 1 - 6 = {}",
        (max_val as f32).log2().floor() as i32,
        (max_val as f32).log2().floor() as i32 + 1 - 6
    );
    println!();

    println!("But this gives a negative number!");
    println!("Let me check the actual OpenJPEG code more carefully...");
    println!();

    println!("From the librarian output, the formula is applied in t1.c:2609-2610");
    println!("But this is for the T1 encoder (tier-1 encoding).");
    println!("The T1_NMSEDEC_FRACBITS might be for a different purpose.");
    println!();

    let mut bpc = BitPlaneCoder::new(2, 2, &hl_data);
    let max_bp = bpc.calculate_max_bit_plane();
    println!(
        "Our BitPlaneCoder::calculate_max_bit_plane() = {:?}",
        max_bp
    );
    println!();

    println!("The issue might be that OpenJPEG's cblk->numbps is NOT directly");
    println!("the same as our max_bp. Let me check if there's a different field.");
    println!();

    println!("From t2.c:759 (packet header writing):");
    println!("  opj_tgt_setvalue(prc->imsbtree, cblkno, band->numbps - (OPJ_INT32)cblk->numbps);");
    println!();
    println!("This suggests zero_bp is calculated at packet writing time,");
    println!("not during codeblock encoding.");
    println!();

    println!("Let me check what band->numbps is:");
    println!("  From tcd.c:1089-1090:");
    println!("  band->numbps = epsilon + guard_bits - 1");
    println!("  For HL: band->numbps = 9 + 2 - 1 = 10");
    println!();

    println!("So if zero_bp = band->numbps - cblk->numbps,");
    println!("and we need to match OpenJPEG, we need to know what cblk->numbps is.");
    println!();

    println!("Hypothesis: The T1_NMSEDEC_FRACBITS subtraction might only apply");
    println!("in lossy mode or for EBCOT rate-distortion optimization.");
    println!("In lossless mode, cblk->numbps might just be floor(log2(max)) + 1.");
}

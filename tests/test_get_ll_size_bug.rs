/// Test get_ll_size calculation
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;

#[test]
#[ignore]
fn test_get_ll_size_bug() {
    let mut encoder = J2kEncoder::new();
    encoder.set_decomposition_levels(1);
    
    // For 40x40 image with 1 level:
    // - Resolution 0: LL should be 20x20
    // - Resolution 1: LL should be 20x20 (from level 0)
    
    let (ll0_w, ll0_h) = encoder.get_ll_size(40, 40, 1, 0);
    println!("Resolution 0 LL size: {}x{}", ll0_w, ll0_h);
    assert_eq!(ll0_w, 20, "LL0 width should be 20");
    assert_eq!(ll0_h, 20, "LL0 height should be 20");
    
    let (ll1_w, ll1_h) = encoder.get_ll_size(40, 40, 1, 1);
    println!("Resolution 1 LL size: {}x{}", ll1_w, ll1_h);
    assert_eq!(ll1_w, 20, "LL1 width should be 20");
    assert_eq!(ll1_h, 20, "LL1 height should be 20");
    
    // Now check extract_subband_coeffs for HL at res=1
    // For HL:
    // - ll_w, ll_h = get_ll_size(40, 40, 1, 1) = ?
    // - prev_ll_w, prev_ll_h = get_ll_size(40, 40, 1, 0) = 20x20
    // - For HL: sb_w = ll_w - prev_ll_w, sb_h = prev_ll_h, start_x = prev_ll_w
    
    // If get_ll_size(40, 40, 1, 1) returns 40x40:
    //   sb_w = 40 - 20 = 20
    //   sb_h = 20
    //   start_x = 20
    //   So HL is extracted from (20, 0) to (39, 19)
    
    // But if get_ll_size(40, 40, 1, 1) returns 20x20:
    //   sb_w = 20 - 20 = 0
    //   sb_h = 20
    //   start_x = 20
    //   So HL is extracted from (20, 0) with width 0!
    
    // This is the BUG! get_ll_size(40, 40, 1, 1) should return 20x20,
    // not 40x40!
    
    println!("\nThe bug is in get_ll_size:");
    println!("For res=1, levels_remaining = num_levels - res = 1 - 1 = 0");
    println!("With 0 remaining levels, it returns the full size (40x40)");
    println!("But for res=1, the LL is at resolution 0, which is 20x20!");
    println!("\nThe fix: levels_remaining should be num_levels.saturating_sub(res + 1)");
    println!("For res=1: levels_remaining = 1 - 2 = 0 (still 0!)");
    println!("Wait, that's not right either...");
    
    // Actually, the issue is that for res=1 (highest resolution),
    // the LL subband comes from res=0 (previous level)
    // So we need levels_remaining = num_levels - (res - 1)
    //                         = 1 - 0 = 1
    // But with the current formula: levels_remaining = num_levels - res = 1 - 1 = 0
    
    // The CORRECT formula should be:
    // levels_remaining = num_levels - (res - 1)
    //                 = num_levels - res + 1
    
    // For res=0: levels_remaining = 1 - 0 + 1 = 2? No that's wrong.
    
    // Let me think again...
    // For res=0: we want LL at resolution 0, which means 1 level of division
    // For res=1: we want LL at resolution 0 (previous level), which means 1 level of division
    // So both should give 20x20
    
    // The formula should be:
    // levels_remaining = num_levels - res + 1  for res > 0?
    // But for res=0: 1 - 0 + 1 = 2 (wrong, should be 1)
    
    // Actually, looking at the code:
    // for _ in 0..levels_remaining {
    //     w = (w + 1) / 2;
    // }
    //
    // For res=0 with num_levels=1: we want to divide once -> levels_remaining = 1
    // For res=1 with num_levels=1: we want to divide once -> levels_remaining = 1
    
    // So levels_remaining should ALWAYS be num_levels for getting the LL size at resolution res
    
    println!("\nActually, I think the formula should be:");
    println!("levels_remaining = num_levels - res + 1");
    println!("For res=0: 1 - 0 + 1 = 2 (wrong!)");
    println!("\nOR levels_remaining = num_levels");
    println!("For res=0: 1 (correct)");
    println!("For res=1: 1 (correct)");
    
    // Let me check if the current code uses a different formula...
}

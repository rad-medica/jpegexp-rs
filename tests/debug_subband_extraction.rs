/// Debug subband extraction to find the issue
#[test]
fn debug_subband_extraction() {
    let width = 40;
    let height = 40;
    let num_levels = 1;
    
    // NEW correct formula (res + 1 reductions)
    fn get_ll_size(width: usize, height: usize, num_levels: usize, res: usize) -> (usize, usize) {
        let reductions = res + 1;
        let mut w = width;
        let mut h = height;
        for _ in 0..reductions {
            w = (w + 1) / 2;
            h = (h + 1) / 2;
        }
        (w.max(1), h.max(1))
    }
    
    println!("For 40x40 image with 1 decomposition level:");
    println!("  get_ll_size(40, 40, 1, 0) = {:?}", get_ll_size(width, height, num_levels, 0));
    println!("  get_ll_size(40, 40, 1, 1) = {:?}", get_ll_size(width, height, num_levels, 1));
    
    // What the code calculates for res=1, HL subband
    let (ll_w, ll_h) = get_ll_size(width, height, num_levels, 1);
    let (prev_ll_w, prev_ll_h) = get_ll_size(width, height, num_levels, 0);
    
    println!("\nFor res=1, HL subband:");
    println!("  ll_w={}, ll_h={} (at res=1)", ll_w, ll_h);
    println!("  prev_ll_w={}, prev_ll_h={} (at res=0)", prev_ll_w, prev_ll_h);
    
    // img_w should be the ORIGINAL image size at resolution res
    // For res=1, img_w = width (not reduced)
    // But the code calculates img_w = (40+1)/2 = 20
    let mut img_w = width;
    let mut img_h = height;
    for _ in 0..1 {
        img_w = (img_w + 1) / 2;
        img_h = (img_h + 1) / 2;
    }
    
    println!("  img_w={}, img_h={} (WRONG - should be 40,40!)", img_w, img_h);
    
    // With the wrong img_w, the HL width becomes:
    let hl_w = img_w.saturating_sub(prev_ll_w);
    let hl_h = prev_ll_h;
    
    println!("  HL subband: {}x{} at ({}, 0)", hl_w, hl_h, prev_ll_w);
    println!("  EXPECTED: 20x20 at (20, 0)");
    
    // The fix: img_w and img_h should be the ORIGINAL image size
    println!("\n=== THE FIX ===");
    let correct_img_w = width; // Use original width, not reduced
    let correct_img_h = height;
    
    let correct_hl_w = correct_img_w - prev_ll_w;
    let correct_hl_h = prev_ll_h;
    
    println!("  Corrected HL: {}x{} at ({}, 0)", correct_hl_w, correct_hl_h, prev_ll_w);
    println!("  Corrected LH: {}x{} at (0, {})", prev_ll_w, correct_img_h - prev_ll_h, prev_ll_h);
    println!("  Corrected HH: {}x{} at ({}, {})", correct_img_w - prev_ll_w, correct_img_h - prev_ll_h, prev_ll_w, prev_ll_h);
}

use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;

#[test]
#[ignore]
fn dump_subband_coefficient_ranges() {
    let width = 64;
    let height = 64;
    let mut pixels = vec![0i32; width * height];
    
    for y in 0..height {
        for x in 0..width {
            pixels[y * width + x] = ((x * 4 + y * 4) % 256) as i32 - 128;
        }
    }
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(2);
    
    let coeffs_result = encoder.apply_forward_dwt_2d(&mut pixels.clone(), width, height).unwrap();
    
    println!("\n=== DWT Coefficient Ranges (2 levels) ===");
    
    fn get_ll_size(width: usize, height: usize, num_levels: usize, res: usize) -> (usize, usize) {
        let levels_remaining = num_levels - res;
        let mut w = width;
        let mut h = height;
        for _ in 0..levels_remaining {
            w = (w + 1) / 2;
            h = (h + 1) / 2;
        }
        (w.max(1), h.max(1))
    }
    
    // Resolution 0: LL
    let (ll_w, ll_h) = get_ll_size(width, height, 2, 0);
    let mut ll_min = i32::MAX;
    let mut ll_max = i32::MIN;
    for y in 0..ll_h {
        for x in 0..ll_w {
            let val = coeffs_result[y * width + x];
            ll_min = ll_min.min(val);
            ll_max = ll_max.max(val);
        }
    }
    println!("Resolution 0 LL ({}x{}): range=[{}, {}]", ll_w, ll_h, ll_min, ll_max);
    
    // Resolution 1: HL, LH, HH
    let (ll_w, ll_h) = get_ll_size(width, height, 2, 1);
    let (prev_w, prev_h) = get_ll_size(width, height, 2, 0);
    
    let bands = [
        ("HL", ll_w - prev_w, prev_h, prev_w, 0),
        ("LH", prev_w, ll_h - prev_h, 0, prev_h),
        ("HH", ll_w - prev_w, ll_h - prev_h, prev_w, prev_h),
    ];
    
    for (name, sb_w, sb_h, start_x, start_y) in bands.iter() {
        let mut min_val = i32::MAX;
        let mut max_val = i32::MIN;
        for y in 0..*sb_h {
            for x in 0..*sb_w {
                let val = coeffs_result[(start_y + y) * width + (start_x + x)];
                min_val = min_val.min(val);
                max_val = max_val.max(val);
            }
        }
        println!("Resolution 1 {} ({}x{} at ({},{})): range=[{}, {}]", 
                 name, sb_w, sb_h, start_x, start_y, min_val, max_val);
    }
    
    // Resolution 2: HL, LH, HH
    let (ll_w, ll_h) = get_ll_size(width, height, 2, 2);
    let (prev_w, prev_h) = get_ll_size(width, height, 2, 1);
    
    let bands = [
        ("HL", ll_w - prev_w, prev_h, prev_w, 0),
        ("LH", prev_w, ll_h - prev_h, 0, prev_h),
        ("HH", ll_w - prev_w, ll_h - prev_h, prev_w, prev_h),
    ];
    
    for (name, sb_w, sb_h, start_x, start_y) in bands.iter() {
        let mut min_val = i32::MAX;
        let mut max_val = i32::MIN;
        for y in 0..*sb_h {
            for x in 0..*sb_w {
                let val = coeffs_result[(start_y + y) * width + (start_x + x)];
                min_val = min_val.min(val);
                max_val = max_val.max(val);
            }
        }
        println!("Resolution 2 {} ({}x{} at ({},{})): range=[{}, {}]", 
                 name, sb_w, sb_h, start_x, start_y, min_val, max_val);
    }
}

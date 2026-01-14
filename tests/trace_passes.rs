use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;
use jpegexp_rs::jpeg2000::dwt::Dwt53;

#[test]
fn trace_codeblock_passes() {
    let pixels: Vec<u8> = (0..16).map(|i| (i * 255 / 15) as u8).collect();
    let level_shifted: Vec<i32> = pixels.iter().map(|&p| p as i32 - 128).collect();
    
    let mut coeffs = level_shifted.clone();
    
    for y in 0..4 {
        let row: Vec<i32> = (0..4).map(|x| coeffs[y * 4 + x]).collect();
        let mut out_l = vec![0i32; 2];
        let mut out_h = vec![0i32; 2];
        Dwt53::forward(&row, &mut out_l, &mut out_h);
        coeffs[y * 4 + 0] = out_l[0];
        coeffs[y * 4 + 1] = out_l[1];
        coeffs[y * 4 + 2] = out_h[0];
        coeffs[y * 4 + 3] = out_h[1];
    }
    
    for x in 0..4 {
        let col: Vec<i32> = (0..4).map(|y| coeffs[y * 4 + x]).collect();
        let mut out_l = vec![0i32; 2];
        let mut out_h = vec![0i32; 2];
        Dwt53::forward(&col, &mut out_l, &mut out_h);
        coeffs[0 * 4 + x] = out_l[0];
        coeffs[1 * 4 + x] = out_l[1];
        coeffs[2 * 4 + x] = out_h[0];
        coeffs[3 * 4 + x] = out_h[1];
    }
    
    println!("DWT coefficients:");
    for y in 0..4 {
        for x in 0..4 {
            print!("{:5} ", coeffs[y * 4 + x]);
        }
        println!();
    }
    
    let ll: Vec<i32> = vec![coeffs[0], coeffs[1], coeffs[4], coeffs[5]];
    let hl: Vec<i32> = vec![coeffs[2], coeffs[3], coeffs[6], coeffs[7]];
    let lh: Vec<i32> = vec![coeffs[8], coeffs[9], coeffs[12], coeffs[13]];
    let hh: Vec<i32> = vec![coeffs[10], coeffs[11], coeffs[14], coeffs[15]];
    
    println!("\n=== LL subband ===");
    println!("Data: {:?}", ll);
    let mut bpc_ll = BitPlaneCoder::new(2, 2, &ll);
    let max_bp_ll = bpc_ll.calculate_max_bit_plane();
    let min_bp_ll = bpc_ll.calculate_min_bit_plane();
    println!("max_bp: {:?}, min_bp: {}", max_bp_ll, min_bp_ll);
    if let Some(max_bp) = max_bp_ll {
        let passes = bpc_ll.encode_codeblock(max_bp, min_bp_ll, 0);
        bpc_ll.mq.flush();
        let encoded = bpc_ll.mq.get_buffer();
        println!("Passes: {}, Encoded: {} bytes", passes, encoded.len());
        println!("Data: {:02X?}", encoded);
    }
    
    println!("\n=== HL subband ===");
    println!("Data: {:?}", hl);
    let mut bpc_hl = BitPlaneCoder::new(2, 2, &hl);
    let max_bp_hl = bpc_hl.calculate_max_bit_plane();
    let min_bp_hl = bpc_hl.calculate_min_bit_plane();
    println!("max_bp: {:?}, min_bp: {}", max_bp_hl, min_bp_hl);
    if let Some(max_bp) = max_bp_hl {
        let passes = bpc_hl.encode_codeblock(max_bp, min_bp_hl, 1);
        bpc_hl.mq.flush();
        let encoded = bpc_hl.mq.get_buffer();
        println!("Passes: {}, Encoded: {} bytes", passes, encoded.len());
        println!("Data: {:02X?}", encoded);
    }
    
    println!("\n=== LH subband ===");
    println!("Data: {:?}", lh);
    let mut bpc_lh = BitPlaneCoder::new(2, 2, &lh);
    let max_bp_lh = bpc_lh.calculate_max_bit_plane();
    let min_bp_lh = bpc_lh.calculate_min_bit_plane();
    println!("max_bp: {:?}, min_bp: {}", max_bp_lh, min_bp_lh);
    if let Some(max_bp) = max_bp_lh {
        let passes = bpc_lh.encode_codeblock(max_bp, min_bp_lh, 2);
        bpc_lh.mq.flush();
        let encoded = bpc_lh.mq.get_buffer();
        println!("Passes: {}, Encoded: {} bytes", passes, encoded.len());
        println!("Data: {:02X?}", encoded);
    }
    
    println!("\n=== HH subband ===");
    println!("Data: {:?}", hh);
    let mut bpc_hh = BitPlaneCoder::new(2, 2, &hh);
    let max_bp_hh = bpc_hh.calculate_max_bit_plane();
    let min_bp_hh = bpc_hh.calculate_min_bit_plane();
    println!("max_bp: {:?}, min_bp: {}", max_bp_hh, min_bp_hh);
    if let Some(max_bp) = max_bp_hh {
        let passes = bpc_hh.encode_codeblock(max_bp, min_bp_hh, 3);
        bpc_hh.mq.flush();
        let encoded = bpc_hh.mq.get_buffer();
        println!("Passes: {}, Encoded: {} bytes", passes, encoded.len());
        println!("Data: {:02X?}", encoded);
    } else {
        println!("All zeros - no encoding needed");
    }
}

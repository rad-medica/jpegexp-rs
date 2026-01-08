use jpegexp_rs::jpeg2000::dwt::Dwt53;

fn main() {
    let input = vec![-128, 127, -128, 127, 0, 255, 0, 255];
    let len = input.len();
    let l_len = (len + 1) / 2;
    let h_len = len / 2;
    let mut l = vec![0i32; l_len];
    let mut h = vec![0i32; h_len];

    Dwt53::forward(&input, &mut l, &mut h);
    println!("L: {:?}", l);
    println!("H: {:?}", h);

    let mut output = vec![0i32; len];
    Dwt53::inverse(&l, &h, &mut output);
    println!("Output: {:?}", output);

    for i in 0..len {
        if input[i] != output[i] {
            println!("MISMATCH at {}: expected {}, got {}", i, input[i], output[i]);
        }
    }
    
    // Test 2D
    let width = 4;
    let height = 4;
    let mut input2d = vec![0i32; width * height];
    for i in 0..width*height { input2d[i] = (i as i32 % 256) - 128; }
    
    let l_2d = vec![0i32; 4]; // LL 2x2
    let hl_2d = vec![0i32; 4]; // HL 2x2
    let lh_2d = vec![0i32; 4]; // LH 2x2
    let hh_2d = vec![0i32; 4]; // HH 2x2
    
    // Simplified 2D forward (only 1 level)
    let mut temp = input2d.clone();
    // Rows
    for y in 0..height {
        let mut row_l = vec![0i32; 2];
        let mut row_h = vec![0i32; 2];
        Dwt53::forward(&temp[y*width..y*width+width], &mut row_l, &mut row_h);
        temp[y*width] = row_l[0]; temp[y*width+1] = row_l[1];
        temp[y*width+2] = row_h[0]; temp[y*width+3] = row_h[1];
    }
    // Cols
    for x in 0..width {
        let col = vec![temp[x], temp[width+x], temp[2*width+x], temp[3*width+x]];
        let mut col_l = vec![0i32; 2];
        let mut col_h = vec![0i32; 2];
        Dwt53::forward(&col, &mut col_l, &mut col_h);
        temp[x] = col_l[0]; temp[width+x] = col_l[1];
        temp[2*width+x] = col_h[0]; temp[3*width+x] = col_h[1];
    }
    
    // Subbands extraction
    let ll = vec![temp[0], temp[1], temp[width], temp[width+1]];
    let hl = vec![temp[2], temp[3], temp[width+2], temp[width+3]];
    let lh = vec![temp[2*width], temp[2*width+1], temp[3*width], temp[3*width+1]];
    let hh = vec![temp[2*width+2], temp[2*width+3], temp[3*width+2], temp[3*width+3]];
    
    let mut output2d = vec![0i32; width * height];
    Dwt53::inverse_2d(&ll, &hl, &lh, &hh, width as u32, height as u32, &mut output2d);
    
    let mut diffs = 0;
    for i in 0..width*height {
        if input2d[i] != output2d[i] {
            diffs += 1;
            println!("2D MISMATCH at {}: exp {}, got {}", i, input2d[i], output2d[i]);
        }
    }
    println!("2D Total Mismatches: {}", diffs);
}

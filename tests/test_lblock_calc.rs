// Test to verify lblock calculation matches OpenJPEG
#[test]
fn test_lblock_calculation() {
    // Test cases: (data_len, num_passes) -> expected (increment, lbits)
    // Formula: increment = floor(log2(len)) + 1 - (numlenbits + floor(log2(nump)))
    // Where numlenbits = 3 for first inclusion
    let test_cases = vec![
        // Small data: floor(log2(n)) where n is data_len
        (1, 1, 0, 3),    // floor(log2(1)) + 1 = 0 + 1 = 1, floor(log2(1)) = 0, inc = max(0, 1-3-0) = 0, lbits = 3+0 = 3
        (2, 1, 0, 3),    // floor(log2(2)) + 1 = 1 + 1 = 2, floor(log2(1)) = 0, inc = max(0, 2-3-0) = 0, lbits = 3+0 = 3
        (4, 1, 0, 3),    // floor(log2(4)) + 1 = 2 + 1 = 3, floor(log2(1)) = 0, inc = max(0, 3-3-0) = 0, lbits = 3+0 = 3
        (8, 1, 1, 4),    // floor(log2(8)) + 1 = 3 + 1 = 4, floor(log2(1)) = 0, inc = max(0, 4-3-0) = 1, lbits = 4+0 = 4
        (16, 1, 2, 5),   // floor(log2(16)) + 1 = 4 + 1 = 5, floor(log2(1)) = 0, inc = max(0, 5-3-0) = 2, lbits = 5+0 = 5
        
        // Multiple passes
        (8, 2, 0, 4),    // floor(log2(8)) + 1 = 4, floor(log2(2)) = 1, inc = max(0, 4-3-1) = 0, lbits = 3+1 = 4
        (16, 2, 1, 5),   // floor(log2(16)) + 1 = 5, floor(log2(2)) = 1, inc = max(0, 5-3-1) = 1, lbits = 4+1 = 5
        (32, 4, 1, 6),   // floor(log2(32)) + 1 = 6, floor(log2(4)) = 2, inc = max(0, 6-3-2) = 1, lbits = 4+2 = 6
        
        // Large data
        (100, 10, 1, 7),  // floor(log2(100)) + 1 = 6 + 1 = 7, floor(log2(10)) = 3, inc = max(0, 7-3-3) = 1, lbits = 4+3 = 7
        (255, 10, 2, 8), // floor(log2(255)) + 1 = 7 + 1 = 8, floor(log2(10)) = 3, inc = max(0, 8-3-3) = 2, lbits = 5+3 = 8
    ];
    
    for (data_len, num_passes, expected_inc, expected_lbits) in test_cases {
        let bits_needed = if data_len > 0 {
            (32 - (data_len as u32).leading_zeros()) as i32
        } else {
            1
        };
        
        let log2_passes = if num_passes > 0 {
            (31 - (num_passes as u32).leading_zeros()) as i32
        } else {
            0
        };
        
        let numlenbits = 3;
        let increment = (bits_needed - numlenbits - log2_passes).max(0);
        let lblock = numlenbits + increment;
        let lbits = lblock + log2_passes;
        
        println!("data_len={}, num_passes={}: bits_needed={}, log2_passes={}, lblock={}, inc={}, lbits={}",
                 data_len, num_passes, bits_needed, log2_passes, lblock, increment, lbits);
        
        assert_eq!(increment, expected_inc, 
                   "Incorrect increment for data_len={}, num_passes={}", data_len, num_passes);
        assert_eq!(lbits, expected_lbits, 
                   "Incorrect lbits for data_len={}, num_passes={}", data_len, num_passes);
    }
}

#[test]
fn test_floor_log2() {
    // Verify floor(log2(n)) calculation using: 31 - leading_zeros(n) for n > 0
    // For n = 0, result is undefined (we use 32 - leading_zeros which gives 32)
    assert_eq!((31 - 1u32.leading_zeros()) as i32, 0); // floor(log2(1)) = 0
    assert_eq!((31 - 2u32.leading_zeros()) as i32, 1); // floor(log2(2)) = 1
    assert_eq!((31 - 3u32.leading_zeros()) as i32, 1); // floor(log2(3)) = 1
    assert_eq!((31 - 4u32.leading_zeros()) as i32, 2); // floor(log2(4)) = 2
    assert_eq!((31 - 8u32.leading_zeros()) as i32, 3); // floor(log2(8)) = 3
    assert_eq!((31 - 10u32.leading_zeros()) as i32, 3); // floor(log2(10)) = 3
    assert_eq!((31 - 16u32.leading_zeros()) as i32, 4); // floor(log2(16)) = 4
    assert_eq!((31 - 100u32.leading_zeros()) as i32, 6); // floor(log2(100)) = 6
    assert_eq!((31 - 255u32.leading_zeros()) as i32, 7); // floor(log2(255)) = 7
    
    // For floor(log2(n)) + 1 we use: 32 - leading_zeros(n)
    assert_eq!((32 - 1u32.leading_zeros()) as i32, 1);  // floor(log2(1)) + 1 = 1
    assert_eq!((32 - 2u32.leading_zeros()) as i32, 2);  // floor(log2(2)) + 1 = 2
    assert_eq!((32 - 4u32.leading_zeros()) as i32, 3);  // floor(log2(4)) + 1 = 3
    assert_eq!((32 - 8u32.leading_zeros()) as i32, 4);  // floor(log2(8)) + 1 = 4
    assert_eq!((32 - 16u32.leading_zeros()) as i32, 5); // floor(log2(16)) + 1 = 5
    assert_eq!((32 - 100u32.leading_zeros()) as i32, 7); // floor(log2(100)) + 1 = 7
    assert_eq!((32 - 255u32.leading_zeros()) as i32, 8); // floor(log2(255)) + 1 = 8
}

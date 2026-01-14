/// Verify the context calculation bug for LH orientation

#[test]
fn verify_lh_context_calculation() {
    println!("\n=== Verifying LH Context Calculation ===\n");
    
    // Test case: h=1, v=2, d=0
    // This should give different results for HL vs LH
    
    let h = 1;
    let v = 2;
    let d = 0;
    
    // Our current HL logic (orientation=1)
    let hl_context = if v == 2 {
        8
    } else if v == 1 {
        if h >= 1 {
            7
        } else if d >= 1 {
            6
        } else {
            5
        }
    } else if h == 2 {
        4
    } else if h == 1 {
        3
    } else if d >= 2 {
        2
    } else if d == 1 {
        1
    } else {
        0
    };
    
    // Our current LH logic (orientation=2) - WRONG!
    let lh_context_wrong = if h == 2 {
        8
    } else if h == 1 {
        if v >= 1 {
            7
        } else if d >= 1 {
            6
        } else {
            5
        }
    } else if v == 2 {
        4
    } else if v == 1 {
        3
    } else if d >= 2 {
        2
    } else if d == 1 {
        1
    } else {
        0
    };
    
    // Correct LH logic (should swap h and v, then use HL logic)
    let lh_context_correct = {
        let h_swapped = v;
        let v_swapped = h;
        if v_swapped == 2 {
            8
        } else if v_swapped == 1 {
            if h_swapped >= 1 {
                7
            } else if d >= 1 {
                6
            } else {
                5
            }
        } else if h_swapped == 2 {
            4
        } else if h_swapped == 1 {
            3
        } else if d >= 2 {
            2
        } else if d == 1 {
            1
        } else {
            0
        }
    };
    
    println!("Test case: h={}, v={}, d={}", h, v, d);
    println!("  HL context (orient=1): {}", hl_context);
    println!("  LH context (our current, orient=2): {}", lh_context_wrong);
    println!("  LH context (correct, should swap h/v): {}", lh_context_correct);
    
    if lh_context_wrong != lh_context_correct {
        println!("\n❌ BUG FOUND! LH context calculation is WRONG!");
        println!("   Current: {}, Should be: {}", lh_context_wrong, lh_context_correct);
    } else {
        println!("\n✅ LH context calculation is correct");
    }
    
    // OpenJPEG's logic for orient=0,1 (after swap for orient=2):
    // if (!h) {
    //     if (!v) {
    //         if (!d) n = 0;
    //         else if (d == 1) n = 1;
    //         else n = 2;
    //     } else if (v == 1) n = 3;
    //     else n = 4;
    // } else if (h == 1) {
    //     if (!v) {
    //         if (!d) n = 5;
    //         else n = 6;
    //     } else n = 7;
    // } else n = 8;
    
    // For h=1, v=2, d=0 (after swap for LH: h=2, v=1, d=0):
    // h=2, so n=8
    
    println!("\nOpenJPEG expected for LH with h={}, v={}, d={}:", h, v, d);
    println!("  After swap: h={}, v={}, d={}", v, h, d);
    println!("  Expected context: 8");
    
    assert_eq!(lh_context_correct, 8, "LH context should be 8 after swap");
}

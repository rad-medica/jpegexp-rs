/// Debug DWT coefficients to find the exact divergence point
#[test]
fn debug_dwt_coefficients() {
    use jpegexp_rs::jpeg2000::dwt::Dwt53;
    
    // Test with the same pattern as our 40x40 test: (x + y) mod 256
    // For a single row, this is just x mod 256
    let input: Vec<i32> = (0..40).map(|i| (i as i32) % 256).collect();
    println!("Input signal (40 samples): {:?}", &input[..10]);
    
    // Apply forward DWT
    let l_len = (input.len() + 1) / 2;  // 20
    let h_len = input.len() / 2;        // 20
    let mut l = vec![0i32; l_len];
    let mut h = vec![0i32; h_len];
    
    Dwt53::forward(&input, &mut l, &mut h);
    
    println!("\nLow coefficients (first 5): {:?}", &l[..5]);
    println!("High coefficients (first 5): {:?}", &h[..5]);
    println!("High coefficients (last 5): {:?}", &h[h_len-5..]);
    
    // Apply inverse DWT
    let mut output = vec![0i32; input.len()];
    Dwt53::inverse(&l, &h, &mut output);
    
    println!("\nOutput signal (first 10): {:?}", &output[..10]);
    println!("Output signal (last 10): {:?}", &output[30..]);
    
    // Check for errors
    let mut errors = 0;
    for i in 0..input.len() {
        if input[i] != output[i] {
            if errors < 5 {
                println!("Error at {}: input={}, output={}, diff={}", i, input[i], output[i], input[i] - output[i]);
            }
            errors += 1;
        }
    }
    
    println!("\nTotal errors: {} / {}", errors, input.len());
    
    if errors > 0 {
        // Analyze the error pattern
        println!("\nError analysis:");
        let first_error = output.iter().enumerate()
            .find(|(i, &v)| v != input[*i])
            .map(|(i, _)| i)
            .unwrap();
        
        println!("First error at index: {}", first_error);
        println!("This is {} (mod 2) = {}", first_error, first_error % 2);
        
        if first_error % 2 == 1 {
            println!("Error is at an ODD index (prediction step)");
        } else {
            println!("Error is at an EVEN index (update step)");
        }
        
        // Check if all errors are at the end
        let last_error = output.iter().enumerate()
            .rfind(|(i, &v)| v != input[*i])
            .map(|(i, _)| i)
            .unwrap();
        println!("Last error at index: {}", last_error);
    }
    
    // Don't assert - we want to see all errors
    if errors > 0 {
        println!("\n⚠️  DWT has {} errors", errors);
    } else {
        println!("\n✅ DWT is perfectly reversible");
    }
}

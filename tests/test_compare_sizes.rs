/// Test 8x8 (works) vs 10x10 (fails) comparison
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;

#[test]
#[ignore]
fn compare_8x8_vs_10x10() {
    for &size in &[8, 10] {
        let mut pixels = vec![0u8; (size * size * 2) as usize];
        for y in 0..size {
            for x in 0..size {
                let idx = ((y * size + x) * 2) as usize;
                let val = ((x * 4 + y * 4) % 256) as u16;
                pixels[idx] = (val & 0xFF) as u8;
                pixels[idx + 1] = (val >> 8) as u8;
            }
        }
        
        let mut encoder = J2kEncoder::new();
        encoder.set_irreversible(false);
        encoder.set_decomposition_levels(2);
        encoder.set_include_tlm(false);
        encoder.set_include_plt(false);
        
        let frame_info = FrameInfo {
            width: size,
            height: size,
            bits_per_sample: 16,
            component_count: 1,
        };
        
        let mut output = vec![0u8; 1024 * 1024];
        let output_size = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
        
        let filename = format!("compare_{}x{}.j2k", size, size);
        fs::write(&filename, &output[..output_size]).unwrap();
        println!("Created {} ({} bytes)", filename, output_size);
    }
    
    // Now hex dump both files side-by-side around the SOD marker
    println!("\n=== Hex comparison ===");
    
    let data8 = fs::read("compare_8x8.j2k").unwrap();
    let data10 = fs::read("compare_10x10.j2k").unwrap();
    
    // Find SOD marker (FF 93) in both
    let find_sod = |data: &[u8]| -> Option<usize> {
        for i in 0..data.len()-1 {
            if data[i] == 0xFF && data[i+1] == 0x93 {
                return Some(i);
            }
        }
        None
    };
    
    if let (Some(sod8), Some(sod10)) = (find_sod(&data8), find_sod(&data10)) {
        println!("\n8x8 SOD at offset {:#x}", sod8);
        println!("8x8 packet 0: {:02x?}", &data8[sod8+2..sod8+2+6]);
        
        println!("\n10x10 SOD at offset {:#x}", sod10);
        println!("10x10 packet 0: {:02x?}", &data10[sod10+2..sod10+2+6]);
    }
}

use jpegexp_rs::jpeg2000::bit_io::J2kBitWriter;
use jpegexp_rs::jpeg2000::tag_tree::TagTree;

#[test]
fn test_full_packet_header_simulation() {
    println!("\n=== Full Packet Header Simulation (4x4 gradient, 1 subband, 1 codeblock) ===\n");
    
    let mut writer = J2kBitWriter::new();
    
    println!("Step 1: Empty packet bit");
    writer.write_bit(1);
    println!("  Wrote: 1 (non-empty packet)\n");
    
    let grid_width = 1;
    let grid_height = 1;
    
    println!("Step 2: Process subband 0 (LL) with {}x{} codeblock grid", grid_width, grid_height);
    
    let mut inclusion_tree = TagTree::new(grid_width, grid_height);
    let mut zero_bp_tree = TagTree::new(grid_width, grid_height);
    
    let cb_x = 0;
    let cb_y = 0;
    let layer_index = 0;
    let threshold = (layer_index + 1) as i32;
    
    println!("\nStep 3: Encode inclusion tag tree for CB({},{}):", cb_x, cb_y);
    println!("  Setting value to layer_index={}", layer_index);
    println!("  Threshold={}", threshold);
    inclusion_tree.set_value(cb_x, cb_y, layer_index as i32);
    
    inclusion_tree.encode(&mut writer, cb_x, cb_y, threshold);
    
    let zero_bp = 1;
    println!("\nStep 4: Encode zero bitplanes for CB({},{}):", cb_x, cb_y);
    println!("  zero_bp={}, threshold={}", zero_bp, zero_bp + 1);
    zero_bp_tree.set_value(cb_x, cb_y, zero_bp as i32);
    
    zero_bp_tree.encode(&mut writer, cb_x, cb_y, zero_bp as i32 + 1);
    
    let num_passes = 10;
    println!("\nStep 5: Encode number of coding passes: {}", num_passes);
    write_coding_passes(&mut writer, num_passes);
    
    let data_len: u32 = 9;
    let numlenbits = 3;
    
    println!("\nStep 6: Encode lblock increment and data length:");
    println!("  data_len={}, numlenbits={}", data_len, numlenbits);
    
    let bits_needed = if data_len > 0 {
        (32 - data_len.leading_zeros()) as i32
    } else {
        1
    };
    
    let log2_passes = if num_passes > 0 {
        (31 - (num_passes as u32).leading_zeros()) as i32
    } else {
        0
    };
    
    let increment = (bits_needed - numlenbits - log2_passes).max(0);
    let lblock = numlenbits + increment;
    let lbits = lblock + log2_passes;
    
    println!("  bits_needed={}, log2_passes={}", bits_needed, log2_passes);
    println!("  increment={}, lblock={}, lbits={}", increment, lblock, lbits);
    
    write_comma_code(&mut writer, increment);
    
    writer.write_bits(data_len, lbits as u8);
    
    println!("\nStep 7: Align to byte boundary");
    writer.align_to_byte();
    
    let bytes = writer.finish();
    
    println!("\n=== Final Packet Header ===");
    println!("Total bytes: {}", bytes.len());
    println!("\nHex dump:");
    for (i, &byte) in bytes.iter().enumerate() {
        println!("Byte {}: {:02X} ({:08b})", i, byte, byte);
    }
    
    println!("\n=== Expected from OpenJPEG (from earlier test) ===");
    println!("Byte 0: DF (11011111)");
    println!("Byte 1: 80 (10000000)");
    
    println!("\n=== Our Output ===");
    if bytes.len() >= 2 {
        println!("Byte 0: {:02X} ({:08b})", bytes[0], bytes[0]);
        println!("Byte 1: {:02X} ({:08b})", bytes[1], bytes[1]);
        
        if bytes[0] == 0xDF && bytes[1] == 0x80 {
            println!("\n✓ MATCHES OpenJPEG!");
        } else {
            println!("\n✗ DOES NOT match OpenJPEG");
            println!("\nBit-by-bit comparison of byte 0:");
            for bit_pos in 0..8 {
                let our_bit = (bytes[0] >> (7 - bit_pos)) & 1;
                let opj_bit = (0xDF >> (7 - bit_pos)) & 1;
                let match_char = if our_bit == opj_bit { "✓" } else { "✗" };
                println!("  Bit {}: ours={}, OpenJPEG={} {}", bit_pos, our_bit, opj_bit, match_char);
            }
        }
    }
}

fn write_coding_passes(writer: &mut J2kBitWriter, passes: u8) {
    match passes {
        1 => writer.write_bit(0),
        2 => {
            writer.write_bit(1);
            writer.write_bit(0);
        }
        3..=5 => {
            writer.write_bit(1);
            writer.write_bit(1);
            writer.write_bits((passes - 3) as u32, 2);
        }
        6..=36 => {
            writer.write_bit(1);
            writer.write_bit(1);
            writer.write_bits(3, 2);
            writer.write_bits((passes - 6) as u32, 5);
        }
        _ => {
            writer.write_bit(1);
            writer.write_bit(1);
            writer.write_bits(3, 2);
            writer.write_bits(31, 5);
            writer.write_bits((passes - 37) as u32, 5);
        }
    }
}

fn write_comma_code(writer: &mut J2kBitWriter, value: i32) {
    for _ in 0..value {
        writer.write_bit(1);
    }
    writer.write_bit(0);
}

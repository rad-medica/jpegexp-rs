use jpegexp_rs::jpeg2000::bit_io::J2kBitWriter;
use jpegexp_rs::jpeg2000::tag_tree::TagTree;

#[test]
fn test_tag_tree_encoding_simple() {
    let mut tt = TagTree::new(1, 1);
    
    tt.set_value(0, 0, 0);
    
    let mut writer = J2kBitWriter::new();
    
    tt.encode(&mut writer, 0, 0, 1);
    
    let bytes = writer.finish();
    
    println!("\n=== Tag Tree Encoding (1x1, value=0, threshold=1) ===");
    println!("Bytes written: {}", bytes.len());
    for (i, &byte) in bytes.iter().enumerate() {
        println!("Byte {}: {:02X} ({:08b})", i, byte, byte);
    }
    
    println!("\nExpected: value=0, threshold=1");
    println!("Should write: 1 bit (value '1') since value(0) == low(0)");
    println!("Then align to byte");
}

#[test]
fn test_tag_tree_encoding_layer_inclusion() {
    println!("\n=== Simulating Packet Header Inclusion Tag Tree ===");
    
    let mut tt = TagTree::new(2, 2);
    
    println!("\nScenario: 2x2 grid, all codeblocks included in layer 0");
    println!("Setting all values to 0 (included in layer 0)");
    
    for y in 0..2 {
        for x in 0..2 {
            tt.set_value(x, y, 0);
        }
    }
    
    let mut writer = J2kBitWriter::new();
    
    println!("\nEncoding inclusion for codeblock (0,0) with threshold=1:");
    tt.encode(&mut writer, 0, 0, 1);
    
    let bytes_after_first = writer.finish();
    println!("Bits written for first CB: {} bytes", bytes_after_first.len());
    for (i, &byte) in bytes_after_first.iter().enumerate() {
        println!("  Byte {}: {:02X} ({:08b})", i, byte, byte);
    }
    
    let mut writer2 = J2kBitWriter::new();
    tt.set_value(0, 0, 0);
    tt.encode(&mut writer2, 0, 0, 1);
    tt.set_value(1, 0, 0);
    tt.encode(&mut writer2, 1, 0, 1);
    
    let bytes_after_second = writer2.finish();
    println!("\nWith second CB (1,0) encoded:");
    println!("Total bits: {} bytes", bytes_after_second.len());
    for (i, &byte) in bytes_after_second.iter().enumerate() {
        println!("  Byte {}: {:02X} ({:08b})", i, byte, byte);
    }
}

#[test]
fn test_tag_tree_parent_sync() {
    println!("\n=== Testing Tag Tree Parent Synchronization ===");
    
    let mut tt = TagTree::new(2, 2);
    
    println!("Setting value (0,0) = 0, (1,0) = 0");
    tt.set_value(0, 0, 0);
    tt.set_value(1, 0, 0);
    
    let mut writer = J2kBitWriter::new();
    
    println!("\nFirst encode (0,0):");
    tt.encode(&mut writer, 0, 0, 1);
    
    println!("Then encode (1,0) - should share parent node state:");
    tt.encode(&mut writer, 1, 0, 1);
    
    let bytes = writer.finish();
    
    println!("\nTotal output:");
    for (i, &byte) in bytes.iter().enumerate() {
        println!("Byte {}: {:02X} ({:08b})", i, byte, byte);
    }
    
    println!("\n=== Now compare with fresh tag tree ===");
    let mut tt2 = TagTree::new(2, 2);
    let mut writer2 = J2kBitWriter::new();
    
    println!("Setting values BEFORE any encoding:");
    tt2.set_value(0, 0, 0);
    tt2.set_value(1, 0, 0);
    
    println!("Encoding (0,0):");
    tt2.encode(&mut writer2, 0, 0, 1);
    
    println!("Encoding (1,0):");
    tt2.encode(&mut writer2, 1, 0, 1);
    
    let bytes2 = writer2.finish();
    
    println!("\nTotal output (pre-set values):");
    for (i, &byte) in bytes2.iter().enumerate() {
        println!("Byte {}: {:02X} ({:08b})", i, byte, byte);
    }
    
    if bytes == bytes2 {
        println!("\n✓ SAME output regardless of when set_value is called");
    } else {
        println!("\n✗ DIFFERENT output - timing of set_value matters!");
    }
}

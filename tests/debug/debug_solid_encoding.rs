use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::FrameInfo;
use std::fs;

#[test]
fn debug_solid_4x4_encoding() {
    let width = 4;
    let height = 4;
    let pixels = vec![128u8; (width * height) as usize];
    
    let mut encoder = J2kEncoder::new();
    encoder.set_irreversible(false);
    encoder.set_decomposition_levels(1);
    
    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };
    
    let mut output = vec![0u8; pixels.len() * 20];
    let bytes_written = encoder.encode(&pixels, &frame_info, &mut output).unwrap();
    output.truncate(bytes_written);
    
    println!("Encoded {} bytes", output.len());
    
    let mut sod_pos = None;
    for i in 0..output.len() - 1 {
        if output[i] == 0xFF && output[i + 1] == 0x93 {
            sod_pos = Some(i);
            break;
        }
    }
    
    if let Some(pos) = sod_pos {
        let tile_data = &output[pos + 2..];
        println!("SOD at offset {}", pos);
        println!("Tile data ({} bytes): {:02X?}", tile_data.len(), tile_data);
        println!("Binary: {:08b} {:08b}", tile_data[0], tile_data[1]);
    }
    
    fs::write("debug_solid_4x4.j2k", &output).unwrap();
    
    let pgm_header = format!("P5\n{} {}\n255\n", width, height);
    let mut pgm_data = Vec::new();
    pgm_data.extend_from_slice(pgm_header.as_bytes());
    pgm_data.extend_from_slice(&pixels);
    fs::write("debug_solid_4x4.pgm", &pgm_data).unwrap();
    
    let output = std::process::Command::new("libs/bin/opj_compress.exe")
        .args(&["-i", "debug_solid_4x4.pgm", "-o", "debug_solid_4x4_opj.j2k", "-n", "2"])
        .output()
        .expect("Failed to run opj_compress");
    
    if output.status.success() {
        let opj_data = fs::read("debug_solid_4x4_opj.j2k").unwrap();
        println!("\nOpenJPEG encoded {} bytes", opj_data.len());
        
        for i in 0..opj_data.len() - 1 {
            if opj_data[i] == 0xFF && opj_data[i + 1] == 0x93 {
                let tile_data = &opj_data[i + 2..];
                println!("OpenJPEG SOD at offset {}", i);
                println!("OpenJPEG tile data ({} bytes): {:02X?}", tile_data.len(), tile_data);
                println!("OpenJPEG binary: {:08b}", tile_data[0]);
                break;
            }
        }
    } else {
        println!("OpenJPEG failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    let _ = fs::remove_file("debug_solid_4x4.pgm");
    let _ = fs::remove_file("debug_solid_4x4.j2k");
    let _ = fs::remove_file("debug_solid_4x4_opj.j2k");
}

#[test] 
fn debug_packet_header() {
    use jpegexp_rs::jpeg2000::bit_plane_coder::BitPlaneCoder;
    
    let data = vec![0i32; 16];
    let mut bpc = BitPlaneCoder::new(4, 4, &data);
    
    let max_bp = bpc.calculate_max_bit_plane();
    println!("max_bp: {:?}", max_bp);
    
    bpc.mq.init_encoder();
    
    if let Some(bp) = max_bp {
        let passes = bpc.encode_codeblock(bp, 0, 0);
        println!("Encoded {} passes", passes);
    } else {
        println!("All zeros - no significant bits");
    }
    
    bpc.mq.flush();
    let buf = bpc.mq.get_buffer();
    println!("Buffer: {:02X?}", buf);
}

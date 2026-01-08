use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use std::fs;

fn main() {
    let data = fs::read("libs/openhtj2k_src/test_openhtj2k_2x2_black.j2c").expect("Failed to read file");
    
    println!("File size: {} bytes", data.len());
    println!("First 40 bytes (hex): {}", hex::encode(&data[..40.min(data.len())]));
    
    let mut reader = JpegStreamReader::new(&data);
    let mut decoder = J2kDecoder::new(&mut reader);
    
    match decoder.decode() {
        Ok(image) => {
            println!("\nDecode successful!");
            println!("Image: {}x{}", image.width, image.height);
            println!("Components: {}", image.component_count);
            
            if let Ok(pixels) = image.reconstruct_pixels() {
                println!("Pixel count: {}", pixels.len());
                println!("Pixels: {:?}", pixels);
                
                // Calculate expected vs actual
                let expected = vec![0u8; 4];
                let mae: f64 = pixels.iter().zip(expected.iter())
                    .map(|(&a, &b)| (a as i32 - b as i32).abs() as f64)
                    .sum::<f64>() / pixels.len() as f64;
                println!("MAE: {}", mae);
            }
        }
        Err(e) => {
            eprintln!("Decode failed: {:?}", e);
        }
    }
}

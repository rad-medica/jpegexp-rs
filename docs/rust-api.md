# Rust API Reference

The jpegexp-rs library provides native Rust APIs for encoding and decoding JPEG images.

## Crate

```toml
[dependencies]
jpegexp-rs = "0.1.0"
```

## Modules

- `jpegexp_rs::jpegls` - JPEG-LS encoder/decoder
- `jpegexp_rs::jpeg1` - JPEG 1 encoder/decoder
- `jpegexp_rs::jpeg2000` - JPEG 2000/HTJ2K decoder

## JPEG-LS

### Decoding

```rust
use jpegexp_rs::jpegls::JpeglsDecoder;

fn decode_jpegls(data: &[u8]) -> Result<Vec<u8>, jpegexp_rs::JpeglsError> {
    let mut decoder = JpeglsDecoder::new(data);
    decoder.read_header()?;

    let info = decoder.frame_info();
    println!("Image: {}x{}", info.width, info.height);

    let size = (info.width * info.height * info.component_count as u32) as usize;
    let mut pixels = vec![0u8; size];
    decoder.decode(&mut pixels)?;

    Ok(pixels)
}
```

### Encoding

```rust
use jpegexp_rs::jpegls::JpeglsEncoder;
use jpegexp_rs::FrameInfo;

fn encode_jpegls(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, jpegexp_rs::JpeglsError> {
    let mut output = vec![0u8; pixels.len() * 2];
    let mut encoder = JpeglsEncoder::new(&mut output);

    encoder.set_frame_info(FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    })?;

    let len = encoder.encode(pixels)?;
    output.truncate(len);

    Ok(output)
}
```

## JPEG 1

### Decoding

```rust
use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;

fn decode_jpeg(data: &[u8]) -> Result<Vec<u8>, jpegexp_rs::JpeglsError> {
    let mut decoder = Jpeg1Decoder::new(data);
    decoder.read_header()?;

    let info = decoder.frame_info();
    let size = (info.width * info.height * info.component_count as u32) as usize;
    let mut pixels = vec![0u8; size];
    decoder.decode(&mut pixels)?;

    Ok(pixels)
}
```

### Encoding

```rust
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::FrameInfo;

fn encode_jpeg(pixels: &[u8], width: u32, height: u32) -> Result<Vec<u8>, jpegexp_rs::JpeglsError> {
    let mut output = vec![0u8; pixels.len() * 2];
    let mut encoder = Jpeg1Encoder::new();

    let frame_info = FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    };

    let len = encoder.encode(pixels, &frame_info, &mut output)?;
    output.truncate(len);

    Ok(output)
}
```

## JPEG 2000

### Decoding

```rust
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;

fn decode_j2k(data: &[u8]) -> Result<(), jpegexp_rs::JpeglsError> {
    let mut reader = JpegStreamReader::new(data);
    let mut decoder = J2kDecoder::new(&mut reader);

    let image = decoder.decode()?;

    println!("Image: {}x{}", image.width, image.height);
    println!("Components: {}", image.component_count);
    println!("Tile size: {}x{}", image.tile_width, image.tile_height);

    if let Some(cod) = &image.cod {
        println!("DWT levels: {}", cod.decomposition_levels);
        println!("Quality layers: {}", cod.number_of_layers);
    }

    if let Some(cap) = &image.cap {
        let is_htj2k = (cap.pcap & (1 << 14)) != 0;
        println!("HTJ2K: {}", is_htj2k);
    }

    if image.icc_profile.is_some() {
        println!("ICC Profile: present");
    }

    Ok(())
}
```

## Complete Example

```rust
use jpegexp_rs::jpegls::{JpeglsDecoder, JpeglsEncoder};
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::FrameInfo;

fn transcode_to_jpeg(input: &[u8]) -> Result<Vec<u8>, jpegexp_rs::JpeglsError> {
    // Decode JPEG-LS
    let mut decoder = JpeglsDecoder::new(input);
    decoder.read_header()?;
    let info = decoder.frame_info();

    let pixel_count = (info.width * info.height * info.component_count as u32) as usize;
    let mut pixels = vec![0u8; pixel_count];
    decoder.decode(&mut pixels)?;

    // Encode to JPEG
    let mut output = vec![0u8; pixel_count * 2];
    let mut encoder = Jpeg1Encoder::new();

    let len = encoder.encode(&pixels, &info, &mut output)?;
    output.truncate(len);

    Ok(output)
}

fn main() {
    let data = std::fs::read("input.jls").expect("Failed to read file");
    let jpeg = transcode_to_jpeg(&data).expect("Failed to transcode");
    std::fs::write("output.jpg", &jpeg).expect("Failed to write file");
    println!("Transcoded {} bytes -> {} bytes", data.len(), jpeg.len());
}
```

## Error Handling

All operations return `Result<T, JpeglsError>`:

```rust
use jpegexp_rs::JpeglsError;

match decoder.decode(&mut pixels) {
    Ok(()) => println!("Success"),
    Err(JpeglsError::InvalidData) => eprintln!("Corrupt data"),
    Err(JpeglsError::ParameterValueNotSupported) => eprintln!("Unsupported format"),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

## Thread Safety

Decoders and encoders are not `Send` or `Sync`. Create new instances per thread for parallel processing.

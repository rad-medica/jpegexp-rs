# jpegexp-rs

**jpegexp-rs** is a high-performance, pure Rust "Universal JPEG" library. It provides native support for several JPEG standards, starting with a comprehensive port of the CharLS JPEG-LS implementation.

## Project Vision
The goal of `jpegexp-rs` is to become a single, safe, and efficient library for all common JPEG variants used in medical imaging (DICOM), geospatial data, and professional photography.

### Roadmap
- **Phase 1: JPEG-LS** (ISO/IEC 14495-1) - **Completed**
- **Phase 2: JPEG 1** (ISO/IEC 10918-1) - **Completed**
  - Baseline DCT support.
  - Huffman coding (Encoder/Decoder).
  - DQT/DHT/SOF0/SOS support.
- **Phase 3: JPEG 2000** (ISO/IEC 15444-1) - **In Progress**
  - **Tier-1 Coding**: MQ Coder, Context Modeling (Bitplane Coding) - **Completed**.
  - **Tier-2 Coding**: Tag Trees, Packet Header Parsing/Writing - **Completed**.
  - **Codestream**: Parsing and Writing (SOC, SIZ, COD, QCD, SOT, SOD, EOC) - **Completed**.
  - **Wavelet Transform**: DWT/IDWT 5-3 (Lossless) and 9-7 (Lossy) - **In Progress**.
  - **Quantization**: Scalar quantization - **Implemented**.
- **Phase 4: HTJ2K** (High Throughput JPEG 2000) - **Planned**

## Core Features
- **Pure Rust**: Zero unsafe code in the core logic, ensuring memory safety and easier cross-compilation (including WASM).
- **JPEG-LS Support**: 
  - Lossless and near-lossless compression.
  - Support for 2-bit to 16-bit bit-depths.
  - SPIFF (Still Picture Interchange File Format) support.
  - Ported from the industry-standard CharLS implementation.
- **Generic Pixel API**: Uses Rust traits to handle `u8` and `u16` samples without code duplication.

## Getting Started

### Installation
Add `jpegexp-rs` to your `Cargo.toml`:

```toml
[dependencies]
jpegexp-rs = "0.1.0"
```

### Basic Decoding
```rust
use jpegexp_rs::decoder::JpeglsDecoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_data = std::fs::read("image.jls")?;
    let mut decoder = JpeglsDecoder::new(&source_data);
    
    // Read header information
    decoder.read_header()?;
    let info = decoder.frame_info();
    println!("Image: {}x{}, {} bits", info.width, info.height, info.bits_per_sample);
    
    // Decode pixels
    let mut destination = vec![0u8; (info.width * info.height * (info.bits_per_sample / 8).max(1) as u32) as usize];
    decoder.decode(&mut destination)?;
    
    Ok(())
}
```

### Basic Encoding
```rust
use jpegexp_rs::encoder::JpeglsEncoder;
use jpegexp_rs::FrameInfo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 512;
    let height = 512;
    let pixels = vec![0u8; (width * height) as usize];
    
    let mut destination = vec![0u8; (width * height * 2) as usize]; // Buffer for compressed data
    let mut encoder = JpeglsEncoder::new(&mut destination);
    
    encoder.set_frame_info(FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: 1,
    })?;
    
    let bytes_written = encoder.encode(&pixels)?;
    println!("Compressed to {} bytes", bytes_written);
    
    Ok(())
}
```

## Architecture
The library is divided into several logical layers:
1. **Public API (`encoder.rs`, `decoder.rs`)**: High-level interfaces for reading/writing JPEG files.
2. **Stream Management (`jpeg_stream_reader.rs`, `jpeg_stream_writer.rs`)**: Handles marker segments and byte-level I/O.
3. **Scan Logic (`scan_encoder.rs`, `scan_decoder.rs`)**: Implements the actual JPEG-LS predictive coding and entropy coding.
4. **Contexts (`regular_mode_context.rs`, `run_mode_context.rs`)**: Maintains probability models for the encoder/decoder.

## License
MIT OR Apache-2.0

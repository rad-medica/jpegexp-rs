# jpegexp-rs

**jpegexp-rs** is a high-performance, pure Rust "Universal JPEG" library. It provides native support for several JPEG standards, starting with a comprehensive port of the CharLS JPEG-LS implementation.

## Project Vision
The goal of `jpegexp-rs` is to become a single, safe, and efficient library for all common JPEG variants used in medical imaging (DICOM), geospatial data, and professional photography.

### Roadmap
- **Phase 1: JPEG-LS** (ISO/IEC 14495-1) - **Completed**
- **Phase 2: JPEG 1** (ISO/IEC 10918-1) - **Completed**
  - Baseline DCT support (8-bit, interleaved/non-interleaved)
  - **Progressive Mode** support (spectral selection + successive approximation)
  - **Lossless Mode (Process 14)** with predictors 1-7
  - Huffman coding, DQT/DHT/SOF0/SOF2/SOF3/SOS, Restart Markers
- **Phase 3: JPEG 2000** (ISO/IEC 15444-1) - **Completed**
  - Tier-1 Coding: MQ Coder, Context Modeling (Bitplane Coding)
  - Tier-2 Coding: Tag Trees, Packet Header Parsing/Writing
  - Codestream: SOC, SIZ, COD, QCD, SOT, SOD, EOC
  - Wavelet Transform: DWT/IDWT 5-3 and 9-7
  - **JP2 Container Support** (box parsing, codestream extraction)
  - **LRCP/RPCL Progression Orders**
- **Phase 4: HTJ2K** (ISO/IEC 15444-15) - **Completed**
  - High-Throughput block coder (Cleanup, SigProp, MagRef passes)
  - CAP marker parsing and HT coder selection

## Core Features
- **Pure Rust**: Zero unsafe code in the core logic, ensuring memory safety.
- **JPEG-LS Support**: Lossless and near-lossless, 2-16 bit depths, SPIFF headers.
- **JPEG 1 Support**: Baseline, Progressive, and Lossless modes.
- **JPEG 2000/HTJ2K Support**: Full codestream parsing, DWT, and block coding.
- **Generic Pixel API**: Uses Rust traits to handle `u8` and `u16` samples.

## Getting Started

### Installation
```toml
[dependencies]
jpegexp-rs = "0.1.0"
```

### JPEG-LS Decoding
```rust
use jpegexp_rs::jpegls::JpeglsDecoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_data = std::fs::read("image.jls")?;
    let mut decoder = JpeglsDecoder::new(&source_data);
    decoder.read_header()?;
    let info = decoder.frame_info();
    let mut destination = vec![0u8; (info.width * info.height) as usize];
    decoder.decode(&mut destination)?;
    Ok(())
}
```

### JPEG 1 Decoding (Baseline/Progressive)
```rust
use jpegexp_rs::jpeg1::Jpeg1Decoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_data = std::fs::read("image.jpg")?;
    let mut decoder = Jpeg1Decoder::new(&source_data);
    decoder.read_header()?;
    // Decode to RGB buffer...
    Ok(())
}
```

### HTJ2K Decoding
```rust
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_data = std::fs::read("image.jph")?;  // or .jp2
    let mut reader = JpegStreamReader::new(&source_data);
    let mut decoder = J2kDecoder::new(&mut reader);
    let image = decoder.decode()?;
    println!("Image: {}x{}", image.width, image.height);
    Ok(())
}
```

## Architecture
The library is divided into several logical layers:
1. **Public API**: High-level interfaces for each JPEG standard.
2. **Stream Management**: Handles marker segments and byte-level I/O.
3. **Transform Logic**: DCT/IDCT, DWT/IDWT, quantization.
4. **Entropy Coding**: Huffman, MQ, Golomb-Rice, HT block coder.

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed module descriptions.

## License
MIT OR Apache-2.0

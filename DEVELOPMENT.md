# Development Guide

## Getting Started

### Prerequisites
- Rust 1.70+ (Rust 2024 edition)
- Python 3.11+ (for testing)
- Git

### Quick Setup

**Option 1: GitHub Codespaces (Recommended)**
1. Click the Codespaces badge in README.md
2. Wait for environment to build
3. Start coding!

**Option 2: Local Development with DevContainer**
1. Install VS Code and Docker
2. Install "Dev Containers" extension
3. Open repository in VS Code
4. Click "Reopen in Container"

**Option 3: Manual Setup**
```bash
# Clone repository
git clone https://github.com/rad-medica/jpegexp-rs.git
cd jpegexp-rs

# Install Rust components
rustup component add rustfmt clippy rust-src

# Install Python dependencies for testing
pip install numpy pillow imagecodecs

# Build project
cargo build --release

# Run tests
cargo test
```

## Project Structure

```
jpegexp-rs/
├── .devcontainer/          # DevContainer configuration for Codespaces
├── src/
│   ├── bin/
│   │   └── jpegexp.rs      # CLI application
│   ├── jpeg1/              # JPEG 1 (baseline) codec
│   │   ├── decoder.rs      # ✅ Production ready (with RGB subsampling)
│   │   ├── encoder.rs      # ✅ Production ready
│   │   ├── huffman.rs      # Huffman coding
│   │   └── dct.rs          # DCT/IDCT implementation
│   ├── jpegls/             # JPEG-LS codec
│   │   ├── decoder.rs      # ⚠️ Partial implementation
│   │   ├── encoder.rs      # ⚠️ Has bugs
│   │   ├── scan_decoder.rs # ⚠️ Needs architectural fixes
│   │   └── scan_encoder.rs # ⚠️ Needs architectural fixes
│   ├── jpeg2000/           # JPEG 2000 codec
│   │   ├── decoder.rs      # ⚠️ Stub implementation
│   │   ├── encoder.rs      # ⚠️ Stub implementation
│   │   └── dwt.rs          # DWT implementation
│   └── lib.rs              # Library root
├── tests/
│   └── comprehensive_test.py  # Codec comparison tests
├── CODEC_TEST_RESULTS.md   # Detailed test results
├── SUMMARY.md              # Project summary
└── README.md               # Main documentation
```

## Building and Testing

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Watch for changes and rebuild
cargo watch -x build
```

### Testing

```bash
# Run Rust unit tests
cargo test

# Run integration tests
cargo test --test '*'

# Run comprehensive codec tests (Python)
python3 tests/comprehensive_test.py

# Run specific test
cargo test test_name
```

### Linting and Formatting

```bash
# Run clippy (linter)
cargo clippy

# Fix clippy warnings automatically
cargo clippy --fix

# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check
```

### Running the CLI

```bash
# Show help
cargo run --bin jpegexp -- help

# Decode image
cargo run --release --bin jpegexp -- decode -i input.jpg -o output.raw -f raw

# Encode image
cargo run --release --bin jpegexp -- encode -i input.raw -o output.jpg -w 512 -H 512 -n 3 -c jpeg1
```

## Codec Status and Development Priorities

### ✅ JPEG 1 (Production Ready)
**Status**: Fully functional for production use

**Capabilities**:
- Grayscale decoding/encoding: MAE < 1.0
- RGB with chroma subsampling (4:2:0, 4:2:2, 4:4:4)
- Progressive JPEG support
- Baseline and extended sequential

**Recent Improvements**:
- Fixed RGB decoder for chroma subsampling (commit 07189c9)
- Proper MCU dimension calculation
- Component upsampling support

**Known Issues**:
- Some edge cases with certain RGB images (low priority)

**Development Priority**: ⭐ Maintenance only

### ⚠️ JPEG-LS (Requires Major Work)
**Status**: Partial implementation, not production ready

**Issues Identified**:
1. Decoder outputs corrupted data (max diff 255)
2. Encoder produces invalid bitstreams
3. Bitstream management problems
4. Byte-stuffing (0xFF handling) bugs
5. Buffer layout mismatch between encoder/decoder

**What Works**:
- Basic structure and framework
- JPEG-LS header parsing
- Context modeling (partial)

**What Doesn't Work**:
- Roundtrip encoding/decoding
- Standard library interoperability
- Run mode encoding/decoding

**Development Priority**: ⭐⭐⭐ High (2-3 weeks estimated)

**Recommended Approach**:
1. Study reference implementation (CharLS)
2. Rewrite scan_encoder.rs bitstream logic
3. Rewrite scan_decoder.rs bitstream logic
4. Add comprehensive unit tests
5. Validate against standard test images

### ⚠️ JPEG 2000 (Stub Implementation)
**Status**: Proof-of-concept only

**Issues**:
- Encoder doesn't use pixel data (parameter unused)
- Encoder only writes empty packets
- Decoder fails reconstruction
- Falls back to constant value (all 128)

**Development Priority**: ⭐⭐ Medium (4-8 weeks estimated)

**Recommended Approach**:
1. Complete DWT implementation
2. Implement bit-plane coding
3. Implement MQ/HT coder
4. Complete packet formation
5. Extensive testing

## Contributing

### Code Style
- Follow Rust standard naming conventions
- Use `rustfmt` for formatting
- Address all `clippy` warnings
- Add documentation comments for public APIs

### Commit Messages
```
<type>: <short description>

<detailed description>

<references>
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

Example:
```
fix: Add proper RGB subsampling support to JPEG 1 decoder

- Calculate MCU dimensions based on max sampling factors
- Allocate correct buffer sizes per component
- Implement component upsampling during reconstruction

Fixes #123
```

### Pull Request Process
1. Create a feature branch
2. Make changes with tests
3. Ensure all tests pass
4. Run `cargo clippy` and `cargo fmt`
5. Update documentation
6. Submit PR with clear description

## Debugging

### Using LLDB/GDB
```bash
# With VS Code
# Set breakpoints in code, press F5

# Command line
rust-lldb target/debug/jpegexp
```

### Logging
```rust
// Add to code for debugging
eprintln!("Debug: value = {:?}", value);

// With RUST_BACKTRACE
RUST_BACKTRACE=1 cargo run
```

### Profiling
```bash
# Build with profiling
cargo build --release --features profiling

# Use perf (Linux)
perf record target/release/jpegexp decode -i test.jpg -o out.raw
perf report
```

## Resources

### JPEG Standards
- JPEG 1: ISO/IEC 10918-1 ([ITU-T T.81](https://www.itu.int/rec/T-REC-T.81))
- JPEG-LS: ISO/IEC 14495-1 ([ITU-T T.87](https://www.itu.int/rec/T-REC-T.87))
- JPEG 2000: ISO/IEC 15444-1 ([ITU-T T.800](https://www.itu.int/rec/T-REC-T.800))

### Reference Implementations
- JPEG 1: [libjpeg-turbo](https://github.com/libjpeg-turbo/libjpeg-turbo) (v3.0+)
- JPEG-LS: [CharLS](https://github.com/team-charls/charls) (v2.4+)
- JPEG 2000: [OpenJPEG](https://github.com/uclouvain/openjpeg) (v2.5+)

### Learning Resources
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [JPEG Specification](https://www.w3.org/Graphics/JPEG/)

## FAQ

**Q: Why is JPEG-LS not working?**  
A: The JPEG-LS implementation has fundamental architectural issues that require a complete rewrite of the encoder/decoder core logic. This is documented in CODEC_TEST_RESULTS.md.

**Q: Can I use JPEG 2000?**  
A: Not yet. The current implementation is a stub that only writes/reads headers. The actual wavelet transform and bit-plane coding are not implemented.

**Q: Which codec should I use for production?**  
A: Use JPEG 1 for both grayscale and RGB images. It's fully tested and production-ready.

**Q: How can I help?**  
A: Check the GitHub issues for "good first issue" labels, or tackle the JPEG-LS refactoring if you're experienced with bitstream coding.

## Support

- GitHub Issues: https://github.com/rad-medica/jpegexp-rs/issues
- Documentation: See CODEC_TEST_RESULTS.md and SUMMARY.md

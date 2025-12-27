# CLI Reference

The `jpegexp` command-line utility provides full access to the jpegexp-rs codec library for encoding, decoding, and transcoding JPEG images.

## Installation

```bash
cargo install --path .
```

Or run directly:

```bash
cargo run --bin jpegexp -- <command>
```

## Commands

### decode

Decode a JPEG image to raw pixels or PPM format.

```bash
jpegexp decode -i <input> -o <output> [-f <format>]
```

**Options:**

- `-i, --input <file>` - Input JPEG file (JPEG, JPEG-LS, J2K, JP2, HTJ2K)
- `-o, --output <file>` - Output file path
- `-f, --format <format>` - Output format: `raw` (default) or `ppm`

**Examples:**

```bash
# Decode JPEG to raw pixels
jpegexp decode -i photo.jpg -o pixels.raw

# Decode JPEG 2000 to PPM
jpegexp decode -i medical.j2k -o image.ppm -f ppm

# Decode JPEG-LS
jpegexp decode -i scan.jls -o output.raw
```

### encode

Encode raw pixels to a JPEG format.

```bash
jpegexp encode -i <input> -o <output> -w <width> -H <height> [-n <components>] [-c <codec>]
```

**Options:**

- `-i, --input <file>` - Input raw pixel file
- `-o, --output <file>` - Output JPEG file
- `-w, --width <pixels>` - Image width
- `-H, --height <pixels>` - Image height
- `-n, --components <count>` - Number of components (1=grayscale, 3=RGB), default: 1
- `-c, --codec <codec>` - Target codec: `jpeg`, `jpegls`, default: jpeg
- `-q, --quality <1-100>` - Quality level (lossy codecs only), default: 85

**Examples:**

```bash
# Encode grayscale to JPEG
jpegexp encode -i pixels.raw -o output.jpg -w 512 -H 512

# Encode to JPEG-LS (lossless)
jpegexp encode -i pixels.raw -o output.jls -w 1024 -H 1024 -c jpegls

# Encode RGB to JPEG
jpegexp encode -i rgb_pixels.raw -o photo.jpg -w 800 -H 600 -n 3
```

### transcode

Convert between JPEG formats.

```bash
jpegexp transcode -i <input> -o <output> -c <codec>
```

**Options:**

- `-i, --input <file>` - Input JPEG file
- `-o, --output <file>` - Output JPEG file
- `-c, --codec <codec>` - Target codec: `jpeg`, `jpegls`

**Examples:**

```bash
# Convert JPEG to JPEG-LS
jpegexp transcode -i photo.jpg -o photo.jls -c jpegls

# Convert JPEG-LS to JPEG
jpegexp transcode -i lossless.jls -o compressed.jpg -c jpeg
```

### info

Display image metadata.

```bash
jpegexp info -i <input> [-e]
```

**Options:**

- `-i, --input <file>` - Input file to inspect
- `-e, --extended` - Show extended metadata

**Examples:**

```bash
# Basic info
jpegexp info -i image.jpg

# Extended info for JPEG 2000
jpegexp info -i medical.j2k -e
```

**Sample Output:**

```
File: "image.j2k"
Size: 45678 bytes

Format: JPEG 2000 Codestream
  Dimensions: 512x512
  Components: 1
  Tile size:  512x512
  DWT levels: 5
  Layers:     1
  Progression: LRCP
  HTJ2K:      No
```

### list

List supported codecs and capabilities.

```bash
jpegexp list
```

**Output:**

```
Supported Codecs:

  JPEG (jpeg)
    Standard: ISO/IEC 10918-1 / ITU-T T.81
    Modes:    Baseline DCT, Progressive, Lossless (Process 14)
    Encode:   ✓  Decode: ✓

  JPEG-LS (jpegls)
    Standard: ISO/IEC 14495-1 / ITU-T T.87
    Modes:    Lossless, Near-Lossless
    Encode:   ✓  Decode: ✓

  JPEG 2000 (j2k)
    Standard: ISO/IEC 15444-1
    Features: DWT, EBCOT, Quality Layers, ROI, ICC Profiles
    Encode:   ✗  Decode: ✓

  HTJ2K (htj2k)
    Standard: ISO/IEC 15444-15
    Features: High-Throughput block coding (10x+ faster)
    Encode:   ✗  Decode: ✓
```

## Exit Codes

- `0` - Success
- `1` - Error (invalid input, unsupported format, I/O error)

## Environment

No environment variables are required. All options are passed via command-line arguments.

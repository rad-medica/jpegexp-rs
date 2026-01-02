# CLI Reference

The `jpegexp` command-line utility provides full access to the jpegexp-rs codec library for encoding, decoding, and transcoding JPEG images.

## Codec Support Status

| Codec | Encode | Decode | Quality |
|-------|--------|--------|---------|
| JPEG | ✓ | ✓ | Production ready |
| JPEG-LS (grayscale) | ✓ | ✓ | Lossless (MAE=0) |
| JPEG-LS (RGB) | ✗ | ✗ | Not yet supported |
| JPEG 2000 | ✗ | ⚠️ | Stub implementation |

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

Decode a JPEG image to raw pixels or standard image formats.

```bash
jpegexp decode [OPTIONS]
```

**Options:**

- `-i, --input <INPUT>` - Path to input file
- `-o, --output <OUTPUT>` - Path for the decoded output file
- `-f, --format <FORMAT>` - Output format (raw, ppm, png, jpg) [default: raw]
- `-h, --help` - Print help

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
jpegexp encode [OPTIONS] --input <INPUT> --output <OUTPUT> --width <WIDTH> --height <HEIGHT>
```

**Options:**

- `-i, --input <INPUT>` - Path to raw pixel data file
- `-o, --output <OUTPUT>` - Path for the encoded output file
- `-w, --width <WIDTH>` - Image width in pixels
- `-H, --height <HEIGHT>` - Image height in pixels
- `-n, --components <COMPONENTS>` - Number of color components (1=grayscale, 3=RGB) [default: 1]
- `-c, --codec <CODEC>` - Target codec for encoding (jpeg, jpegls, j2k, htj2k) [default: jpeg]
- `-q, --quality <QUALITY>` - Quality level (1-100, only for lossy codecs) [default: 85]
- `--near-lossless <NEAR_LOSSLESS>` - Enable near-lossless mode for JPEG-LS (0=lossless, 1-255=near-lossless) [default: 0]
- `-h, --help` - Print help

**Examples:**

```bash
# Encode grayscale to JPEG
jpegexp encode -i pixels.raw -o output.jpg -w 512 -H 512

# Encode to JPEG-LS (lossless grayscale)
jpegexp encode -i pixels.raw -o output.jls -w 1024 -H 1024 -c jpegls

# Encode 16-bit grayscale to JPEG-LS
jpegexp encode -i pixels16.raw -o output.jls -w 512 -H 512 -c jpegls --bits 16

# Encode RGB to JPEG
jpegexp encode -i rgb_pixels.raw -o photo.jpg -w 800 -H 600 -n 3
```

**Note:** JPEG-LS currently only supports grayscale images (1 component). 
RGB encoding with `-n 3` is not yet supported for JPEG-LS.

### transcode

Transcode between JPEG formats.

```bash
jpegexp transcode [OPTIONS] --input <INPUT> --output <OUTPUT> --codec <CODEC>
```

**Options:**

- `-i, --input <INPUT>` - Path to input file
- `-o, --output <OUTPUT>` - Path for the transcoded output file
- `-c, --codec <CODEC>` - Target codec for transcoding (jpeg, jpegls, j2k, htj2k)
- `-q, --quality <QUALITY>` - Quality level (1-100, only for lossy codecs) [default: 85]
- `-h, --help` - Print help

**Examples:**

```bash
# Convert JPEG to JPEG-LS
jpegexp transcode -i photo.jpg -o photo.jls -c jpegls

# Convert JPEG-LS to JPEG
jpegexp transcode -i lossless.jls -o compressed.jpg -c jpeg
```

### info

Display image metadata and codec information.

```bash
jpegexp info [OPTIONS] --input <INPUT>
```

**Options:**

- `-i, --input <INPUT>` - Path to input file
- `-h, --help` - Print help

**Examples:**

```bash
# Basic info
jpegexp info -i image.jpg
```

### list

List supported codecs and their capabilities.

```bash
jpegexp list
```

## Exit Codes

- `0` - Success
- `1` - Error (invalid input, unsupported format, I/O error)

## Environment

No environment variables are required. All options are passed via command-line arguments.

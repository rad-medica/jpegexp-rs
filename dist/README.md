# jpegexp CLI Distribution

Self-contained CLI executable for jpegexp - Universal JPEG Codec.

## Available Platforms

| Platform | Architecture          | Filename                      |
| -------- | --------------------- | ----------------------------- |
| Windows  | x86_64                | `jpegexp-windows-x86_64.zip`  |
| Linux    | x86_64                | `jpegexp-linux-x86_64.tar.gz` |
| Linux    | ARM64                 | `jpegexp-linux-arm64.tar.gz`  |
| macOS    | x86_64 (Intel)        | `jpegexp-macos-x86_64.tar.gz` |
| macOS    | ARM64 (Apple Silicon) | `jpegexp-macos-arm64.tar.gz`  |

## Download

Get the latest release from:
https://github.com/rad-medica/jpegexp-rs/releases

## Usage

```bash
# Windows
.\jpegexp.exe --help

# Linux / macOS
chmod +x jpegexp
./jpegexp --help
```

## Commands

### Decode

```bash
jpegexp decode -i image.jpg -o pixels.raw
jpegexp decode -i image.j2k -o image.ppm -f ppm
```

### Encode

```bash
jpegexp encode -i pixels.raw -o image.jpg -w 512 -H 512 -c jpeg
jpegexp encode -i pixels.raw -o image.jls -w 512 -H 512 -c jpegls
jpegexp encode -i rgb_pixels.raw -o photo.jpg -w 800 -H 600 -n 3
```

### Transcode

```bash
jpegexp transcode -i image.jpg -o image.jls -c jpegls
```

### Info

```bash
jpegexp info -i image.j2k
jpegexp info -i image.jpg -e  # Extended info
```

### List Codecs

```bash
jpegexp list
```

## Supported Formats

| Format                                 | Encode | Decode |
| -------------------------------------- | ------ | ------ |
| JPEG (Baseline, Progressive, Lossless) | ✓      | ✓      |
| JPEG-LS (Lossless, Near-Lossless)      | ✓      | ✓      |
| JPEG 2000                              | ✗      | ✓      |
| HTJ2K                                  | ✗      | ✓      |

## License

MIT License - © 2024 Rad Medica

## Source

https://github.com/rad-medica/jpegexp-rs

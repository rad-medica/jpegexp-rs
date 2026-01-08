# Python API Reference

The jpegexp Python module provides bindings to the jpegexp-rs codec library.

## Installation

```bash
cd python
maturin develop  # For development
maturin build    # For wheel distribution
pip install .
```

## Module

```python
import jpegexp
```

## Functions

### decode

Decode any supported JPEG format to raw pixels.

```python
def decode(data: bytes) -> bytes
```

**Parameters:**

- `data` - JPEG file bytes

**Returns:** Raw pixel data

**Example:**

```python
import jpegexp

with open("image.jpg", "rb") as f:
    data = f.read()

pixels = jpegexp.decode(data)
print(f"Decoded {len(pixels)} bytes")
```

### decode_file

Decode a JPEG file directly from path.

```python
def decode_file(path: str) -> bytes
```

**Example:**

```python
pixels = jpegexp.decode_file("image.jpg")
```

### get_info

Get image information without full decode.

```python
def get_info(data: bytes) -> ImageInfo
```

**Returns:** `ImageInfo` object with:

- `width: int`
- `height: int`
- `components: int`
- `bits_per_sample: int`
- `format: str` - "jpeg", "jpeg-progressive", "jpeg-lossless", "jpegls", "j2k", or "htj2k"

**Example:**

```python
with open("medical.j2k", "rb") as f:
    data = f.read()

info = jpegexp.get_info(data)
print(f"Image: {info.width}x{info.height}")
print(f"Components: {info.components}")
print(f"Format: {info.format}")
```

### encode_jpeg

Encode raw pixels to JPEG.

```python
def encode_jpeg(pixels: bytes, width: int, height: int, components: int) -> bytes
```

**Example:**

```python
# Create grayscale gradient
width, height = 256, 256
pixels = bytes([x for y in range(height) for x in range(width)])

# Encode to JPEG
jpeg_data = jpegexp.encode_jpeg(pixels, width, height, 1)

with open("output.jpg", "wb") as f:
    f.write(jpeg_data)
```

### encode_jpegls

Encode raw pixels to JPEG-LS (lossless).

```python
def encode_jpegls(pixels: bytes, width: int, height: int, components: int) -> bytes
```

**Example:**

```python
# Lossless encode
jls_data = jpegexp.encode_jpegls(pixels, width, height, 1)

with open("output.jls", "wb") as f:
    f.write(jls_data)
```

### transcode

Transcode between JPEG formats.

```python
def transcode(data: bytes, target: str) -> bytes
```

**Parameters:**

- `data` - Input JPEG bytes
- `target` - Target format: "jpeg" or "jpegls"

**Example:**

```python
# Convert JPEG to JPEG-LS
with open("photo.jpg", "rb") as f:
    jpeg_data = f.read()

jls_data = jpegexp.transcode(jpeg_data, "jpegls")

with open("photo.jls", "wb") as f:
    f.write(jls_data)
```

## Complete Example

```python
import jpegexp

def process_medical_image(input_path: str, output_path: str):
    """Convert any JPEG format to lossless JPEG-LS."""

    # Read input
    with open(input_path, "rb") as f:
        data = f.read()

    # Get info
    info = jpegexp.get_info(data)
    print(f"Input: {info.width}x{info.height} {info.format}")
    print(f"  Components: {info.components}")
    print(f"  Bits: {info.bits_per_sample}")

    # Transcode to lossless
    lossless = jpegexp.transcode(data, "jpegls")

    # Write output
    with open(output_path, "wb") as f:
        f.write(lossless)

    print(f"Output: {len(lossless)} bytes (lossless)")

    return info

# Usage
info = process_medical_image("scan.jpg", "scan.jls")
```

## NumPy Integration

```python
import jpegexp
import numpy as np

def decode_to_numpy(path: str) -> np.ndarray:
    """Decode JPEG to NumPy array."""
    with open(path, "rb") as f:
        data = f.read()

    info = jpegexp.get_info(data)
    pixels = jpegexp.decode(data)

    if info.components == 1:
        return np.frombuffer(pixels, dtype=np.uint8).reshape(
            (info.height, info.width)
        )
    else:
        return np.frombuffer(pixels, dtype=np.uint8).reshape(
            (info.height, info.width, info.components)
        )

def encode_from_numpy(arr: np.ndarray, format: str = "jpeg") -> bytes:
    """Encode NumPy array to JPEG."""
    if arr.ndim == 2:
        height, width = arr.shape
        components = 1
    else:
        height, width, components = arr.shape

    pixels = arr.astype(np.uint8).tobytes()

    if format == "jpegls":
        return jpegexp.encode_jpegls(pixels, width, height, components)
    else:
        return jpegexp.encode_jpeg(pixels, width, height, components)

# Usage
img = decode_to_numpy("photo.jpg")
print(f"Shape: {img.shape}")

# Modify and re-encode
img = img * 0.5  # Darken
jpeg_data = encode_from_numpy(img.astype(np.uint8))
```

## Error Handling

All functions raise `ValueError` on error:

```python
try:
    pixels = jpegexp.decode(invalid_data)
except ValueError as e:
    print(f"Decode failed: {e}")
```

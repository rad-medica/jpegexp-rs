# C API Reference

The jpegexp-rs C API provides a foreign function interface for using the codec from C/C++ projects.

## Building

Build the library as a C dynamic library:

```bash
cargo build --release
```

Generate the C header:

```bash
cbindgen --output jpegexp.h
```

## Header

```c
#include "jpegexp.h"
```

## Types

### JpegExpDecoder

Opaque decoder handle. Created with `jpegexp_decoder_new()` and freed with `jpegexp_decoder_free()`.

### JpegExpImageInfo

```c
typedef struct {
    uint32_t width;
    uint32_t height;
    uint32_t components;
    uint32_t bits_per_sample;
} JpegExpImageInfo;
```

### JpegExpError

```c
typedef enum {
    JPEGEXP_OK = 0,
    JPEGEXP_INVALID_DATA = 1,
    JPEGEXP_BUFFER_TOO_SMALL = 2,
    JPEGEXP_UNSUPPORTED_FORMAT = 3,
    JPEGEXP_INTERNAL_ERROR = 4,
} JpegExpError;
```

## Functions

### Decoder API

#### jpegexp_decoder_new

```c
JpegExpDecoder* jpegexp_decoder_new(const uint8_t* data, size_t len);
```

Create a new decoder from raw JPEG data.

**Parameters:**

- `data` - Pointer to JPEG file bytes
- `len` - Length of data in bytes

**Returns:** Decoder handle, or `NULL` on error.

#### jpegexp_decoder_free

```c
void jpegexp_decoder_free(JpegExpDecoder* decoder);
```

Free a decoder handle.

#### jpegexp_decoder_read_header

```c
int jpegexp_decoder_read_header(JpegExpDecoder* decoder, JpegExpImageInfo* info);
```

Read the image header and populate image info.

**Returns:** `JPEGEXP_OK` on success.

#### jpegexp_decoder_decode

```c
int jpegexp_decoder_decode(JpegExpDecoder* decoder, uint8_t* output, size_t output_len);
```

Decode the image to raw pixels.

**Parameters:**

- `output` - Buffer for decoded pixels
- `output_len` - Size of output buffer (must be at least width _ height _ components)

**Returns:** `JPEGEXP_OK` on success.

### Encoder API

#### jpegexp_encode_jpeg

```c
int jpegexp_encode_jpeg(
    const uint8_t* pixels,
    uint32_t width,
    uint32_t height,
    uint32_t components,
    uint8_t* output,
    size_t output_len,
    size_t* bytes_written
);
```

Encode raw pixels to JPEG.

#### jpegexp_encode_jpegls

```c
int jpegexp_encode_jpegls(
    const uint8_t* pixels,
    uint32_t width,
    uint32_t height,
    uint32_t components,
    uint8_t* output,
    size_t output_len,
    size_t* bytes_written
);
```

Encode raw pixels to JPEG-LS.

## Example

```c
#include <stdio.h>
#include <stdlib.h>
#include "jpegexp.h"

int main() {
    // Read JPEG file
    FILE* f = fopen("image.jpg", "rb");
    fseek(f, 0, SEEK_END);
    size_t len = ftell(f);
    fseek(f, 0, SEEK_SET);
    uint8_t* data = malloc(len);
    fread(data, 1, len, f);
    fclose(f);

    // Create decoder
    JpegExpDecoder* decoder = jpegexp_decoder_new(data, len);
    if (!decoder) {
        fprintf(stderr, "Failed to create decoder\n");
        return 1;
    }

    // Read header
    JpegExpImageInfo info;
    if (jpegexp_decoder_read_header(decoder, &info) != JPEGEXP_OK) {
        fprintf(stderr, "Failed to read header\n");
        jpegexp_decoder_free(decoder);
        return 1;
    }

    printf("Image: %dx%d, %d components\n",
           info.width, info.height, info.components);

    // Allocate output buffer
    size_t pixel_count = info.width * info.height * info.components;
    uint8_t* pixels = malloc(pixel_count);

    // Decode
    if (jpegexp_decoder_decode(decoder, pixels, pixel_count) != JPEGEXP_OK) {
        fprintf(stderr, "Failed to decode\n");
        jpegexp_decoder_free(decoder);
        return 1;
    }

    // Use pixels...
    printf("Decoded %zu bytes\n", pixel_count);

    // Cleanup
    free(pixels);
    free(data);
    jpegexp_decoder_free(decoder);

    return 0;
}
```

## Linking

Link against the generated `.so` or `.dll`:

```bash
gcc -o myapp myapp.c -L./target/release -ljpegexp_rs
```

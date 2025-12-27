# WebAssembly / JavaScript API Reference

The jpegexp WASM module provides JavaScript bindings for use in browsers and Node.js.

## Building

```bash
wasm-pack build --target web
```

This generates:

- `pkg/jpegexp.js` - JavaScript wrapper
- `pkg/jpegexp_bg.wasm` - WebAssembly binary
- `pkg/jpegexp.d.ts` - TypeScript definitions

## Usage

### Browser (ES Modules)

```html
<script type="module">
  import init, {
    decode_jpeg,
    encode_jpeg,
    get_image_info,
  } from "./pkg/jpegexp.js";

  async function main() {
    await init();

    // Load image
    const response = await fetch("image.jpg");
    const data = new Uint8Array(await response.arrayBuffer());

    // Get info
    const info = get_image_info(data);
    console.log(`Image: ${info.width}x${info.height}`);

    // Decode
    const pixels = decode_jpeg(data);
    console.log(`Decoded ${pixels.length} bytes`);
  }

  main();
</script>
```

### Node.js

```javascript
const { readFileSync } = require("fs");
const init = require("./pkg/jpegexp.js");

async function main() {
  const wasm = await init();

  const data = new Uint8Array(readFileSync("image.jpg"));
  const pixels = wasm.decode_jpeg(data);
  console.log(`Decoded ${pixels.length} bytes`);
}

main();
```

## Types

### ImageInfo

```typescript
interface ImageInfo {
  width: number;
  height: number;
  components: number;
  bits_per_sample: number;
}
```

## Functions

### get_image_info

Get image metadata without decoding.

```typescript
function get_image_info(data: Uint8Array): ImageInfo;
```

**Example:**

```javascript
const info = get_image_info(jpegData);
console.log(`${info.width}x${info.height}, ${info.components} components`);
```

### decode_jpeg

Decode a JPEG 1 image to raw pixels.

```typescript
function decode_jpeg(data: Uint8Array): Uint8Array;
```

### decode_jpegls

Decode a JPEG-LS image to raw pixels.

```typescript
function decode_jpegls(data: Uint8Array): Uint8Array;
```

### encode_jpeg

Encode raw pixels to JPEG.

```typescript
function encode_jpeg(
  pixels: Uint8Array,
  width: number,
  height: number,
  components: number
): Uint8Array;
```

### encode_jpegls

Encode raw pixels to JPEG-LS.

```typescript
function encode_jpegls(
  pixels: Uint8Array,
  width: number,
  height: number,
  components: number
): Uint8Array;
```

### transcode_to_jpegls

Transcode JPEG to lossless JPEG-LS.

```typescript
function transcode_to_jpegls(data: Uint8Array): Uint8Array;
```

## Complete Example

### Image Viewer

```html
<!DOCTYPE html>
<html>
  <head>
    <title>jpegexp Image Viewer</title>
  </head>
  <body>
    <input type="file" id="fileInput" accept=".jpg,.jpeg,.jls,.j2k,.jp2" />
    <canvas id="canvas"></canvas>
    <div id="info"></div>

    <script type="module">
      import init, { decode_jpeg, get_image_info } from "./pkg/jpegexp.js";

      await init();

      const canvas = document.getElementById("canvas");
      const ctx = canvas.getContext("2d");
      const infoDiv = document.getElementById("info");

      document
        .getElementById("fileInput")
        .addEventListener("change", async (e) => {
          const file = e.target.files[0];
          const arrayBuffer = await file.arrayBuffer();
          const data = new Uint8Array(arrayBuffer);

          try {
            // Get info
            const info = get_image_info(data);
            infoDiv.textContent = `${info.width}x${info.height}, ${info.components} components`;

            // Decode
            const pixels = decode_jpeg(data);

            // Display on canvas
            canvas.width = info.width;
            canvas.height = info.height;

            const imageData = ctx.createImageData(info.width, info.height);

            if (info.components === 1) {
              // Grayscale to RGBA
              for (let i = 0; i < pixels.length; i++) {
                imageData.data[i * 4] = pixels[i];
                imageData.data[i * 4 + 1] = pixels[i];
                imageData.data[i * 4 + 2] = pixels[i];
                imageData.data[i * 4 + 3] = 255;
              }
            } else {
              // RGB to RGBA
              for (let i = 0; i < pixels.length / 3; i++) {
                imageData.data[i * 4] = pixels[i * 3];
                imageData.data[i * 4 + 1] = pixels[i * 3 + 1];
                imageData.data[i * 4 + 2] = pixels[i * 3 + 2];
                imageData.data[i * 4 + 3] = 255;
              }
            }

            ctx.putImageData(imageData, 0, 0);
          } catch (e) {
            infoDiv.textContent = `Error: ${e}`;
          }
        });
    </script>
  </body>
</html>
```

### Batch Transcoding

```javascript
import init, { transcode_to_jpegls, get_image_info } from "./pkg/jpegexp.js";

await init();

async function processImages(files) {
  const results = [];

  for (const file of files) {
    const data = new Uint8Array(await file.arrayBuffer());
    const info = get_image_info(data);

    console.log(`Processing: ${file.name} (${info.width}x${info.height})`);

    const lossless = transcode_to_jpegls(data);

    results.push({
      name: file.name.replace(/\.\w+$/, ".jls"),
      data: lossless,
      originalSize: data.length,
      losslessSize: lossless.length,
    });
  }

  return results;
}
```

## Bundle Size

The WASM module is approximately 200KB gzipped, suitable for web applications.

## Browser Support

- Chrome 57+
- Firefox 52+
- Safari 11+
- Edge 16+

Requires WebAssembly support.

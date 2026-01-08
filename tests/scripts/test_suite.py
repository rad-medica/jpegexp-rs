import jpegexp
import numpy as np
import os
import time


def test_roundtrip(name, width, height, components, depth, lossless=True):
    print(f"\n--- Test: {name} ({width}x{height}, {components} comps, {depth}-bit) ---")

    # Generate Synthetic Image
    # Match Rust logic:
    # Grayscale: (x+y)*scale
    # Color: R=x*scale, G=y*scale, B=(x+y)*scale

    max_val = (1 << depth) - 1
    dtype = np.uint8 if depth <= 8 else np.uint16

    img = np.zeros((height, width, components), dtype=dtype)

    for y in range(height):
        for x in range(width):
            if components == 1:
                val = int((x + y) * max_val / (width + height))
                img[y, x, 0] = val
            else:
                r = int((x * max_val) / width)
                g = int((y * max_val) / height)
                b = int(((x + y) * max_val) / (width + height))
                if components == 3:
                    img[y, x] = [r, g, b]
                # Add more components if needed

    # Flatten for API
    raw_bytes = img.tobytes()

    # Encode
    t0 = time.time()
    try:
        # encode_j2k(pixels, width, height, components, quality, bits_per_sample, lossless)
        # We need to pass None for quality to ensure lossless default or use explicit lossless arg
        encoded = jpegexp.encode_j2k(
            raw_bytes, width, height, components, None, depth, lossless
        )
    except Exception as e:
        print(f"Encoding FAILED: {e}")
        return False
    t1 = time.time()

    print(f"Encoded size: {len(encoded)} bytes")
    print(f"Ratio: {len(raw_bytes) / len(encoded):.2f}:1")
    print(f"Encode time: {(t1 - t0) * 1000:.2f} ms")

    # Decode
    t0 = time.time()
    try:
        decoded_bytes = jpegexp.decode(encoded)
    except Exception as e:
        print(f"Decoding FAILED: {e}")
        return False
    t1 = time.time()

    print(f"Decode time: {(t1 - t0) * 1000:.2f} ms")

    if len(decoded_bytes) != len(raw_bytes):
        print(f"Size Mismatch! Orig: {len(raw_bytes)}, Decoded: {len(decoded_bytes)}")
        return False

    # Verify
    decoded_img = np.frombuffer(decoded_bytes, dtype=dtype).reshape(
        (height, width, components)
    )

    diff = np.abs(img.astype(np.int32) - decoded_img.astype(np.int32))
    max_diff = diff.max()
    mae = diff.mean()
    mismatches = np.count_nonzero(diff)

    print(f"Max Diff: {max_diff}")
    print(f"MAE: {mae:.4f}")
    print(f"Mismatches: {mismatches} / {width * height * components}")

    if max_diff == 0:
        print("RESULT: PASS (Perfect Lossless)")
        return True
    else:
        print("RESULT: FAIL (Artifacts detected)")
        return False


if __name__ == "__main__":
    print("Running Python Test Suite for jpegexp-rs")

    # 1. 8-bit Grayscale
    test_roundtrip("Grayscale 8-bit", 256, 256, 1, 8)

    # 2. 12-bit Grayscale (Large)
    test_roundtrip("Grayscale 12-bit", 64, 64, 1, 12)

    # 3. 12-bit Color (Small)
    test_roundtrip("Color 12-bit Small", 4, 4, 3, 12)

    # 4. 12-bit Color (Large) - Known Issue
    print(
        "\n(Note: The following test is expected to fail due to known issue with large signed blocks)"
    )
    test_roundtrip("Color 12-bit Large", 64, 64, 3, 12)

    # 5. JPEG-LS Grayscale
    # (Using encode_jpegls)
    # ... logic similar to above but calling encode_jpegls

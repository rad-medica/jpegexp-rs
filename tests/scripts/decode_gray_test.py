#!/usr/bin/env python3
"""Decode a grayscale JPEG-LS file with CharLS to check reference values."""

import numpy as np

try:
    from charlsrle import decode_jls
except ImportError:
    print("ERROR: charlsrle not installed. Install with: pip install charlsrle")
    exit(1)


def decode_and_show(jls_path):
    """Decode JPEG-LS file and print pixel values."""
    try:
        with open(jls_path, "rb") as f:
            jls_data = f.read()

        # Decode with CharLS
        image = decode_jls(jls_data)

        print(f"File: {jls_path}")
        print(f"Shape: {image.shape}")
        print(f"Dtype: {image.dtype}")
        print(f"Min: {image.min()}, Max: {image.max()}")
        print("\nFirst 8x8 pixels:")
        if len(image.shape) == 2:
            # Grayscale
            for y in range(min(8, image.shape[0])):
                row = image[y, : min(8, image.shape[1])]
                print(f"Row {y}: {row.tolist()}")
        else:
            # RGB or other
            for y in range(min(8, image.shape[0])):
                row = image[y, : min(8, image.shape[1]), :]
                print(f"Row {y}: {row.tolist()}")

        return image

    except Exception as e:
        print(f"ERROR: {e}")
        return None


if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1:
        jls_path = sys.argv[1]
    else:
        jls_path = "tests/fixtures/jpegls/tiny_8x8_gray_gradient.jls"

    decode_and_show(jls_path)

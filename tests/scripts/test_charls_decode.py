#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Test if CharLS can decode the RGB checker file"""

import imagecodecs
import numpy as np
import sys

# Force UTF-8 output on Windows
if sys.platform == "win32":
    sys.stdout.reconfigure(encoding="utf-8")

jls_file = "tests/fixtures/jpegls/small_16x16_rgb_checker.jls"
raw_file = "tests/fixtures/jpegls/small_16x16_rgb_checker.raw"

# Try to decode with imagecodecs (uses CharLS)
try:
    with open(jls_file, "rb") as f:
        jls_data = f.read()

    decoded = imagecodecs.jpegls_decode(jls_data)
    print("OK: CharLS decoded successfully!")
    print(f"  Shape: {decoded.shape}")
    print(f"  Dtype: {decoded.dtype}")
    print(f"  First pixel: {decoded[0, 0, :]}")
    print(f"  Pixel (0,1): {decoded[0, 1, :]}")

    # Load expected
    with open(raw_file, "rb") as f:
        raw_data = np.frombuffer(f.read(), dtype=np.uint8)
    expected = raw_data.reshape(16, 16, 3)

    # Compare
    if np.array_equal(decoded, expected):
        print("OK: Perfect match with expected data!")
    else:
        diff = np.sum(decoded != expected)
        print(f"ERROR: {diff} mismatches found")

except Exception as e:
    print(f"ERROR: CharLS decode failed: {e}")
    import traceback

    traceback.print_exc()

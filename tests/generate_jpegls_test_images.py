#!/usr/bin/env python3
"""
Generate comprehensive JPEG-LS test images using CharLS (via imagecodecs)
for validating jpegexp-rs decoder implementation.
"""

import os
import numpy as np
import imagecodecs
from pathlib import Path

# Create test images directory
TEST_DIR = Path("tests/jpegls_test_images")
TEST_DIR.mkdir(parents=True, exist_ok=True)

def generate_test_image(name, width, height, channels, bit_depth=8, pattern="gradient"):
    """Generate a test image with specified parameters."""
    print(f"Generating {name}: {width}x{height}x{channels} @ {bit_depth}bit, pattern={pattern}")
    
    if bit_depth == 8:
        dtype = np.uint8
        max_val = 255
    elif bit_depth == 16:
        dtype = np.uint16
        max_val = 65535
    else:
        raise ValueError(f"Unsupported bit depth: {bit_depth}")
    
    if pattern == "gradient":
        if channels == 1:
            # Grayscale gradient
            data = np.linspace(0, max_val, width * height, dtype=dtype).reshape((height, width))
        elif channels == 3:
            # RGB gradients
            data = np.zeros((height, width, 3), dtype=dtype)
            data[:, :, 0] = np.linspace(0, max_val, width * height, dtype=dtype).reshape((height, width))  # R
            data[:, :, 1] = np.linspace(0, max_val, height * width, dtype=dtype).reshape((height, width)).T  # G (transposed)
            data[:, :, 2] = max_val // 2  # B (constant middle value)
    elif pattern == "noise":
        if channels == 1:
            data = np.random.randint(0, max_val + 1, (height, width), dtype=dtype)
        elif channels == 3:
            data = np.random.randint(0, max_val + 1, (height, width, 3), dtype=dtype)
    elif pattern == "checker":
        if channels == 1:
            data = np.zeros((height, width), dtype=dtype)
            data[::2, ::2] = max_val
            data[1::2, 1::2] = max_val
        elif channels == 3:
            data = np.zeros((height, width, 3), dtype=dtype)
            data[::2, ::2, :] = max_val
            data[1::2, 1::2, :] = max_val
    elif pattern == "solid":
        if channels == 1:
            data = np.full((height, width), max_val // 2, dtype=dtype)
        elif channels == 3:
            data = np.full((height, width, 3), max_val // 2, dtype=dtype)
    else:
        raise ValueError(f"Unknown pattern: {pattern}")
    
    return data

def save_test_case(data, name):
    """Save test image as raw data and JPEG-LS (CharLS encoded)."""
    # Save raw data
    raw_path = TEST_DIR / f"{name}.raw"
    data.tofile(raw_path)
    print(f"  Saved raw: {raw_path}")
    
    # Encode with CharLS (via imagecodecs)
    try:
        jls_data = imagecodecs.jpegls_encode(data)
        jls_path = TEST_DIR / f"{name}.jls"
        with open(jls_path, 'wb') as f:
            f.write(jls_data)
        print(f"  Saved JLS: {jls_path} ({len(jls_data)} bytes)")
        
        # Verify CharLS can decode it
        decoded = imagecodecs.jpegls_decode(jls_data)
        if np.array_equal(data, decoded):
            print(f"  ✓ CharLS roundtrip: PASS")
        else:
            print(f"  ✗ CharLS roundtrip: FAIL (max diff: {np.max(np.abs(data.astype(int) - decoded.astype(int)))})")
        
        # Save metadata
        meta_path = TEST_DIR / f"{name}.txt"
        with open(meta_path, 'w') as f:
            f.write(f"Name: {name}\n")
            f.write(f"Shape: {data.shape}\n")
            f.write(f"Dtype: {data.dtype}\n")
            f.write(f"Min: {np.min(data)}\n")
            f.write(f"Max: {np.max(data)}\n")
            f.write(f"Mean: {np.mean(data):.2f}\n")
            f.write(f"JLS size: {len(jls_data)} bytes\n")
            f.write(f"Raw size: {data.nbytes} bytes\n")
            f.write(f"Compression ratio: {data.nbytes / len(jls_data):.2f}x\n")
        
        return True
    except Exception as e:
        print(f"  ✗ CharLS encode failed: {e}")
        return False

def main():
    """Generate all test images."""
    print("=" * 60)
    print("JPEG-LS Test Image Generator")
    print("Using CharLS via imagecodecs")
    print("=" * 60)
    print()
    
    test_cases = [
        # Small images - for quick testing
        ("tiny_8x8_gray_gradient", 8, 8, 1, 8, "gradient"),
        ("tiny_8x8_gray_noise", 8, 8, 1, 8, "noise"),
        ("tiny_8x8_gray_checker", 8, 8, 1, 8, "checker"),
        ("tiny_8x8_gray_solid", 8, 8, 1, 8, "solid"),
        
        # Various sizes - grayscale
        ("small_16x16_gray_gradient", 16, 16, 1, 8, "gradient"),
        ("small_32x32_gray_gradient", 32, 32, 1, 8, "gradient"),
        ("medium_64x64_gray_gradient", 64, 64, 1, 8, "gradient"),
        ("medium_128x128_gray_gradient", 128, 128, 1, 8, "gradient"),
        ("large_256x256_gray_gradient", 256, 256, 1, 8, "gradient"),
        
        # Non-square dimensions
        ("rect_16x32_gray_gradient", 16, 32, 1, 8, "gradient"),
        ("rect_32x16_gray_gradient", 32, 16, 1, 8, "gradient"),
        
        # RGB images
        ("tiny_8x8_rgb_gradient", 8, 8, 3, 8, "gradient"),
        ("small_16x16_rgb_gradient", 16, 16, 3, 8, "gradient"),
        ("small_32x32_rgb_gradient", 32, 32, 3, 8, "gradient"),
        ("medium_64x64_rgb_gradient", 64, 64, 3, 8, "gradient"),
        
        # RGB with different patterns
        ("small_16x16_rgb_noise", 16, 16, 3, 8, "noise"),
        ("small_16x16_rgb_checker", 16, 16, 3, 8, "checker"),
        
        # 16-bit images
        ("small_16x16_gray16_gradient", 16, 16, 1, 16, "gradient"),
        ("small_32x32_gray16_gradient", 32, 32, 1, 16, "gradient"),
        
        # Edge cases
        ("edge_1x1_gray", 1, 1, 1, 8, "solid"),
        ("edge_1x8_gray", 1, 8, 1, 8, "gradient"),
        ("edge_8x1_gray", 8, 1, 1, 8, "gradient"),
    ]
    
    success_count = 0
    fail_count = 0
    
    for test_case in test_cases:
        name, width, height, channels, bit_depth, pattern = test_case
        try:
            data = generate_test_image(name, width, height, channels, bit_depth, pattern)
            if save_test_case(data, name):
                success_count += 1
            else:
                fail_count += 1
        except Exception as e:
            print(f"  ✗ Failed to generate {name}: {e}")
            fail_count += 1
        print()
    
    print("=" * 60)
    print(f"Test image generation complete")
    print(f"Success: {success_count}, Failed: {fail_count}")
    print(f"Images saved to: {TEST_DIR.absolute()}")
    print("=" * 60)
    
    # Create a summary file
    summary_path = TEST_DIR / "README.md"
    with open(summary_path, 'w') as f:
        f.write("# JPEG-LS Test Images\n\n")
        f.write("Generated using CharLS (via imagecodecs) for jpegexp-rs validation.\n\n")
        f.write(f"Total test cases: {success_count}\n\n")
        f.write("## Test Cases\n\n")
        f.write("| Name | Size | Channels | Bit Depth | Pattern |\n")
        f.write("|------|------|----------|-----------|----------|\n")
        for test_case in test_cases:
            name, width, height, channels, bit_depth, pattern = test_case
            f.write(f"| {name} | {width}x{height} | {channels} | {bit_depth} | {pattern} |\n")
        f.write("\n## Files\n\n")
        f.write("Each test case includes:\n")
        f.write("- `.raw` - Raw pixel data\n")
        f.write("- `.jls` - JPEG-LS encoded (CharLS)\n")
        f.write("- `.txt` - Metadata and compression info\n")

if __name__ == "__main__":
    main()

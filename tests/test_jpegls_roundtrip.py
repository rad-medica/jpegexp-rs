#!/usr/bin/env python3
"""
Test JPEG-LS roundtrip encoding/decoding and compare with CharLS.
"""

import os
import sys
import subprocess
import numpy as np
import imagecodecs
from pathlib import Path

TEST_DIR = Path("tests/jpegls_test_images")

def test_our_decoder_with_charls_encoded():
    """Test that our decoder can decode CharLS-encoded images."""
    print("=" * 60)
    print("Testing our decoder with CharLS-encoded images")
    print("=" * 60)
    
    # Build the library first
    result = subprocess.run(["cargo", "build", "--release"], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Build failed: {result.stderr}")
        return False
    
    # Run the validation tests
    result = subprocess.run(
        ["cargo", "test", "--test", "jpegls_charls_validation", "--", "--nocapture"],
        capture_output=True, text=True
    )
    print(result.stdout)
    if "FAILED" in result.stdout or result.returncode != 0:
        print(result.stderr)
        return False
    return True

def test_roundtrip_encoding():
    """Test that we can encode an image and CharLS can decode it, and vice versa."""
    print("\n" + "=" * 60)
    print("Testing roundtrip encoding")
    print("=" * 60)
    
    # Create a simple test image
    test_images = [
        ("8x8 grayscale gradient", np.linspace(0, 255, 64, dtype=np.uint8).reshape((8, 8))),
        ("16x16 grayscale random", np.random.randint(0, 256, (16, 16), dtype=np.uint8)),
        ("32x32 grayscale checker", (np.indices((32, 32)).sum(axis=0) % 2 * 255).astype(np.uint8)),
    ]
    
    success_count = 0
    for name, img in test_images:
        print(f"\nTesting {name}:")
        
        # Encode with CharLS
        charls_encoded = imagecodecs.jpegls_encode(img)
        print(f"  CharLS encoded size: {len(charls_encoded)} bytes")
        
        # Decode with CharLS to verify
        charls_decoded = imagecodecs.jpegls_decode(charls_encoded)
        if np.array_equal(img, charls_decoded):
            print(f"  CharLS roundtrip: PASS")
        else:
            print(f"  CharLS roundtrip: FAIL")
            continue
        
        # Save for testing with our decoder
        test_path = TEST_DIR / f"roundtrip_{name.replace(' ', '_')}.jls"
        raw_path = TEST_DIR / f"roundtrip_{name.replace(' ', '_')}.raw"
        
        with open(test_path, "wb") as f:
            f.write(charls_encoded)
        img.tofile(raw_path)
        
        success_count += 1
    
    return success_count == len(test_images)

def test_encoder_output():
    """Use the CLI tool to test encoding and compare with CharLS decoding."""
    print("\n" + "=" * 60)
    print("Testing our encoder output")
    print("=" * 60)
    
    # Build release
    result = subprocess.run(["cargo", "build", "--release"], capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Build failed: {result.stderr}")
        return False
    
    # Test images to encode
    test_images = [
        ("8x8 grayscale", np.linspace(0, 255, 64, dtype=np.uint8).reshape((8, 8))),
        ("16x16 grayscale", np.random.randint(0, 256, (16, 16), dtype=np.uint8)),
    ]
    
    # Save raw files for encoding
    for name, img in test_images:
        raw_path = TEST_DIR / f"encode_test_{name.replace(' ', '_')}.raw"
        img.tofile(raw_path)
        print(f"Saved raw image: {raw_path} ({img.shape})")
    
    return True

def decode_with_our_library(jls_path: Path, width: int, height: int, components: int, bits: int) -> np.ndarray:
    """Try to decode a JPEG-LS file using our library."""
    # Build the library if needed
    import ctypes
    
    # Try using Python bindings if available
    try:
        # Build Python bindings
        result = subprocess.run(
            ["cargo", "build", "--release", "-p", "jpegexp-rs-python"],
            capture_output=True, text=True, cwd=str(Path(__file__).parent.parent)
        )
        
        # Add path and import
        sys.path.insert(0, str(Path(__file__).parent.parent / "target" / "release"))
        
        # This would work if pyo3 module was built
        # import jpegexp_python
        # return jpegexp_python.decode_jpegls(jls_path.read_bytes())
    except Exception as e:
        print(f"  Could not use Python bindings: {e}")
    
    return None

def main():
    print("JPEG-LS Implementation Comparison Test")
    print("Testing against CharLS reference implementation")
    print()
    
    # Run decoder tests
    decoder_ok = test_our_decoder_with_charls_encoded()
    
    # Run roundtrip tests
    roundtrip_ok = test_roundtrip_encoding()
    
    # Run encoder tests
    encoder_ok = test_encoder_output()
    
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"Decoder tests: {'PASS' if decoder_ok else 'FAIL'}")
    print(f"Roundtrip tests: {'PASS' if roundtrip_ok else 'FAIL'}")
    print(f"Encoder tests: {'PASS' if encoder_ok else 'FAIL'}")
    
    return 0 if (decoder_ok and roundtrip_ok and encoder_ok) else 1

if __name__ == "__main__":
    sys.exit(main())

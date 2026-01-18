import numpy as np
import imagecodecs
import os

def create_ref_j2k():
    width = 4
    height = 4
    # Create 4x4 image, pixel 0,0=0, others=128
    img = np.full((height, width), 128, dtype=np.uint8)
    img[0, 0] = 0

    # Encode using OpenJPEG
    # Using reversible (lossless) compression
    # resolutions=2 means 1 decomposition level
    encoded = imagecodecs.jpeg2k_encode(img, resolutions=2, reversible=True, codecformat='j2k')

    with open("test_4x4_ref.j2k", "wb") as f:
        f.write(encoded)

    print(f"Generated test_4x4_ref.j2k: {len(encoded)} bytes")

    # Also save raw bytes for Rust to use as input
    with open("test_4x4_input.raw", "wb") as f:
        f.write(img.tobytes())

if __name__ == "__main__":
    create_ref_j2k()

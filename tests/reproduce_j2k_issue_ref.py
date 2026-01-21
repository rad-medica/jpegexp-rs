import numpy as np
import imagecodecs
import os

def create_ref_4x4():
    img = np.full((4, 4), 128, dtype=np.uint8)
    img[0, 0] = 0

    # Encode using OpenJPEG
    try:
        encoded = imagecodecs.jpeg2k_encode(img, resolutions=2, reversible=True, codecformat='j2k')
        with open("test_4x4_ref.j2k", "wb") as f:
            f.write(encoded)
        with open("test_4x4_input.raw", "wb") as f:
            f.write(img.tobytes())
        print(f"Generated test_4x4_ref.j2k: {len(encoded)} bytes")
    except Exception as e:
        print(f"Failed to encode: {e}")
        exit(1)

if __name__ == "__main__":
    create_ref_4x4()

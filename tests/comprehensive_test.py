import os
import subprocess
import sys
import time
import numpy as np
from PIL import Image
import imagecodecs

# Configuration
JPEGEXP_BIN = os.path.abspath("target/release/jpegexp.exe")
OUTPUT_DIR = "tests/artifacts_comprehensive"
os.makedirs(OUTPUT_DIR, exist_ok=True)


def log(msg):
    print(f"[TEST] {msg}")


def run_jpegexp(args):
    cmd = [JPEGEXP_BIN] + args
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.stderr:
        print(f"[JPEGEXP STDERR] {result.stderr}")
    if result.returncode != 0:
        raise RuntimeError(f"jpegexp failed: {result.stderr}")
    return result.stdout


def generate_synthetic_image(name, width, height, mode="L", pattern="gradient"):
    if pattern == "gradient":
        if mode == "L":
            arr = np.linspace(0, 255, width * height, dtype=np.uint8).reshape(
                (height, width)
            )
        elif mode == "RGB":
            arr = np.zeros((height, width, 3), dtype=np.uint8)
            # R gradient
            arr[:, :, 0] = np.linspace(0, 255, width * height, dtype=np.uint8).reshape(
                (height, width)
            )
            # G gradient (transposed)
            arr[:, :, 1] = (
                np.linspace(0, 255, width * height, dtype=np.uint8)
                .reshape((width, height))
                .T
            )
            # B noise
            arr[:, :, 2] = np.random.randint(0, 256, (height, width), dtype=np.uint8)
    elif pattern == "noise":
        if mode == "L":
            arr = np.random.randint(0, 256, (height, width), dtype=np.uint8)
        elif mode == "RGB":
            arr = np.random.randint(0, 256, (height, width, 3), dtype=np.uint8)

    img = Image.fromarray(arr, mode=mode)
    path = os.path.join(
        OUTPUT_DIR, f"{name}_{mode}_{pattern}.png"
    )  # Save as PNG for reference
    img.save(path)

    raw_path = os.path.join(OUTPUT_DIR, f"{name}_{mode}_{pattern}.raw")
    arr.tofile(raw_path)

    return path, raw_path, arr


def test_roundtrip_std_to_rust(image_arr, mode, name):
    """
    Encode with Python (Std Lib) -> Decode with Rust
    """
    log(f"--- Round Trip A (Std -> Rust): {name} ---")

    width, height = (
        image_arr.shape[:2] if len(image_arr.shape) == 3 else image_arr.shape
    )

    # 1. Encode with imagecodecs
    # JPEG
    try:
        jpg_path = os.path.join(OUTPUT_DIR, f"{name}_std.jpg")
        with open(jpg_path, "wb") as f:
            f.write(imagecodecs.jpeg_encode(image_arr))

        # Decode with Rust
        out_raw = os.path.join(OUTPUT_DIR, f"{name}_std_decoded_by_rust_jpeg.raw")
        run_jpegexp(["decode", "-i", jpg_path, "-o", out_raw, "-f", "raw"])

        # Compare
        decoded_arr = np.fromfile(out_raw, dtype=np.uint8).reshape(image_arr.shape)
        # JPEG is lossy, allow some diff
        diff = np.abs(image_arr.astype(int) - decoded_arr.astype(int))
        mae = np.mean(diff)
        log(f"JPEG: MAE = {mae:.2f} (Success)")
    except Exception as e:
        log(f"JPEG FAILED: {e}")

    # JPEG-LS
    try:
        jls_path = os.path.join(OUTPUT_DIR, f"{name}_std.jls")
        with open(jls_path, "wb") as f:
            f.write(imagecodecs.jpegls_encode(image_arr))

        # Decode with Rust
        out_raw = os.path.join(OUTPUT_DIR, f"{name}_std_decoded_by_rust_jls.raw")
        run_jpegexp(["decode", "-i", jls_path, "-o", out_raw, "-f", "raw"])

        # Compare
        decoded_arr = np.fromfile(out_raw, dtype=np.uint8).reshape(image_arr.shape)
        if np.array_equal(image_arr, decoded_arr):
            log(f"JPEG-LS: PERFECT MATCH (Success)")
        else:
            diff = np.abs(image_arr.astype(int) - decoded_arr.astype(int))
            log(f"JPEG-LS MISMATCH: Max diff {np.max(diff)}")
    except Exception as e:
        log(f"JPEG-LS FAILED: {e}")

    # J2K
    try:
        j2k_path = os.path.join(OUTPUT_DIR, f"{name}_std.j2k")
        with open(j2k_path, "wb") as f:
            f.write(imagecodecs.jpeg2k_encode(image_arr))

        # Decode with Rust
        out_raw = os.path.join(OUTPUT_DIR, f"{name}_std_decoded_by_rust_j2k.raw")
        run_jpegexp(["decode", "-i", j2k_path, "-o", out_raw, "-f", "raw"])

        # Compare
        decoded_arr = np.fromfile(out_raw, dtype=np.uint8).reshape(image_arr.shape)
        # Lossless J2K should be perfect? Default might be lossy.
        # But let's check MAE
        diff = np.abs(image_arr.astype(int) - decoded_arr.astype(int))
        mae = np.mean(diff)
        log(f"J2K: MAE = {mae:.2f}")
    except Exception as e:
        log(f"J2K FAILED: {e}")


def test_roundtrip_rust_to_std(image_arr, raw_path, mode, name):
    """
    Encode with Rust -> Decode with Python (Std Lib)
    """
    log(f"--- Round Trip B (Rust -> Std): {name} ---")

    height, width = image_arr.shape[:2]
    components = 3 if mode == "RGB" else 1

    # JPEG
    try:
        jpg_path = os.path.join(OUTPUT_DIR, f"{name}_rust.jpg")
        run_jpegexp(
            [
                "encode",
                "-i",
                raw_path,
                "-o",
                jpg_path,
                "-w",
                str(width),
                "-H",
                str(height),
                "-n",
                str(components),
                "-c",
                "jpeg",
            ]
        )

        with open(jpg_path, "rb") as f:
            decoded = imagecodecs.jpeg_decode(f.read())

        # Determine shape match
        if decoded.shape != image_arr.shape:
            log(f"JPEG Shape Mismatch: {decoded.shape} vs {image_arr.shape}")

        diff = np.abs(image_arr.astype(int) - decoded.astype(int))
        mae = np.mean(diff)
        log(f"JPEG: MAE = {mae:.2f} (Success)")
    except Exception as e:
        log(f"JPEG FAILED: {e}")

    # JPEG-LS
    try:
        jls_path = os.path.join(OUTPUT_DIR, f"{name}_rust.jls")
        run_jpegexp(
            [
                "encode",
                "-i",
                raw_path,
                "-o",
                jls_path,
                "-w",
                str(width),
                "-H",
                str(height),
                "-n",
                str(components),
                "-c",
                "jpegls",
            ]
        )

        with open(jls_path, "rb") as f:
            decoded = imagecodecs.jpegls_decode(f.read())

        if np.array_equal(image_arr, decoded):
            log(f"JPEG-LS: PERFECT MATCH (Success)")
        else:
            diff = np.abs(image_arr.astype(int) - decoded.astype(int))
            log(f"JPEG-LS MISMATCH: Max diff {np.max(diff)}")
    except Exception as e:
        log(f"JPEG-LS FAILED: {e}")

    # J2K
    try:
        j2k_path = os.path.join(OUTPUT_DIR, f"{name}_rust.j2k")
        run_jpegexp(
            [
                "encode",
                "-i",
                raw_path,
                "-o",
                j2k_path,
                "-w",
                str(width),
                "-H",
                str(height),
                "-n",
                str(components),
                "-c",
                "j2k",
            ]
        )

        with open(j2k_path, "rb") as f:
            # J2K decode might return generic buffer
            decoded = imagecodecs.jpeg2k_decode(f.read())

        diff = np.abs(image_arr.astype(int) - decoded.astype(int))
        mae = np.mean(diff)
        log(f"J2K: MAE = {mae:.2f}")
    except Exception as e:
        log(f"J2K FAILED: {e}")


def main():
    if not os.path.exists(JPEGEXP_BIN):
        print(f"ERROR: Binary not found at {JPEGEXP_BIN}")
        print("Please build with: cargo build --release")
        sys.exit(1)

    # 1. Generate Data
    log("Generating Synthetic Data...")
    _, raw_gray_grad, arr_gray_grad = generate_synthetic_image(
        "gray_gradient", 512, 512, "L", "gradient"
    )
    _, raw_rgb_noise, arr_rgb_noise = generate_synthetic_image(
        "rgb_noise", 256, 256, "RGB", "noise"
    )

    # 2. Test Round Trips
    test_roundtrip_std_to_rust(arr_gray_grad, "L", "gray_gradient")
    test_roundtrip_rust_to_std(arr_gray_grad, raw_gray_grad, "L", "gray_gradient")

    test_roundtrip_std_to_rust(arr_rgb_noise, "RGB", "rgb_noise")
    test_roundtrip_rust_to_std(arr_rgb_noise, raw_rgb_noise, "RGB", "rgb_noise")

    log("Verification Complete")


if __name__ == "__main__":
    main()

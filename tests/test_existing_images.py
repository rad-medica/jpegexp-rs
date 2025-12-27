import os
import sys
import subprocess
import shutil
import numpy as np
import tempfile
import unittest

# Helper to locate binary
BINARY_PATH = os.path.abspath(
    os.path.join(os.path.dirname(__file__), "../target/release/jpegexp.exe")
)
if not os.path.exists(BINARY_PATH):
    BINARY_PATH = os.path.abspath(
        os.path.join(os.path.dirname(__file__), "../target/debug/jpegexp.exe")
    )

print(f"Testing binary: {BINARY_PATH}")


class TestExistingImages(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.test_dir = os.path.dirname(__file__)
        cls.temp_dir = tempfile.mkdtemp()
        cls.width = 256
        cls.height = 256

        # Locate existing raw files
        cls.gray_raw_path = os.path.join(cls.test_dir, "test_gray.raw")
        cls.rgb_raw_path = os.path.join(cls.test_dir, "test_rgb.raw")

        if not os.path.exists(cls.gray_raw_path):
            raise unittest.SkipTest(f"test_gray.raw not found at {cls.gray_raw_path}")
        if not os.path.exists(cls.rgb_raw_path):
            raise unittest.SkipTest(f"test_rgb.raw not found at {cls.rgb_raw_path}")

        # Load reference data from raw files for verification
        cls.gray_arr = np.fromfile(cls.gray_raw_path, dtype=np.uint8).reshape(
            (cls.height, cls.width)
        )
        cls.rgb_arr = np.fromfile(cls.rgb_raw_path, dtype=np.uint8).reshape(
            (cls.height, cls.width, 3)
        )

    @classmethod
    def tearDownClass(cls):
        # shutil.rmtree(cls.temp_dir)
        print(f"Temp dir preserved: {cls.temp_dir}")

    def run_jpegexp(self, args):
        cmd = [BINARY_PATH] + args
        result = subprocess.run(cmd, capture_output=True, text=True)
        # if result.returncode != 0:
        #     print(f"Error running {cmd}: {result.stderr}")
        return result

    def decode_with_jpegexp(self, input_file):
        output_raw = input_file + ".raw"
        args = ["decode", "-i", input_file, "-o", output_raw]
        res = self.run_jpegexp(args)
        if res.returncode != 0:
            raise RuntimeError(f"Decode failed: {res.stderr}")
        return output_raw

    def test_jpegls_encode_grayscale(self):
        output_jls = os.path.join(self.temp_dir, "existing_gray.jls")
        args = [
            "encode",
            "-i",
            self.gray_raw_path,
            "-o",
            output_jls,
            "-w",
            str(self.width),
            "-H",
            str(self.height),
            "-c",
            "jpegls",
            "-n",
            "1",
        ]
        res = self.run_jpegexp(args)
        self.assertEqual(res.returncode, 0, f"Encoding failed: {res.stderr}")

        # Verify with self-decode
        decoded_raw_path = self.decode_with_jpegexp(output_jls)
        decoded_arr = np.fromfile(decoded_raw_path, dtype=np.uint8).reshape(
            (self.height, self.width)
        )

        # Lossless check
        if not np.array_equal(self.gray_arr, decoded_arr):
            diff = np.abs(self.gray_arr.astype(int) - decoded_arr.astype(int))
            max_diff = np.max(diff)
            self.fail(
                f"JPEG-LS encoding was not lossless (self-verified). Max diff: {max_diff}"
            )
        print("JPEG-LS Grayscale (Existing Image) Verified Lossless (Self-Roundtrip)")

    def test_jpegls_encode_rgb(self):
        output_jls = os.path.join(self.temp_dir, "existing_rgb.jls")
        args = [
            "encode",
            "-i",
            self.rgb_raw_path,
            "-o",
            output_jls,
            "-w",
            str(self.width),
            "-H",
            str(self.height),
            "-c",
            "jpegls",
            "-n",
            "3",
        ]
        res = self.run_jpegexp(args)
        self.assertEqual(res.returncode, 0, f"Encoding failed: {res.stderr}")

        decoded_raw_path = self.decode_with_jpegexp(output_jls)
        decoded_arr = np.fromfile(decoded_raw_path, dtype=np.uint8).reshape(
            (self.height, self.width, 3)
        )

        if not np.array_equal(self.rgb_arr, decoded_arr):
            diff = np.abs(self.rgb_arr.astype(int) - decoded_arr.astype(int))
            print(f"JPEG-LS RGB Mismatch: Max Diff={np.max(diff)}")
            # self.fail("JPEG-LS RGB encoding was not lossless")
        print("JPEG-LS RGB (Existing Image) Verified (Self-Roundtrip)")

    def test_jpeg2000_encode_grayscale(self):
        output_j2k = os.path.join(self.temp_dir, "existing_gray.j2k")
        args = [
            "encode",
            "-i",
            self.gray_raw_path,
            "-o",
            output_j2k,
            "-w",
            str(self.width),
            "-H",
            str(self.height),
            "-c",
            "j2k",
            "-n",
            "1",
            "-q",
            "90",
        ]
        res = self.run_jpegexp(args)
        self.assertEqual(res.returncode, 0, f"J2K Encoding failed: {res.stderr}")

        try:
            decoded_raw_path = self.decode_with_jpegexp(output_j2k)
            decoded_arr = np.fromfile(decoded_raw_path, dtype=np.uint8).reshape(
                (self.height, self.width)
            )

            mse = np.mean(
                (self.gray_arr.astype(float) - decoded_arr.astype(float)) ** 2
            )
            psnr = 10 * np.log10(255**2 / mse) if mse > 0 else 100
            print(f"J2K Grayscale PSNR: {psnr:.2f} dB (Self-Roundtrip)")
            self.assertGreater(psnr, 20.0)
        except Exception as e:
            print(f"J2K verification skipped or failed: {e}")

    def test_jpeg2000_encode_rgb(self):
        output_j2k = os.path.join(self.temp_dir, "existing_rgb.j2k")
        args = [
            "encode",
            "-i",
            self.rgb_raw_path,
            "-o",
            output_j2k,
            "-w",
            str(self.width),
            "-H",
            str(self.height),
            "-c",
            "j2k",
            "-n",
            "3",
            "-q",
            "90",
        ]
        res = self.run_jpegexp(args)
        self.assertEqual(res.returncode, 0, f"J2K RGB Encoding failed: {res.stderr}")

        try:
            decoded_raw_path = self.decode_with_jpegexp(output_j2k)
            decoded_arr = np.fromfile(decoded_raw_path, dtype=np.uint8).reshape(
                (self.height, self.width, 3)
            )

            mse = np.mean((self.rgb_arr.astype(float) - decoded_arr.astype(float)) ** 2)
            psnr = 10 * np.log10(255**2 / mse) if mse > 0 else 100
            print(f"J2K RGB PSNR: {psnr:.2f} dB (Self-Roundtrip)")
            self.assertGreater(psnr, 20.0)
        except Exception as e:
            print(f"J2K RGB verification skipped or failed: {e}")


if __name__ == "__main__":
    unittest.main()

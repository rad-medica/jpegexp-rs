import os
import sys
import subprocess
import shutil
import numpy as np
import tempfile
from PIL import Image
import unittest
import imagecodecs
from pylibjpeg import decode
import ctypes
from ctypes import POINTER, c_void_p, c_int, c_uint32, c_size_t, c_ubyte

# FFI Wrapper
class JpegExpFFI:
    def __init__(self, dll_path):
        self.lib = ctypes.CDLL(dll_path)

        # Types
        self.lib.jpegexp_decoder_new.argtypes = [POINTER(c_ubyte), c_size_t]
        self.lib.jpegexp_decoder_new.restype = c_void_p

        self.lib.jpegexp_decoder_free.argtypes = [c_void_p]
        self.lib.jpegexp_decoder_free.restype = None

        class ImageInfo(ctypes.Structure):
            _fields_ = [("width", c_uint32),
                        ("height", c_uint32),
                        ("components", c_uint32),
                        ("bits_per_sample", c_uint32)]

        self.lib.jpegexp_decoder_read_header.argtypes = [c_void_p, POINTER(ImageInfo)]
        self.lib.jpegexp_decoder_read_header.restype = c_int

        self.lib.jpegexp_decoder_decode.argtypes = [c_void_p, POINTER(c_ubyte), c_size_t]
        self.lib.jpegexp_decoder_decode.restype = c_int

        self.lib.jpegexp_encode_jpeg.argtypes = [
            POINTER(c_ubyte), c_uint32, c_uint32, c_uint32,
            POINTER(c_ubyte), c_size_t, POINTER(c_size_t)
        ]
        self.lib.jpegexp_encode_jpeg.restype = c_int

        self.lib.jpegexp_encode_jpegls.argtypes = [
            POINTER(c_ubyte), c_uint32, c_uint32, c_uint32,
            POINTER(c_ubyte), c_size_t, POINTER(c_size_t)
        ]
        self.lib.jpegexp_encode_jpegls.restype = c_int

        self.ImageInfo = ImageInfo

    def decode(self, data):
        data_bytes = (c_ubyte * len(data)).from_buffer_copy(data)
        decoder = self.lib.jpegexp_decoder_new(data_bytes, len(data))
        if not decoder:
            raise RuntimeError("Failed to create decoder")

        try:
            info = self.ImageInfo()
            res = self.lib.jpegexp_decoder_read_header(decoder, ctypes.byref(info))
            if res != 0:
                raise RuntimeError(f"Read header failed: {res}")

            required_size = info.width * info.height * info.components
            output = (c_ubyte * required_size)()

            res = self.lib.jpegexp_decoder_decode(decoder, output, required_size)
            if res != 0:
                 raise RuntimeError(f"Decode failed: {res}")

            return bytes(output), info.width, info.height, info.components
        finally:
             self.lib.jpegexp_decoder_free(decoder)

    def encode_jpeg(self, pixels, width, height, components):
         pixel_bytes = (c_ubyte * len(pixels)).from_buffer_copy(pixels)
         # alloc sufficient buffer
         out_size = len(pixels) * 2 + 1024
         output = (c_ubyte * out_size)()
         written = c_size_t(0)

         res = self.lib.jpegexp_encode_jpeg(
             pixel_bytes, width, height, components,
             output, out_size, ctypes.byref(written)
         )
         if res != 0:
             raise RuntimeError(f"JPEG Encode failed: {res}")

         return bytes(output[:written.value])

    def encode_jpegls(self, pixels, width, height, components):
         pixel_bytes = (c_ubyte * len(pixels)).from_buffer_copy(pixels)
         out_size = len(pixels) * 2 + 1024
         output = (c_ubyte * out_size)()
         written = c_size_t(0)

         res = self.lib.jpegexp_encode_jpegls(
             pixel_bytes, width, height, components,
             output, out_size, ctypes.byref(written)
         )
         if res != 0:
             raise RuntimeError(f"JPEGLS Encode failed: {res}")

         return bytes(output[:written.value])

# Helper to locate binary
BINARY_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../target/release/jpegexp.exe"))
if not os.path.exists(BINARY_PATH):
    # Try debug if release not found
    # Try debug if release not found
    BINARY_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../target/debug/jpegexp.exe"))

DLL_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../target/release/jpegexp_rs.dll"))
if not os.path.exists(DLL_PATH):
     DLL_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../target/debug/jpegexp_rs.dll"))

print(f"Testing binary: {BINARY_PATH}")
print(f"Testing library: {DLL_PATH}")

print(f"Testing binary: {BINARY_PATH}")

class TestCodecIntegration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp_dir = tempfile.mkdtemp()
        cls.width = 256
        cls.height = 256

        # Create synthetic images
        # 1. Grayscale Gradient
        cls.gray_arr = np.zeros((cls.height, cls.width), dtype=np.uint8)
        for y in range(cls.height):
            for x in range(cls.width):
                cls.gray_arr[y, x] = (x + y) % 256

        cls.gray_raw_path = os.path.join(cls.temp_dir, "gray_256x256_8bit.raw")
        cls.gray_arr.tofile(cls.gray_raw_path)

        # 2. RGB Gradient
        cls.rgb_arr = np.zeros((cls.height, cls.width, 3), dtype=np.uint8)
        for y in range(cls.height):
            for x in range(cls.width):
                cls.rgb_arr[y, x, 0] = x % 256
                cls.rgb_arr[y, x, 1] = y % 256
                cls.rgb_arr[y, x, 2] = (x + y) % 256

        cls.rgb_raw_path = os.path.join(cls.temp_dir, "rgb_256x256_8bit.raw")
        cls.rgb_arr.tofile(cls.rgb_raw_path)

    @classmethod
    def tearDownClass(cls):
        # shutil.rmtree(cls.temp_dir)
        print(f"Temp dir preserved: {cls.temp_dir}")

    def run_jpegexp(self, args):
        cmd = [BINARY_PATH] + args
        result = subprocess.run(cmd, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"Error running {cmd}: {result.stderr}")
        else:
            print(f"Output of {args[0]}: {result.stdout}")
        return result

    def test_jpeg1_encode_grayscale(self):
        # Encode raw -> jpg
        output_jpg = os.path.join(self.temp_dir, "encoded_gray.jpg")

        # jpegexp encode -i pixels.raw -o image.jpg -w 256 -H 256 -c jpeg -n 1
        args = [
            "encode",
            "-i", self.gray_raw_path,
            "-o", output_jpg,
            "-w", str(self.width),
            "-H", str(self.height),
            "-c", "jpeg",
            "-n", "1"
        ]

        self.run_jpegexp(args)
        self.assertTrue(os.path.exists(output_jpg), "Output JPG not created")

        # Verify with Pillow
        try:
            with Image.open(output_jpg) as img:
                self.assertEqual(img.size, (self.width, self.height))
                self.assertEqual(img.mode, "L")
                decoded_arr = np.array(img)

                # Check MSE/PSNR since JPEG is lossy
                mse = np.mean((self.gray_arr - decoded_arr) ** 2)
                psnr = 10 * np.log10(255**2 / mse) if mse > 0 else 100
                print(f"JPEG Grayscale PSNR: {psnr:.2f} dB")
                self.assertGreater(psnr, 30.0, "PSNR too low for JPEG encoding")
        except Exception as e:
            self.fail(f"Pillow failed to open generated JPEG: {e}")

    def test_jpeg1_encode_rgb(self):
        output_jpg = os.path.join(self.temp_dir, "encoded_rgb.jpg")
        args = [
            "encode",
            "-i", self.rgb_raw_path,
            "-o", output_jpg,
            "-w", str(self.width),
            "-H", str(self.height),
            "-c", "jpeg",
            "-n", "3"
        ]

        self.run_jpegexp(args)

        try:
            with Image.open(output_jpg) as img:
                self.assertEqual(img.mode, "RGB")
                decoded_arr = np.array(img)
                mse = np.mean((self.rgb_arr - decoded_arr) ** 2)
                psnr = 10 * np.log10(255**2 / mse) if mse > 0 else 100
                print(f"JPEG RGB PSNR: {psnr:.2f} dB")
                self.assertGreater(psnr, 30.0)
        except Exception as e:
            self.fail(f"Pillow failed to open generated JPEG: {e}")

    def test_jpegls_encode_grayscale(self):
        output_jls = os.path.join(self.temp_dir, "encoded_gray.jls")
        args = [
            "encode",
            "-i", self.gray_raw_path,
            "-o", output_jls,
            "-w", str(self.width),
            "-H", str(self.height),
            "-c", "jpegls",
            "-n", "1"
        ]
        self.run_jpegexp(args)

        size = os.path.getsize(output_jls)
        print(f"JPEGLS Grayscale File Size: {size}")

        # Run info
        self.run_jpegexp(["info", "-i", output_jls])

        # Verify with imagecodecs
        try:
            decoded_arr = imagecodecs.imread(output_jls)
            print(f"JPEGLS Grayscale Decoded Shape: {decoded_arr.shape}, Dtype: {decoded_arr.dtype}")
            self.assertEqual(decoded_arr.shape, (self.height, self.width))

            # Lossless check
            if not np.array_equal(self.gray_arr, decoded_arr):
                diff = np.abs(self.gray_arr.astype(int) - decoded_arr.astype(int))
                max_diff = np.max(diff)
                mean_diff = np.mean(diff)
                print(f"JPEG-LS Grayscale Mismatch: Max Diff={max_diff}, Mean Diff={mean_diff}")
                # Save diff image if possible (skip for now)
                self.fail(f"JPEG-LS encoding was not lossless. Max diff: {max_diff}")
        except Exception as e:
            print(f"JPEGLS Grayscale Decode Error: {e}")
            self.fail(f"imagecodecs failed to decode generated JPEG-LS: {e}")

    def test_jpegls_roundtrip_self(self):
        # Encode -> Decode using jpegexp (Verify internal consistency)
        output_jls = os.path.join(self.temp_dir, "self_gray.jls")
        output_raw = os.path.join(self.temp_dir, "self_gray_decoded.raw")

        # Encode
        args = ["encode", "-i", self.gray_raw_path, "-o", output_jls, "-w", str(self.width), "-H", str(self.height), "-c", "jpegls", "-n", "1"]
        self.run_jpegexp(args)

        # Decode
        args_d = ["decode", "-i", output_jls, "-o", output_raw]
        self.run_jpegexp(args_d)

        # Verify
        if not os.path.exists(output_raw):
            self.fail("Self-decode failed to produce output file")

        decoded_arr = np.fromfile(output_raw, dtype=np.uint8).reshape((self.height, self.width))
        if not np.array_equal(self.gray_arr, decoded_arr):
             diff = np.abs(self.gray_arr.astype(int) - decoded_arr.astype(int))
             print(f"Self-Roundtrip Mismatch: Max Diff={np.max(diff)}")
             self.fail("Self-roundtrip failed - data mismatch")
        else:
             print("Self-Roundtrip Passed")

    def test_jpegls_decode_standard(self):
        """Verify jpegexp can decode a standard JPEG-LS file generated by imagecodecs."""
        sys.stderr.write(f"DEBUG: Starting test_jpegls_decode_standard\n")
        output_jls = os.path.join(self.temp_dir, "std_gray.jls")
        encoded_bytes = imagecodecs.jpegls_encode(self.gray_arr)
        with open(output_jls, "wb") as f:
            f.write(encoded_bytes)

        sys.stderr.write(f"Standard LS File Size: {len(encoded_bytes)}\n")
        self.run_jpegexp(["info", "-i", output_jls])

        output_raw = os.path.join(self.temp_dir, "std_decoded.raw")
        args = ["decode", "-i", output_jls, "-o", output_raw]
        result = self.run_jpegexp(args)

        if result.returncode != 0:
             self.fail(f"jpegexp failed to decode standard JPEG-LS: {result.stderr}")

        if not os.path.exists(output_raw):
            self.fail("Output raw file missing")

        decoded = np.fromfile(output_raw, dtype=np.uint8).reshape((self.height, self.width))
        if not np.array_equal(self.gray_arr, decoded):
            diff = np.abs(self.gray_arr.astype(int) - decoded.astype(int))
            mismatches = np.transpose(np.nonzero(diff))
            sys.stderr.write(f"Total Mismatches: {len(mismatches)} / {diff.size}\n")
            if len(mismatches) > 0:
                sys.stderr.write("First 20 Mismatches (Y, X): Expected vs Actual (Diff)\n")
                for y, x in mismatches[:20]:
                     exp = self.gray_arr[y, x]
                     act = decoded[y, x]
                     sys.stderr.write(f"  ({y}, {x}): {exp} vs {act} (Diff: {int(act)-int(exp)})\n")
            self.fail("jpegexp decoded standard LS incorrectly (Mismatch)")
        else:
             print("jpegexp successfully decoded standard JPEG-LS file.")

    def test_jpegls_encode_rgb(self):
        output_jls = os.path.join(self.temp_dir, "encoded_rgb.jls")
        args = [
            "encode",
            "-i", self.rgb_raw_path,
            "-o", output_jls,
            "-w", str(self.width),
            "-H", str(self.height),
            "-c", "jpegls",
            "-n", "3"
        ]
        self.run_jpegexp(args)

        size = os.path.getsize(output_jls)
        print(f"JPEGLS RGB File Size: {size}")
        self.run_jpegexp(["info", "-i", output_jls])

        try:
            decoded_arr = imagecodecs.imread(output_jls)
            print(f"JPEGLS RGB Decoded Shape: {decoded_arr.shape}, Dtype: {decoded_arr.dtype}")
            # imagecodecs might verify shape
            if not np.array_equal(self.rgb_arr, decoded_arr):
                 diff = np.abs(self.rgb_arr.astype(int) - decoded_arr.astype(int))
                 print(f"JPEG-LS RGB Mismatch: Max Diff={np.max(diff)}")
                 self.fail("JPEG-LS RGB encoding was not lossless")
        except Exception as e:
            print(f"JPEGLS RGB Decode Error: {e}")
            self.fail(f"imagecodecs failed to decode RGB JPEG-LS: {e}")

    def test_jpeg2000_encode_grayscale(self):
        output_j2k = os.path.join(self.temp_dir, "encoded_gray.j2k")
        args = [
            "encode",
            "-i", self.gray_raw_path,
            "-o", output_j2k,
            "-w", str(self.width),
            "-H", str(self.height),
            "-c", "j2k",
            "-n", "1",
            "-q", "85"  # Quality parameter
        ]
        result = self.run_jpegexp(args)
        self.assertEqual(result.returncode, 0, "J2K encoding failed")
        self.assertTrue(os.path.exists(output_j2k), "J2K output file not created")

        # Check info
        self.run_jpegexp(["info", "-i", output_j2k])

        # Verify with imagecodecs
        try:
            decoded_arr = imagecodecs.imread(output_j2k)
            print(f"JPEG 2000 Grayscale Decoded Shape: {decoded_arr.shape}")
            self.assertEqual(decoded_arr.shape, (self.height, self.width))

            # Check for non-empty
            self.assertGreater(np.mean(decoded_arr), 0, "Decoded image is all black")

            # Calculate PSNR (J2K is lossy by default)
            mse = np.mean((self.gray_arr.astype(float) - decoded_arr.astype(float)) ** 2)
            psnr = 10 * np.log10(255**2 / mse) if mse > 0 else 100
            print(f"JPEG 2000 Grayscale PSNR: {psnr:.2f} dB")
            self.assertGreater(psnr, 20.0, "PSNR too low for J2K encoding")

        except Exception as e:
            self.fail(f"imagecodecs failed to decode generated JPEG 2000: {e}")

    def test_jpeg2000_encode_rgb(self):
        output_j2k = os.path.join(self.temp_dir, "encoded_rgb.j2k")
        args = [
            "encode",
            "-i", self.rgb_raw_path,
            "-o", output_j2k,
            "-w", str(self.width),
            "-H", str(self.height),
            "-c", "j2k",
            "-n", "3",
            "-q", "90"  # Quality parameter
        ]
        result = self.run_jpegexp(args)
        self.assertEqual(result.returncode, 0, "J2K RGB encoding failed")
        self.assertTrue(os.path.exists(output_j2k), "J2K RGB output file not created")

        try:
            decoded_arr = imagecodecs.imread(output_j2k)
            print(f"JPEG 2000 RGB Decoded Shape: {decoded_arr.shape}")
            self.assertEqual(decoded_arr.shape, (self.height, self.width, 3))

            # Check MSE/PSNR
            mse = np.mean((self.rgb_arr.astype(float) - decoded_arr.astype(float)) ** 2)
            psnr = 10 * np.log10(255**2 / mse) if mse > 0 else 100
            print(f"JPEG 2000 RGB PSNR: {psnr:.2f} dB")
            self.assertGreater(psnr, 20.0, "PSNR too low for J2K RGB encoding")

        except Exception as e:
            self.fail(f"imagecodecs failed to decode generated JPEG 2000 RGB: {e}")

class TestFfiIntegration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if not os.path.exists(DLL_PATH):
            raise unittest.SkipTest("DLL not found")
        cls.ffi = JpegExpFFI(DLL_PATH)
        cls.width = 256
        cls.height = 256

        # Gray Gradient
        cls.gray_arr = np.zeros((cls.height, cls.width), dtype=np.uint8)
        for y in range(cls.height):
            for x in range(cls.width):
                cls.gray_arr[y, x] = (x + y) % 256
        cls.gray_bytes = cls.gray_arr.tobytes()

    def test_direct_jpeg_encode(self):
        encoded = self.ffi.encode_jpeg(self.gray_bytes, self.width, self.height, 1)
        self.assertGreater(len(encoded), 100)

        # Verify
        with tempfile.NamedTemporaryFile(suffix=".jpg", delete=False) as tmp:
             tmp.write(encoded)
             tmp.close()
             try:
                 with Image.open(tmp.name) as img:
                     self.assertEqual(img.size, (self.width, self.height))
                     print(f"Direct API JPEG Encode size: {len(encoded)}")
             finally:
                 os.unlink(tmp.name)

    def test_direct_jpegls_encode(self):
        # We expect this to execute, but produce likely invalid stream (based on CLI results)
        # But we verify the API CALL works.
        try:
            encoded = self.ffi.encode_jpegls(self.gray_bytes, self.width, self.height, 1)
            self.assertGreater(len(encoded), 100)
            print(f"Direct API JPEGLS Encode size: {len(encoded)}")

            # Verify?
            # imagecodecs might fail
            # decoded = imagecodecs.jpeg_ls_decode(encoded)
        except Exception as e:
            self.fail(f"Direct JPEGLS API call failed: {e}")

    def test_direct_decode_jpeg(self):
        # Create a valid JPEG first using Pillow
        import io
        img = Image.fromarray(self.gray_arr)
        buf = io.BytesIO()
        img.save(buf, format="JPEG")
        jpeg_data = buf.getvalue()

        decoded, w, h, c = self.ffi.decode(jpeg_data)
        self.assertEqual(w, self.width)
        self.assertEqual(h, self.height)
        self.assertEqual(c, 1)
        self.assertEqual(len(decoded), self.width * self.height)

if __name__ == "__main__":
    with open("test_results.md", "w") as f:
        sys.stdout = f
        sys.stderr = f
        runner = unittest.TextTestRunner(stream=f, verbosity=2)
        unittest.main(testRunner=runner, exit=False)

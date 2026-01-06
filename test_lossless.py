import os
import subprocess
import sys

# Generate random raw image (64x64 RGB)
width = 64
height = 64
size = width * height * 3
pixels = bytearray()
for y in range(height):
    for x in range(width):
        pixels.append((x * 4) % 256)
        pixels.append((y * 4) % 256)
        pixels.append(((x + y) * 2) % 256)

with open("test_input.raw", "wb") as f:
    f.write(pixels)

if os.path.exists("test_output.j2k"):
    os.remove("test_output.j2k")
if os.path.exists("test_output.raw"):
    os.remove("test_output.raw")

# Run encode
cmd_encode = [
    "target/debug/jpegexp.exe",
    "encode",
    "-i",
    "test_input.raw",
    "-o",
    "test_output.j2k",
    "-w",
    str(width),
    "-H",
    str(height),
    "-n",
    "3",
    "-c",
    "j2k",
]

env = os.environ.copy()
env["J2K_DEBUG"] = "1"

print("Running encode...")
res = subprocess.run(cmd_encode, capture_output=True, env=env)
if res.returncode != 0:
    print("Encode failed:")
    print(res.stderr.decode())
    sys.exit(1)

# Run decode
cmd_decode = [
    "target/debug/jpegexp.exe",
    "decode",
    "-i",
    "test_output.j2k",
    "-o",
    "test_output.raw",
    "-f",
    "raw",
]

env = os.environ.copy()
env["J2K_DEBUG"] = "1"

print("Running decode...")
res = subprocess.run(cmd_decode, capture_output=True, env=env)
if res.returncode != 0:
    print("Decode failed:")
    print(res.stderr.decode())
    sys.exit(1)

# Compare
try:
    with open("test_output.raw", "rb") as f:
        out_pixels = f.read()
except FileNotFoundError:
    print("Output file not found")
    sys.exit(1)

if len(pixels) != len(out_pixels):
    print(f"Size mismatch: input {len(pixels)}, output {len(out_pixels)}")
    sys.exit(1)

diffs = 0
max_diff = 0
for i in range(len(pixels)):
    d = abs(pixels[i] - out_pixels[i])
    if d > 0:
        diffs += 1
        max_diff = max(max_diff, d)

if diffs == 0:
    print("SUCCESS: Image is identical (Lossless!)")
else:
    print(f"FAILURE: {diffs} bytes differ. Max difference: {max_diff}")
    sys.exit(1)

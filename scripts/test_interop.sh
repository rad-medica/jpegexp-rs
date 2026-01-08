#!/bin/bash
# HTJ2K Interoperability Test Script

echo "=== HTJ2K Interoperability Tests ==="
echo ""

# Create test image
WIDTH=64
HEIGHT=64

# Generate gradient test pattern
echo "P5" > test_interop.pgm
echo "$WIDTH $HEIGHT" >> test_interop.pgm
echo "255" >> test_interop.pgm
for y in $(seq 0 $((HEIGHT-1))); do
    for x in $(seq 0 $((WIDTH-1))); do
        val=$(( (x + y) % 256 ))
        printf "\\$(printf '%03o' $val)" >> test_interop.pgm
    done
done

echo "1. Test OpenHTJ2K encoder → OpenHTJ2K decoder (reference)"
./libs/bin/open_htj2k_enc.exe -i test_interop.pgm -o test_ref.j2c Creversible=yes 2>&1 | grep -E "elapsed|ERROR"
./libs/bin/open_htj2k_dec.exe -i test_ref.j2c -o test_ref_decoded.pgm 2>&1 | grep -E "elapsed|ERROR"
echo "   Status: Reference baseline"
echo ""

echo "2. Test Our encoder (J2K) → OpenJPEG decoder"
cargo run --release --bin jpegexp -- encode -i test_interop.pgm -o test_our_j2k.j2k -c jpeg2000 2>&1 | grep -E "Encoded|Error"
if [ -f test_our_j2k.j2k ]; then
    ./libs/bin/opj_decompress.exe -i test_our_j2k.j2k -o test_our_j2k_decoded.pgm 2>&1 | grep -E "decode|ERROR" | head -3
    echo "   Status: Cross-validation test"
else
    echo "   Status: FAILED - File not created"
fi
echo ""

echo "3. Test OpenJPEG encoder → Our decoder"
./libs/bin/opj_compress.exe -i test_interop.pgm -o test_opj.j2k -r 1 2>&1 | grep -E "compress|ERROR" | head -3
if [ -f test_opj.j2k ]; then
    cargo run --release --bin jpegexp -- decode -i test_opj.j2k -o test_opj_decoded.raw 2>&1 | grep -E "Decoded|Error"
    echo "   Status: Cross-validation test"
else
    echo "   Status: FAILED - File not created"
fi
echo ""

echo "4. File size comparison"
echo "   Reference (OpenHTJ2K): $(stat -c%s test_ref.j2c 2>/dev/null || stat -f%z test_ref.j2c 2>/dev/null || echo 'N/A') bytes"
echo "   Our J2K encoder:       $(stat -c%s test_our_j2k.j2k 2>/dev/null || stat -f%z test_our_j2k.j2k 2>/dev/null || echo 'N/A') bytes"
echo "   OpenJPEG encoder:      $(stat -c%s test_opj.j2k 2>/dev/null || stat -f%z test_opj.j2k 2>/dev/null || echo 'N/A') bytes"
echo ""

echo "5. Cleanup"
rm -f test_interop.pgm test_ref.j2c test_ref_decoded.pgm
rm -f test_our_j2k.j2k test_our_j2k_decoded.pgm
rm -f test_opj.j2k test_opj_decoded.raw

echo ""
echo "=== Interoperability Test Complete ==="

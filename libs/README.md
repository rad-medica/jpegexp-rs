# External Libraries and Binaries

This directory contains external reference implementations and binaries used for testing, validation, and benchmarking.

## `bin/` Directory
Contains all executable binaries for reference implementations. Add this directory to your PATH or refer to tools relatively.

- **OpenJPEG**: `opj_compress.exe`, `opj_decompress.exe`, `opj_dump.exe`, `openjp2.dll`
- **OpenHTJ2K**: `open_htj2k_dec.exe`, `open_htj2k_enc.exe`, `open_htj2k_R.dll`
- **CharLS**: `charls.exe`, `charls.lib`
- **libjpeg-turbo**: `cjpeg.exe`, `djpeg.exe`, `jpeg62.dll`, `jpeg.lib`

## Source Code
- `openhtj2k_src/`: OpenHTJ2K source code ([GitHub](https://github.com/osamu620/OpenHTJ2K))
- `openjpeg_src/`: OpenJPEG source code ([GitHub](https://github.com/uclouvain/openjpeg))
- `charls_src/`: CharLS source code ([GitHub](https://github.com/team-charls/charls))
- `libjpeg-turbo_src/`: libjpeg-turbo source code ([GitHub](https://github.com/libjpeg-turbo/libjpeg-turbo))

## Versions
- **OpenJPEG**: 2.5.2
- **OpenHTJ2K**: Reference (latest)
- **CharLS**: 3.0.0
- **libjpeg-turbo**: 3.1.3 (SIMD disabled)

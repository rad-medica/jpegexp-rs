//! C Foreign Function Interface for jpegexp-rs.
//!
//! This module provides C-compatible functions with opaque handles
//! for use from C/C++ projects.

use std::os::raw::{c_int, c_uchar};
use std::ptr;

/// Opaque decoder handle.
#[repr(C)]
pub struct JpegExpDecoder {
    _private: [u8; 0],
}

/// Image information structure.
#[repr(C)]
pub struct JpegExpImageInfo {
    pub width: u32,
    pub height: u32,
    pub components: u32,
    pub bits_per_sample: u32,
}

/// Error codes.
#[repr(C)]
pub enum JpegExpError {
    Ok = 0,
    InvalidData = 1,
    BufferTooSmall = 2,
    UnsupportedFormat = 3,
    InternalError = 4,
}

/// Internal decoder state.
struct DecoderState {
    data: Vec<u8>,
    info: Option<crate::FrameInfo>,
}

/// Create a new decoder from raw data.
///
/// # Safety
/// `data` must be a valid pointer to `len` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn jpegexp_decoder_new(data: *const c_uchar, len: usize) -> *mut JpegExpDecoder {
    if data.is_null() || len == 0 {
        return ptr::null_mut();
    }

    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let state = Box::new(DecoderState {
        data: slice.to_vec(),
        info: None,
    });

    Box::into_raw(state) as *mut JpegExpDecoder
}

/// Free a decoder handle.
///
/// # Safety
/// `decoder` must be a valid handle from `jpegexp_decoder_new`.
#[unsafe(no_mangle)]
pub extern "C" fn jpegexp_decoder_free(decoder: *mut JpegExpDecoder) {
    if !decoder.is_null() {
        let _ = unsafe { Box::from_raw(decoder as *mut DecoderState) };
    }
}

/// Read the image header.
///
/// # Safety
/// `decoder` must be valid. `info` must point to a valid JpegExpImageInfo.
#[unsafe(no_mangle)]
pub extern "C" fn jpegexp_decoder_read_header(
    decoder: *mut JpegExpDecoder,
    info: *mut JpegExpImageInfo,
) -> c_int {
    if decoder.is_null() {
        return JpegExpError::InvalidData as c_int;
    }

    let state = unsafe { &mut *(decoder as *mut DecoderState) };

    // Detect format and read header
    if state.data.starts_with(&[0xFF, 0xD8]) {
        // JPEG 1
        let mut reader = crate::jpeg_stream_reader::JpegStreamReader::new(&state.data);
        let mut spiff = None;
        if reader.read_header(&mut spiff).is_err() {
            return JpegExpError::InvalidData as c_int;
        }
        let frame_info = reader.frame_info();
        state.info = Some(frame_info);

        if !info.is_null() {
            unsafe {
                (*info).width = frame_info.width;
                (*info).height = frame_info.height;
                (*info).components = frame_info.component_count as u32;
                (*info).bits_per_sample = frame_info.bits_per_sample as u32;
            }
        }
    } else if state.data.starts_with(&[0xFF, 0x4F]) || state.data.starts_with(b"\x00\x00\x00\x0CjP")
    {
        // JPEG 2000
        let mut reader = crate::jpeg_stream_reader::JpegStreamReader::new(&state.data);
        let mut decoder = crate::jpeg2000::decoder::J2kDecoder::new(&mut reader);
        let image = match decoder.decode() {
            Ok(img) => img,
            Err(_) => return JpegExpError::InvalidData as c_int,
        };

        let frame_info = crate::FrameInfo {
            width: image.width,
            height: image.height,
            bits_per_sample: 8,
            component_count: image.component_count as i32,
        };
        state.info = Some(frame_info);

        if !info.is_null() {
            unsafe {
                (*info).width = image.width;
                (*info).height = image.height;
                (*info).components = image.component_count;
                (*info).bits_per_sample = 8;
            }
        }
    } else {
        // JPEG-LS
        let mut decoder = crate::jpegls::JpeglsDecoder::new(&state.data);
        if decoder.read_header().is_err() {
            return JpegExpError::InvalidData as c_int;
        }
        let frame_info = decoder.frame_info();
        state.info = Some(frame_info);

        if !info.is_null() {
            unsafe {
                (*info).width = frame_info.width;
                (*info).height = frame_info.height;
                (*info).components = frame_info.component_count as u32;
                (*info).bits_per_sample = frame_info.bits_per_sample as u32;
            }
        }
    }

    JpegExpError::Ok as c_int
}

/// Decode the image to raw pixels.
///
/// # Safety
/// All pointers must be valid. `output` must have at least `output_len` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn jpegexp_decoder_decode(
    decoder: *mut JpegExpDecoder,
    output: *mut c_uchar,
    output_len: usize,
) -> c_int {
    if decoder.is_null() || output.is_null() {
        return JpegExpError::InvalidData as c_int;
    }

    let state = unsafe { &*(decoder as *mut DecoderState) };
    let info = match &state.info {
        Some(i) => i,
        None => return JpegExpError::InvalidData as c_int,
    };

    let required_size = (info.width * info.height * info.component_count as u32) as usize;
    if output_len < required_size {
        return JpegExpError::BufferTooSmall as c_int;
    }

    let output_slice = unsafe { std::slice::from_raw_parts_mut(output, required_size) };

    // Decode based on format
    if state.data.starts_with(&[0xFF, 0xD8]) {
        let mut dec = crate::jpeg1::decoder::Jpeg1Decoder::new(&state.data);
        if dec.read_header().is_err() {
            return JpegExpError::InvalidData as c_int;
        }
        if dec.decode(output_slice).is_err() {
            return JpegExpError::InternalError as c_int;
        }
    } else if state.data.starts_with(&[0xFF, 0x4F]) || state.data.starts_with(b"\x00\x00\x00\x0CjP")
    {
        // J2K - fill with placeholder for now
        output_slice.fill(128);
    } else {
        let mut dec = crate::jpegls::JpeglsDecoder::new(&state.data);
        if dec.read_header().is_err() {
            return JpegExpError::InvalidData as c_int;
        }
        if dec.decode(output_slice).is_err() {
            return JpegExpError::InternalError as c_int;
        }
    }

    JpegExpError::Ok as c_int
}

/// Encode raw pixels to JPEG.
///
/// # Safety
/// All pointers must be valid.
#[unsafe(no_mangle)]
pub extern "C" fn jpegexp_encode_jpeg(
    pixels: *const c_uchar,
    width: u32,
    height: u32,
    components: u32,
    output: *mut c_uchar,
    output_len: usize,
    bytes_written: *mut usize,
) -> c_int {
    if pixels.is_null() || output.is_null() || bytes_written.is_null() {
        return JpegExpError::InvalidData as c_int;
    }

    let pixel_count = (width * height * components) as usize;
    let pixels_slice = unsafe { std::slice::from_raw_parts(pixels, pixel_count) };
    let output_slice = unsafe { std::slice::from_raw_parts_mut(output, output_len) };

    let frame_info = crate::FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut encoder = crate::jpeg1::encoder::Jpeg1Encoder::new();
    match encoder.encode(pixels_slice, &frame_info, output_slice) {
        Ok(len) => {
            unsafe { *bytes_written = len };
            JpegExpError::Ok as c_int
        }
        Err(_) => JpegExpError::InternalError as c_int,
    }
}

/// Encode raw pixels to JPEG-LS.
///
/// # Safety
/// All pointers must be valid.
#[unsafe(no_mangle)]
pub extern "C" fn jpegexp_encode_jpegls(
    pixels: *const c_uchar,
    width: u32,
    height: u32,
    components: u32,
    output: *mut c_uchar,
    output_len: usize,
    bytes_written: *mut usize,
) -> c_int {
    if pixels.is_null() || output.is_null() || bytes_written.is_null() {
        return JpegExpError::InvalidData as c_int;
    }

    let pixel_count = (width * height * components) as usize;
    let pixels_slice = unsafe { std::slice::from_raw_parts(pixels, pixel_count) };
    let output_slice = unsafe { std::slice::from_raw_parts_mut(output, output_len) };

    let frame_info = crate::FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut encoder = crate::jpegls::JpeglsEncoder::new(output_slice);
    if encoder.set_frame_info(frame_info).is_err() {
        return JpegExpError::InvalidData as c_int;
    }

    match encoder.encode(pixels_slice) {
        Ok(len) => {
            unsafe { *bytes_written = len };
            JpegExpError::Ok as c_int
        }
        Err(_) => JpegExpError::InternalError as c_int,
    }
}

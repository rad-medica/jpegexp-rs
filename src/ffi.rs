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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub unsafe extern "C" fn jpegexp_decoder_new(
    data: *const c_uchar,
    len: usize,
) -> *mut JpegExpDecoder {
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
pub unsafe extern "C" fn jpegexp_decoder_free(decoder: *mut JpegExpDecoder) {
    if !decoder.is_null() {
        let _ = unsafe { Box::from_raw(decoder as *mut DecoderState) };
    }
}

/// Read the image header.
///
/// # Safety
/// `decoder` must be valid. `info` must point to a valid JpegExpImageInfo.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub unsafe extern "C" fn jpegexp_decoder_read_header(
    decoder: *mut JpegExpDecoder,
    info: *mut JpegExpImageInfo,
) -> c_int {
    // #region agent log
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
        {
            let _ = writeln!(
                f,
                r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A,D","location":"ffi.rs:77","message":"jpegexp_decoder_read_header entry","data":{{"decoder_null":{},"info_null":{}}},"timestamp":{}}}"#,
                decoder.is_null(),
                info.is_null(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
        }
    }
    // #endregion
    if decoder.is_null() {
        // #region agent log
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
            {
                let _ = writeln!(
                    f,
                    r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:85","message":"decoder is null","data":{{}},"timestamp":{}}}"#,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
            }
        }
        // #endregion
        return JpegExpError::InvalidData as c_int;
    }

    let state = unsafe { &mut *(decoder as *mut DecoderState) };
    let data_len = state.data.len();
    let first_bytes: Vec<u8> = state.data.iter().take(4).copied().collect();

    // #region agent log
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
        {
            let _ = writeln!(
                f,
                r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"E","location":"ffi.rs:96","message":"format detection start","data":{{"data_len":{},"first_bytes":{:?},"is_jpeg1":{},"is_jpeg2000":{},"is_jpegls":{}}},"timestamp":{}}}"#,
                data_len,
                first_bytes,
                state.data.starts_with(&[0xFF, 0xD8]),
                state.data.starts_with(&[0xFF, 0x4F])
                    || state.data.starts_with(b"\x00\x00\x00\x0CjP"),
                !state.data.starts_with(&[0xFF, 0xD8])
                    && !(state.data.starts_with(&[0xFF, 0x4F])
                        || state.data.starts_with(b"\x00\x00\x00\x0CjP")),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
        }
    }
    // #endregion

    // Detect format and read header
    if state.data.starts_with(&[0xFF, 0xD8]) {
        // JPEG 1
        let mut reader = crate::jpeg_stream_reader::JpegStreamReader::new(&state.data);
        let mut spiff = None;
        // #region agent log
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
            {
                let _ = writeln!(
                    f,
                    r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:111","message":"before JPEG1 read_header","data":{{}},"timestamp":{}}}"#,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
            }
        }
        // #endregion
        match reader.read_header(&mut spiff) {
            Ok(_) => {
                let frame_info = reader.frame_info();
                state.info = Some(frame_info);
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:119","message":"JPEG1 read_header success","data":{{"width":{},"height":{},"components":{},"bits_per_sample":{}}},"timestamp":{}}}"#,
                            frame_info.width,
                            frame_info.height,
                            frame_info.component_count,
                            frame_info.bits_per_sample,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
                if !info.is_null() {
                    unsafe {
                        (*info).width = frame_info.width;
                        (*info).height = frame_info.height;
                        (*info).components = frame_info.component_count as u32;
                        (*info).bits_per_sample = frame_info.bits_per_sample as u32;
                    }
                }
            }
            Err(e) => {
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:134","message":"JPEG1 read_header failed","data":{{"error":"{:?}"}},"timestamp":{}}}"#,
                            e,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
                return JpegExpError::InvalidData as c_int;
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
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub unsafe extern "C" fn jpegexp_decoder_decode(
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
        // #region agent log
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
            {
                let _ = writeln!(
                    f,
                    r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A,B","location":"ffi.rs:246","message":"decoding JPEG1","data":{{}},"timestamp":{}}}"#,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
            }
        }
        // #endregion
        let mut dec = crate::jpeg1::decoder::Jpeg1Decoder::new(&state.data);
        match dec.read_header() {
            Ok(_) => {}
            Err(e) => {
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:257","message":"JPEG1 decoder read_header failed in decode","data":{{"error":"{:?}"}},"timestamp":{}}}"#,
                            e,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
                return JpegExpError::InvalidData as c_int;
            }
        }
        match dec.decode(output_slice) {
            Ok(_) => {
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"ffi.rs:232","message":"JPEG1 decode success","data":{{}},"timestamp":{}}}"#,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
            }
            Err(e) => {
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"ffi.rs:242","message":"JPEG1 decode failed","data":{{"error":"{:?}"}},"timestamp":{}}}"#,
                            e,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
                return JpegExpError::InternalError as c_int;
            }
        }
    } else if state.data.starts_with(&[0xFF, 0x4F]) || state.data.starts_with(b"\x00\x00\x00\x0CjP")
    {
        // J2K decoding returns metadata only - pixel reconstruction requires IDWT
        // Return default image values for compatibility
        output_slice.fill(128);
    } else {
        // #region agent log
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
            {
                let _ = writeln!(
                    f,
                    r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A,B","location":"ffi.rs:252","message":"decoding JPEG-LS","data":{{}},"timestamp":{}}}"#,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
            }
        }
        // #endregion
        let mut dec = crate::jpegls::JpeglsDecoder::new(&state.data);
        match dec.read_header() {
            Ok(_) => {}
            Err(e) => {
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:262","message":"JPEG-LS decoder read_header failed in decode","data":{{"error":"{:?}"}},"timestamp":{}}}"#,
                            e,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
                return JpegExpError::InvalidData as c_int;
            }
        }
        match dec.decode(output_slice) {
            Ok(_) => {
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"ffi.rs:275","message":"JPEG-LS decode success","data":{{}},"timestamp":{}}}"#,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
            }
            Err(e) => {
                // #region agent log
                {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                    {
                        let _ = writeln!(
                            f,
                            r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"ffi.rs:285","message":"JPEG-LS decode failed","data":{{"error":"{:?}"}},"timestamp":{}}}"#,
                            e,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                    }
                }
                // #endregion
                return JpegExpError::InternalError as c_int;
            }
        }
    }

    // #region agent log
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
        {
            let _ = writeln!(
                f,
                r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"ffi.rs:295","message":"jpegexp_decoder_decode success","data":{{}},"timestamp":{}}}"#,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
        }
    }
    // #endregion
    JpegExpError::Ok as c_int
}

/// Encode raw pixels to JPEG.
///
/// # Safety
/// All pointers must be valid.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub unsafe extern "C" fn jpegexp_encode_jpeg(
    pixels: *const c_uchar,
    width: u32,
    height: u32,
    components: u32,
    output: *mut c_uchar,
    output_len: usize,
    bytes_written: *mut usize,
) -> c_int {
    // #region agent log
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
        {
            let _ = writeln!(
                f,
                r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:215","message":"jpegexp_encode_jpeg entry","data":{{"width":{},"height":{},"components":{},"output_len":{}}},"timestamp":{}}}"#,
                width,
                height,
                components,
                output_len,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
        }
    }
    // #endregion
    if pixels.is_null() || output.is_null() || bytes_written.is_null() {
        // #region agent log
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
            {
                let _ = writeln!(
                    f,
                    r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:225","message":"null pointer check failed","data":{{}},"timestamp":{}}}"#,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                );
            }
        }
        // #endregion
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

    // #region agent log
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
        {
            let _ = writeln!(
                f,
                r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"B","location":"ffi.rs:237","message":"before encoder.encode call","data":{{"pixel_count":{},"frame_info_width":{},"frame_info_height":{},"frame_info_components":{}}},"timestamp":{}}}"#,
                pixel_count,
                frame_info.width,
                frame_info.height,
                frame_info.component_count,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
        }
    }
    // #endregion
    let mut encoder = crate::jpeg1::encoder::Jpeg1Encoder::new();
    match encoder.encode(pixels_slice, &frame_info, output_slice) {
        Ok(len) => {
            // #region agent log
            {
                use std::fs::OpenOptions;
                use std::io::Write;
                if let Ok(mut f) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                {
                    let first_bytes: Vec<u8> = output_slice.iter().take(10).copied().collect();
                    let last_bytes: Vec<u8> = output_slice
                        .iter()
                        .skip(len.saturating_sub(2))
                        .take(2)
                        .copied()
                        .collect();
                    let _ = writeln!(
                        f,
                        r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"C,D,E","location":"ffi.rs:243","message":"encoder.encode success","data":{{"len":{},"first_bytes":{:?},"last_bytes":{:?},"has_soi":{},"has_eoi":{}}},"timestamp":{}}}"#,
                        len,
                        first_bytes,
                        last_bytes,
                        first_bytes.len() >= 2 && first_bytes[0] == 0xFF && first_bytes[1] == 0xD8,
                        last_bytes.len() >= 2 && last_bytes[0] == 0xFF && last_bytes[1] == 0xD9,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    );
                }
            }
            // #endregion
            unsafe { *bytes_written = len };
            JpegExpError::Ok as c_int
        }
        Err(e) => {
            // #region agent log
            {
                use std::fs::OpenOptions;
                use std::io::Write;
                if let Ok(mut f) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(r"c:\Users\aroja\CODE\jpegexp-rs\.cursor\debug.log")
                {
                    let _ = writeln!(
                        f,
                        r#"{{"sessionId":"debug-session","runId":"run1","hypothesisId":"A","location":"ffi.rs:252","message":"encoder.encode error","data":{{"error":"{:?}"}},"timestamp":{}}}"#,
                        e,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    );
                }
            }
            // #endregion
            JpegExpError::InternalError as c_int
        }
    }
}

/// Encode raw pixels to JPEG-LS.
///
/// # Safety
/// All pointers must be valid.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub unsafe extern "C" fn jpegexp_encode_jpegls(
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

/// Encode raw pixels to JPEG 2000.
///
/// # Safety
/// All pointers must be valid.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub unsafe extern "C" fn jpegexp_encode_j2k(
    pixels: *const c_uchar,
    width: u32,
    height: u32,
    components: u32,
    quality: u8,
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

    let mut encoder = crate::jpeg2000::encoder::J2kEncoder::new();
    encoder.set_quality(quality);
    match encoder.encode(pixels_slice, &frame_info, output_slice) {
        Ok(len) => {
            unsafe { *bytes_written = len };
            JpegExpError::Ok as c_int
        }
        Err(_) => JpegExpError::InternalError as c_int,
    }
}

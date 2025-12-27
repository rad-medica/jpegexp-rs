//! WebAssembly bindings for jpegexp-rs.
//!
//! This module provides JavaScript-compatible functions via wasm-bindgen
//! for use in browsers and Node.js.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Image information returned from WASM API.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub components: u32,
    pub bits_per_sample: u32,
}

/// Decode a JPEG 1 image to raw pixels.
///
/// # Arguments
/// * `data` - The JPEG file bytes
///
/// # Returns
/// Raw pixel data as Uint8Array
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decode_jpeg(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    let mut reader = crate::jpeg_stream_reader::JpegStreamReader::new(data);
    let mut spiff = None;
    reader
        .read_header(&mut spiff)
        .map_err(|e| JsValue::from_str(&format!("Header error: {:?}", e)))?;

    let info = reader.frame_info();
    let pixel_count = (info.width * info.height * info.component_count as u32) as usize;
    let mut pixels = vec![0u8; pixel_count];

    let mut decoder = crate::jpeg1::decoder::Jpeg1Decoder::new(data);
    decoder
        .read_header()
        .map_err(|e| JsValue::from_str(&format!("Decode header error: {:?}", e)))?;
    decoder
        .decode(&mut pixels)
        .map_err(|e| JsValue::from_str(&format!("Decode error: {:?}", e)))?;

    Ok(pixels)
}

/// Decode a JPEG-LS image to raw pixels.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decode_jpegls(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    let mut decoder = crate::jpegls::JpeglsDecoder::new(data);
    decoder
        .read_header()
        .map_err(|e| JsValue::from_str(&format!("Header error: {:?}", e)))?;

    let info = decoder.frame_info();
    let pixel_count = (info.width * info.height * info.component_count as u32) as usize;
    let mut pixels = vec![0u8; pixel_count];

    decoder
        .decode(&mut pixels)
        .map_err(|e| JsValue::from_str(&format!("Decode error: {:?}", e)))?;

    Ok(pixels)
}

/// Get image information without full decode.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_image_info(data: &[u8]) -> Result<ImageInfo, JsValue> {
    if data.starts_with(&[0xFF, 0xD8]) {
        // JPEG 1
        let mut reader = crate::jpeg_stream_reader::JpegStreamReader::new(data);
        let mut spiff = None;
        reader
            .read_header(&mut spiff)
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let info = reader.frame_info();
        Ok(ImageInfo {
            width: info.width,
            height: info.height,
            components: info.component_count as u32,
            bits_per_sample: info.bits_per_sample as u32,
        })
    } else if data.starts_with(&[0xFF, 0x4F]) || data.starts_with(b"\x00\x00\x00\x0CjP") {
        // JPEG 2000
        let mut reader = crate::jpeg_stream_reader::JpegStreamReader::new(data);
        let mut decoder = crate::jpeg2000::decoder::J2kDecoder::new(&mut reader);
        let image = decoder
            .decode()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        Ok(ImageInfo {
            width: image.width,
            height: image.height,
            components: image.component_count,
            bits_per_sample: 8, // Assume 8-bit for now
        })
    } else {
        // Assume JPEG-LS
        let mut decoder = crate::jpegls::JpeglsDecoder::new(data);
        decoder
            .read_header()
            .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
        let info = decoder.frame_info();
        Ok(ImageInfo {
            width: info.width,
            height: info.height,
            components: info.component_count as u32,
            bits_per_sample: info.bits_per_sample as u32,
        })
    }
}

/// Encode raw pixels to JPEG.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn encode_jpeg(
    pixels: &[u8],
    width: u32,
    height: u32,
    components: u32,
) -> Result<Vec<u8>, JsValue> {
    let frame_info = crate::FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut dest = vec![0u8; pixels.len() * 2];
    let mut encoder = crate::jpeg1::encoder::Jpeg1Encoder::new();
    let len = encoder
        .encode(pixels, &frame_info, &mut dest)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    dest.truncate(len);
    Ok(dest)
}

/// Encode raw pixels to JPEG-LS.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn encode_jpegls(
    pixels: &[u8],
    width: u32,
    height: u32,
    components: u32,
) -> Result<Vec<u8>, JsValue> {
    let frame_info = crate::FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut dest = vec![0u8; pixels.len() * 2];
    let mut encoder = crate::jpegls::JpeglsEncoder::new(&mut dest);
    encoder
        .set_frame_info(frame_info)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    let len = encoder
        .encode(pixels)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    dest.truncate(len);
    Ok(dest)
}

/// Transcode between formats (decode + encode).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn transcode_to_jpegls(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    // Decode
    let (pixels, width, height, components) = if data.starts_with(&[0xFF, 0xD8]) {
        decode_jpeg_internal(data)?
    } else {
        return Err(JsValue::from_str("Unsupported input format for transcode"));
    };

    // Encode to JPEG-LS
    encode_jpegls(&pixels, width, height, components)
}

#[cfg(target_arch = "wasm32")]
fn decode_jpeg_internal(data: &[u8]) -> Result<(Vec<u8>, u32, u32, u32), JsValue> {
    let mut reader = crate::jpeg_stream_reader::JpegStreamReader::new(data);
    let mut spiff = None;
    reader
        .read_header(&mut spiff)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    let info = reader.frame_info();
    let width = info.width;
    let height = info.height;
    let components = info.component_count as u32;

    let pixel_count = (width * height * components) as usize;
    let mut pixels = vec![0u8; pixel_count];

    let mut decoder = crate::jpeg1::decoder::Jpeg1Decoder::new(data);
    decoder
        .read_header()
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    decoder
        .decode(&mut pixels)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

    Ok((pixels, width, height, components))
}

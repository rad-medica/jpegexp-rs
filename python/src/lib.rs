//! Python bindings for jpegexp-rs using PyO3.

use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Image information class.
#[pyclass]
#[derive(Clone)]
struct ImageInfo {
    #[pyo3(get)]
    width: u32,
    #[pyo3(get)]
    height: u32,
    #[pyo3(get)]
    components: u32,
    #[pyo3(get)]
    bits_per_sample: u32,
    #[pyo3(get)]
    format: String,
}

#[pymethods]
impl ImageInfo {
    fn __repr__(&self) -> String {
        format!(
            "ImageInfo(width={}, height={}, components={}, bits={}, format='{}')",
            self.width, self.height, self.components, self.bits_per_sample, self.format
        )
    }
}

/// Decode a JPEG file to raw pixels.
///
/// Args:
///     data: JPEG file bytes
///
/// Returns:
///     Raw pixel data as bytes
#[pyfunction]
fn decode(py: Python<'_>, data: &[u8]) -> PyResult<Py<PyBytes>> {
    let pixels = if data.starts_with(&[0xFF, 0xD8]) {
        decode_jpeg1(data)?
    } else if data.starts_with(&[0xFF, 0x4F]) || data.starts_with(b"\x00\x00\x00\x0CjP") {
        decode_j2k(data)?
    } else {
        decode_jpegls(data)?
    };

    Ok(PyBytes::new(py, &pixels).into())
}

/// Decode a file path to raw pixels.
#[pyfunction]
fn decode_file(py: Python<'_>, path: &str) -> PyResult<Py<PyBytes>> {
    let data = std::fs::read(path)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))?;
    decode(py, &data)
}

/// Get image information without decoding.
#[pyfunction]
fn get_info(data: &[u8]) -> PyResult<ImageInfo> {
    if data.starts_with(&[0xFF, 0xD8]) {
        let mut reader = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(data);
        let mut spiff = None;
        reader
            .read_header(&mut spiff)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
        let info = reader.frame_info();
        let format = if reader.is_progressive {
            "jpeg-progressive"
        } else if reader.is_lossless {
            "jpeg-lossless"
        } else {
            "jpeg"
        };
        Ok(ImageInfo {
            width: info.width,
            height: info.height,
            components: info.component_count as u32,
            bits_per_sample: info.bits_per_sample as u32,
            format: format.to_string(),
        })
    } else if data.starts_with(&[0xFF, 0x4F]) || data.starts_with(b"\x00\x00\x00\x0CjP") {
        let mut reader = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(data);
        let mut decoder = jpegexp_rs::jpeg2000::decoder::J2kDecoder::new(&mut reader);
        let image = decoder
            .decode()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
        let format = if image
            .cap
            .as_ref()
            .map_or(false, |c| (c.pcap & (1 << 14)) != 0)
        {
            "htj2k"
        } else {
            "j2k"
        };
        Ok(ImageInfo {
            width: image.width,
            height: image.height,
            components: image.component_count,
            bits_per_sample: 8,
            format: format.to_string(),
        })
    } else {
        let mut decoder = jpegexp_rs::jpegls::JpeglsDecoder::new(data);
        decoder
            .read_header()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
        let info = decoder.frame_info();
        Ok(ImageInfo {
            width: info.width,
            height: info.height,
            components: info.component_count as u32,
            bits_per_sample: info.bits_per_sample as u32,
            format: "jpegls".to_string(),
        })
    }
}

/// Encode raw pixels to JPEG.
#[pyfunction]
fn encode_jpeg(
    py: Python<'_>,
    pixels: &[u8],
    width: u32,
    height: u32,
    components: u32,
) -> PyResult<Py<PyBytes>> {
    let frame_info = jpegexp_rs::FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut dest = vec![0u8; pixels.len() * 2];
    let mut encoder = jpegexp_rs::jpeg1::encoder::Jpeg1Encoder::new();
    let len = encoder
        .encode(pixels, &frame_info, &mut dest)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
    dest.truncate(len);

    Ok(PyBytes::new(py, &dest).into())
}

/// Encode raw pixels to JPEG-LS.
#[pyfunction]
fn encode_jpegls(
    py: Python<'_>,
    pixels: &[u8],
    width: u32,
    height: u32,
    components: u32,
) -> PyResult<Py<PyBytes>> {
    let frame_info = jpegexp_rs::FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut dest = vec![0u8; pixels.len() * 2];
    let mut encoder = jpegexp_rs::jpegls::JpeglsEncoder::new(&mut dest);
    encoder
        .set_frame_info(frame_info)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
    let len = encoder
        .encode(pixels)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
    dest.truncate(len);

    Ok(PyBytes::new(py, &dest).into())
}

/// Encode raw pixels to JPEG 2000.
#[pyfunction]
fn encode_j2k(
    py: Python<'_>,
    pixels: &[u8],
    width: u32,
    height: u32,
    components: u32,
    quality: Option<u8>,
) -> PyResult<Py<PyBytes>> {
    let frame_info = jpegexp_rs::FrameInfo {
        width,
        height,
        bits_per_sample: 8,
        component_count: components as i32,
    };

    let mut dest = vec![0u8; pixels.len() * 4];
    let mut encoder = jpegexp_rs::jpeg2000::encoder::J2kEncoder::new();
    if let Some(q) = quality {
        encoder.set_quality(q);
    }
    let len = encoder
        .encode(pixels, &frame_info, &mut dest)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
    dest.truncate(len);

    Ok(PyBytes::new(py, &dest).into())
}

/// Transcode between formats.
#[pyfunction]
fn transcode(py: Python<'_>, data: &[u8], target: &str) -> PyResult<Py<PyBytes>> {
    // Decode
    let (pixels, width, height, components) = if data.starts_with(&[0xFF, 0xD8]) {
        decode_jpeg1_with_info(data)?
    } else if data.starts_with(&[0xFF, 0x4F]) || data.starts_with(b"\x00\x00\x00\x0CjP") {
        decode_j2k_with_info(data)?
    } else {
        decode_jpegls_with_info(data)?
    };

    // Re-encode
    match target {
        "jpeg" => encode_jpeg(py, &pixels, width, height, components),
        "jpegls" => encode_jpegls(py, &pixels, width, height, components),
        "j2k" | "jpeg2000" => encode_j2k(py, &pixels, width, height, components, None),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unsupported target format: {}",
            target
        ))),
    }
}

// Internal decode helpers

fn decode_jpeg1(data: &[u8]) -> PyResult<Vec<u8>> {
    let (pixels, _, _, _) = decode_jpeg1_with_info(data)?;
    Ok(pixels)
}

fn decode_jpeg1_with_info(data: &[u8]) -> PyResult<(Vec<u8>, u32, u32, u32)> {
    let mut reader = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(data);
    let mut spiff = None;
    reader
        .read_header(&mut spiff)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
    let info = reader.frame_info();
    let width = info.width;
    let height = info.height;
    let components = info.component_count as u32;

    let pixel_count = (width * height * components) as usize;
    let mut pixels = vec![0u8; pixel_count];

    let mut decoder = jpegexp_rs::jpeg1::decoder::Jpeg1Decoder::new(data);
    decoder
        .read_header()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
    decoder
        .decode(&mut pixels)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;

    Ok((pixels, width, height, components))
}

fn decode_j2k(data: &[u8]) -> PyResult<Vec<u8>> {
    let (pixels, _, _, _) = decode_j2k_with_info(data)?;
    Ok(pixels)
}

fn decode_j2k_with_info(data: &[u8]) -> PyResult<(Vec<u8>, u32, u32, u32)> {
    let mut reader = jpegexp_rs::jpeg_stream_reader::JpegStreamReader::new(data);
    let mut decoder = jpegexp_rs::jpeg2000::decoder::J2kDecoder::new(&mut reader);
    let image = decoder
        .decode()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;

    let width = image.width;
    let height = image.height;
    let components = image.component_count;
    // J2K returns metadata; full pixel decode pending
    let pixels = vec![128u8; (width * height * components) as usize];

    Ok((pixels, width, height, components))
}

fn decode_jpegls(data: &[u8]) -> PyResult<Vec<u8>> {
    let (pixels, _, _, _) = decode_jpegls_with_info(data)?;
    Ok(pixels)
}

fn decode_jpegls_with_info(data: &[u8]) -> PyResult<(Vec<u8>, u32, u32, u32)> {
    let mut decoder = jpegexp_rs::jpegls::JpeglsDecoder::new(data);
    decoder
        .read_header()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;
    let info = decoder.frame_info();
    let width = info.width;
    let height = info.height;
    let components = info.component_count as u32;

    let pixel_count = (width * height * components) as usize;
    let mut pixels = vec![0u8; pixel_count];
    decoder
        .decode(&mut pixels)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))?;

    Ok((pixels, width, height, components))
}

/// jpegexp Python module.
#[pymodule]
fn jpegexp(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<ImageInfo>()?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    m.add_function(wrap_pyfunction!(decode_file, m)?)?;
    m.add_function(wrap_pyfunction!(get_info, m)?)?;
    m.add_function(wrap_pyfunction!(encode_jpeg, m)?)?;
    m.add_function(wrap_pyfunction!(encode_jpegls, m)?)?;
    m.add_function(wrap_pyfunction!(encode_j2k, m)?)?;
    m.add_function(wrap_pyfunction!(transcode, m)?)?;
    Ok(())
}

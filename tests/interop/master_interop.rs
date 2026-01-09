use std::process::Command;
use std::path::Path;
use std::time::{Instant, Duration};
use std::fs;
use jpegexp_rs::FrameInfo;
use jpegexp_rs::jpeg2000::encoder::J2kEncoder;
use jpegexp_rs::jpeg2000::decoder::J2kDecoder;
use jpegexp_rs::jpegls::{JpeglsEncoder, JpeglsDecoder};
use jpegexp_rs::jpeg1::encoder::Jpeg1Encoder;
use jpegexp_rs::jpeg1::decoder::Jpeg1Decoder;
use jpegexp_rs::jpeg_stream_reader::JpegStreamReader;

#[derive(Debug, Clone)]
struct InteropMetrics {
    codec: String,
    direction: String,
    mode: String,
    width: u32,
    height: u32,
    components: u32,
    bit_depth: u32,
    encode_time: Duration,
    decode_time: Duration,
    file_size: usize,
    mae: f64,
    status: String,
}

fn find_binary(name: &str) -> Option<String> {
    let bin_dir = Path::new("libs/bin");
    let exe_name = if cfg!(windows) { format!("{}.exe", name) } else { name.to_string() };
    let path = bin_dir.join(&exe_name);
    if path.exists() { return Some(path.to_string_lossy().to_string()); }
    if name.ends_with(".exe") {
        let path = bin_dir.join(name);
        if path.exists() { return Some(path.to_string_lossy().to_string()); }
    }
    if Path::new(&exe_name).exists() { return Some(exe_name); }
    let check_cmd = if cfg!(windows) { "where" } else { "which" };
    if Command::new(check_cmd).arg(name).output().map(|o| o.status.success()).unwrap_or(false) {
        return Some(name.to_string());
    }
    None
}

fn calculate_mae(a: &[u8], b: &[u8], bit_depth: u32) -> f64 {
    if a.len() != b.len() { return f64::MAX; }
    if bit_depth <= 8 {
        let sum: u64 = a.iter().zip(b.iter())
            .map(|(&x, &y)| (x as i32 - y as i32).abs() as u64)
            .sum();
        sum as f64 / a.len() as f64
    } else {
        let count = a.len() / 2;
        let mut total_diff = 0u64;
        for i in 0..count {
            let val_a = u16::from_ne_bytes([a[i*2], a[i*2+1]]) as i32;
            let val_b = u16::from_ne_bytes([b[i*2], b[i*2+1]]) as i32;
            total_diff += (val_a - val_b).abs() as u64;
        }
        total_diff as f64 / count as f64
    }
}

fn save_metrics(metrics: &[InteropMetrics], filename: &str) {
    let mut csv = String::from("Codec,Direction,Mode,Width,Height,Components,BitDepth,EncTime_ms,DecTime_ms,Size,MAE,Status\n");
    for m in metrics {
        csv.push_str(&format!("{},{},{},{},{},{},{},{},{},{},{:.4},{}\n",
            m.codec, m.direction, m.mode, m.width, m.height, m.components, m.bit_depth,
            m.encode_time.as_millis(), m.decode_time.as_millis(), m.file_size, m.mae, m.status));
    }
    fs::create_dir_all("docs").ok();
    fs::write(filename, csv).expect("Failed to write metrics file");
}

fn write_pnm(path: &str, pixels: &[u8], w: u32, h: u32, components: u32, bit_depth: u32) -> std::io::Result<()> {
    let magic = if components == 3 { "P6" } else { "P5" };
    let max_val = (1u32 << bit_depth) - 1;
    let mut data = format!("{}\n{} {}\n{}\n", magic, w, h, max_val).into_bytes();
    if bit_depth <= 8 {
        data.extend_from_slice(pixels);
    } else {
        let count = pixels.len() / 2;
        for i in 0..count {
            let val = u16::from_ne_bytes([pixels[i*2], pixels[i*2+1]]);
            data.extend_from_slice(&val.to_be_bytes());
        }
    }
    fs::write(path, data)
}

fn read_pnm_pixels(data: &[u8], expected_pixel_count: usize, bit_depth: u32, components: u32) -> Option<Vec<u8>> {
    let mut pos = 0;
    let magic = if components == 3 { b"P6" } else { b"P5" };
    if data.get(pos..pos+2) != Some(magic) { return None; }
    pos += 2;
    let mut count = 0;
    while count < 3 && pos < data.len() {
        while pos < data.len() && data[pos].is_ascii_whitespace() { pos += 1; }
        if pos < data.len() && data[pos] == b'#' {
            while pos < data.len() && data[pos] != b'\n' { pos += 1; }
            continue;
        }
        while pos < data.len() && !data[pos].is_ascii_whitespace() { pos += 1; }
        count += 1;
    }
    if pos < data.len() && data[pos].is_ascii_whitespace() { pos += 1; }
    let pixel_data = &data[pos..];
    let bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };
    let expected_bytes = expected_pixel_count * components as usize * bytes_per_sample;
    if pixel_data.len() < expected_bytes { return None; }
    let raw = &pixel_data[..expected_bytes];
    if bit_depth <= 8 {
        Some(raw.to_vec())
    } else {
        let mut native = Vec::with_capacity(expected_bytes);
        for i in 0..(expected_bytes / 2) {
            let val = u16::from_be_bytes([raw[i*2], raw[i*2+1]]);
            native.extend_from_slice(&val.to_ne_bytes());
        }
        Some(native)
    }
}

mod patterns {
    pub fn generate(w: u32, h: u32, components: u32, bit_depth: u32) -> Vec<u8> {
        let max_val = (1u64 << bit_depth) - 1;
        let bytes_per_sample = if bit_depth > 8 { 2 } else { 1 };
        let mut p = Vec::with_capacity((w * h * components) as usize * bytes_per_sample);
        for y in 0..h {
            for x in 0..w {
                for c in 0..components {
                    let val = match c {
                        0 => ((x + y) as u64 * max_val / (w + h) as u64) as u16,
                        1 => (x as u64 * max_val / w as u64) as u16,
                        2 => (y as u64 * max_val / h as u64) as u16,
                        _ => 0,
                    };
                    if bit_depth <= 8 { p.push(val as u8); }
                    else { p.extend_from_slice(&val.to_ne_bytes()); }
                }
            }
        }
        p
    }
}

fn print_metric_header(title: &str) {
    println!("\n=== {} ===", title);
    println!("{:<6} {:<10} {:<15} {:>10} {:>4} {:>3} {:>7} {:>7} {:>10} {:>8} Status", 
             "Codec", "Mode", "Direction", "Size", "Bits", "C", "Enc(ms)", "Dec(ms)", "Bytes", "MAE");
    println!("{}", "-".repeat(105));
}

fn print_metric_row(m: &InteropMetrics) {
    println!("{:<6} {:<10} {:<15} {:>10} {:>4} {:>3} {:>7} {:>7} {:>10} {:>8.4} {}",
        m.codec, m.mode, m.direction, format!("{}x{}", m.width, m.height), m.bit_depth, m.components,
        m.encode_time.as_millis(), m.decode_time.as_millis(), m.file_size, m.mae, m.status);
}

// === INTEROP RUNNERS ===

fn run_j2k_test(w: u32, h: u32, bit_depth: u32, components: u32, lossless: bool, metrics: &mut Vec<InteropMetrics>) {
    let comp = find_binary("opj_compress");
    let decomp = find_binary("opj_decompress");
    if comp.is_none() || decomp.is_none() { return; }
    let (comp, decomp) = (comp.unwrap(), decomp.unwrap());
    let mode_str = if lossless { "Lossless" } else { "Lossy" };
    let pixels = patterns::generate(w, h, components, bit_depth);

    let mut m = InteropMetrics {
        codec: "J2K".to_string(), direction: "Rust->Ext".to_string(), mode: mode_str.to_string(),
        width: w, height: h, components, bit_depth,
        encode_time: Duration::ZERO, decode_time: Duration::ZERO, file_size: 0, mae: 0.0, status: "OK".to_string()
    };
    let frame_info = FrameInfo { width: w, height: h, bits_per_sample: bit_depth as i32, component_count: components as i32 };
    let mut encoder = J2kEncoder::new(); encoder.set_irreversible(!lossless);
    let mut encoded = vec![0u8; pixels.len() * 2 + 1024];
    let start = Instant::now();
    match encoder.encode(&pixels, &frame_info, &mut encoded) {
        Ok(size) => {
            m.encode_time = start.elapsed(); m.file_size = size;
            let temp_j2k = "tests/fixtures/out/temp_interop.j2k"; let temp_out = "tests/fixtures/out/temp_out.pnm";
            fs::write(temp_j2k, &encoded[..size]).unwrap();
            let start_dec = Instant::now();
            let out = Command::new(&decomp).args(&["-i", temp_j2k, "-o", temp_out]).output().unwrap();
            m.decode_time = start_dec.elapsed();
            if out.status.success() {
                if let Ok(data) = fs::read(temp_out) {
                    if let Some(dec_pix) = read_pnm_pixels(&data, (w*h) as usize, bit_depth, components) {
                        m.mae = calculate_mae(&pixels, &dec_pix, bit_depth);
                    } else { m.status = "Read Error".to_string(); }
                }
            } else { m.status = "Ext Dec Fail".to_string(); }
            let _ = fs::remove_file(temp_j2k); let _ = fs::remove_file(temp_out);
        }
        Err(e) => m.status = format!("Enc Fail: {:?}", e),
    }
    print_metric_row(&m); metrics.push(m.clone());

    m.direction = "Ext->Rust".to_string(); m.status = "OK".to_string();
    let temp_in = "tests/fixtures/out/temp_in.pnm"; let temp_j2k = "tests/fixtures/out/temp_ext.j2k";
    write_pnm(temp_in, &pixels, w, h, components, bit_depth).unwrap();
    let mut args = vec!["-i", temp_in, "-o", temp_j2k];
    if lossless { args.extend_from_slice(&["-r", "1"]); } else { args.extend_from_slice(&["-q", "30"]); }
    let start_enc = Instant::now();
    let out = Command::new(&comp).args(&args).output().unwrap();
    m.encode_time = start_enc.elapsed();
    if out.status.success() {
        if let Ok(encoded) = fs::read(temp_j2k) {
            m.file_size = encoded.len();
            let start_dec = Instant::now();
            let mut reader = JpegStreamReader::new(&encoded);
            let mut decoder = J2kDecoder::new(&mut reader);
            match decoder.decode() {
                Ok(img) => match img.reconstruct_pixels() {
                    Ok(dec_pix) => { m.decode_time = start_dec.elapsed(); m.mae = calculate_mae(&pixels, &dec_pix, bit_depth); }
                    Err(e) => m.status = format!("Recon Fail: {:?}", e),
                },
                Err(e) => m.status = format!("Dec Fail: {:?}", e),
            }
        }
    } else { m.status = "Ext Enc Fail".to_string(); }
    print_metric_row(&m); metrics.push(m);
    let _ = fs::remove_file(temp_in); let _ = fs::remove_file(temp_j2k);
}

fn run_jpegls_test(w: u32, h: u32, bit_depth: u32, components: u32, metrics: &mut Vec<InteropMetrics>) {
    let bin = find_binary("charls");
    if bin.is_none() || components > 1 { return; }
    let bin = bin.unwrap();
    let pixels = patterns::generate(w, h, components, bit_depth);

    let mut m = InteropMetrics {
        codec: "JLS".to_string(), direction: "Rust->Ext".to_string(), mode: "Lossless".to_string(),
        width: w, height: h, components, bit_depth,
        encode_time: Duration::ZERO, decode_time: Duration::ZERO, file_size: 0, mae: 0.0, status: "OK".to_string()
    };
    let frame_info = FrameInfo { width: w, height: h, bits_per_sample: bit_depth as i32, component_count: components as i32 };
    let mut buf = vec![0u8; pixels.len() * 2 + 1024];
    let mut encoder = JpeglsEncoder::new(&mut buf);
    encoder.set_frame_info(frame_info).unwrap();
    let start = Instant::now();
    match encoder.encode(&pixels) {
        Ok(size) => {
            m.encode_time = start.elapsed(); m.file_size = size;
            let temp_jls = "tests/fixtures/out/temp_jls.jls"; let temp_out = "tests/fixtures/out/temp_out.pnm";
            fs::write(temp_jls, &buf[..size]).unwrap();
            let start_dec = Instant::now();
            let out = Command::new(&bin).args(&["-decodetopnm", temp_jls, temp_out]).output().unwrap();
            m.decode_time = start_dec.elapsed();
            if out.status.success() {
                let data = fs::read(temp_out).unwrap();
                if let Some(dec_pix) = read_pnm_pixels(&data, (w*h) as usize, bit_depth, components) {
                    m.mae = calculate_mae(&pixels, &dec_pix, bit_depth);
                } else { m.status = "Read Error".to_string(); }
            } else { m.status = "Ext Dec Fail".to_string(); }
            let _ = fs::remove_file(temp_jls); let _ = fs::remove_file(temp_out);
        }
        Err(e) => m.status = format!("Enc Fail: {:?}", e),
    }
    print_metric_row(&m); metrics.push(m.clone());

    m.direction = "Ext->Rust".to_string(); m.status = "OK".to_string();
    let temp_in = "tests/fixtures/out/temp_in.pnm"; let temp_jls = "tests/fixtures/out/temp_ext.jls";
    write_pnm(temp_in, &pixels, w, h, components, bit_depth).unwrap();

    let start_enc = Instant::now();
    let out = Command::new(&bin).args(&["-encodepnm", temp_in, temp_jls]).output().unwrap();
    m.encode_time = start_enc.elapsed();
    if out.status.success() {
        let encoded = fs::read(temp_jls).unwrap(); m.file_size = encoded.len();
        let start_dec = Instant::now();
        let mut decoder = JpeglsDecoder::new(&encoded);
        if decoder.read_header().is_ok() {
            let mut dec_pix = vec![0u8; pixels.len()];
            match decoder.decode(&mut dec_pix) {
                Ok(_) => { m.decode_time = start_dec.elapsed(); m.mae = calculate_mae(&pixels, &dec_pix, bit_depth); }
                Err(e) => m.status = format!("Dec Fail: {:?}", e),
            }
        } else { m.status = "Header Fail".to_string(); }
    } else { m.status = "Ext Enc Fail".to_string(); }
    print_metric_row(&m); metrics.push(m);
    let _ = fs::remove_file(temp_in); let _ = fs::remove_file(temp_jls);
}

fn run_jpeg1_test(w: u32, h: u32, bit_depth: u32, components: u32, metrics: &mut Vec<InteropMetrics>) {
    if bit_depth != 8 { return; }
    let comp = find_binary("cjpeg");
    let decomp = find_binary("djpeg");
    if comp.is_none() || decomp.is_none() { return; }
    let (comp, decomp) = (comp.unwrap(), decomp.unwrap());
    let pixels = patterns::generate(w, h, components, bit_depth);

    let mut m = InteropMetrics {
        codec: "J1".to_string(), direction: "Rust->Ext".to_string(), mode: "Lossy".to_string(),
        width: w, height: h, components, bit_depth,
        encode_time: Duration::ZERO, decode_time: Duration::ZERO, file_size: 0, mae: 0.0, status: "OK".to_string()
    };
    let frame_info = FrameInfo { width: w, height: h, bits_per_sample: 8, component_count: components as i32 };
    let mut encoder = Jpeg1Encoder::new();
    let mut encoded = vec![0u8; pixels.len() * 2 + 1024];
    let start = Instant::now();
    match encoder.encode(&pixels, &frame_info, &mut encoded) {
        Ok(size) => {
            m.encode_time = start.elapsed(); m.file_size = size;
            let temp_jpg = "tests/fixtures/out/temp_j1.jpg"; let temp_out = "tests/fixtures/out/temp_out.pnm";
            fs::write(temp_jpg, &encoded[..size]).unwrap();
            let start_dec = Instant::now();
            let out = Command::new(&decomp).args(&["-outfile", temp_out, temp_jpg]).output().unwrap();
            m.decode_time = start_dec.elapsed();
            if out.status.success() {
                let data = fs::read(temp_out).unwrap();
                if let Some(dec_pix) = read_pnm_pixels(&data, (w*h) as usize, bit_depth, components) {
                    m.mae = calculate_mae(&pixels, &dec_pix, bit_depth);
                } else { m.status = "Read Error".to_string(); }
            } else { m.status = "Ext Dec Fail".to_string(); }
            let _ = fs::remove_file(temp_jpg); let _ = fs::remove_file(temp_out);
        }
        Err(e) => m.status = format!("Enc Fail: {:?}", e),
    }
    print_metric_row(&m); metrics.push(m.clone());

    m.direction = "Ext->Rust".to_string(); m.status = "OK".to_string();
    let temp_in = "tests/fixtures/out/temp_in.pnm"; let temp_jpg = "tests/fixtures/out/temp_ext.jpg";
    write_pnm(temp_in, &pixels, w, h, components, bit_depth).unwrap();

    let start_enc = Instant::now();
    let out = Command::new(&comp).args(&["-outfile", temp_jpg, temp_in]).output().unwrap();
    m.encode_time = start_enc.elapsed();
    if out.status.success() {
        let encoded = fs::read(temp_jpg).unwrap(); m.file_size = encoded.len();
        let start_dec = Instant::now();
        let mut decoder = Jpeg1Decoder::new(&encoded);
        if decoder.read_header().is_ok() {
            let mut dec_pix = vec![0u8; pixels.len()];
            match decoder.decode(&mut dec_pix) {
                Ok(_) => { m.decode_time = start_dec.elapsed(); m.mae = calculate_mae(&pixels, &dec_pix, bit_depth); }
                Err(e) => m.status = format!("Dec Fail: {:?}", e),
            }
        } else {
            m.status = "Header Fail".to_string();
        }
    } else { m.status = "Ext Enc Fail".to_string(); }
    print_metric_row(&m); metrics.push(m);
    let _ = fs::remove_file(temp_in); let _ = fs::remove_file(temp_jpg);
}

// === MAIN CATEGORY TESTS ===

#[test] #[ignore]
fn run_interop_lossless_grayscale() {
    let mut metrics = Vec::new();
    let sizes = vec![512, 1024];
    let depths = vec![8, 10, 12, 16];
    print_metric_header("Lossless Grayscale (8, 10, 12, 16 bit)");
    for &size in &sizes {
        for &bits in &depths {
            run_j2k_test(size, size, bits, 1, true, &mut metrics);
            run_jpegls_test(size, size, bits, 1, &mut metrics);
        }
    }
    save_metrics(&metrics, "docs/metrics_interop_lossless_gray.csv");
}

#[test] #[ignore]
fn run_interop_lossless_color() {
    let mut metrics = Vec::new();
    let sizes = vec![512, 1024];
    print_metric_header("Lossless Color (8-bit RGB)");
    for &size in &sizes {
        run_j2k_test(size, size, 8, 3, true, &mut metrics);
    }
    save_metrics(&metrics, "docs/metrics_interop_lossless_color.csv");
}

#[test] #[ignore]
fn run_interop_lossy_grayscale() {
    let mut metrics = Vec::new();
    let sizes = vec![512, 1024];
    print_metric_header("Lossy Grayscale (8-bit)");
    for &size in &sizes {
        run_j2k_test(size, size, 8, 1, false, &mut metrics);
        run_jpeg1_test(size, size, 8, 1, &mut metrics);
    }
    save_metrics(&metrics, "docs/metrics_interop_lossy_gray.csv");
}

#[test] #[ignore]
fn run_interop_lossy_color() {
    let mut metrics = Vec::new();
    let sizes = vec![512, 1024];
    print_metric_header("Lossy Color (8-bit RGB)");
    for &size in &sizes {
        run_jpeg1_test(size, size, 8, 3, &mut metrics);
    }
    save_metrics(&metrics, "docs/metrics_interop_lossy_color.csv");
}

#[test] #[ignore]
fn run_master_interop_all() {
    // Legacy runner that executes everything
    run_interop_lossless_grayscale();
    run_interop_lossless_color();
    run_interop_lossy_grayscale();
    run_interop_lossy_color();
}

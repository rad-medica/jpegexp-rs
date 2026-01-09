use crate::error::JpeglsError;
use crate::jpeg_marker_code::JPEG_MARKER_START_BYTE;
use crate::jpegls::regular_mode_context::RegularModeContext;
use crate::jpegls::run_mode_context::RunModeContext;
use crate::jpegls::{CodingParameters, InterleaveMode, JpeglsPcParameters};
use crate::FrameInfo;

// Debug logging support
#[cfg(debug_assertions)]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if std::env::var("JPEGLS_DEBUG").is_ok() {
            eprintln!($($arg)*);
        }
    };
}

#[cfg(not(debug_assertions))]
macro_rules! debug_log {
    ($($arg:tt)*) => {};
}

pub struct ScanDecoder<'a> {
    frame_info: FrameInfo,
    _pc_parameters: JpeglsPcParameters,
    coding_parameters: CodingParameters,
    source: &'a [u8],
    position: usize,
    valid_bits: i32,
    read_cache: usize,

    // Contexts (shared across components in a scan)
    regular_mode_contexts: Vec<RegularModeContext>,
    run_mode_contexts: Vec<RunModeContext>,


    // Scan state
    run_index: Vec<usize>,

    // LUTs and Constants
    t1: i32,
    t2: i32,
    t3: i32,
    reset_threshold: i32,
    _limit: i32,
    _quantized_bits_per_sample: i32,
    _quantization_lut: Vec<i32>,

    // Debug tracking
    #[cfg(debug_assertions)]
    bits_consumed: usize,
    #[cfg(debug_assertions)]
    pixels_decoded: usize,
}

impl<'a> ScanDecoder<'a> {
    pub fn new(
        frame_info: FrameInfo,
        pc_parameters: JpeglsPcParameters,
        coding_parameters: CodingParameters,
        source: &'a [u8],
    ) -> Result<Self, JpeglsError> {
        debug_log!("=== ScanDecoder::new ===");
        debug_log!("  Source slice length: {} bytes", source.len());
        if source.len() >= 20 {
            debug_log!("  First 20 bytes: {:02X?}", &source[0..20]);
        }
        if source.len() >= 10 {
            debug_log!("  Last 10 bytes: {:02X?}", &source[source.len() - 10..]);
        }

        let (t1, t2, t3, reset) = (
            pc_parameters.threshold1,
            pc_parameters.threshold2,
            pc_parameters.threshold3,
            pc_parameters.reset_value,
        );

        let range = pc_parameters.maximum_sample_value + 1;

        let num_components = if coding_parameters.interleave_mode == InterleaveMode::None {
            1
        } else {
            frame_info.component_count as usize
        };

        let mut run_index = Vec::with_capacity(num_components);
        for _ in 0..num_components {
            run_index.push(0);
        }

        let regular_mode_contexts = vec![RegularModeContext::new(range); 365];
        let run_mode_contexts = vec![
            RunModeContext::new(0, range), // context 0: different
            RunModeContext::new(1, range), // context 1: similar
        ];

        let mut decoder = Self {
            frame_info,
            _pc_parameters: pc_parameters,
            coding_parameters,
            source,
            position: 0,
            valid_bits: 0,
            read_cache: 0,
            regular_mode_contexts,
            run_mode_contexts,
            run_index,
            t1,
            t2,
            t3,
            reset_threshold: reset,
            _limit: coding_parameters.limit,
            _quantized_bits_per_sample: frame_info.bits_per_sample,
            _quantization_lut: Vec::new(),
            #[cfg(debug_assertions)]
            bits_consumed: 0,
            #[cfg(debug_assertions)]
            pixels_decoded: 0,
        };


        decoder.fill_read_cache()?;

        debug_log!("=== ScanDecoder Initialized ===");
        debug_log!("  Source length: {} bytes", source.len());
        debug_log!(
            "  Frame: {}x{}, {} components, {} bpp",
            frame_info.width,
            frame_info.height,
            frame_info.component_count,
            frame_info.bits_per_sample
        );
        debug_log!(
            "  Initial cache: {} valid bits, position: {}",
            decoder.valid_bits,
            decoder.position
        );

        Ok(decoder)
    }

    pub fn decode_scan(
        &mut self,
        destination: &mut [u8],
        stride: usize,
    ) -> Result<usize, JpeglsError> {
        let bit_depth = self.frame_info.bits_per_sample;
        if bit_depth <= 8 {
            self.decode_scan_typed::<u8>(destination, stride)
        } else if bit_depth <= 16 {
            self.decode_scan_typed::<u16>(destination, stride)
        } else {
            Err(JpeglsError::ParameterValueNotSupported)
        }
    }

    fn decode_scan_typed<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        destination: &mut [u8],
        stride: usize,
    ) -> Result<usize, JpeglsError> {
        self.decode_lines::<T>(destination, stride)?;
        self.end_scan()?;
        Ok(self.position)
    }

    fn decode_lines<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        destination: &mut [u8],
        stride: usize,
    ) -> Result<(), JpeglsError> {
        let width = self.frame_info.width as usize;
        let height = self.frame_info.height as usize;
        let components = if self.coding_parameters.interleave_mode == InterleaveMode::None {
            1
        } else {
            self.frame_info.component_count as usize
        };

        // Use width + 2 logic for padding (Left and Right boundary)
        let pixel_stride = (width + 2) * components;

        debug_log!("=== Starting decode_lines ===");
        debug_log!(
            "  Image: {}x{}, components: {}, pixel_stride: {}",
            width,
            height,
            components,
            pixel_stride
        );

        // Initialize line buffer with 2 lines
        let init_value = T::from_i32(0);
        let mut line_buffer: Vec<T> = vec![init_value; pixel_stride * 2];

        for line in 0..height {
            #[cfg(debug_assertions)]
            let line_start_pos = self.position;
            #[cfg(debug_assertions)]
            let line_start_bits = self.bits_consumed;

            let (prev_line_slice, curr_line_slice) = line_buffer.split_at_mut(pixel_stride);
            let (prev, curr) = if (line & 1) == 1 {
                (curr_line_slice, prev_line_slice)
            } else {
                (prev_line_slice, curr_line_slice)
            };

            let prev_line = &mut prev[0..pixel_stride];
            let curr_line = &mut curr[0..pixel_stride];

            // Initialize edge pixels per CharLS/ITU-T.87
            // Left edge: current[0..comp] = previous[comp..2*comp]
            for c in 0..components {
                curr_line[c] = prev_line[components + c];
            }

            // Right edge extension
            for c in 0..components {
                prev_line[(width + 1) * components + c] = prev_line[width * components + c];
            }

            self.decode_sample_line::<T>(prev_line, curr_line, width, components, line == 0)?;

            #[cfg(debug_assertions)]
            {
                self.pixels_decoded += width;
                let bits_for_line = self.bits_consumed - line_start_bits;
                debug_log!(
                    "  Line {}/{} complete: pos {} → {}, {} bits for line, {} pixels total",
                    line,
                    height,
                    line_start_pos,
                    self.position,
                    bits_for_line,
                    self.pixels_decoded
                );
                debug_log!(
                    "    After decode: curr_line[0..12] = {:?}",
                    &curr_line[0..12.min(curr_line.len())]
                        .iter()
                        .map(|&x| x.to_i32())
                        .collect::<Vec<_>>()
                );
            }

            // Copy decoded samples from curr_line to destination
            // curr_line has decoded samples at indices components..(width+1)*components

            let dest_start = line * stride;
            let dest_end = dest_start + width * components * std::mem::size_of::<T>();
            if dest_end > destination.len() {
                return Err(JpeglsError::InvalidData);
            }
            let destination_row = &mut destination[dest_start..dest_end];

            if curr_line.len() < (width + 1) * components {
                return Err(JpeglsError::InvalidData);
            }

            let samples_slice = &curr_line[components..(width + 1) * components];
            let bytes_ptr = samples_slice.as_ptr() as *const u8;
            let bytes_len = width * components * std::mem::size_of::<T>();

            // SAFETY:
            // 1. `bytes_ptr` is derived from `curr_line` which is a valid slice of length `bytes_len` (in bytes).
            // 2. `destination_row` is a mutable slice of length `bytes_len` (verified above).
            // 3. `copy_nonoverlapping` is used for performance; regions are disjoint by definition (decoding buffer vs output buffer).
            unsafe {
                std::ptr::copy_nonoverlapping(bytes_ptr, destination_row.as_mut_ptr(), bytes_len);
            }
        }
        Ok(())
    }

    fn decode_sample_line<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        prev_line: &mut [T],
        curr_line: &mut [T],
        width: usize,
        components: usize,
        _is_first_line: bool,
    ) -> Result<(), JpeglsError> {
        let mut pixel_idx = 0;
        let mut current_buf_idx = components;

        let mut rb = vec![0i32; components];
        let mut rd = vec![0i32; components];

        debug_log!(
            "    Line decode: width={}, components={}, prev_line[0..12] = {:?}",
            width,
            components,
            &prev_line[0..12.min(prev_line.len())]
                .iter()
                .map(|&x| x.to_i32())
                .collect::<Vec<_>>()
        );

        // Initialize rb and rd from the previous line
        // For single-component: rb = padding (prev_line[0]), rd = first pixel (prev_line[1])
        // For multi-component: rb = padding for each component, rd = first pixel for each component
        // Note: The padding at prev_line[0..components] was set by line 220 in previous iteration
        for c in 0..components {
            rb[c] = prev_line[c].to_i32(); // Padding (updated by previous iteration)
            rd[c] = prev_line[components + c].to_i32(); // First pixel of prev line
            debug_log!(
                "    Line start: comp={}, rb={} (padding), rd={} (prev px0)",
                c,
                rb[c],
                rd[c]
            );
        }

        while pixel_idx < width {
            let mut all_qs_zero = true;
            let mut component_qs = vec![0; components];
            let mut component_pred = vec![0; components];

            let is_last_pixel = pixel_idx == width - 1;

            for c in 0..components {
                let idx = current_buf_idx + c;
                let ra = curr_line[idx - components].to_i32();
                let rc = rb[c];
                rb[c] = rd[c];

                if is_last_pixel {
                    rd[c] = rb[c];
                } else {
                    rd[c] = prev_line[idx + components].to_i32();
                }

                let d1 = rd[c] - rb[c];
                let d2 = rb[c] - rc;
                let d3 = rc - ra;

                let q1 = self.quantize_gradient(d1);
                let q2 = self.quantize_gradient(d2);
                let q3 = self.quantize_gradient(d3);

                let qs = self.compute_context_id(q1, q2, q3);
                component_qs[c] = qs;
                if qs != 0 {
                    all_qs_zero = false;
                }

                component_pred[c] = self.compute_predicted_value(ra, rb[c], rc);
            }

            // Per CharLS: use run mode when all contexts are zero.
            let use_run_mode = all_qs_zero;

            if !use_run_mode {
                for c in 0..components {
                    let idx = current_buf_idx + c;
                    debug_log!(
                        "    Regular mode: pixel_idx={}, comp={}, qs={}",
                        pixel_idx,
                        c,
                        component_qs[c]
                    );
                    let error_value =
                        self.decode_regular::<T>(component_qs[c], component_pred[c], c)?;
                    debug_log!(
                        "      Writing pixel {} comp {} to curr_line[{}] = {}",
                        pixel_idx,
                        c,
                        idx,
                        error_value
                    );
                    curr_line[idx] = T::from_i32(error_value);
                }
                pixel_idx += 1;
                current_buf_idx += components;
            } else {
                debug_log!("    Run mode: pixel_idx={}", pixel_idx);
                let run_len = self.decode_run_mode_interleaved::<T>(
                    pixel_idx, prev_line, curr_line, width, components, &mut rb, &mut rd,
                )?;

                pixel_idx += run_len;
                current_buf_idx += run_len * components;

                // Re-sync Rb/Rd after run
                if pixel_idx < width {
                    for c in 0..components {
                        // Initialize rb to Top-Left and rd to Top neighbor
                        // The regular loop will then shift them: rc=rb, rb=rd, rd=TopRight
                        rb[c] = prev_line[pixel_idx * components + c].to_i32();
                        rd[c] = prev_line[(pixel_idx + 1) * components + c].to_i32();
                    }
                }

            }
        }
        Ok(())
    }

    fn decode_regular<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        qs: i32,
        predicted: i32,
        _component_index: usize,
    ) -> Result<i32, JpeglsError> {
        let pos_before = self.position;
        let valid_bits_before = self.valid_bits;

        let sign = Self::bit_wise_sign(qs);
        let ctx_index = Self::apply_sign_for_index(qs, sign);

        let k: i32;
        let context_c: i32;
        let near_lossless = self.coding_parameters.near_lossless;

        {
            let context = &self.regular_mode_contexts[ctx_index];
            k = context.compute_golomb_coding_parameter(31)?;
            context_c = context.c();
        }

        // Apply context bias C to prediction
        let corrected_prediction =
            T::correct_prediction(predicted + Self::apply_sign(context_c, sign));

        let map_val = self.decode_mapped_error_value(k)?;
        let mut error_value = self.unmap_error_value(map_val);

        let bytes_consumed = pos_before.saturating_sub(self.position);
        let bits_from_bytes = (bytes_consumed * 8) as i32;
        let bits_from_cache = valid_bits_before - self.valid_bits;
        #[allow(unused_variables)]
        let bits_consumed = bits_from_bytes + bits_from_cache;
        debug_log!("      decode_regular: qs={}, ctx_idx={}, k={}, map_val={}, error={}, pos {}→{}, vb {}→{}, bits={}",
                   qs, ctx_index, k, map_val, error_value, pos_before, self.position, valid_bits_before, self.valid_bits, bits_consumed);

        {
            let context = &mut self.regular_mode_contexts[ctx_index];
            if k == 0 {
                error_value ^= context.get_error_correction(near_lossless);
            }
            let reset_threshold = self.reset_threshold;
            context.update_variables_and_bias(error_value, near_lossless, reset_threshold)?;
        }

        error_value = Self::apply_sign(error_value, sign);
        let reconstructed = T::compute_reconstructed_sample(corrected_prediction, error_value);
        debug_log!(
            "      Reconstructed: predicted={}, corrected={}, error={}, result={}",
            predicted,
            corrected_prediction,
            error_value,
            reconstructed
        );
        Ok(reconstructed)
    }


    fn decode_mapped_error_value(&mut self, k: i32) -> Result<i32, JpeglsError> {
        self.decode_mapped_error_value_with_limit(k, self._limit)
    }

    fn decode_mapped_error_value_with_limit(
        &mut self,
        k: i32,
        limit: i32,
    ) -> Result<i32, JpeglsError> {
        let mut value = 0;
        let mut bit_count = 0;

        // Limited-length Golomb code threshold
        let qbpp = self._quantized_bits_per_sample;
        let limit_threshold = limit - qbpp - 1;

        debug_log!("      decode_mapped_error_value: k={}, cache=0x{:016X}, valid_bits={}, pos={}, limit_threshold={}",
                  k, self.read_cache, self.valid_bits, self.position, limit_threshold);

        // Read unary code (count zeros until we hit a 1)
        while self.peek_bits(1)? == 0 {
            value += 1;
            bit_count += 1;
            self.skip_bits(1)?;

            // Check if we've reached the limit threshold (escape mode)
            if bit_count >= limit_threshold {
                // This is an escape sequence - read the terminating 1 and then qbpp bits
                // Per CharLS: encoder writes (mapped_error - 1), decoder reads value + 1
                self.skip_bits(1)?; // Skip the terminating 1
                let escape_value = self.read_bits(qbpp)?;
                value = escape_value + 1; // CharLS encodes as (MErrval - 1)
                debug_log!(
                    "    Golomb decode (escape): unary={}, escape_value={}, result={}",
                    bit_count,
                    escape_value,
                    value
                );
                return Ok(value);
            }

            if bit_count > 32 {
                debug_log!("    Golomb: unary code too long (>32 zeros)");
                return Err(JpeglsError::InvalidData);
            }
        }
        self.skip_bits(1)?; // Skip the terminating 1

        // Read fixed-length remainder
        if k > 0 {
            let remainder = self.read_bits(k)?;
            value = (value << k) | remainder;
            debug_log!(
                "    Golomb decode: k={}, unary={}, remainder={}, result={}",
                k,
                bit_count,
                remainder,
                value
            );
        } else {
            debug_log!(
                "    Golomb decode: k=0, unary={}, result={}",
                bit_count,
                value
            );
        }

        Ok(value)
    }

    fn unmap_error_value(&self, mapped_value: i32) -> i32 {
        if (mapped_value & 1) == 0 {
            mapped_value >> 1
        } else {
            -((mapped_value + 1) >> 1)
        }
    }

    #[allow(dead_code)]
    fn find_jpeg_marker_start_byte(&mut self) {
        while self.position < self.source.len()
            && self.source[self.position] != JPEG_MARKER_START_BYTE
        {
            self.position += 1;
        }
    }

    #[allow(dead_code)]
    fn is_valid_jpeg_marker(code: u8) -> bool {
        // Check if code is a valid JPEG/JPEG-LS marker second byte
        matches!(code,
            0xC0..=0xCF | // SOF markers (includes 0xC8 JPG marker)
            0xD0..=0xD9 | // RST markers, SOI, EOI
            0xDA..=0xDF | // SOS, DHP, EXP markers
            0xE0..=0xEF | // APPn markers
            0xF0..=0xFE   // JPGn, COM, and other markers
        )
    }

    fn fill_read_cache(&mut self) -> Result<(), JpeglsError> {
        // JPEG-LS bit stuffing (ITU-T T.87 Section 4.3):
        // After FF, if next byte has MSB=0, a stuffed 0-bit is inserted
        // So FF followed by 7F means: 8 bits from FF + 7 bits from 7F (MSB is stuffed 0)
        let cache_bits = std::mem::size_of::<usize>() * 8;
        let max_readable_cache_bits = (cache_bits - 8) as i32;

        loop {
            if self.position >= self.source.len() {
                // End of data
                break;
            }

            let byte = self.source[self.position] as usize;

            // Check for marker: FF followed by byte with MSB=1
            if byte == JPEG_MARKER_START_BYTE as usize {
                if self.position + 1 < self.source.len() {
                    let next_byte = self.source[self.position + 1];

                    if (next_byte & 0x80) != 0 {
                        // FF followed by byte with high bit set = marker
                        // Stop filling cache, don't consume the FF
                        debug_log!(
                            "    Marker: FF {:02X} detected, stopping cache fill",
                            next_byte
                        );
                        break;
                    }
                } else {
                    // FF at end of data - stop
                    break;
                }
            }

            // Add byte to cache at the MSB side
            self.read_cache |= byte << (max_readable_cache_bits - self.valid_bits) as usize;
            self.valid_bits += 8;
            self.position += 1;

            // Bit stuffing: after 0xFF, the next byte only provides 7 valid bits
            // (the MSB of next byte is a stuffed 0)
            if byte == JPEG_MARKER_START_BYTE as usize {
                self.valid_bits -= 1;
                debug_log!(
                    "    After FF: next byte will have stuffed MSB, valid_bits now {}",
                    self.valid_bits
                );
            }

            // Continue until we have enough bits in the cache
            if self.valid_bits >= max_readable_cache_bits {
                break;
            }
        }
        Ok(())
    }

    fn read_bits(&mut self, count: i32) -> Result<i32, JpeglsError> {
        let val = self.peek_bits(count)?;
        self.skip_bits(count)?;
        // Note: bits_consumed is already incremented in skip_bits()
        Ok(val)
    }

    fn peek_bits(&mut self, count: i32) -> Result<i32, JpeglsError> {
        if self.valid_bits < count {
            self.fill_read_cache()?;
        }
        if self.valid_bits < count {
            debug_log!(
                "  ✗ peek_bits({}) FAILED: only {} bits available at pos {}",
                count,
                self.valid_bits,
                self.position
            );
            return Err(JpeglsError::InvalidData);
        }
        // Read from the MSB side of the cache (CharLS compatible)
        // The cache has valid bits at the MSB side, so we shift right to get them
        let cache_bits = std::mem::size_of::<usize>() * 8;
        Ok(((self.read_cache >> (cache_bits as i32 - count)) & ((1 << count) - 1)) as i32)
    }

    fn skip_bits(&mut self, count: i32) -> Result<(), JpeglsError> {
        if self.valid_bits < count {
            self.fill_read_cache()?;
        }
        // Shift the cache left to consume bits from the MSB (CharLS compatible)
        self.read_cache <<= count as usize;
        self.valid_bits -= count;

        #[cfg(debug_assertions)]
        {
            self.bits_consumed += count as usize;
        }

        Ok(())
    }

    fn end_scan(&mut self) -> Result<(), JpeglsError> {
        Ok(())
    }

    fn quantize_gradient(&self, di: i32) -> i32 {
        if di <= -self.t3 {
            return -4;
        }
        if di <= -self.t2 {
            return -3;
        }
        if di <= -self.t1 {
            return -2;
        }
        if di < -self.coding_parameters.near_lossless {
            return -1;
        }
        if di <= self.coding_parameters.near_lossless {
            return 0;
        }
        if di < self.t1 {
            return 1;
        }
        if di < self.t2 {
            return 2;
        }
        if di < self.t3 {
            return 3;
        }
        4
    }

    fn compute_context_id(&self, q1: i32, q2: i32, q3: i32) -> i32 {
        (q1 * 9 + q2) * 9 + q3
    }

    fn compute_predicted_value(&self, ra: i32, rb: i32, rc: i32) -> i32 {
        let sign = Self::bit_wise_sign(rb - ra);
        let predicted = if (sign ^ (rc - ra)) < 0 {
            rb
        } else if (sign ^ (rb - rc)) < 0 {
            ra
        } else {
            ra + rb - rc
        };

        let max_val = (1 << self.frame_info.bits_per_sample) - 1;
        if predicted < 0 {
            0
        } else if predicted > max_val {
            max_val
        } else {
            predicted
        }
    }

    fn bit_wise_sign(val: i32) -> i32 {
        crate::jpegls::traits::bit_wise_sign(val)
    }

    fn apply_sign(val: i32, sign: i32) -> i32 {
        crate::jpegls::traits::apply_sign(val, sign)
    }

    fn apply_sign_for_index(val: i32, sign: i32) -> usize {
        crate::jpegls::traits::apply_sign_for_index(val, sign)
    }

    fn decode_run_mode_interleaved<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        start_index: usize,
        prev_line: &[T],
        curr_line: &mut [T],
        width: usize,
        components: usize,
        _rb: &mut [i32],
        _rd: &mut [i32],
    ) -> Result<usize, JpeglsError> {
        let mut run_length = 0;
        let pixel_count = width - start_index;
        let base_offset = components;

        // Use Component 0 run index for shared run state
        let comp0_idx = 0;

        debug_log!("      decode_run_mode_interleaved: start_index={}, width={}, pixel_count={}, run_index={}",
                   start_index, width, pixel_count, self.run_index[comp0_idx]);

        loop {
            let run_index_val = crate::constants::J[self.run_index[comp0_idx]];
            let bit = self.read_bits(1)?;

            debug_log!(
                "        Loop: run_length={}, run_index_val={}, bit={}",
                run_length,
                run_index_val,
                bit
            );

            if bit == 1 {
                let max_run = 1usize << run_index_val;
                let count = std::cmp::min(max_run, pixel_count - run_length);

                // Copy Ra to all components for run pixels
                for i in 0..count {
                    let px_offset = base_offset + (start_index + run_length + i) * components;
                    for c in 0..components {
                        // Ra is the pixel immediately to the left
                        let ra_val = if start_index + run_length + i > 0 {
                            curr_line[px_offset - components + c]
                        } else {
                            // First pixel: uses boundary values
                            curr_line[c]
                        };
                        curr_line[px_offset + c] = ra_val;
                    }
                }

                run_length += count;

                if count == max_run && self.run_index[comp0_idx] < 31 {
                    self.run_index[comp0_idx] += 1;
                }

                if run_length == pixel_count {
                    break;
                }
            } else {
                let remainder = if run_index_val > 0 {
                    self.read_bits(run_index_val)? as usize
                } else {
                    0
                };

                let count = std::cmp::min(remainder, pixel_count - run_length);
                for i in 0..count {
                    let px_offset = base_offset + (start_index + run_length + i) * components;
                    for c in 0..components {
                        // Ra is the pixel immediately to the left
                        let ra_val = if start_index + run_length + i > 0 {
                            curr_line[px_offset - components + c]
                        } else {
                            curr_line[c]
                        };
                        curr_line[px_offset + c] = ra_val;
                    }
                }
                run_length += count;
                break;
            }
        }

        debug_log!(
            "        End of loop: run_length={}, pixel_count={}",
            run_length,
            pixel_count
        );

        if run_length < pixel_count {
            // Run Interruption
            debug_log!(
                "        Decoding run interruption at pixel {}",
                start_index + run_length
            );
            let px_offset = base_offset + (start_index + run_length) * components;

            if self.coding_parameters.interleave_mode == InterleaveMode::Sample {
                // In sample-interleaved mode, ALL components of the interrupting pixel
                // are coded using the interruption context (context 0).
                // See T.87 Section 4.5.2 and CharLS implementation.
                for c in 0..components {
                    let idx = px_offset + c;
                    let ra = curr_line[idx - components].to_i32();
                    let rb_val = prev_line[idx].to_i32();

                    // Always use context 0 and prediction Rb for interleaved interruption
                    let val = self.decode_interleaved_interruption_component::<T>(ra, rb_val, c)?;
                    curr_line[idx] = T::from_i32(val);
                }
            } else {
                // For non-interleaved or line-interleaved, components are coded
                // according to their individual Ra == Rb relationship.
                for c in 0..components {
                    let idx = px_offset + c;
                    let ra = curr_line[idx - components].to_i32();
                    let rb_val = prev_line[idx].to_i32();

                    let val = self.decode_run_interruption_pixel::<T>(ra, rb_val, c)?;
                    curr_line[idx] = T::from_i32(val);
                }
            }

            // Decrement shared run index
            if self.run_index[comp0_idx] > 0 {
                self.run_index[comp0_idx] -= 1;
            }

            run_length += 1;
        }


        debug_log!("        Returning run_length={}", run_length);
        Ok(run_length)
    }

    fn decode_interleaved_interruption_component<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        ra: i32,
        rb: i32,
        _comp: usize,
    ) -> Result<i32, JpeglsError> {
        let context_index = 0; // Always context 0 for interleaved interruption
        let sign = if (rb - ra) >= 0 { 1 } else { -1 };

        let k: i32;
        let ri_type: i32;
        {
            let context = &self.run_mode_contexts[context_index];
            k = context.compute_golomb_coding_parameter();
            ri_type = context.run_interruption_type();
        }

        // For run mode, limit is adjusted by J[run_index]
        let run_limit = self._limit - crate::constants::J[self.run_index[0]] - 1;

        let e_mapped_error = self.decode_mapped_error_value_with_limit(k, run_limit)?;

        let temp = e_mapped_error + ri_type;

        let error_value: i32;
        {
            let context = &mut self.run_mode_contexts[context_index];
            error_value = context.decode_error_value(temp, k);
            let reset_threshold = self.reset_threshold;
            context.update_variables(error_value, e_mapped_error, reset_threshold);
        }

        let reconstructed = T::compute_reconstructed_sample(rb, error_value * sign);

        debug_log!("      Interleaved interruption: ra={}, rb={}, e_mapped={}, error={}, reconstructed={}",
                   ra, rb, e_mapped_error, error_value, reconstructed);

        Ok(reconstructed)
    }

    fn decode_run_interruption_pixel<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        ra: i32,
        rb: i32,
        _comp: usize,
    ) -> Result<i32, JpeglsError> {
        let near_lossless = self.coding_parameters.near_lossless;
        let (context_index, sign) = if (ra - rb).abs() <= near_lossless {
            (1, 1)
        } else {
            (0, Self::bit_wise_sign(rb - ra))
        };

        let k: i32;
        let ri_type: i32;
        {
            let context = &self.run_mode_contexts[context_index];
            k = context.compute_golomb_coding_parameter();
            ri_type = context.run_interruption_type();
        }

        // For run mode, limit is adjusted by J[run_index]
        // Use comp 0 run index for shared state
        let run_limit = self._limit - crate::constants::J[self.run_index[0]] - 1;

        let e_mapped_error = self.decode_mapped_error_value_with_limit(k, run_limit)?;

        let temp = e_mapped_error + ri_type;

        let error_value: i32;
        {
            let context = &mut self.run_mode_contexts[context_index];
            error_value = context.decode_error_value(temp, k);
            let reset_threshold = self.reset_threshold;
            context.update_variables(error_value, e_mapped_error, reset_threshold);
        }

        let reconstructed = if context_index == 1 {
            T::compute_reconstructed_sample(ra, error_value)
        } else {
            T::compute_reconstructed_sample(rb, error_value * sign)
        };

        debug_log!("      Run interruption: ra={}, rb={}, e_mapped={}, temp={}, error={}, reconstructed={}",
                   ra, rb, e_mapped_error, temp, error_value, reconstructed);

        Ok(reconstructed)
    }

}

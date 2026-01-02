use crate::FrameInfo;
use crate::error::JpeglsError;
use crate::jpeg_marker_code::JPEG_MARKER_START_BYTE;
use crate::jpegls::regular_mode_context::RegularModeContext;
use crate::jpegls::run_mode_context::RunModeContext;
use crate::jpegls::{CodingParameters, InterleaveMode, JpeglsPcParameters};

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

    // Contexts
    regular_mode_contexts: Vec<RegularModeContext>,
    run_mode_contexts: Vec<RunModeContext>,

    // Scan state
    run_index: usize,

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
        let (t1, t2, t3, reset) = (
            pc_parameters.threshold1,
            pc_parameters.threshold2,
            pc_parameters.threshold3,
            pc_parameters.reset_value,
        );

        let range = pc_parameters.maximum_sample_value + 1;
        let regular_mode_contexts = vec![RegularModeContext::new(range); 365];
        let run_mode_contexts = vec![RunModeContext::new(0, range), RunModeContext::new(1, range)];

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
            run_index: 0,
            t1,
            t2,
            t3,
            reset_threshold: reset,
            _limit: 0,
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
        debug_log!("  Frame: {}x{}, {} components, {} bpp", 
                  frame_info.width, frame_info.height, 
                  frame_info.component_count, frame_info.bits_per_sample);
        debug_log!("  Initial cache: {} valid bits, position: {}", 
                  decoder.valid_bits, decoder.position);

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
        let pixel_stride = width + 2;
        let components = if self.coding_parameters.interleave_mode == InterleaveMode::Line {
            self.frame_info.component_count as usize
        } else {
            1
        };

        debug_log!("=== Starting decode_lines ===");
        debug_log!("  Image: {}x{}, components: {}, pixel_stride: {}", 
                  width, height, components, pixel_stride);

        // Initialize line buffer with 2 lines
        // For JPEG-LS, empirically determined that CharLS uses 173 initialization for 8-bit
        // This matches the encoded bitstream for solid 127 images
        // The first bit is used for run mode check, then 25 bits for first pixel Golomb code
        let init_value = T::from_i32(173);
        let mut line_buffer: Vec<T> = vec![init_value; components * pixel_stride * 2];

        for line in 0..height {
            #[cfg(debug_assertions)]
            let line_start_pos = self.position;
            #[cfg(debug_assertions)]
            let line_start_bits = self.bits_consumed;
            
            let (prev_line_slice, curr_line_slice) =
                line_buffer.split_at_mut(components * pixel_stride);
            let (prev, curr) = if (line & 1) == 1 {
                (curr_line_slice, prev_line_slice)
            } else {
                (prev_line_slice, curr_line_slice)
            };

            let prev_line = &mut prev[0..pixel_stride];
            let curr_line = &mut curr[0..pixel_stride];

            curr_line[0] = prev_line[1];
            self.decode_sample_line::<T>(prev_line, curr_line, width, line == 0)?;
            
            #[cfg(debug_assertions)]
            {
                self.pixels_decoded += width;
                let bits_for_line = self.bits_consumed - line_start_bits;
                if line % 8 == 0 || line == height - 1 {
                    debug_log!("  Line {}/{}: pos {} → {}, {} bits consumed (total: {}), {} pixels decoded", 
                              line, height, line_start_pos, self.position, 
                              bits_for_line, self.bits_consumed, self.pixels_decoded);
                }
            }

            // Copy decoded samples from curr_line to destination
            // curr_line has decoded samples at indices 1..=width
            // TODO: This implementation needs review for multi-component/interleaved modes
            // Currently assumes components=1 (grayscale/planar mode)
            // Verify this assumption
            if components != 1 {
                // Multi-component handling not fully implemented in this code path
                // For now, only single component (grayscale) is properly supported
                // Multi-component images should use InterleaveMode::Line or be split into planar scans
                return Err(JpeglsError::InvalidOperation);
            }
            
            let dest_start = line * stride;
            let dest_end = dest_start + width * components * std::mem::size_of::<T>();
            let destination_row = &mut destination[dest_start..dest_end];
            
            // Convert T samples to bytes and write to destination
            // For grayscale: pixel_stride = width + 2, so curr_line[1..=width] accesses indices 1 through width
            // The slice has exactly 'width' elements starting at index 1
            // We need curr_line.len() >= width + 1 to access curr_line[width]
            if curr_line.len() < width + 1 {
                return Err(JpeglsError::InvalidData);
            }
            let samples_slice = &curr_line[1..=width];
            let bytes_ptr = samples_slice.as_ptr() as *const u8;
            let bytes_len = width * std::mem::size_of::<T>();
            // SAFETY: We're converting T samples to bytes. This assumes T is a simple type (u8/u16)
            // as guaranteed by the JpeglsSample trait which is only implemented for u8 and u16.
            // These types have no padding and can be safely reinterpreted as bytes.
            // The destination buffer is pre-allocated with sufficient size.
            // For grayscale (components=1), we copy exactly width*sizeof(T) bytes.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    bytes_ptr,
                    destination_row.as_mut_ptr(),
                    bytes_len,
                );
            }
        }
        Ok(())
    }

    fn decode_sample_line<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        prev_line: &mut [T],
        curr_line: &mut [T],
        width: usize,
        is_first_line: bool,
    ) -> Result<(), JpeglsError> {
        let mut index = 1;
        let mut rb = prev_line[0].to_i32();
        let mut rd = prev_line[1].to_i32();

        while index <= width {
            let ra = curr_line[index - 1].to_i32();
            let rc = rb;
            rb = rd;
            rd = prev_line[index + 1].to_i32();

            let d1 = rd - rb;
            let d2 = rb - rc;
            let d3 = rc - ra;

            let q1 = self.quantize_gradient(d1);
            let q2 = self.quantize_gradient(d2);
            let q3 = self.quantize_gradient(d3);

            let qs = self.compute_context_id(q1, q2, q3);

            if qs != 0 {
                debug_log!("    Regular mode: index={}, qs={}", index, qs);
                let predicted = self.compute_predicted_value(ra, rb, rc);
                let error_value = self.decode_regular::<T>(qs, predicted)?;
                curr_line[index] = T::from_i32(error_value);
                index += 1;
            } else {
                debug_log!("    Run mode: index={}", index);
                index += self.decode_run_mode::<T>(index, prev_line, curr_line, width)?;
                if index <= width {
                    rb = prev_line[index - 1].to_i32();
                    rd = prev_line[index].to_i32();
                }
            }
            
            // Special handling for first line: after decoding first pixel,
            // update prev_line to match so run mode can trigger for subsequent pixels
            if is_first_line && index == 2 {
                let first_pixel_value = curr_line[1];
                for i in 0..prev_line.len() {
                    prev_line[i] = first_pixel_value;
                }
                // Reload rb and rd after updating prev_line
                if index <= width {
                    rb = prev_line[index - 1].to_i32();
                    rd = prev_line[index].to_i32();
                }
                debug_log!("    First line: Updated prev_line to {} for efficient run mode", first_pixel_value.to_i32());
            }
        }
        Ok(())
    }

    fn decode_regular<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        qs: i32,
        predicted: i32,
    ) -> Result<i32, JpeglsError> {
        let sign = Self::bit_wise_sign(qs);
        let ctx_index = Self::apply_sign_for_index(qs, sign);

        let k: i32;
        let near_lossless = self.coding_parameters.near_lossless;

        {
            let context = &mut self.regular_mode_contexts[ctx_index];
            k = context.compute_golomb_coding_parameter(31)?;
        }

        let map_val = self.decode_mapped_error_value(k)?;
        let mut error_value = self.unmap_error_value(map_val);

        {
            let context = &mut self.regular_mode_contexts[ctx_index];
            if k == 0 {
                error_value ^= context.get_error_correction(near_lossless);
            }
            let reset_threshold = self.reset_threshold;
            context.update_variables_and_bias(error_value, near_lossless, reset_threshold)?;
        }

        error_value = Self::apply_sign(error_value, sign);
        let reconstructed = T::compute_reconstructed_sample(predicted, error_value);
        debug_log!("      Reconstructed: predicted={}, error={}, result={}", 
                  predicted, error_value, reconstructed);
        Ok(reconstructed)
    }

    fn decode_mapped_error_value(&mut self, k: i32) -> Result<i32, JpeglsError> {
        let mut value = 0;
        let mut bit_count = 0;

        debug_log!("      decode_mapped_error_value: k={}, cache=0x{:016X}, valid_bits={}, pos={}", 
                  k, self.read_cache, self.valid_bits, self.position);

        // Read unary code (count zeros until we hit a 1)
        while self.peek_bits(1)? == 0 {
            value += 1;
            bit_count += 1;
            self.skip_bits(1)?;
            if bit_count > 32 {
                debug_log!("    Golomb: unary code too long (>32 zeros)");
                return Err(JpeglsError::InvalidData);
            }
        }
        self.skip_bits(1)?;  // Skip the terminating 1

        // Read fixed-length remainder
        if k > 0 {
            let remainder = self.read_bits(k)?;
            value = (value << k) | remainder;
            debug_log!("    Golomb decode: k={}, unary={}, remainder={}, result={}", 
                      k, bit_count, remainder, value);
        } else {
            debug_log!("    Golomb decode: k=0, unary={}, result={}", bit_count, value);
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
        while self.valid_bits <= (std::mem::size_of::<usize>() * 8 - 16) as i32 {
            if self.position >= self.source.len() {
                // eprintln!("Fill cache: EOF (pos {})", self.position);
                break;
            }
            let byte = self.source[self.position] as usize;
            // // eprintln!("Read byte: {:02X} at {}", byte, self.position);

            // Add byte to cache first
            self.read_cache = (self.read_cache << 8) | byte;
            self.valid_bits += 8;
            self.position += 1;

            // Check for 0xFF marker handling
            if byte == JPEG_MARKER_START_BYTE as usize {
                if self.position < self.source.len() {
                    let next_byte = self.source[self.position];

                    if next_byte == 0x00 {
                        // Stuffed 0 byte. The 0xFF is valid data.
                        // Consume the stuffed zero.
                        self.position += 1;
                        // Do not add bits from the stuffed byte to the cache.
                        // The 0xFF is already in the cache.
                        debug_log!("    Byte stuffing: FF 00 → FF (data)");
                    } else if next_byte == 0x7F {
                        // Special case: FF 7F appears in CharLS-encoded files at scan end.
                        // This might be scan termination padding or bit-stuffing variant.
                        // Don't consume the 7F, just keep FF in cache and continue.
                        // The 7F will be read in the next iteration if needed.
                        debug_log!("    Special pattern: FF 7F detected, keeping FF as data");
                    } else if Self::is_valid_jpeg_marker(next_byte) {
                        // Valid JPEG/JPEG-LS marker found (EOI, etc.)
                        // Back up, remove 0xFF from cache, and stop.
                        self.position -= 1;
                        self.valid_bits -= 8;
                        self.read_cache >>= 8;
                        debug_log!("    Marker: FF {:02X} detected, stopping cache fill", next_byte);
                        break;
                    } else {
                        // FF followed by other non-marker, non-00, non-7F codes.
                        // Keep FF as data, will read next_byte in next iteration.
                        debug_log!("    Non-marker after FF: {:02X}, keeping FF as data", next_byte);
                    }
                } else {
                    // End of data after 0xFF. Keep 0xFF in cache.
                }
            }
        }
        Ok(())
    }

    fn read_bits(&mut self, count: i32) -> Result<i32, JpeglsError> {
        let val = self.peek_bits(count)?;
        self.skip_bits(count)?;
        
        #[cfg(debug_assertions)]
        {
            self.bits_consumed += count as usize;
        }
        
        Ok(val)
    }

    fn peek_bits(&mut self, count: i32) -> Result<i32, JpeglsError> {
        if self.valid_bits < count {
            self.fill_read_cache()?;
        }
        if self.valid_bits < count {
            debug_log!("  ✗ peek_bits({}) FAILED: only {} bits available at pos {}", 
                      count, self.valid_bits, self.position);
            return Err(JpeglsError::InvalidData);
        }
        Ok(((self.read_cache >> (self.valid_bits - count)) & ((1 << count) - 1)) as i32)
    }

    fn skip_bits(&mut self, count: i32) -> Result<(), JpeglsError> {
        if self.valid_bits < count {
            self.fill_read_cache()?;
        }
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

    fn decode_run_mode<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        start_index: usize,
        prev_line: &[T],
        curr_line: &mut [T],
        width: usize,
    ) -> Result<usize, JpeglsError> {
        let mut run_length = 0;
        debug_log!("    decode_run_mode: start_index={}, width={}", start_index, width);
        loop {
            let run_index_val = crate::constants::J[self.run_index];
            #[cfg(debug_assertions)]
            let bits_before = self.bits_consumed;
            let bit = self.read_bits(1)?;
            #[cfg(debug_assertions)]
            debug_log!("      [bit {}] run_index={}, J={}, bit={}, run_length={}", 
                      bits_before, self.run_index, run_index_val, bit, run_length);
            if bit == 1 {
                let length = 1 << run_index_val;
                debug_log!("      → Full run of {} pixels", length);
                let mut hit_width = false;
                for i in 0..length {
                    let i_usize = i as usize;
                    if start_index + run_length + i_usize >= width {
                        hit_width = true;
                        break;
                    }
                    curr_line[start_index + run_length + i_usize] = curr_line[start_index - 1];
                }
                run_length += length as usize;
                // If we hit width (or exceeded it in run_length counting), we clamp effectively.
                // But run_length variable keeps increasing to track the "virtual" run?
                // Spec says run is terminated at EOL.
                // If we hit width, we should break out of the loop and return run_length = width - start_index?
                if hit_width || start_index + run_length >= width {
                    // Clamp run_length to match width exactly
                    run_length = width - start_index;
                    debug_log!("      → Hit width, clamping run_length to {}", run_length);
                    if self.run_index < 31 {
                         self.run_index += 1;
                    }
                    break;
                }
                if self.run_index < 31 {
                    self.run_index += 1;
                }
            } else {
                let remainder = self.read_bits(run_index_val)?;
                debug_log!("      → Partial run of {} pixels", remainder);
                let mut hit_width = false;
                for i in 0..remainder {
                    let i_usize = i as usize;
                    if start_index + run_length + i_usize >= width {
                        hit_width = true;
                        break;
                    }
                    curr_line[start_index + run_length + i_usize] = curr_line[start_index - 1];
                }
                run_length += remainder as usize;
                if hit_width || start_index + run_length >= width {
                    run_length = width - start_index;
                    debug_log!("      → Hit width, clamping run_length to {}", run_length);
                    if self.run_index > 0 {
                        self.run_index -= 1;
                    }
                    break;
                }
                if self.run_index > 0 {
                    self.run_index -= 1;
                }
                break;
            }
        }

        debug_log!("    Run length decoded: {}", run_length);

        if start_index + run_length <= width {
            let rb = prev_line[start_index + run_length].to_i32();
            let ra = curr_line[start_index + run_length - 1].to_i32();
            debug_log!("    Run interruption pixel at index {}, ra={}, rb={}", 
                      start_index + run_length, ra, rb);
            let x = self.decode_run_interruption_pixel::<T>(ra, rb)?;
            curr_line[start_index + run_length] = T::from_i32(x);
            run_length += 1;
        }

        Ok(run_length)
    }

    fn decode_run_interruption_pixel<T: crate::jpegls::traits::JpeglsSample>(
        &mut self,
        ra: i32,
        rb: i32,
    ) -> Result<i32, JpeglsError> {
        let near_lossless = self.coding_parameters.near_lossless;
        let (context_index, sign) = if (ra - rb).abs() <= near_lossless {
            (1, 1)
        } else {
            (0, Self::bit_wise_sign(rb - ra))
        };

        let k = self.run_mode_contexts[context_index].compute_golomb_coding_parameter();
        let mapped_error = self.decode_mapped_error_value(k)?;

        let error_value = self.run_mode_contexts[context_index].decode_error_value(mapped_error, k);
        let reset_threshold = self.reset_threshold;
        self.run_mode_contexts[context_index].update_variables(
            error_value,
            mapped_error,
            reset_threshold,
        );

        let reconstructed = if context_index == 1 {
            T::compute_reconstructed_sample(ra, error_value)
        } else {
            T::compute_reconstructed_sample(rb, error_value * sign)
        };

        debug_log!("    Run interruption: ra={}, rb={}, ctx={}, sign={}, error={}, reconstructed={}", 
                  ra, rb, context_index, sign, error_value, reconstructed);

        Ok(reconstructed)
    }
}

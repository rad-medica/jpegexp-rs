use crate::FrameInfo;
use crate::error::JpeglsError;
use crate::jpeg_marker_code::JPEG_MARKER_START_BYTE;
use crate::jpegls::JpeglsPcParameters;
use crate::jpegls::coding_parameters::CodingParameters;
use crate::jpegls::regular_mode_context::RegularModeContext;
use crate::jpegls::run_mode_context::RunModeContext;
use crate::jpegls::traits::JpeglsSample;
use crate::jpegls::InterleaveMode;

pub struct ScanEncoder<'a> {
    frame_info: FrameInfo,
    pc_parameters: JpeglsPcParameters,
    coding_parameters: CodingParameters,
    destination: &'a mut [u8],
    position: usize,
    bit_buffer: u32,
    free_bit_count: i32,
    is_ff_written: bool,

    // Contexts (per component)
    // regular_mode_contexts[component_index][context_id]
    regular_mode_contexts: Vec<Vec<RegularModeContext>>,
    // run_mode_contexts[component_index][0..1]
    run_mode_contexts: Vec<Vec<RunModeContext>>,
    // run_index[component_index]
    run_index: Vec<usize>,

    // Parameters
    t1: i32,
    t2: i32,
    t3: i32,
    reset_threshold: i32,
}

impl<'a> ScanEncoder<'a> {
    pub fn new(
        frame_info: FrameInfo,
        pc_parameters: JpeglsPcParameters,
        coding_parameters: CodingParameters,
        destination: &'a mut [u8],
    ) -> Self {
        let range = pc_parameters.maximum_sample_value + 1;
        let num_components = if coding_parameters.interleave_mode == InterleaveMode::None {
            1
        } else {
            frame_info.component_count as usize
        };

        let mut regular_mode_contexts = Vec::with_capacity(num_components);
        let mut run_mode_contexts = Vec::with_capacity(num_components);
        let mut run_index = Vec::with_capacity(num_components);

        for _ in 0..num_components {
            regular_mode_contexts.push(vec![RegularModeContext::new(range); 365]);
            run_mode_contexts.push(vec![
                RunModeContext::new(0, range),
                RunModeContext::new(1, range),
            ]);
            run_index.push(0);
        }

        Self {
            frame_info,
            pc_parameters,
            coding_parameters,
            destination,
            position: 0,
            bit_buffer: 0,
            free_bit_count: 32,
            is_ff_written: false,
            regular_mode_contexts,
            run_mode_contexts,
            run_index,
            t1: pc_parameters.threshold1,
            t2: pc_parameters.threshold2,
            t3: pc_parameters.threshold3,
            reset_threshold: pc_parameters.reset_value,
        }
    }

    fn apply_sign(val: i32, sign: i32) -> i32 {
        crate::jpegls::traits::apply_sign(val, sign)
    }

    fn bit_wise_sign(val: i32) -> i32 {
        crate::jpegls::traits::bit_wise_sign(val)
    }

    pub fn encode_scan<T: JpeglsSample>(
        &mut self,
        source: &[T],
        stride: usize,
    ) -> Result<usize, JpeglsError> {
        self.initialize();
        self.encode_lines(source, stride)?;
        self.end_scan();
        Ok(self.get_length())
    }

    fn initialize(&mut self) {
        self.bit_buffer = 0;
        self.free_bit_count = 32;
        self.is_ff_written = false;
    }

    fn append_to_bit_stream(&mut self, bits: u32, bit_count: i32) {
        if bit_count == 0 {
            return;
        }
        let bit_count = bit_count.clamp(0, 31);
        if self.free_bit_count < bit_count {
            let bits_that_fit = self.free_bit_count.max(0);
            if bits_that_fit > 0 {
                let shift = bit_count - bits_that_fit;
                let mask = ((1u32 << bit_count) - 1) >> shift;
                let high_bits = (bits >> shift) & mask;
                self.bit_buffer |= high_bits << (self.free_bit_count - bits_that_fit);
                self.free_bit_count -= bits_that_fit;
            }
            self.flush();
            let remaining = bit_count - bits_that_fit;
            if remaining > 0 && self.free_bit_count >= remaining {
                let low_mask = (1u32 << remaining) - 1;
                let low_bits = bits & low_mask;
                self.bit_buffer |= low_bits << (self.free_bit_count - remaining);
                self.free_bit_count -= remaining;
            }
        } else {
            self.bit_buffer |= bits << (self.free_bit_count - bit_count);
            self.free_bit_count -= bit_count;
        }
    }

    fn flush(&mut self) {
        while self.free_bit_count <= 24 {
            let byte_val = (self.bit_buffer >> 24) as u8;
            self.bit_buffer <<= 8;
            self.free_bit_count += 8;
            if self.position < self.destination.len() {
                self.destination[self.position] = byte_val;
                self.position += 1;
            }
            if byte_val == JPEG_MARKER_START_BYTE && self.position < self.destination.len() {
                self.destination[self.position] = 0x00;
                self.position += 1;
            }
        }
    }

    fn end_scan(&mut self) {
        let used_bits = 32 - self.free_bit_count;
        let remainder = used_bits % 8;
        if remainder != 0 {
            self.append_to_bit_stream(0, 8 - remainder);
        }
        self.flush();
    }

    fn get_length(&self) -> usize {
        self.position
    }

    fn encode_lines<T: JpeglsSample>(
        &mut self,
        source: &[T],
        _stride: usize,
    ) -> Result<(), JpeglsError> {
        let width = self.frame_info.width as usize;
        let height = self.frame_info.height as usize;
        let interleave_mode = self.coding_parameters.interleave_mode;

        let components = if interleave_mode == InterleaveMode::None {
            1
        } else {
            self.frame_info.component_count as usize
        };

        let pixel_stride = width * components;
        let buffer_width = (width + 1) * components;

        // Per ITU-T T.87 specification, initialize previous line to 2^(P-1)
        // For 8-bit: 1 << 7 = 128, for 16-bit: 1 << 15 = 32768
        // This ensures neutral starting bias for predictive compression
        let init_value = T::from_i32(1 << (self.frame_info.bits_per_sample - 1));
        let mut line_buffer: Vec<T> = vec![init_value; buffer_width * 2];
        let mut source_idx = 0;

        for line in 0..height {
            let (prev_line_slice, curr_line_slice) =
                line_buffer.split_at_mut(buffer_width);
            let (prev, curr) = if (line & 1) == 1 {
                (curr_line_slice, prev_line_slice)
            } else {
                (prev_line_slice, curr_line_slice)
            };

            let current_source_row = &source[source_idx..source_idx + pixel_stride];
            curr[components..buffer_width].copy_from_slice(current_source_row);

            // Replicate boundary pixels for padding
            for c in 0..components {
                 curr[c] = prev[components + c];
            }

            self.encode_sample_line(prev, curr, width, components, line == 0)?;
            source_idx += pixel_stride;
        }
        Ok(())
    }

    fn encode_sample_line<T: JpeglsSample>(
        &mut self,
        prev_line: &mut [T],
        curr_line: &mut [T],
        width: usize,
        components: usize,
        is_first_line: bool,
    ) -> Result<(), JpeglsError> {
        let mut pixel_idx = 0;
        let mut current_buf_idx = components;

        let mut rb = vec![0i32; components];
        let mut rd = vec![0i32; components];

        for c in 0..components {
            rb[c] = prev_line[c].to_i32();
            rd[c] = prev_line[components + c].to_i32();
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

            // CharLS uses REGULAR mode for the first pixel of the first line,
            // even when all_qs_zero is true. This ensures compatibility with
            // CharLS decoder. For the very first pixel, use regular mode.
            let use_regular_mode = !all_qs_zero || (is_first_line && pixel_idx == 0);
            
            if use_regular_mode {
                for c in 0..components {
                    let idx = current_buf_idx + c;
                    let val = curr_line[idx].to_i32();
                    self.encode_regular::<T>(component_qs[c], val, component_pred[c], c)?;
                }
                pixel_idx += 1;
                current_buf_idx += components;
            } else {
                let start_pixel_idx = pixel_idx;

                let encoded_len = self.encode_run_mode_interleaved(
                     start_pixel_idx,
                     prev_line,
                     curr_line,
                     width,
                     components,
                     &mut rb,
                     &mut rd
                )?;

                pixel_idx += encoded_len;
                current_buf_idx += encoded_len * components;

                // Re-sync Rb/Rd
                if pixel_idx < width {
                     let is_last = pixel_idx == width - 1;
                     for c in 0..components {
                         let comp_offset = components + c;

                         rb[c] = prev_line[(pixel_idx - 1) * components + comp_offset].to_i32();
                         if is_last {
                             rd[c] = rb[c];
                         } else {
                             rd[c] = prev_line[pixel_idx * components + comp_offset].to_i32();
                         }
                     }
                }
            }
        }
        Ok(())
    }

    fn encode_regular<T: JpeglsSample>(
        &mut self,
        qs: i32,
        x: i32,
        predicted: i32,
        component_index: usize,
    ) -> Result<(), JpeglsError> {
        let sign = Self::bit_wise_sign(qs);
        let ctx_index = crate::jpegls::traits::apply_sign_for_index(qs, sign);

        let limit = self.coding_parameters.limit;
        let near_lossless = self.coding_parameters.near_lossless;

        let k: i32;
        let c_val: i32;
        let correction: i32;

        {
            let context = &mut self.regular_mode_contexts[component_index][ctx_index];
            k = context.compute_golomb_coding_parameter(31)?;
            c_val = context.c();
            correction = context.get_error_correction(near_lossless | k);
        }

        let predicted_value = T::correct_prediction(predicted + Self::apply_sign(c_val, sign));
        let error_val = self.compute_error_value(Self::apply_sign(x - predicted_value, sign));
        let mapped_error = self.map_error_value(correction ^ error_val);

        self.encode_mapped_value(k, mapped_error, limit);

        let reset_threshold = self.reset_threshold;
        let context = &mut self.regular_mode_contexts[component_index][ctx_index];
        context.update_variables_and_bias(error_val, near_lossless, reset_threshold)?;
        Ok(())
    }

    fn compute_error_value(&self, e: i32) -> i32 {
        self.modulo_range(self.quantize(e))
    }

    fn quantize(&self, e: i32) -> i32 {
        if e > 0 {
            (e + self.coding_parameters.near_lossless)
                / (2 * self.coding_parameters.near_lossless + 1)
        } else {
            -(self.coding_parameters.near_lossless - e)
                / (2 * self.coding_parameters.near_lossless + 1)
        }
    }

    fn modulo_range(&self, mut error_value: i32) -> i32 {
        let range = self.pc_parameters.maximum_sample_value + 1;
        if error_value < 0 {
            error_value += range;
        }
        if error_value >= (range + 1) / 2 {
            error_value -= range;
        }
        error_value
    }

    fn map_error_value(&self, error_value: i32) -> i32 {
        // Per ITU-T T.87 specification:
        // - Positive error (>= 0): MErrval = 2 × error (produces EVEN)
        // - Negative error (< 0): MErrval = -2 × error - 1 (produces ODD)
        // This can be expressed as XOR-based formula:
        let bit_count = 32;
        (error_value >> (bit_count - 2)) ^ (2 * error_value)
    }

    fn encode_mapped_value(&mut self, k: i32, mapped_error: i32, limit: i32) {
        let high_bits = mapped_error >> k;
        let qbpp = self.coding_parameters.quantized_bits_per_sample;

        if high_bits < limit - qbpp - 1 {
            for _ in 0..high_bits {
                self.append_to_bit_stream(0, 1);
            }
            self.append_to_bit_stream(1, 1);
            let k_clamped = k.min(31);
            self.append_to_bit_stream((mapped_error & ((1i32 << k_clamped) - 1)) as u32, k_clamped);
        } else {
            let remaining = limit - qbpp;
            if remaining > 31 {
                self.append_to_bit_stream(0, 31);
                let remaining_clamped = (remaining - 31).min(31);
                self.append_to_bit_stream(1, remaining_clamped);
            } else {
                self.append_to_bit_stream(1, remaining.min(31));
            }
            let qbpp_clamped = qbpp.min(31);
            self.append_to_bit_stream(((mapped_error - 1) & ((1i32 << qbpp_clamped) - 1)) as u32, qbpp_clamped);
        }
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

    // Updated for Interleaved
    fn encode_run_mode_interleaved<T: JpeglsSample>(
        &mut self,
        start_pixel_idx: usize,
        prev_line: &[T],
        curr_line: &mut [T],
        width: usize,
        components: usize,
        _rb: &mut [i32],
        _rd: &mut [i32],
    ) -> Result<usize, JpeglsError> {
        // Run length is number of PIXELS where all components match Ra
        let mut run_length = 0;
        let count_type_remain = width - start_pixel_idx;

        let base_offset = components;

        // Capture Ra for all components (left neighbor)
        let mut ra = vec![T::default(); components];
        for c in 0..components {
            let ra_idx = if start_pixel_idx > 0 {
                base_offset + (start_pixel_idx - 1) * components + c
            } else {
                // First pixel: use boundary pixel at index c
                c
            };
            ra[c] = curr_line[ra_idx];
        }

        while run_length < count_type_remain {
            let mut all_match = true;
            for c in 0..components {
                let val = curr_line[base_offset + (start_pixel_idx + run_length) * components + c];
                if !T::is_near(
                    val.to_i32(),
                    ra[c].to_i32(),
                    self.coding_parameters.near_lossless,
                ) {
                    all_match = false;
                    break;
                }
            }
            if !all_match {
                break;
            }

            for c in 0..components {
                curr_line[base_offset + (start_pixel_idx + run_length) * components + c] = ra[c];
            }
            run_length += 1;
        }

        // Use Component 0 run index for shared run
        self.encode_run_pixels(run_length, run_length == count_type_remain, 0);

        if run_length == count_type_remain {
            return Ok(run_length);
        }

        // Interruption
        let interruption_pixel_idx = start_pixel_idx + run_length;

        let mut interruption_comp = 0;
        let mut found_break = false;

        for c in 0..components {
            let val = curr_line[base_offset + interruption_pixel_idx * components + c];
            if !T::is_near(
                 val.to_i32(),
                 ra[c].to_i32(),
                 self.coding_parameters.near_lossless
            ) {
                 interruption_comp = c;
                 found_break = true;
                 break;
            }
        }

        if !found_break {
             return Ok(run_length);
        }

        // Handle interruption component
        let c = interruption_comp;
        let up_val = prev_line[base_offset + interruption_pixel_idx * components + c];
        let val = curr_line[base_offset + interruption_pixel_idx * components + c];

        let interruption_val = self.encode_run_interruption_pixel::<T>(
             val.to_i32(),
             ra[c].to_i32(),
             up_val.to_i32(),
             c // Use component c context
        );
        curr_line[base_offset + interruption_pixel_idx * components + c] = T::from_i32(interruption_val);

        self.decrement_run_index(0);

        for next_c in (c + 1)..components {
             let idx = base_offset + interruption_pixel_idx * components + next_c;

             let r_a = curr_line[idx - components].to_i32();
             let r_up = prev_line[idx].to_i32(); // Rb
             let r_up_left = prev_line[idx - components].to_i32(); // Rc

             let r_up_right = if interruption_pixel_idx == width - 1 {
                 r_up // Rd = Rb at end of line
             } else {
                 prev_line[idx + components].to_i32() // Rd
             };

             let d1 = r_up_right - r_up;
             let d2 = r_up - r_up_left;
             let d3 = r_up_left - r_a;

             let q1 = self.quantize_gradient(d1);
             let q2 = self.quantize_gradient(d2);
             let q3 = self.quantize_gradient(d3);

             let qs = self.compute_context_id(q1, q2, q3);
             let predicted = self.compute_predicted_value(r_a, r_up, r_up_left);

             self.encode_regular::<T>(
                 qs,
                 curr_line[idx].to_i32(),
                 predicted,
                 next_c
             )?;
        }

        Ok(run_length + 1)
    }

    fn encode_run_pixels(&mut self, mut run_length: usize, end_of_line: bool, comp: usize) {
        while run_length >= (1 << crate::constants::J[self.run_index[comp]]) {
            self.append_ones_to_bit_stream(1);
            run_length -= 1 << crate::constants::J[self.run_index[comp]];
            self.increment_run_index(comp);
        }

        if end_of_line {
            if run_length != 0 {
                self.append_ones_to_bit_stream(1);
            }
        } else {
            self.append_to_bit_stream(run_length as u32, crate::constants::J[self.run_index[comp]] + 1);
        }
    }

    fn encode_run_interruption_pixel<T: JpeglsSample>(
        &mut self, x: i32, ra: i32, rb: i32, comp: usize
    ) -> i32 {
        let near_lossless = self.coding_parameters.near_lossless;
        if (ra - rb).abs() <= near_lossless {
            let error_value = self.compute_error_value(x - ra);
            self.encode_run_interruption_error(1, error_value, comp);
            T::compute_reconstructed_sample(ra, error_value)
        } else {
            let sign = Self::bit_wise_sign(rb - ra);
            let error_value = self.compute_error_value((x - rb) * sign);
            self.encode_run_interruption_error(0, error_value, comp);
            T::compute_reconstructed_sample(rb, error_value * sign)
        }
    }

    fn encode_run_interruption_error(&mut self, context_index: usize, error_value: i32, comp: usize) {
        let (k, e_mapped_error_value) = {
            let context = &self.run_mode_contexts[comp][context_index];
            let k = context.compute_golomb_coding_parameter();
            let map = context.compute_map(error_value, k);
            // Mapping formula for run interruption (ITU-T T.87 Section A.7.2.2)
            // MErrval = 2 * |Errval| - RIType + (if !map { 1 } else { 0 })
            // This ensures the LSB encodes the map bit and decoder can reconstruct correctly
            let val =
                2 * error_value.abs() - context.run_interruption_type() + (if map { 0 } else { 1 });
            (k, val)
        };

        let limit = self.coding_parameters.limit - crate::constants::J[self.run_index[comp]] - 1;
        self.encode_mapped_value(k, e_mapped_error_value, limit);

        let reset_threshold = self.reset_threshold;
        let context = &mut self.run_mode_contexts[comp][context_index];
        context.update_variables(error_value, e_mapped_error_value, reset_threshold);
    }

    fn increment_run_index(&mut self, comp: usize) {
        if self.run_index[comp] < 31 {
            self.run_index[comp] += 1;
        }
    }

    fn decrement_run_index(&mut self, comp: usize) {
        if self.run_index[comp] > 0 {
            self.run_index[comp] -= 1;
        }
    }

    fn append_ones_to_bit_stream(&mut self, bit_count: i32) {
        if bit_count == 0 {
            return;
        }
        let bit_count = bit_count.min(31);
        self.append_to_bit_stream((1u32 << bit_count).wrapping_sub(1), bit_count);
    }
}

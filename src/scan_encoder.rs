use crate::coding_parameters::CodingParameters;
use crate::error::JpeglsError;
use crate::regular_mode_context::RegularModeContext;
use crate::run_mode_context::RunModeContext;
use crate::{FrameInfo, JpeglsPcParameters};
use crate::traits::JpeglsSample;
use crate::jpeg_marker_code::JPEG_MARKER_START_BYTE;

pub struct ScanEncoder<'a> {
    frame_info: FrameInfo,
    pc_parameters: JpeglsPcParameters,
    coding_parameters: CodingParameters,
    destination: &'a mut [u8],
    position: usize,
    bit_buffer: u32,
    free_bit_count: i32,
    is_ff_written: bool,
    
    // Contexts
    regular_mode_contexts: Vec<RegularModeContext>,
    run_mode_contexts: Vec<RunModeContext>,
    run_index: usize,
    
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
         let regular_mode_contexts = vec![RegularModeContext::new(range); 365];
         let run_mode_contexts = vec![
            RunModeContext::new(0, range),
            RunModeContext::new(1, range),
         ];

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
            run_index: 0,
            t1: pc_parameters.threshold1,
            t2: pc_parameters.threshold2,
            t3: pc_parameters.threshold3,
            reset_threshold: pc_parameters.reset_value,
         }
    }

    fn apply_sign(val: i32, sign: i32) -> i32 {
        crate::traits::apply_sign(val, sign)
    }

    fn bit_wise_sign(val: i32) -> i32 {
        crate::traits::bit_wise_sign(val)
    }

    pub fn encode_scan<T: JpeglsSample>(&mut self, source: &[T], stride: usize) -> Result<usize, JpeglsError> {
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
        debug_assert!(bit_count >= 0 && bit_count < 32);
        
        self.free_bit_count -= bit_count;
        if self.free_bit_count >= 0 {
            self.bit_buffer |= bits << self.free_bit_count;
        } else {
             self.bit_buffer |= bits >> (-self.free_bit_count);
             self.flush();
             
             if self.free_bit_count < 0 {
                 self.bit_buffer |= bits >> (-self.free_bit_count);
                 self.flush();
             }
             
             debug_assert!(self.free_bit_count >= 0);
             self.bit_buffer |= bits << self.free_bit_count;
        }
    }
    
    fn flush(&mut self) {
         if self.free_bit_count >= 32 {
             return;
         }
         
         for _ in 0..4 {
             if self.free_bit_count >= 32 {
                 break;
             }
             
             let byte_val = if self.is_ff_written {
                  let val = (self.bit_buffer >> 25) as u8;
                  self.bit_buffer <<= 7;
                  self.free_bit_count += 7;
                  val
             } else {
                  let val = (self.bit_buffer >> 24) as u8;
                  self.bit_buffer <<= 8;
                  self.free_bit_count += 8;
                  val
             };
             
             if self.position < self.destination.len() {
                 self.destination[self.position] = byte_val;
                 self.position += 1;
             }
             
             self.is_ff_written = byte_val == JPEG_MARKER_START_BYTE;
         }
         
         self.free_bit_count = 32; 
    }
    
    fn end_scan(&mut self) {
        self.flush();
        if self.is_ff_written {
             self.append_to_bit_stream(0, (self.free_bit_count - 1) % 8);
        }
        self.flush();
    }
    
    fn get_length(&self) -> usize {
        self.position
    }

    fn encode_lines<T: JpeglsSample>(&mut self, source: &[T], _stride: usize) -> Result<(), JpeglsError> {
        let width = self.frame_info.width as usize;
        let height = self.frame_info.height as usize;
        let components = 1; 
        
        let pixel_stride = width + 2;
        let mut line_buffer: Vec<T> = vec![T::default(); components * pixel_stride * 2];
        let mut source_idx = 0;
        
        for line in 0..height {
             let (prev_line_slice, curr_line_slice) = line_buffer.split_at_mut(components * pixel_stride);
             let (prev, curr) = if (line & 1) == 1 {
                 (curr_line_slice, prev_line_slice)
             } else {
                 (prev_line_slice, curr_line_slice)
             };
             
             let prev_line = &mut prev[0..pixel_stride];
             let curr_line = &mut curr[0..pixel_stride];
             
             let current_source_row = &source[source_idx..source_idx + width];
             curr_line[1..width+1].copy_from_slice(current_source_row);
             curr_line[0] = prev_line[1]; 
             
             self.encode_sample_line(prev_line, curr_line, width)?;
             source_idx += width; 
        }
        Ok(())
    }

    fn encode_sample_line<T: JpeglsSample>(&mut self, prev_line: &[T], curr_line: &mut [T], width: usize) -> Result<(), JpeglsError> {
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
                let predicted = self.compute_predicted_value(ra, rb, rc);
                self.encode_regular::<T>(qs, curr_line[index].to_i32(), predicted)?;
                index += 1;
            } else {
                index += self.encode_run_mode(index, prev_line, curr_line, width)?;
                if index <= width {
                    rb = prev_line[index - 1].to_i32();
                    rd = prev_line[index].to_i32();
                }
            }
        }
        Ok(())
    }

    fn encode_regular<T: JpeglsSample>(&mut self, qs: i32, x: i32, predicted: i32) -> Result<(), JpeglsError> {
        let sign = Self::bit_wise_sign(qs);
        let ctx_index = crate::traits::apply_sign_for_index(qs, sign);
        
        let limit = self.coding_parameters.limit;
        let near_lossless = self.coding_parameters.near_lossless;

        let k: i32;
        let c_val: i32;
        let correction: i32;
        
        {
             let context = &mut self.regular_mode_contexts[ctx_index];
             k = context.compute_golomb_coding_parameter(31)?;
             c_val = context.c();
             correction = context.get_error_correction(near_lossless | k);
        }
        
        let predicted_value = T::correct_prediction(predicted + Self::apply_sign(c_val, sign));
        let error_val = self.compute_error_value(Self::apply_sign(x - predicted_value, sign));
        let mapped_error = self.map_error_value(correction ^ error_val);
        
        self.encode_mapped_value(k, mapped_error, limit);
        
        let reset_threshold = self.reset_threshold;
        let context = &mut self.regular_mode_contexts[ctx_index];
        context.update_variables_and_bias(error_val, near_lossless, reset_threshold)?;
        Ok(())
    }

    fn compute_error_value(&self, e: i32) -> i32 {
         self.modulo_range(self.quantize(e))
    }
    
    fn quantize(&self, e: i32) -> i32 {
        if e > 0 {
            (e + self.coding_parameters.near_lossless) / (2 * self.coding_parameters.near_lossless + 1)
        } else {
            -(self.coding_parameters.near_lossless - e) / (2 * self.coding_parameters.near_lossless + 1)
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
        let bit_count = 32;
        let mapped = (error_value >> (bit_count - 2)) ^ (2 * error_value);
        mapped
    }

    fn encode_mapped_value(&mut self, k: i32, mapped_error: i32, limit: i32) {
        let high_bits = mapped_error >> k;
        let qbpp = self.coding_parameters.quantized_bits_per_sample;
        
        if high_bits < limit - qbpp - 1 {
             for _ in 0..high_bits { self.append_to_bit_stream(0, 1); }
             self.append_to_bit_stream(1, 1);
             self.append_to_bit_stream((mapped_error & ((1 << k) - 1)) as u32, k);
        } else {
             if limit - qbpp > 31 {
                  self.append_to_bit_stream(0, 31);
                  self.append_to_bit_stream(1, limit - qbpp - 31);
             } else {
                  self.append_to_bit_stream(1, limit - qbpp); 
             }
             self.append_to_bit_stream(((mapped_error - 1) & ((1 << qbpp) - 1)) as u32, qbpp);
        }
    }

    fn quantize_gradient(&self, di: i32) -> i32 {
        if di <= -self.t3 { return -4; }
        if di <= -self.t2 { return -3; }
        if di <= -self.t1 { return -2; }
        if di < -self.coding_parameters.near_lossless { return -1; }
        if di <= self.coding_parameters.near_lossless { return 0; }
        if di < self.t1 { return 1; }
        if di < self.t2 { return 2; }
        if di < self.t3 { return 3; }
        4
    }

    fn compute_context_id(&self, q1: i32, q2: i32, q3: i32) -> i32 {
        (q1 * 9 + q2) * 9 + q3
    }
    
    fn compute_predicted_value(&self, ra: i32, rb: i32, rc: i32) -> i32 {
         let sign = Self::bit_wise_sign(rb - ra);
         if (sign ^ (rc - ra)) < 0 {
             rb
         } else if (sign ^ (rb - rc)) < 0 {
             ra
         } else {
             ra + rb - rc
         }
    }
    
    fn encode_run_mode<T: JpeglsSample>(&mut self, start_index: usize, prev_line: &[T], curr_line: &mut [T], width: usize) -> Result<usize, JpeglsError> {
        let count_type_remain = width - (start_index - 1);
        let mut run_length = 0;
        let ra = curr_line[start_index - 1];
        
        while run_length < count_type_remain {
             let val = curr_line[start_index + run_length];
             if !T::is_near(val.to_i32(), ra.to_i32(), self.coding_parameters.near_lossless) {
                 break;
             }
             curr_line[start_index + run_length] = ra; 
             run_length += 1;
        }
        
        self.encode_run_pixels(run_length, run_length == count_type_remain);
        
        if run_length == count_type_remain {
             return Ok(run_length);
        }
        
        let rb = prev_line[start_index + run_length];
        let x = curr_line[start_index + run_length];
        
        let interruption_val = self.encode_run_interruption_pixel::<T>(x.to_i32(), ra.to_i32(), rb.to_i32());
        curr_line[start_index + run_length] = T::from_i32(interruption_val);
        
        self.decrement_run_index();
        Ok(run_length + 1)
    }

    fn encode_run_pixels(&mut self, mut run_length: usize, end_of_line: bool) {
        while run_length >= (1 << crate::constants::J[self.run_index]) {
             self.append_ones_to_bit_stream(1);
             run_length -= 1 << crate::constants::J[self.run_index];
             self.increment_run_index();
        }
         
         if end_of_line {
              if run_length != 0 {
                   self.append_ones_to_bit_stream(1);
              }
         } else {
             self.append_to_bit_stream(run_length as u32, crate::constants::J[self.run_index] + 1);
        }
    }

    fn encode_run_interruption_pixel<T: JpeglsSample>(&mut self, x: i32, ra: i32, rb: i32) -> i32 {
         let near_lossless = self.coding_parameters.near_lossless;
         if (ra - rb).abs() <= near_lossless {
              let error_value = self.compute_error_value(x - ra);
              self.encode_run_interruption_error(1, error_value);
              T::compute_reconstructed_sample(ra, error_value)
         } else {
              let sign = Self::bit_wise_sign(rb - ra);
              let error_value = self.compute_error_value((x - rb) * sign);
              self.encode_run_interruption_error(0, error_value);
              T::compute_reconstructed_sample(rb, error_value * sign)
         }
    }
    
    fn encode_run_interruption_error(&mut self, context_index: usize, error_value: i32) {
         let (k, e_mapped_error_value) = {
             let context = &self.run_mode_contexts[context_index];
             let k = context.compute_golomb_coding_parameter();
             let map = context.compute_map(error_value, k);
             let val = 2 * error_value.abs() - context.run_interruption_type() - (if map {1} else {0});
             (k, val)
         };
         
         let limit = self.coding_parameters.limit - crate::constants::J[self.run_index] - 1;
         self.encode_mapped_value(k, e_mapped_error_value, limit);
         
         let reset_threshold = self.reset_threshold;
         let context = &mut self.run_mode_contexts[context_index];
         context.update_variables(error_value, e_mapped_error_value, reset_threshold);
    }

    fn increment_run_index(&mut self) {
        if self.run_index < 31 {
            self.run_index += 1;
        }
    }
    
    fn decrement_run_index(&mut self) {
        if self.run_index > 0 {
            self.run_index -= 1;
        }
    }
    
    fn append_ones_to_bit_stream(&mut self, bit_count: i32) {
         self.append_to_bit_stream((1 << bit_count) - 1, bit_count);
    }
}

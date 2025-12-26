use crate::error::JpeglsError;
use crate::jpeg_marker_code::JPEG_MARKER_START_BYTE;
use crate::regular_mode_context::RegularModeContext;
use crate::run_mode_context::RunModeContext;
use crate::{FrameInfo, InterleaveMode, JpeglsPcParameters, CodingParameters};

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
        let run_mode_contexts = vec![
            RunModeContext::new(0, range),
            RunModeContext::new(1, range),
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
            run_index: 0,
            t1,
            t2,
            t3,
            reset_threshold: reset,
            _limit: 0, 
            _quantized_bits_per_sample: frame_info.bits_per_sample, 
            _quantization_lut: Vec::new(), 
        };
        
        decoder.find_jpeg_marker_start_byte();
        decoder.fill_read_cache()?;

        Ok(decoder)
    }

    pub fn decode_scan(&mut self, destination: &mut [u8], stride: usize) -> Result<usize, JpeglsError> {
        let bit_depth = self.frame_info.bits_per_sample;
        if bit_depth <= 8 {
             self.decode_scan_typed::<u8>(destination, stride)
        } else if bit_depth <= 16 {
             self.decode_scan_typed::<u16>(destination, stride)
        } else {
             Err(JpeglsError::ParameterValueNotSupported)
        }
    }

    fn decode_scan_typed<T: crate::traits::JpeglsSample>(&mut self, destination: &mut [u8], stride: usize) -> Result<usize, JpeglsError> {
        self.decode_lines::<T>(destination, stride)?;
        self.end_scan()?;
        Ok(self.position)
    }

    fn decode_lines<T: crate::traits::JpeglsSample>(&mut self, destination: &mut [u8], stride: usize) -> Result<(), JpeglsError> {
         let width = self.frame_info.width as usize;
         let height = self.frame_info.height as usize;
         let pixel_stride = width + 2;
         let components = if self.coding_parameters.interleave_mode == InterleaveMode::Line {
             self.frame_info.component_count as usize
         } else {
             1
         };

         let mut line_buffer: Vec<T> = vec![T::default(); components * pixel_stride * 2];
         
         for line in 0..height {
             let (prev_line_slice, curr_line_slice) = line_buffer.split_at_mut(components * pixel_stride);
             let (prev, curr) = if (line & 1) == 1 {
                 (curr_line_slice, prev_line_slice)
             } else {
                 (prev_line_slice, curr_line_slice)
             };

             let prev_line = &mut prev[0..pixel_stride];
             let curr_line = &mut curr[0..pixel_stride];
             
             curr_line[0] = prev_line[1]; 
             self.decode_sample_line::<T>(prev_line, curr_line, width)?;
             
             let _destination_row = &mut destination[(line * stride).. (line * stride + width * components * std::mem::size_of::<T>())];
         }
         Ok(())
    }

    fn decode_sample_line<T: crate::traits::JpeglsSample>(&mut self, prev_line: &[T], curr_line: &mut [T], width: usize) -> Result<(), JpeglsError> {
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
                let error_value = self.decode_regular::<T>(qs, predicted)?;
                curr_line[index] = T::from_i32(error_value);
                index += 1;
            } else {
                index += self.decode_run_mode::<T>(index, prev_line, curr_line, width)?;
                if index <= width {
                    rb = prev_line[index - 1].to_i32();
                    rd = prev_line[index].to_i32();
                }
            }
        }
        Ok(())
    }

    fn decode_regular<T: crate::traits::JpeglsSample>(&mut self, qs: i32, predicted: i32) -> Result<i32, JpeglsError> {
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
                 error_value = error_value ^ context.get_error_correction(near_lossless);
            }
            let reset_threshold = self.reset_threshold;
            context.update_variables_and_bias(error_value, near_lossless, reset_threshold)?;
        }
        
        error_value = Self::apply_sign(error_value, sign);
        Ok(T::compute_reconstructed_sample(predicted, error_value))
    }
    
    fn decode_mapped_error_value(&mut self, k: i32) -> Result<i32, JpeglsError> {
        let mut value = 0;
        let mut bit_count = 0;
        
        while self.peek_bits(1)? == 0 {
            value += 1;
            bit_count += 1;
            self.skip_bits(1)?;
            if bit_count > 32 { return Err(JpeglsError::InvalidData); }
        }
        self.skip_bits(1)?;
        
        if k > 0 {
            let remainder = self.read_bits(k)?;
            value = (value << k) | remainder;
        }
        Ok(value)
    }

    fn unmap_error_value(&self, mapped_value: i32) -> i32 {
        if (mapped_value & 1) == 0 {
            mapped_value >> 1
        } else {
            -( (mapped_value + 1) >> 1)
        }
    }

    fn find_jpeg_marker_start_byte(&mut self) {
         while self.position < self.source.len() && self.source[self.position] != JPEG_MARKER_START_BYTE {
             self.position += 1;
         }
    }

    fn fill_read_cache(&mut self) -> Result<(), JpeglsError> {
        while self.valid_bits <= (std::mem::size_of::<usize>() * 8 - 16) as i32 {
            if self.position >= self.source.len() {
                break;
            }
            let byte = self.source[self.position] as usize;
            self.read_cache = (self.read_cache << 8) | byte;
            self.valid_bits += 8;
            self.position += 1;
            
            if byte == JPEG_MARKER_START_BYTE as usize {
                 if self.position < self.source.len() && self.source[self.position] == 0 {
                      self.position += 1; 
                 } else {
                      break;
                 }
            }
        }
        Ok(())
    }

    fn read_bits(&mut self, count: i32) -> Result<i32, JpeglsError> {
        let val = self.peek_bits(count)?;
        self.skip_bits(count)?;
        Ok(val)
    }

    fn peek_bits(&mut self, count: i32) -> Result<i32, JpeglsError> {
        if self.valid_bits < count {
            self.fill_read_cache()?;
        }
        if self.valid_bits < count {
             return Err(JpeglsError::InvalidData);
        }
        Ok(((self.read_cache >> (self.valid_bits - count)) & ((1 << count) - 1)) as i32)
    }

    fn skip_bits(&mut self, count: i32) -> Result<(), JpeglsError> {
        if self.valid_bits < count {
             self.fill_read_cache()?;
        }
        self.valid_bits -= count;
        Ok(())
    }

    fn end_scan(&mut self) -> Result<(), JpeglsError> {
         Ok(())
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

    fn bit_wise_sign(val: i32) -> i32 {
        crate::traits::bit_wise_sign(val)
    }

    fn apply_sign(val: i32, sign: i32) -> i32 {
         crate::traits::apply_sign(val, sign)
    }

    fn apply_sign_for_index(val: i32, sign: i32) -> usize {
         crate::traits::apply_sign_for_index(val, sign)
    }

    fn decode_run_mode<T: crate::traits::JpeglsSample>(&mut self, start_index: usize, prev_line: &[T], curr_line: &mut [T], width: usize) -> Result<usize, JpeglsError> {
        let mut run_length = 0;
        loop {
            let run_index_val = crate::constants::J[self.run_index];
            let bit = self.read_bits(1)?;
            if bit == 1 {
                let length = 1 << run_index_val;
                for i in 0..length {
                    let i_usize = i as usize;
                    if start_index + run_length + i_usize > width { return Err(JpeglsError::InvalidData); }
                    curr_line[start_index + run_length + i_usize] = curr_line[start_index - 1];
                }
                run_length += length as usize;
                if self.run_index < 31 { self.run_index += 1; }
                if start_index + run_length > width { break; }
            } else {
                let remainder = self.read_bits(run_index_val as i32)?;
                for i in 0..remainder {
                    let i_usize = i as usize;
                    if start_index + run_length + i_usize > width { return Err(JpeglsError::InvalidData); }
                    curr_line[start_index + run_length + i_usize] = curr_line[start_index - 1];
                }
                run_length += remainder as usize;
                if self.run_index > 0 { self.run_index -= 1; }
                break;
            }
        }
        
        if start_index + run_length <= width {
            let rb = prev_line[start_index + run_length].to_i32();
            let ra = curr_line[start_index + run_length - 1].to_i32();
            let x = self.decode_run_interruption_pixel::<T>(ra, rb)?;
            curr_line[start_index + run_length] = T::from_i32(x);
            run_length += 1;
        }
        
        Ok(run_length)
    }

    fn decode_run_interruption_pixel<T: crate::traits::JpeglsSample>(&mut self, ra: i32, rb: i32) -> Result<i32, JpeglsError> {
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
        self.run_mode_contexts[context_index].update_variables(error_value, mapped_error, reset_threshold);
        
        let reconstructed = if context_index == 1 {
            T::compute_reconstructed_sample(ra, error_value)
        } else {
            T::compute_reconstructed_sample(rb, error_value * sign)
        };
        
        Ok(reconstructed)
    }
}

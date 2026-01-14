/// Common test utilities for interoperability testing
///
/// This module provides:
/// - Image generation utilities (gradients, checkerboards, patterns)
/// - Pixel comparison utilities (MAE, PSNR, exact matching)
/// - Codec wrapper traits for unified testing
/// - Comprehensive synthetic image generation

use std::error::Error;

pub mod synthetic_images;

/// Image generation utilities
pub mod image_gen {
    /// Generate a linear gradient image from black to white
    ///
    /// # Arguments
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `bit_depth` - Bit depth (8, 10, 12, or 16)
    ///
    /// # Returns
    /// Vector of pixel values as bytes (big-endian for 16-bit)
    pub fn gradient(width: u32, height: u32, bit_depth: u8) -> Vec<u8> {
        let max_value = (1u32 << bit_depth) - 1;
        let total_pixels = (width * height) as usize;
        
        match bit_depth {
            8 => {
                (0..total_pixels)
                    .map(|i| {
                        // Use correct scaling to hit 255
                        let ratio = i as f64 / (total_pixels - 1).max(1) as f64;
                        (ratio * max_value as f64).round() as u8
                    })
                    .collect()
            }
            16 => {
                let mut pixels = Vec::with_capacity(total_pixels * 2);
                for i in 0..total_pixels {
                    // Use correct scaling to hit 65535
                    let ratio = i as f64 / (total_pixels - 1).max(1) as f64;
                    let value = (ratio * max_value as f64).round() as u16;
                    // Big-endian storage
                    pixels.push((value >> 8) as u8);
                    pixels.push((value & 0xFF) as u8);
                }
                pixels
            }
            12 => {
                // For 12-bit, store as 16-bit big-endian with upper 12 bits used
                let mut pixels = Vec::with_capacity(total_pixels * 2);
                for i in 0..total_pixels {
                    let ratio = i as f64 / (total_pixels - 1).max(1) as f64;
                    let value = ((ratio * max_value as f64).round() as u16) & 0x0FFF;
                    pixels.push((value >> 8) as u8);
                    pixels.push((value & 0xFF) as u8);
                }
                pixels
            }
            _ => panic!("Unsupported bit depth: {}", bit_depth),
        }
    }

    /// Generate a checkerboard pattern
    ///
    /// # Arguments
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `bit_depth` - Bit depth (8, 12, or 16)
    /// * `square_size` - Size of each checkerboard square
    pub fn checkerboard(width: u32, height: u32, bit_depth: u8, square_size: u32) -> Vec<u8> {
        let max_value = (1u32 << bit_depth) - 1;
        let bytes_per_pixel = if bit_depth == 8 { 1 } else { 2 };
        let mut pixels = Vec::with_capacity((width * height) as usize * bytes_per_pixel);

        for y in 0..height {
            for x in 0..width {
                let checker = ((x / square_size) + (y / square_size)) % 2;
                let value = if checker == 0 { 0 } else { max_value };

                if bit_depth == 8 {
                    pixels.push(value as u8);
                } else {
                    // Big-endian 16-bit
                    pixels.push((value >> 8) as u8);
                    pixels.push((value & 0xFF) as u8);
                }
            }
        }
        pixels
    }

    /// Generate a constant-value image
    pub fn constant(width: u32, height: u32, bit_depth: u8, value: u32) -> Vec<u8> {
        let max_value = (1u32 << bit_depth) - 1;
        let clamped_value = value.min(max_value);
        let bytes_per_pixel = if bit_depth == 8 { 1 } else { 2 };
        let total_pixels = (width * height) as usize;

        if bit_depth == 8 {
            vec![clamped_value as u8; total_pixels]
        } else {
            let mut pixels = Vec::with_capacity(total_pixels * bytes_per_pixel);
            for _ in 0..total_pixels {
                pixels.push((clamped_value >> 8) as u8);
                pixels.push((clamped_value & 0xFF) as u8);
            }
            pixels
        }
    }

    /// Generate random noise pattern (for testing compression limits)
    pub fn random_noise(width: u32, height: u32, bit_depth: u8, seed: u64) -> Vec<u8> {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let max_value = (1u32 << bit_depth) - 1;
        let bytes_per_pixel = if bit_depth == 8 { 1 } else { 2 };
        let total_pixels = (width * height) as usize;
        let mut pixels = Vec::with_capacity(total_pixels * bytes_per_pixel);

        let hasher_builder = RandomState::new();
        for i in 0..total_pixels {
            let mut hasher = hasher_builder.build_hasher();
            (seed + i as u64).hash(&mut hasher);
            let hash_value = hasher.finish();
            let value = (hash_value as u32 % (max_value + 1)) as u32;

            if bit_depth == 8 {
                pixels.push(value as u8);
            } else {
                pixels.push((value >> 8) as u8);
                pixels.push((value & 0xFF) as u8);
            }
        }
        pixels
    }

    /// Generate a sine wave pattern (for frequency analysis testing)
    pub fn sine_wave(width: u32, height: u32, bit_depth: u8, frequency: f64) -> Vec<u8> {
        let max_value = (1u32 << bit_depth) - 1;
        let bytes_per_pixel = if bit_depth == 8 { 1 } else { 2 };
        let mut pixels = Vec::with_capacity((width * height) as usize * bytes_per_pixel);

        for y in 0..height {
            for x in 0..width {
                let phase = (x as f64 + y as f64) * frequency * 2.0 * std::f64::consts::PI / width as f64;
                let sine = phase.sin();
                let normalized = (sine + 1.0) / 2.0; // Map [-1,1] to [0,1]
                let value = (normalized * max_value as f64) as u32;

                if bit_depth == 8 {
                    pixels.push(value as u8);
                } else {
                    pixels.push((value >> 8) as u8);
                    pixels.push((value & 0xFF) as u8);
                }
            }
        }
        pixels
    }
}

/// Pixel comparison utilities
pub mod comparison {
    /// Result of pixel comparison between two images
    #[derive(Debug, Clone)]
    pub struct PixelComparison {
        /// Mean Absolute Error (0.0 = perfect match)
        pub mae: f64,
        /// Peak Signal-to-Noise Ratio (higher is better, infinite for perfect match)
        pub psnr: f64,
        /// Maximum absolute difference between any pixel pair
        pub max_error: u32,
        /// True if all pixels match exactly (MAE = 0)
        pub pixels_match: bool,
        /// Number of pixels compared
        pub pixel_count: usize,
    }

    /// Compare two pixel buffers and compute metrics
    ///
    /// # Arguments
    /// * `expected` - Expected pixel values
    /// * `actual` - Actual pixel values
    /// * `bit_depth` - Bit depth (8, 12, or 16)
    ///
    /// # Returns
    /// PixelComparison struct with MAE, PSNR, and max error
    pub fn compare_pixels(expected: &[u8], actual: &[u8], bit_depth: u8) -> PixelComparison {
        assert_eq!(expected.len(), actual.len(), "Pixel buffer sizes must match");

        let bytes_per_pixel = if bit_depth == 8 { 1 } else { 2 };
        let pixel_count = expected.len() / bytes_per_pixel;
        let max_value = (1u64 << bit_depth) - 1;

        let mut sum_abs_error = 0u64;
        let mut max_error = 0u32;

        if bit_depth == 8 {
            for i in 0..pixel_count {
                let exp = expected[i] as i32;
                let act = actual[i] as i32;
                let abs_diff = (exp - act).abs() as u32;
                sum_abs_error += abs_diff as u64;
                max_error = max_error.max(abs_diff);
            }
        } else {
            // 16-bit big-endian
            for i in 0..pixel_count {
                let exp = ((expected[i * 2] as u16) << 8 | expected[i * 2 + 1] as u16) as i32;
                let act = ((actual[i * 2] as u16) << 8 | actual[i * 2 + 1] as u16) as i32;
                let abs_diff = (exp - act).abs() as u32;
                sum_abs_error += abs_diff as u64;
                max_error = max_error.max(abs_diff);
            }
        }

        let mae = sum_abs_error as f64 / pixel_count as f64;
        let pixels_match = mae == 0.0;

        // Calculate PSNR
        let psnr = if pixels_match {
            f64::INFINITY
        } else {
            let mse = sum_abs_error as f64 / pixel_count as f64;
            let mse_squared = mse * mse;
            20.0 * (max_value as f64).log10() - 10.0 * mse_squared.log10()
        };

        PixelComparison {
            mae,
            psnr,
            max_error,
            pixels_match,
            pixel_count,
        }
    }

    /// Assert that two pixel buffers match within a tolerance
    pub fn assert_pixels_match(expected: &[u8], actual: &[u8], bit_depth: u8, tolerance: f64) {
        let comparison = compare_pixels(expected, actual, bit_depth);
        assert!(
            comparison.mae <= tolerance,
            "MAE {} exceeds tolerance {}. Max error: {}, PSNR: {:.2}",
            comparison.mae,
            tolerance,
            comparison.max_error,
            comparison.psnr
        );
    }

    /// Assert exact pixel match (MAE = 0)
    pub fn assert_pixels_exact(expected: &[u8], actual: &[u8], bit_depth: u8) {
        assert_pixels_match(expected, actual, bit_depth, 0.0);
    }
}

/// Decoded image data with metadata
#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub bit_depth: u8,
    pub components: u8,
}

/// Common trait for codec testing
pub trait CodecTest {
    /// Encode raw pixels to compressed bitstream
    fn encode(
        &self,
        pixels: &[u8],
        width: u32,
        height: u32,
        bit_depth: u8,
        components: u8,
    ) -> Result<Vec<u8>, Box<dyn Error>>;

    /// Decode compressed bitstream to raw pixels
    fn decode(&self, bitstream: &[u8]) -> Result<DecodedImage, Box<dyn Error>>;

    /// Get codec name for reporting
    fn name(&self) -> &str;
}

/// Test file management utilities
pub mod file_io {
    use std::path::{Path, PathBuf};
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Get path to a test output file, ensuring the directory exists
    pub fn get_test_output_path(filename: &str) -> PathBuf {
        let output_dir = Path::new("test_output");
        
        // Ensure directory exists (only once per run ideally, but FS check is cheap)
        INIT.call_once(|| {
            if !output_dir.exists() {
                std::fs::create_dir_all(output_dir).expect("Failed to create test_output directory");
            }
        });
        // Still check every time in case it was deleted
        if !output_dir.exists() {
             std::fs::create_dir_all(output_dir).expect("Failed to create test_output directory");
        }

        output_dir.join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gradient_8bit() {
        let pixels = image_gen::gradient(256, 1, 8);
        assert_eq!(pixels.len(), 256);
        assert_eq!(pixels[0], 0);
        assert_eq!(pixels[255], 255);
    }

    #[test]
    fn test_gradient_16bit() {
        let pixels = image_gen::gradient(256, 1, 16);
        assert_eq!(pixels.len(), 512); // 256 pixels * 2 bytes
        // First pixel should be 0x0000
        assert_eq!(pixels[0], 0);
        assert_eq!(pixels[1], 0);
        // Last pixel should be 0xFFFF
        assert_eq!(pixels[510], 0xFF);
        assert_eq!(pixels[511], 0xFF);
    }

    #[test]
    fn test_checkerboard() {
        let pixels = image_gen::checkerboard(4, 4, 8, 2);
        assert_eq!(pixels.len(), 16);
        // Top-left 2x2 should be 0
        assert_eq!(pixels[0], 0);
        // Top-right 2x2 should be 255
        assert_eq!(pixels[2], 255);
    }

    #[test]
    fn test_comparison_exact_match() {
        let pixels1 = vec![1, 2, 3, 4, 5];
        let pixels2 = vec![1, 2, 3, 4, 5];
        let result = comparison::compare_pixels(&pixels1, &pixels2, 8);
        assert_eq!(result.mae, 0.0);
        assert!(result.pixels_match);
        assert_eq!(result.max_error, 0);
    }

    #[test]
    fn test_comparison_with_error() {
        let pixels1 = vec![0, 0, 0, 0];
        let pixels2 = vec![10, 5, 3, 2];
        let result = comparison::compare_pixels(&pixels1, &pixels2, 8);
        assert_eq!(result.mae, 5.0); // (10 + 5 + 3 + 2) / 4
        assert!(!result.pixels_match);
        assert_eq!(result.max_error, 10);
    }
}

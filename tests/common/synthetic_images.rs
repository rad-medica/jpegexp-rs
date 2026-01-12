//! Comprehensive synthetic image generation for interoperability testing
//!
//! This module provides a comprehensive suite of synthetic test images with various:
//! - Patterns (solid, gradient, checkerboard, noise, medical-like)
//! - Bit depths (8, 10, 12, 16 bit)
//! - Color modes (grayscale, RGB)
//! - Resolutions and aspect ratios

use std::collections::HashMap;

/// Image pattern types for testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// Uniform single value
    Solid,
    /// Horizontal gradient (left=0, right=max)
    GradientH,
    /// Vertical gradient (top=0, bottom=max)
    GradientV,
    /// Diagonal gradient (top-left=0, bottom-right=max)
    GradientD,
    /// Checkerboard with specified block size
    Checkerboard,
    /// Deterministic pseudo-random noise
    Noise,
    /// CT-like high contrast edges
    MedicalCT,
    /// Smooth gradients with subtle noise (natural-like)
    Natural,
    /// Sine wave pattern (frequency testing)
    SineWave,
    /// Ramp pattern (each row is constant, incrementing)
    Ramp,
}

impl Pattern {
    pub fn all() -> Vec<Pattern> {
        vec![
            Pattern::Solid,
            Pattern::GradientH,
            Pattern::GradientV,
            Pattern::GradientD,
            Pattern::Checkerboard,
            Pattern::Noise,
            Pattern::MedicalCT,
            Pattern::Natural,
            Pattern::SineWave,
            Pattern::Ramp,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Pattern::Solid => "solid",
            Pattern::GradientH => "gradient_h",
            Pattern::GradientV => "gradient_v",
            Pattern::GradientD => "gradient_d",
            Pattern::Checkerboard => "checkerboard",
            Pattern::Noise => "noise",
            Pattern::MedicalCT => "medical_ct",
            Pattern::Natural => "natural",
            Pattern::SineWave => "sine_wave",
            Pattern::Ramp => "ramp",
        }
    }

    /// Subset of patterns for quick testing
    pub fn quick_set() -> Vec<Pattern> {
        vec![Pattern::Solid, Pattern::GradientD, Pattern::Checkerboard, Pattern::Noise]
    }
}

/// Test image dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Standard test resolutions
    pub fn standard_set() -> Vec<Dimensions> {
        vec![
            Dimensions::new(8, 8),
            Dimensions::new(16, 16),
            Dimensions::new(32, 32),
            Dimensions::new(64, 64),
            Dimensions::new(128, 128),
            Dimensions::new(256, 256),
            Dimensions::new(512, 512),
        ]
    }

    /// Quick test resolutions (smaller for fast iteration)
    pub fn quick_set() -> Vec<Dimensions> {
        vec![
            Dimensions::new(16, 16),
            Dimensions::new(64, 64),
            Dimensions::new(256, 256),
        ]
    }

    /// Non-square and odd dimensions for edge case testing
    pub fn edge_cases() -> Vec<Dimensions> {
        vec![
            Dimensions::new(1, 1),
            Dimensions::new(1, 8),
            Dimensions::new(8, 1),
            Dimensions::new(127, 129),
            Dimensions::new(255, 257),
            Dimensions::new(256, 64),  // 4:1 wide
            Dimensions::new(64, 256),  // 1:4 tall
        ]
    }

    /// Large test resolutions for performance testing
    pub fn large_set() -> Vec<Dimensions> {
        vec![
            Dimensions::new(512, 512),
            Dimensions::new(1024, 1024),
            Dimensions::new(2048, 2048),
        ]
    }

    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
}

/// Synthetic image configuration
#[derive(Debug, Clone)]
pub struct SyntheticImageConfig {
    pub dimensions: Dimensions,
    pub pattern: Pattern,
    pub bit_depth: u32,
    pub components: u32,
    pub seed: u32,
    pub block_size: u32,  // For checkerboard
    pub frequency: f64,   // For sine wave
}

impl Default for SyntheticImageConfig {
    fn default() -> Self {
        Self {
            dimensions: Dimensions::new(256, 256),
            pattern: Pattern::GradientD,
            bit_depth: 8,
            components: 1,
            seed: 42,
            block_size: 8,
            frequency: 4.0,
        }
    }
}

impl SyntheticImageConfig {
    pub fn name(&self) -> String {
        let color = if self.components == 1 { "gray" } else { "rgb" };
        format!(
            "{}_{}_{}x{}_{}_{}bit",
            self.pattern.name(),
            color,
            self.dimensions.width,
            self.dimensions.height,
            self.bit_depth,
            self.components
        )
    }
}

/// Generated synthetic image
#[derive(Debug, Clone)]
pub struct SyntheticImage {
    pub config: SyntheticImageConfig,
    pub pixels: Vec<u8>,
}

impl SyntheticImage {
    pub fn width(&self) -> u32 {
        self.config.dimensions.width
    }

    pub fn height(&self) -> u32 {
        self.config.dimensions.height
    }

    pub fn bit_depth(&self) -> u32 {
        self.config.bit_depth
    }

    pub fn components(&self) -> u32 {
        self.config.components
    }

    pub fn bytes_per_sample(&self) -> usize {
        if self.config.bit_depth <= 8 { 1 } else { 2 }
    }

    pub fn pixel_count(&self) -> usize {
        self.config.dimensions.pixel_count() * self.config.components as usize
    }
}

/// Synthetic image generator
pub struct SyntheticImageGenerator;

impl SyntheticImageGenerator {
    /// Generate a synthetic image with the given configuration
    pub fn generate(config: &SyntheticImageConfig) -> SyntheticImage {
        let w = config.dimensions.width;
        let h = config.dimensions.height;
        let max_val = (1u64 << config.bit_depth) - 1;
        let bytes_per_sample = if config.bit_depth <= 8 { 1 } else { 2 };
        let pixel_count = config.dimensions.pixel_count() * config.components as usize;
        let mut pixels = Vec::with_capacity(pixel_count * bytes_per_sample);

        for y in 0..h {
            for x in 0..w {
                for c in 0..config.components {
                    let val = Self::compute_pixel_value(x, y, c, w, h, max_val, config);
                    Self::write_pixel(&mut pixels, val, config.bit_depth);
                }
            }
        }

        SyntheticImage { config: config.clone(), pixels }
    }

    fn compute_pixel_value(
        x: u32, y: u32, c: u32,
        w: u32, h: u32,
        max_val: u64,
        config: &SyntheticImageConfig,
    ) -> u64 {
        match config.pattern {
            Pattern::Solid => max_val / 2,

            Pattern::GradientH => {
                if w <= 1 { max_val / 2 }
                else { x as u64 * max_val / (w - 1) as u64 }
            }

            Pattern::GradientV => {
                if h <= 1 { max_val / 2 }
                else { y as u64 * max_val / (h - 1) as u64 }
            }

            Pattern::GradientD => {
                if w + h <= 2 { max_val / 2 }
                else { ((x + y) as u64 * max_val) / ((w + h - 2) as u64) }
            }

            Pattern::Checkerboard => {
                let bx = x / config.block_size;
                let by = y / config.block_size;
                if (bx + by) % 2 == 0 { 0 } else { max_val }
            }

            Pattern::Noise => {
                Self::lcg_noise(x, y, c, w, config.seed, max_val)
            }

            Pattern::MedicalCT => {
                // Simulate CT-like patterns with high contrast edges
                let center_x = w / 2;
                let center_y = h / 2;
                let dx = (x as i32 - center_x as i32).abs() as u32;
                let dy = (y as i32 - center_y as i32).abs() as u32;
                let dist = ((dx * dx + dy * dy) as f64).sqrt();
                let radius = (w.min(h) / 3) as f64;

                if dist < radius {
                    // Inner region - high intensity with noise
                    let noise = Self::lcg_noise(x, y, c, w, config.seed, max_val / 10);
                    (max_val * 7 / 10).saturating_add(noise)
                } else if dist < radius * 1.2 {
                    // Edge - sharp transition
                    max_val / 5
                } else {
                    // Outer region - low intensity background with noise
                    let noise = Self::lcg_noise(x, y, c, w, config.seed + 1, max_val / 20);
                    max_val / 10 + noise
                }
            }

            Pattern::Natural => {
                // Natural-like: smooth gradient with subtle noise
                let base = if w + h <= 2 { max_val / 2 }
                    else { ((x + y) as u64 * max_val) / ((w + h - 2) as u64) };
                let noise = Self::lcg_noise(x, y, c, w, config.seed, max_val / 50);
                let signed_noise = (noise as i64) - (max_val / 100) as i64;
                (base as i64 + signed_noise).clamp(0, max_val as i64) as u64
            }

            Pattern::SineWave => {
                let freq = config.frequency;
                let phase = (x as f64 + y as f64) * freq * 2.0 * std::f64::consts::PI / w as f64;
                let sine = phase.sin();
                let normalized = (sine + 1.0) / 2.0;
                (normalized * max_val as f64) as u64
            }

            Pattern::Ramp => {
                // Each row is constant, value increments per row
                if h <= 1 { max_val / 2 }
                else { y as u64 * max_val / (h - 1) as u64 }
            }
        }
    }

    /// Linear congruential generator for deterministic noise
    fn lcg_noise(x: u32, y: u32, c: u32, w: u32, seed: u32, max_val: u64) -> u64 {
        let idx = (y * w + x) * 3 + c;
        let val = ((idx as u64).wrapping_mul(1103515245).wrapping_add(12345).wrapping_add(seed as u64)) % (max_val + 1);
        val
    }

    fn write_pixel(pixels: &mut Vec<u8>, val: u64, bit_depth: u32) {
        if bit_depth <= 8 {
            pixels.push(val as u8);
        } else {
            // Native endian for 16-bit values
            let val16 = val as u16;
            pixels.extend_from_slice(&val16.to_ne_bytes());
        }
    }

    /// Generate a comprehensive test suite of synthetic images
    pub fn generate_test_suite(
        patterns: &[Pattern],
        dimensions: &[Dimensions],
        bit_depths: &[u32],
        components: &[u32],
    ) -> Vec<SyntheticImage> {
        let mut images = Vec::new();

        for &dims in dimensions {
            for &pattern in patterns {
                for &bits in bit_depths {
                    for &comps in components {
                        let config = SyntheticImageConfig {
                            dimensions: dims,
                            pattern,
                            bit_depth: bits,
                            components: comps,
                            seed: 42,
                            block_size: 8.max(dims.width / 8),
                            frequency: 4.0,
                        };
                        images.push(Self::generate(&config));
                    }
                }
            }
        }

        images
    }

    /// Quick test suite for fast iteration
    pub fn quick_test_suite() -> Vec<SyntheticImage> {
        Self::generate_test_suite(
            &Pattern::quick_set(),
            &Dimensions::quick_set(),
            &[8],
            &[1],
        )
    }

    /// Comprehensive test suite for full validation
    pub fn comprehensive_test_suite() -> Vec<SyntheticImage> {
        Self::generate_test_suite(
            &Pattern::all(),
            &Dimensions::standard_set(),
            &[8, 10, 12, 16],
            &[1, 3],
        )
    }

    /// Grayscale-only test suite (for codecs that don't support RGB well)
    pub fn grayscale_test_suite(bit_depths: &[u32]) -> Vec<SyntheticImage> {
        Self::generate_test_suite(
            &Pattern::quick_set(),
            &Dimensions::standard_set(),
            bit_depths,
            &[1],
        )
    }
}

/// Test configuration presets for different codec families
pub struct TestPresets;

impl TestPresets {
    /// JPEG-LS test configurations (supports all bit depths, grayscale + RGB)
    pub fn jpegls() -> Vec<SyntheticImage> {
        let patterns = vec![Pattern::Solid, Pattern::GradientD, Pattern::Checkerboard, Pattern::Noise, Pattern::MedicalCT];
        let dims = vec![
            Dimensions::new(16, 16),
            Dimensions::new(64, 64),
            Dimensions::new(256, 256),
            Dimensions::new(512, 512),
        ];
        let bit_depths = vec![8, 10, 12, 16];
        let components = vec![1, 3];

        SyntheticImageGenerator::generate_test_suite(&patterns, &dims, &bit_depths, &components)
    }

    /// JPEG-LS quick test (for faster CI)
    pub fn jpegls_quick() -> Vec<SyntheticImage> {
        let patterns = vec![Pattern::GradientD, Pattern::Checkerboard];
        let dims = vec![Dimensions::new(64, 64), Dimensions::new(256, 256)];
        let bit_depths = vec![8, 16];
        let components = vec![1];

        SyntheticImageGenerator::generate_test_suite(&patterns, &dims, &bit_depths, &components)
    }

    /// JPEG 1 test configurations (8-bit baseline, extended for 12-bit)
    pub fn jpeg1() -> Vec<SyntheticImage> {
        let patterns = vec![Pattern::Solid, Pattern::GradientD, Pattern::Checkerboard, Pattern::Natural];
        let dims = vec![
            Dimensions::new(16, 16),
            Dimensions::new(64, 64),
            Dimensions::new(256, 256),
            Dimensions::new(512, 512),
        ];
        let bit_depths = vec![8, 12];
        let components = vec![1, 3];

        SyntheticImageGenerator::generate_test_suite(&patterns, &dims, &bit_depths, &components)
    }

    /// JPEG 2000 test configurations (all bit depths, single and multi-component)
    pub fn jpeg2000() -> Vec<SyntheticImage> {
        let patterns = vec![Pattern::Solid, Pattern::GradientD, Pattern::Checkerboard, Pattern::Noise, Pattern::MedicalCT];
        let dims = vec![
            Dimensions::new(64, 64),
            Dimensions::new(256, 256),
            Dimensions::new(512, 512),
        ];
        let bit_depths = vec![8, 10, 12, 16];
        let components = vec![1, 3];

        SyntheticImageGenerator::generate_test_suite(&patterns, &dims, &bit_depths, &components)
    }

    /// HTJ2K test configurations
    pub fn htj2k() -> Vec<SyntheticImage> {
        let patterns = vec![Pattern::GradientD, Pattern::Checkerboard, Pattern::MedicalCT];
        let dims = vec![
            Dimensions::new(64, 64),
            Dimensions::new(256, 256),
            Dimensions::new(512, 512),
        ];
        let bit_depths = vec![8, 12, 16];
        let components = vec![1, 3];

        SyntheticImageGenerator::generate_test_suite(&patterns, &dims, &bit_depths, &components)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solid_pattern() {
        let config = SyntheticImageConfig {
            dimensions: Dimensions::new(4, 4),
            pattern: Pattern::Solid,
            bit_depth: 8,
            components: 1,
            ..Default::default()
        };
        let img = SyntheticImageGenerator::generate(&config);
        assert_eq!(img.pixels.len(), 16);
        // All pixels should be 127 (max/2 for 8-bit)
        assert!(img.pixels.iter().all(|&p| p == 127));
    }

    #[test]
    fn test_gradient_h_pattern() {
        let config = SyntheticImageConfig {
            dimensions: Dimensions::new(256, 1),
            pattern: Pattern::GradientH,
            bit_depth: 8,
            components: 1,
            ..Default::default()
        };
        let img = SyntheticImageGenerator::generate(&config);
        assert_eq!(img.pixels.len(), 256);
        assert_eq!(img.pixels[0], 0);
        assert_eq!(img.pixels[255], 255);
    }

    #[test]
    fn test_checkerboard_pattern() {
        let config = SyntheticImageConfig {
            dimensions: Dimensions::new(16, 16),
            pattern: Pattern::Checkerboard,
            bit_depth: 8,
            components: 1,
            block_size: 8,
            ..Default::default()
        };
        let img = SyntheticImageGenerator::generate(&config);
        assert_eq!(img.pixels.len(), 256);
        // Top-left 8x8 block should be 0
        assert_eq!(img.pixels[0], 0);
        // Top-right 8x8 block should be 255
        assert_eq!(img.pixels[8], 255);
    }

    #[test]
    fn test_16bit_generation() {
        let config = SyntheticImageConfig {
            dimensions: Dimensions::new(16, 16),
            pattern: Pattern::GradientD,
            bit_depth: 16,
            components: 1,
            ..Default::default()
        };
        let img = SyntheticImageGenerator::generate(&config);
        assert_eq!(img.pixels.len(), 16 * 16 * 2); // 2 bytes per pixel
    }

    #[test]
    fn test_rgb_generation() {
        let config = SyntheticImageConfig {
            dimensions: Dimensions::new(8, 8),
            pattern: Pattern::GradientD,
            bit_depth: 8,
            components: 3,
            ..Default::default()
        };
        let img = SyntheticImageGenerator::generate(&config);
        assert_eq!(img.pixels.len(), 8 * 8 * 3); // 3 bytes per pixel
    }

    #[test]
    fn test_quick_suite() {
        let suite = SyntheticImageGenerator::quick_test_suite();
        assert!(!suite.is_empty());
        println!("Quick suite generated {} images", suite.len());
    }

    #[test]
    fn test_jpegls_presets() {
        let suite = TestPresets::jpegls_quick();
        assert!(!suite.is_empty());
        for img in &suite {
            println!("Generated: {}", img.config.name());
        }
    }
}

// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory sizing calculation based on model context window.
//!
//! This module determines L1 Working Memory allocation based on the model's
//! context window size and hardware constraints. Implements the baseline
//! sizing strategy for Phase 0.
//!
//! See Engineering Plan § 4.1.1: L1 Sizing & Context Window.

use crate::error::{MemoryError, Result};

/// Model context window specification.
///
/// Defines the token capacity and other model-specific parameters
/// that determine L1 Working Memory allocation.
///
/// See Engineering Plan § 4.1.1: Context Window Sizing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelContextWindow {
    /// Maximum tokens in context window
    pub max_tokens: u64,
    /// Bytes per token (typically 2-4 bytes for embeddings)
    pub bytes_per_token: u64,
    /// Number of attention heads (for gradient storage)
    pub attention_heads: u32,
    /// Hidden dimension size
    pub hidden_dimension: u64,
}

impl ModelContextWindow {
    /// Creates a new context window specification.
    ///
    /// # Arguments
    ///
    /// * `max_tokens` - Maximum tokens in context
    /// * `bytes_per_token` - Bytes per token embedding
    /// * `attention_heads` - Number of attention heads
    /// * `hidden_dimension` - Hidden dimension size
    pub fn new(
        max_tokens: u64,
        bytes_per_token: u64,
        attention_heads: u32,
        hidden_dimension: u64,
    ) -> Self {
        ModelContextWindow {
            max_tokens,
            bytes_per_token,
            attention_heads,
            hidden_dimension,
        }
    }

    /// Creates a default 128K context window (e.g., Claude-3).
    /// Typical: 128K tokens, 2 bytes/token, 32 heads, 1024 hidden.
    pub fn claude_128k() -> Self {
        ModelContextWindow {
            max_tokens: 128 * 1024,
            bytes_per_token: 2,
            attention_heads: 32,
            hidden_dimension: 1024,
        }
    }

    /// Creates a 32K context window (smaller models).
    pub fn context_32k() -> Self {
        ModelContextWindow {
            max_tokens: 32 * 1024,
            bytes_per_token: 2,
            attention_heads: 16,
            hidden_dimension: 768,
        }
    }

    /// Creates a 512K context window (very large models).
    pub fn context_512k() -> Self {
        ModelContextWindow {
            max_tokens: 512 * 1024,
            bytes_per_token: 4,
            attention_heads: 64,
            hidden_dimension: 2048,
        }
    }

    /// Computes the base L1 size from context tokens.
    /// Formula: max_tokens * bytes_per_token
    pub fn base_l1_size_bytes(&self) -> u64 {
        self.max_tokens.saturating_mul(self.bytes_per_token)
    }

    /// Computes the attention cache size.
    /// Formula: max_tokens^2 * attention_heads * (bytes_per_token/2)
    pub fn attention_cache_bytes(&self) -> u64 {
        let tokens_squared = self.max_tokens.saturating_mul(self.max_tokens);
        let heads_factor = self.attention_heads as u64;
        let bytes_per_attn = (self.bytes_per_token / 2).max(1);

        tokens_squared
            .saturating_mul(heads_factor)
            .saturating_mul(bytes_per_attn)
    }

    /// Computes the gradient/activation cache size.
    /// Formula: max_tokens * hidden_dimension * bytes_per_token
    pub fn gradient_cache_bytes(&self) -> u64 {
        self.max_tokens
            .saturating_mul(self.hidden_dimension)
            .saturating_mul(self.bytes_per_token)
    }
}

/// L1 Memory sizing calculator.
///
/// Determines L1 allocation based on context window and system constraints.
///
/// See Engineering Plan § 4.1.1: L1 Allocation Strategy.
#[derive(Clone, Debug)]
pub struct L1SizingCalculator {
    /// Model context window
    model: ModelContextWindow,
    /// Maximum L1 HBM available (e.g., 8GB for A100)
    max_hbm_bytes: u64,
    /// Overhead factor for metadata/bookkeeping (10% default)
    overhead_factor: f64,
    /// Minimum L1 size (must allocate at least this much)
    min_l1_bytes: u64,
}

impl L1SizingCalculator {
    /// Creates a new L1 sizing calculator.
    ///
    /// # Arguments
    ///
    /// * `model` - Model context window specification
    /// * `max_hbm_bytes` - Maximum HBM available (e.g., 8GB)
    /// * `overhead_factor` - Metadata overhead as a fraction (e.g., 0.10 for 10%)
    pub fn new(
        model: ModelContextWindow,
        max_hbm_bytes: u64,
        overhead_factor: f64,
    ) -> Self {
        L1SizingCalculator {
            model,
            max_hbm_bytes,
            overhead_factor,
            min_l1_bytes: 256 * 1024 * 1024, // 256MB minimum
        }
    }

    /// Creates a calculator for a typical GPU (8GB HBM, Claude-3).
    pub fn typical_gpu_claude3() -> Self {
        L1SizingCalculator {
            model: ModelContextWindow::claude_128k(),
            max_hbm_bytes: 8 * 1024 * 1024 * 1024,
            overhead_factor: 0.10,
            min_l1_bytes: 256 * 1024 * 1024,
        }
    }

    /// Calculates the recommended L1 size.
    ///
    /// Returns the allocation size in bytes, clamped to [min_l1, max_hbm].
    ///
    /// # Strategy
    ///
    /// 1. Sum: base_context + attention_cache + gradient_cache
    /// 2. Apply overhead factor
    /// 3. Clamp to [min_l1, max_hbm]
    pub fn calculate_l1_size(&self) -> Result<u64> {
        let base_size = self.model.base_l1_size_bytes();
        let attn_cache = self.model.attention_cache_bytes();
        let grad_cache = self.model.gradient_cache_bytes();

        let total_before_overhead = base_size
            .saturating_add(attn_cache)
            .saturating_add(grad_cache);

        // Apply overhead factor
        let overhead_bytes = (total_before_overhead as f64 * self.overhead_factor) as u64;
        let total_with_overhead = total_before_overhead.saturating_add(overhead_bytes);

        // Clamp to bounds
        if total_with_overhead < self.min_l1_bytes {
            Ok(self.min_l1_bytes)
        } else if total_with_overhead > self.max_hbm_bytes {
            Err(MemoryError::AllocationFailed {
                requested: total_with_overhead,
                available: self.max_hbm_bytes,
            })
        } else {
            Ok(total_with_overhead)
        }
    }

    /// Calculates L1 size with aggressive packing (skip attention cache).
    /// Used for memory-constrained scenarios.
    pub fn calculate_l1_size_compact(&self) -> Result<u64> {
        let base_size = self.model.base_l1_size_bytes();
        let grad_cache = self.model.gradient_cache_bytes();

        let total_before_overhead = base_size.saturating_add(grad_cache);

        // Apply overhead factor
        let overhead_bytes = (total_before_overhead as f64 * self.overhead_factor) as u64;
        let total_with_overhead = total_before_overhead.saturating_add(overhead_bytes);

        if total_with_overhead < self.min_l1_bytes {
            Ok(self.min_l1_bytes)
        } else if total_with_overhead > self.max_hbm_bytes {
            Err(MemoryError::AllocationFailed {
                requested: total_with_overhead,
                available: self.max_hbm_bytes,
            })
        } else {
            Ok(total_with_overhead)
        }
    }

    /// Returns the model context window reference.
    pub fn model(&self) -> &ModelContextWindow {
        &self.model
    }

    /// Returns the maximum HBM available.
    pub fn max_hbm_bytes(&self) -> u64 {
        self.max_hbm_bytes
    }

    /// Sets the minimum L1 allocation size.
    pub fn set_min_l1_bytes(&mut self, min_bytes: u64) {
        self.min_l1_bytes = min_bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_context_window_claude_128k() {
        let model = ModelContextWindow::claude_128k();
        assert_eq!(model.max_tokens, 128 * 1024);
        assert_eq!(model.bytes_per_token, 2);
        assert_eq!(model.attention_heads, 32);
    }

    #[test]
    fn test_model_context_window_context_32k() {
        let model = ModelContextWindow::context_32k();
        assert_eq!(model.max_tokens, 32 * 1024);
    }

    #[test]
    fn test_model_context_window_context_512k() {
        let model = ModelContextWindow::context_512k();
        assert_eq!(model.max_tokens, 512 * 1024);
        assert_eq!(model.bytes_per_token, 4);
    }

    #[test]
    fn test_model_base_l1_size() {
        let model = ModelContextWindow::new(1024, 2, 16, 512);
        let base_size = model.base_l1_size_bytes();
        assert_eq!(base_size, 1024 * 2); // 2KB
    }

    #[test]
    fn test_model_attention_cache_bytes() {
        let model = ModelContextWindow::new(100, 2, 4, 512);
        let attn_cache = model.attention_cache_bytes();
        // 100 * 100 * 4 * 1 = 40000
        assert_eq!(attn_cache, 40_000);
    }

    #[test]
    fn test_model_gradient_cache_bytes() {
        let model = ModelContextWindow::new(100, 2, 16, 512);
        let grad_cache = model.gradient_cache_bytes();
        // 100 * 512 * 2 = 102400
        assert_eq!(grad_cache, 102_400);
    }

    #[test]
    fn test_l1_sizing_calculator_typical_gpu() {
        let calc = L1SizingCalculator::typical_gpu_claude3();
        let size = calc.calculate_l1_size();
        assert!(size.is_ok());

        let size_bytes = size.unwrap();
        // Should be >= min and <= max
        assert!(size_bytes >= calc.min_l1_bytes);
        assert!(size_bytes <= calc.max_hbm_bytes);
    }

    #[test]
    fn test_l1_sizing_respects_max_hbm() {
        let model = ModelContextWindow::claude_128k();
        let calc = L1SizingCalculator::new(model, 2 * 1024 * 1024, 0.10); // Only 2MB HBM

        let result = calc.calculate_l1_size();
        // Should fail because 2MB < context window size + overhead
        assert!(result.is_err());
    }

    #[test]
    fn test_l1_sizing_respects_min_l1() {
        let model = ModelContextWindow::new(10, 1, 2, 10); // Very small model
        let mut calc = L1SizingCalculator::new(model, 8 * 1024 * 1024, 0.10);
        let min = 512 * 1024 * 1024; // 512MB minimum
        calc.set_min_l1_bytes(min);

        let size = calc.calculate_l1_size().unwrap();
        // Should be at least the minimum
        assert!(size >= min);
    }

    #[test]
    fn test_l1_sizing_compact_mode() {
        let calc = L1SizingCalculator::typical_gpu_claude3();
        let full_size = calc.calculate_l1_size().unwrap();
        let compact_size = calc.calculate_l1_size_compact().unwrap();

        // Compact should be <= full (skips attention cache)
        assert!(compact_size <= full_size);
    }

    #[test]
    fn test_l1_sizing_calculator_bounds() {
        let model = ModelContextWindow::new(1000, 2, 16, 512);
        let calc = L1SizingCalculator::new(model, 4 * 1024 * 1024 * 1024, 0.10);

        let size = calc.calculate_l1_size().unwrap();
        assert!(size >= calc.min_l1_bytes);
        assert!(size <= calc.max_hbm_bytes);
    }

    #[test]
    fn test_model_context_window_new() {
        let model = ModelContextWindow::new(2048, 2, 8, 256);
        assert_eq!(model.max_tokens, 2048);
        assert_eq!(model.bytes_per_token, 2);
        assert_eq!(model.attention_heads, 8);
        assert_eq!(model.hidden_dimension, 256);
    }

    #[test]
    fn test_l1_sizing_overhead_applied() {
        let model = ModelContextWindow::new(1024, 1, 1, 1024);
        let calc = L1SizingCalculator::new(model, 8 * 1024 * 1024, 0.50); // 50% overhead

        let base = model.base_l1_size_bytes();
        let grad = model.gradient_cache_bytes();
        let expected_before = base + grad;
        let expected_with_overhead = expected_before + (expected_before as f64 * 0.50) as u64;

        let size = calc.calculate_l1_size().unwrap();
        assert_eq!(size, expected_with_overhead);
    }

    #[test]
    fn test_l1_sizing_zero_overhead() {
        let model = ModelContextWindow::new(1024, 1, 1, 1024);
        let calc = L1SizingCalculator::new(model, 8 * 1024 * 1024, 0.0); // No overhead

        let base = model.base_l1_size_bytes();
        let grad = model.gradient_cache_bytes();
        let expected = base + grad;

        let size = calc.calculate_l1_size().unwrap();
        assert!(size >= expected); // May be bumped to minimum
    }
}

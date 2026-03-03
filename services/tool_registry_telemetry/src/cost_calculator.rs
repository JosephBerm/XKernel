// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Cost Attribution Calculator for Tool Invocations
//!
//! Provides comprehensive cost tracking and calculation for tool invocations,
//! including token counting, GPU time tracking, and TPC-hours calculation.
//!
//! See Engineering Plan § 2.12.5: Cost Attribution & Metering,
//! and Week 5 Objective: Cost Attribution Metadata.

use alloc::string::String;

use crate::cef::CostAttribution;
use crate::cost_attribution::{TokenCounter, GpuCostCalculator, WallClockTracker, TpcCalculator};
use crate::error::{Result, ToolError};

/// Cost calculator for a single tool invocation.
///
/// Tracks and calculates all costs associated with a tool invocation:
/// - Input token count (from request context)
/// - Output token count (from response data)
/// - GPU compute time (wall-clock measurement)
/// - Wall-clock time (actual elapsed time)
/// - TPC-hours (Token Processing Compute hours)
///
/// Uses the formula from Engineering Plan § 2.12.5:
/// ```
/// TPC-hours = (total_tokens × gpu_hours) / 1_000_000
/// ```
///
/// See Engineering Plan § 2.12.5: Cost Attribution Framework.
#[derive(Clone, Debug)]
pub struct InvocationCostCalculator {
    /// Input token counter
    input_tokens: u64,

    /// Output token counter
    output_tokens: u64,

    /// GPU time in milliseconds
    gpu_ms: u64,

    /// Wall-clock time in milliseconds
    wall_clock_ms: u64,

    /// Start time for wall-clock measurement
    start_time_ms: u64,
}

impl InvocationCostCalculator {
    /// Creates a new invocation cost calculator.
    ///
    /// # Returns
    ///
    /// A new calculator with zero cost.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let calc = InvocationCostCalculator::new();
    /// assert_eq!(calc.total_tokens(), 0);
    /// ```
    pub fn new() -> Self {
        InvocationCostCalculator {
            input_tokens: 0,
            output_tokens: 0,
            gpu_ms: 0,
            wall_clock_ms: 0,
            start_time_ms: 0,
        }
    }

    /// Counts input tokens for the invocation.
    ///
    /// # Arguments
    ///
    /// - `input`: Input context string
    ///
    /// # Returns
    ///
    /// - `Ok(token_count)`: Number of input tokens
    /// - `Err(ToolError)`: Token counting failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut calc = InvocationCostCalculator::new();
    /// let tokens = calc.add_input_tokens("search query")?;
    /// ```
    pub fn add_input_tokens(&mut self, input: &str) -> Result<u64> {
        self.input_tokens = TokenCounter::count_input_tokens(input);
        Ok(self.input_tokens)
    }

    /// Counts output tokens for the invocation.
    ///
    /// # Arguments
    ///
    /// - `output`: Output/response string
    ///
    /// # Returns
    ///
    /// - `Ok(token_count)`: Number of output tokens
    /// - `Err(ToolError)`: Token counting failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut calc = InvocationCostCalculator::new();
    /// calc.add_input_tokens("query")?;
    /// let tokens = calc.add_output_tokens("result")?;
    /// ```
    pub fn add_output_tokens(&mut self, output: &str) -> Result<u64> {
        self.output_tokens = TokenCounter::count_output_tokens(output);
        Ok(self.output_tokens)
    }

    /// Records GPU compute time.
    ///
    /// # Arguments
    ///
    /// - `gpu_ms`: GPU time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(())`: GPU time recorded
    /// - `Err(ToolError)`: Invalid GPU time
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut calc = InvocationCostCalculator::new();
    /// calc.record_gpu_time(100)?; // 100ms
    /// ```
    pub fn record_gpu_time(&mut self, gpu_ms: u64) -> Result<()> {
        self.gpu_ms = gpu_ms;
        Ok(())
    }

    /// Records wall-clock time.
    ///
    /// # Arguments
    ///
    /// - `wall_clock_ms`: Wall-clock time in milliseconds
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Wall-clock time recorded
    /// - `Err(ToolError)`: Invalid wall-clock time
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut calc = InvocationCostCalculator::new();
    /// calc.record_wall_clock_time(150)?; // 150ms
    /// ```
    pub fn record_wall_clock_time(&mut self, wall_clock_ms: u64) -> Result<()> {
        self.wall_clock_ms = wall_clock_ms;
        Ok(())
    }

    /// Returns total token count (input + output).
    ///
    /// # Returns
    ///
    /// Total tokens consumed.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }

    /// Calculates TPC-hours for this invocation.
    ///
    /// Formula: TPC-hours = (total_tokens × gpu_hours) / 1_000_000
    ///
    /// # Returns
    ///
    /// TPC-hours as u64 (rounded down).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut calc = InvocationCostCalculator::new();
    /// calc.add_input_tokens("query")?;
    /// calc.add_output_tokens("result")?;
    /// calc.record_gpu_time(100)?;
    /// let tpc = calc.calculate_tpc_hours();
    /// ```
    pub fn calculate_tpc_hours(&self) -> u64 {
        if self.gpu_ms == 0 {
            return 0;
        }

        let gpu_hours = (self.gpu_ms as f64) / (1000.0 * 3600.0);
        let tpc = ((self.total_tokens() as f64) * gpu_hours) / 1_000_000.0;
        tpc as u64
    }

    /// Builds a CostAttribution from the calculated costs.
    ///
    /// # Returns
    ///
    /// - `Ok(CostAttribution)`: Complete cost attribution
    /// - `Err(ToolError)`: Cost calculation failed
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut calc = InvocationCostCalculator::new();
    /// calc.add_input_tokens("query")?;
    /// calc.add_output_tokens("result")?;
    /// calc.record_gpu_time(100)?;
    /// calc.record_wall_clock_time(150)?;
    /// let cost = calc.build_cost_attribution()?;
    /// ```
    pub fn build_cost_attribution(&self) -> Result<CostAttribution> {
        Ok(CostAttribution {
            tokens: self.total_tokens(),
            gpu_ms: self.gpu_ms,
            wall_clock_ms: self.wall_clock_ms,
            tpc_hours: self.calculate_tpc_hours(),
        })
    }

    /// Returns the input token count.
    pub fn input_tokens(&self) -> u64 {
        self.input_tokens
    }

    /// Returns the output token count.
    pub fn output_tokens(&self) -> u64 {
        self.output_tokens
    }

    /// Returns the GPU time in milliseconds.
    pub fn gpu_ms(&self) -> u64 {
        self.gpu_ms
    }

    /// Returns the wall-clock time in milliseconds.
    pub fn wall_clock_ms(&self) -> u64 {
        self.wall_clock_ms
    }
}

impl Default for InvocationCostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invocation_cost_calculator_creation() {
        let calc = InvocationCostCalculator::new();
        assert_eq!(calc.input_tokens(), 0);
        assert_eq!(calc.output_tokens(), 0);
        assert_eq!(calc.gpu_ms(), 0);
        assert_eq!(calc.wall_clock_ms(), 0);
        assert_eq!(calc.total_tokens(), 0);
    }

    #[test]
    fn test_add_input_tokens() {
        let mut calc = InvocationCostCalculator::new();
        let tokens = calc.add_input_tokens("hello world").unwrap();
        assert!(tokens > 0);
        assert_eq!(calc.input_tokens(), tokens);
    }

    #[test]
    fn test_add_output_tokens() {
        let mut calc = InvocationCostCalculator::new();
        let tokens = calc.add_output_tokens("response data").unwrap();
        assert!(tokens > 0);
        assert_eq!(calc.output_tokens(), tokens);
    }

    #[test]
    fn test_total_tokens() {
        let mut calc = InvocationCostCalculator::new();
        calc.add_input_tokens("input").unwrap();
        let output_tokens = calc.add_output_tokens("output").unwrap();
        
        let total = calc.total_tokens();
        assert_eq!(total, calc.input_tokens() + calc.output_tokens());
    }

    #[test]
    fn test_record_gpu_time() {
        let mut calc = InvocationCostCalculator::new();
        calc.record_gpu_time(100).unwrap();
        assert_eq!(calc.gpu_ms(), 100);
    }

    #[test]
    fn test_record_wall_clock_time() {
        let mut calc = InvocationCostCalculator::new();
        calc.record_wall_clock_time(150).unwrap();
        assert_eq!(calc.wall_clock_ms(), 150);
    }

    #[test]
    fn test_calculate_tpc_hours_zero_gpu_time() {
        let mut calc = InvocationCostCalculator::new();
        calc.add_input_tokens("test").unwrap();
        calc.add_output_tokens("result").unwrap();
        // No GPU time recorded
        assert_eq!(calc.calculate_tpc_hours(), 0);
    }

    #[test]
    fn test_calculate_tpc_hours_with_gpu_time() {
        let mut calc = InvocationCostCalculator::new();
        calc.add_input_tokens("test input data").unwrap();
        calc.add_output_tokens("result output data").unwrap();
        calc.record_gpu_time(3600000).unwrap(); // 1 hour in ms
        
        let tpc = calc.calculate_tpc_hours();
        // With 1 hour GPU time, tpc should be > 0 for any tokens
        assert!(tpc > 0);
    }

    #[test]
    fn test_build_cost_attribution() {
        let mut calc = InvocationCostCalculator::new();
        calc.add_input_tokens("input").unwrap();
        calc.add_output_tokens("output").unwrap();
        calc.record_gpu_time(100).unwrap();
        calc.record_wall_clock_time(150).unwrap();

        let cost = calc.build_cost_attribution().unwrap();
        assert_eq!(cost.gpu_ms, 100);
        assert_eq!(cost.wall_clock_ms, 150);
        assert!(cost.tokens > 0);
    }

    #[test]
    fn test_empty_input_output() {
        let mut calc = InvocationCostCalculator::new();
        calc.add_input_tokens("").unwrap();
        calc.add_output_tokens("").unwrap();
        
        assert_eq!(calc.total_tokens(), 0);
    }

    #[test]
    fn test_large_token_counts() {
        let mut calc = InvocationCostCalculator::new();
        let long_input = "a".repeat(1_000_000);
        let long_output = "b".repeat(1_000_000);
        
        calc.add_input_tokens(&long_input).unwrap();
        calc.add_output_tokens(&long_output).unwrap();
        
        assert!(calc.total_tokens() > 0);
        assert!(calc.total_tokens() > 100_000); // Should be ~500k tokens
    }

    #[test]
    fn test_default_creation() {
        let calc = InvocationCostCalculator::default();
        assert_eq!(calc.total_tokens(), 0);
    }

    #[test]
    fn test_saturating_add() {
        let mut calc = InvocationCostCalculator::new();
        calc.input_tokens = u64::MAX;
        calc.output_tokens = 1;
        
        let total = calc.total_tokens();
        assert_eq!(total, u64::MAX); // Should saturate, not overflow
    }
}

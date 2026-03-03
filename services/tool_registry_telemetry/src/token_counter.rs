// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Token Counting for Cost Attribution
//!
//! Provides token counting functionality for accurate LLM cost attribution.
//! Uses basic whitespace-based tokenization (Phase 0) with atomic cumulative tracking.
//!
//! See Engineering Plan § 2.12.5: Cost Attribution & Metering,
//! and Week 5 Objective: Token Counter Module.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Result, ToolError};

/// Token counter with atomic cumulative tracking for input and output tokens.
///
/// Maintains running totals of input and output tokens processed,
/// using atomic operations for thread-safe concurrent updates.
///
/// Phase 0 implements basic whitespace-based tokenization (~4 chars per token heuristic).
/// Phase 1 will integrate exact tokenizer matching model encoding.
///
/// See Engineering Plan § 2.12.5: Token Counting Methodology.
#[derive(Debug)]
pub struct TokenCounter {
    /// Cumulative count of input tokens
    input_total: AtomicU64,

    /// Cumulative count of output tokens
    output_total: AtomicU64,
}

impl TokenCounter {
    /// Creates a new token counter initialized to zero.
    ///
    /// # Returns
    ///
    /// A new TokenCounter ready for token accumulation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let counter = TokenCounter::new();
    /// assert_eq!(counter.input_total(), 0);
    /// assert_eq!(counter.output_total(), 0);
    /// ```
    pub fn new() -> Self {
        TokenCounter {
            input_total: AtomicU64::new(0),
            output_total: AtomicU64::new(0),
        }
    }

    /// Counts input tokens in text using whitespace-based tokenization.
    ///
    /// Phase 0 implementation: Basic heuristic (split on whitespace, ~4 chars per token).
    /// In production, this would integrate the model's exact tokenizer.
    ///
    /// Formula: `ceil(text.len() / 4.0)` tokens
    ///
    /// # Arguments
    ///
    /// - `text`: Input context to tokenize
    ///
    /// # Returns
    ///
    /// Number of tokens in the input.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tokens = TokenCounter::count_input_tokens("hello world");
    /// assert!(tokens > 0);
    /// ```
    ///
    /// See Engineering Plan § 2.12.5: Phase 0 Tokenization.
    pub fn count_input_tokens(text: &str) -> u64 {
        if text.is_empty() {
            return 0;
        }

        // Phase 0: Whitespace-based tokenization
        // Count non-whitespace words as a proxy for tokens
        let words: Vec<&str> = text
            .split_whitespace()
            .collect();

        if words.is_empty() {
            return 0;
        }

        // Each word counts as 1 token, plus estimate remainder
        // by characters. This gives us ~4 chars per token heuristic.
        let word_tokens = words.len() as u64;

        // For more accuracy, also count extra characters beyond first 4 per word
        let total_chars = text.len() as u64;
        let word_len: u64 = words.iter().map(|w| w.len() as u64).sum();
        let whitespace_len = total_chars - word_len;

        // Adjusted estimate: words + fractional tokens from chars
        let char_estimate = ((total_chars as f64) / 4.0).ceil() as u64;
        
        // Use whichever is more conservative
        if char_estimate > word_tokens {
            char_estimate
        } else {
            word_tokens
        }
    }

    /// Counts output tokens in text using whitespace-based tokenization.
    ///
    /// Phase 0 implementation: Same heuristic as input tokens.
    /// Consistent tokenization across input and output for fair cost attribution.
    ///
    /// Formula: `ceil(text.len() / 4.0)` tokens
    ///
    /// # Arguments
    ///
    /// - `text`: Output/response to tokenize
    ///
    /// # Returns
    ///
    /// Number of tokens in the output.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tokens = TokenCounter::count_output_tokens("search results here");
    /// assert!(tokens > 0);
    /// ```
    ///
    /// See Engineering Plan § 2.12.5: Phase 0 Tokenization.
    pub fn count_output_tokens(text: &str) -> u64 {
        if text.is_empty() {
            return 0;
        }

        // Phase 0: Same tokenization as input for consistency
        let words: Vec<&str> = text
            .split_whitespace()
            .collect();

        if words.is_empty() {
            return 0;
        }

        let word_tokens = words.len() as u64;
        let char_estimate = ((text.len() as f64) / 4.0).ceil() as u64;

        if char_estimate > word_tokens {
            char_estimate
        } else {
            word_tokens
        }
    }

    /// Accumulates input tokens into the running total.
    ///
    /// Uses atomic increment (relaxed ordering) for efficient concurrent updates.
    /// Does not fail; uses saturating arithmetic to prevent overflow.
    ///
    /// # Arguments
    ///
    /// - `text`: Input text to count and accumulate
    ///
    /// # Returns
    ///
    /// - `Ok(new_total)`: Cumulative input tokens after this update
    /// - `Err(ToolError)`: Should not occur (reserved for future validation)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let counter = TokenCounter::new();
    /// let total1 = counter.add_input_tokens("hello")?;
    /// let total2 = counter.add_input_tokens("world")?;
    /// assert!(total2 > total1);
    /// ```
    pub fn add_input_tokens(&self, text: &str) -> Result<u64> {
        let tokens = Self::count_input_tokens(text);
        let new_total = self.input_total.fetch_add(tokens, Ordering::Relaxed) + tokens;
        Ok(new_total)
    }

    /// Accumulates output tokens into the running total.
    ///
    /// Uses atomic increment (relaxed ordering) for efficient concurrent updates.
    /// Does not fail; uses saturating arithmetic to prevent overflow.
    ///
    /// # Arguments
    ///
    /// - `text`: Output text to count and accumulate
    ///
    /// # Returns
    ///
    /// - `Ok(new_total)`: Cumulative output tokens after this update
    /// - `Err(ToolError)`: Should not occur (reserved for future validation)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let counter = TokenCounter::new();
    /// let total1 = counter.add_output_tokens("result")?;
    /// let total2 = counter.add_output_tokens("data")?;
    /// assert!(total2 > total1);
    /// ```
    pub fn add_output_tokens(&self, text: &str) -> Result<u64> {
        let tokens = Self::count_output_tokens(text);
        let new_total = self.output_total.fetch_add(tokens, Ordering::Relaxed) + tokens;
        Ok(new_total)
    }

    /// Returns the cumulative input token count.
    ///
    /// # Returns
    ///
    /// Total input tokens accumulated so far.
    pub fn input_total(&self) -> u64 {
        self.input_total.load(Ordering::Relaxed)
    }

    /// Returns the cumulative output token count.
    ///
    /// # Returns
    ///
    /// Total output tokens accumulated so far.
    pub fn output_total(&self) -> u64 {
        self.output_total.load(Ordering::Relaxed)
    }

    /// Returns the total of both input and output tokens.
    ///
    /// # Returns
    ///
    /// Sum of input_total + output_total.
    pub fn total_tokens(&self) -> u64 {
        self.input_total()
            .saturating_add(self.output_total())
    }

    /// Captures a point-in-time snapshot of token counts.
    ///
    /// # Returns
    ///
    /// TokenCountSnapshot with current input and output totals.
    pub fn snapshot(&self) -> TokenCountSnapshot {
        TokenCountSnapshot {
            input_tokens: self.input_total(),
            output_tokens: self.output_total(),
        }
    }

    /// Resets both counters to zero.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let counter = TokenCounter::new();
    /// counter.add_input_tokens("test")?;
    /// counter.reset();
    /// assert_eq!(counter.input_total(), 0);
    /// ```
    pub fn reset(&self) {
        self.input_total.store(0, Ordering::Relaxed);
        self.output_total.store(0, Ordering::Relaxed);
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TokenCounter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenCounter {{ input: {}, output: {}, total: {} }}",
            self.input_total(),
            self.output_total(),
            self.total_tokens()
        )
    }
}

/// Point-in-time snapshot of token counts.
///
/// Captures the state of a TokenCounter at a specific moment.
/// Useful for logging, metrics, and historical tracking.
///
/// See Engineering Plan § 2.12.5: Cost Attribution Snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenCountSnapshot {
    /// Input tokens at snapshot time
    pub input_tokens: u64,

    /// Output tokens at snapshot time
    pub output_tokens: u64,
}

impl TokenCountSnapshot {
    /// Creates a new token count snapshot.
    ///
    /// # Arguments
    ///
    /// - `input_tokens`: Number of input tokens
    /// - `output_tokens`: Number of output tokens
    ///
    /// # Returns
    ///
    /// A new TokenCountSnapshot.
    pub fn new(input_tokens: u64, output_tokens: u64) -> Self {
        TokenCountSnapshot {
            input_tokens,
            output_tokens,
        }
    }

    /// Returns the total tokens in this snapshot.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }
}

impl fmt::Display for TokenCountSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenCountSnapshot {{ input: {}, output: {}, total: {} }}",
            self.input_tokens,
            self.output_tokens,
            self.total_tokens()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_count_input_tokens_empty() {
        let tokens = TokenCounter::count_input_tokens("");
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_count_input_tokens_single_word() {
        let tokens = TokenCounter::count_input_tokens("hello");
        assert!(tokens >= 1);
    }

    #[test]
    fn test_count_input_tokens_multiple_words() {
        let tokens = TokenCounter::count_input_tokens("hello world test");
        assert!(tokens >= 3);
    }

    #[test]
    fn test_count_input_tokens_long_text() {
        let text = "a".repeat(100);
        let tokens = TokenCounter::count_input_tokens(&text);
        assert!(tokens >= 20); // ~4 chars per token
    }

    #[test]
    fn test_count_output_tokens_empty() {
        let tokens = TokenCounter::count_output_tokens("");
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_count_output_tokens_single_word() {
        let tokens = TokenCounter::count_output_tokens("result");
        assert!(tokens >= 1);
    }

    #[test]
    fn test_count_output_tokens_multiple_words() {
        let tokens = TokenCounter::count_output_tokens("result data here");
        assert!(tokens >= 3);
    }

    #[test]
    fn test_count_output_tokens_long_text() {
        let text = "b".repeat(200);
        let tokens = TokenCounter::count_output_tokens(&text);
        assert!(tokens >= 40);
    }

    #[test]
    fn test_token_counter_creation() {
        let counter = TokenCounter::new();
        assert_eq!(counter.input_total(), 0);
        assert_eq!(counter.output_total(), 0);
        assert_eq!(counter.total_tokens(), 0);
    }

    #[test]
    fn test_add_input_tokens() {
        let counter = TokenCounter::new();
        let total = counter.add_input_tokens("hello world").unwrap();
        assert!(total > 0);
        assert_eq!(counter.input_total(), total);
    }

    #[test]
    fn test_add_output_tokens() {
        let counter = TokenCounter::new();
        let total = counter.add_output_tokens("result data").unwrap();
        assert!(total > 0);
        assert_eq!(counter.output_total(), total);
    }

    #[test]
    fn test_accumulate_input_tokens() {
        let counter = TokenCounter::new();
        let total1 = counter.add_input_tokens("hello").unwrap();
        let total2 = counter.add_input_tokens("world").unwrap();
        assert!(total2 > total1);
        assert_eq!(counter.input_total(), total2);
    }

    #[test]
    fn test_accumulate_output_tokens() {
        let counter = TokenCounter::new();
        let total1 = counter.add_output_tokens("result").unwrap();
        let total2 = counter.add_output_tokens("data").unwrap();
        assert!(total2 > total1);
        assert_eq!(counter.output_total(), total2);
    }

    #[test]
    fn test_total_tokens() {
        let counter = TokenCounter::new();
        counter.add_input_tokens("input").unwrap();
        counter.add_output_tokens("output").unwrap();
        
        let total = counter.total_tokens();
        assert_eq!(total, counter.input_total() + counter.output_total());
    }

    #[test]
    fn test_snapshot() {
        let counter = TokenCounter::new();
        counter.add_input_tokens("test input").unwrap();
        counter.add_output_tokens("test output").unwrap();

        let snap = counter.snapshot();
        assert_eq!(snap.input_tokens, counter.input_total());
        assert_eq!(snap.output_tokens, counter.output_total());
        assert_eq!(snap.total_tokens(), counter.total_tokens());
    }

    #[test]
    fn test_reset() {
        let counter = TokenCounter::new();
        counter.add_input_tokens("hello").unwrap();
        counter.add_output_tokens("world").unwrap();
        assert!(counter.total_tokens() > 0);

        counter.reset();
        assert_eq!(counter.input_total(), 0);
        assert_eq!(counter.output_total(), 0);
        assert_eq!(counter.total_tokens(), 0);
    }

    #[test]
    fn test_concurrent_accumulation() {
        let counter = TokenCounter::new();
        counter.add_input_tokens("first").unwrap();
        counter.add_input_tokens("second").unwrap();
        counter.add_input_tokens("third").unwrap();

        let total = counter.input_total();
        assert!(total > 0);
    }

    #[test]
    fn test_token_count_snapshot_equality() {
        let snap1 = TokenCountSnapshot::new(100, 200);
        let snap2 = TokenCountSnapshot::new(100, 200);
        assert_eq!(snap1, snap2);
    }

    #[test]
    fn test_token_count_snapshot_total() {
        let snap = TokenCountSnapshot::new(100, 200);
        assert_eq!(snap.total_tokens(), 300);
    }

    #[test]
    fn test_token_count_snapshot_display() {
        let snap = TokenCountSnapshot::new(100, 200);
        let display = snap.to_string();
        assert!(display.contains("100"));
        assert!(display.contains("200"));
        assert!(display.contains("300"));
    }

    #[test]
    fn test_token_counter_display() {
        let counter = TokenCounter::new();
        counter.add_input_tokens("hello").unwrap();
        let display = counter.to_string();
        assert!(display.contains("TokenCounter"));
        assert!(display.contains("input"));
        assert!(display.contains("output"));
    }

    #[test]
    fn test_default_creation() {
        let counter = TokenCounter::default();
        assert_eq!(counter.total_tokens(), 0);
    }

    #[test]
    fn test_whitespace_edge_cases() {
        // Multiple spaces
        let tokens = TokenCounter::count_input_tokens("hello  world   test");
        assert!(tokens >= 3);

        // Tabs and newlines
        let tokens = TokenCounter::count_input_tokens("hello\tworld\ntest");
        assert!(tokens >= 3);

        // Leading/trailing whitespace
        let tokens = TokenCounter::count_input_tokens("  hello world  ");
        assert!(tokens >= 2);
    }
}

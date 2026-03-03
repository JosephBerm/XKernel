// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Adapter Utilities
//!
//! Common adapter utility library providing shared translation helpers,
//! serialization utilities, error handling patterns, and result aggregation.
//!
//! Sec 5.2: Common Adapter Utilities
//! Sec 4.2: Shared Translation Infrastructure

use std::collections::BTreeMap;
use crate::error::AdapterError;
use crate::AdapterResult;

/// Result aggregator for collecting multiple operation results.
/// Sec 5.2: Result Aggregation Utilities
#[derive(Debug, Clone)]
pub struct ResultAggregator<T> {
    /// Successful results
    results: Vec<T>,
    /// Errors encountered
    errors: Vec<AdapterError>,
    /// Whether to fail on first error
    fail_fast: bool,
}

impl<T: Clone> ResultAggregator<T> {
    /// Creates a new result aggregator.
    pub fn new(fail_fast: bool) -> Self {
        ResultAggregator {
            results: Vec::new(),
            errors: Vec::new(),
            fail_fast,
        }
    }

    /// Adds a result.
    pub fn add_result(&mut self, result: T) {
        self.results.push(result);
    }

    /// Adds an error.
    pub fn add_error(&mut self, error: AdapterError) -> bool {
        self.errors.push(error);
        self.fail_fast
    }

    /// Gets all successful results.
    pub fn results(&self) -> &[T] {
        &self.results
    }

    /// Gets all errors.
    pub fn errors(&self) -> &[AdapterError] {
        &self.errors
    }

    /// Returns true if aggregator has errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Converts to Result type.
    pub fn into_result(self) -> AdapterResult<Vec<T>> {
        if self.has_errors() {
            Err(self
                .errors
                .first()
                .cloned()
                .unwrap_or_else(|| AdapterError::TranslationError("Unknown error".into())))
        } else {
            Ok(self.results)
        }
    }

    /// Gets count of results.
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    /// Gets count of errors.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

/// Serialization utilities for framework artifact translation.
/// Sec 5.2: Serialization Utilities
pub struct SerializationHelper;

impl SerializationHelper {
    /// Validates JSON-like serialized data.
    pub fn validate_json_string(data: &str) -> AdapterResult<()> {
        // Basic JSON validation (opening and closing braces/brackets)
        let trimmed = data.trim();
        let is_valid_json = (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            || trimmed == "null"
            || trimmed == "true"
            || trimmed == "false"
            || trimmed.parse::<i64>().is_ok()
            || trimmed.parse::<f64>().is_ok()
            || (trimmed.starts_with('"') && trimmed.ends_with('"'));

        if is_valid_json {
            Ok(())
        } else {
            Err(AdapterError::SerializationError(
                format!("Invalid JSON format: {}", trimmed),
            ))
        }
    }

    /// Escapes special characters in strings for JSON.
    pub fn escape_json_string(input: &str) -> String {
        let mut result = String::new();
        for c in input.chars() {
            match c {
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                _ => result.push(c),
            }
        }
        result
    }

    /// Extracts a value from a simple key-value JSON object.
    pub fn extract_json_field(json: &str, key: &str) -> AdapterResult<String> {
        let key_pattern = format!("\"{}\":", key);
        json.find(&key_pattern)
            .ok_or_else(|| {
                AdapterError::SerializationError(format!("Field not found: {}", key))
            })
            .and_then(|pos| {
                let start = pos + key_pattern.len();
                let remaining = &json[start..];
                // Skip whitespace
                let trimmed = remaining.trim_start();
                // Find the value (simplified - handles strings and numbers)
                if trimmed.starts_with('"') {
                    trimmed[1..]
                        .find('"')
                        .map(|end| trimmed[1..=end].to_string())
                        .ok_or_else(|| {
                            AdapterError::SerializationError("Unterminated string".into())
                        })
                } else {
                    // Handle number or boolean
                    let end = trimmed
                        .find(|c: char| c == ',' || c == '}' || c == ']')
                        .unwrap_or(trimmed.len());
                    Ok(trimmed[..end].trim().to_string())
                }
            })
    }
}

/// Validation utilities for adapter configurations and artifacts.
/// Sec 5.2: Validation Utilities
pub struct ValidationHelper;

impl ValidationHelper {
    /// Validates adapter configuration has required fields.
    pub fn validate_required_fields(config: &[(String, Option<String>)]) -> AdapterResult<()> {
        for (field_name, value) in config {
            if value.is_none() || value.as_ref().map_or(true, |v| v.is_empty()) {
                return Err(AdapterError::ConfigurationError(format!(
                    "Required field missing: {}",
                    field_name
                )));
            }
        }
        Ok(())
    }

    /// Validates timeout value is within acceptable range.
    pub fn validate_timeout(timeout_ms: u64) -> AdapterResult<()> {
        const MIN_TIMEOUT_MS: u64 = 100;
        const MAX_TIMEOUT_MS: u64 = 3600000; // 1 hour

        if timeout_ms < MIN_TIMEOUT_MS {
            Err(AdapterError::ConfigurationError(format!(
                "Timeout too short: {} < {}",
                timeout_ms, MIN_TIMEOUT_MS
            )))
        } else if timeout_ms > MAX_TIMEOUT_MS {
            Err(AdapterError::ConfigurationError(format!(
                "Timeout too long: {} > {}",
                timeout_ms, MAX_TIMEOUT_MS
            )))
        } else {
            Ok(())
        }
    }

    /// Validates memory capacity is within reasonable bounds.
    pub fn validate_memory_capacity(tokens: u64) -> AdapterResult<()> {
        const MIN_TOKENS: u64 = 256;
        const MAX_TOKENS: u64 = 1_000_000_000; // 1B tokens

        if tokens < MIN_TOKENS {
            Err(AdapterError::ConfigurationError(format!(
                "Memory capacity too small: {} < {}",
                tokens, MIN_TOKENS
            )))
        } else if tokens > MAX_TOKENS {
            Err(AdapterError::ConfigurationError(format!(
                "Memory capacity too large: {} > {}",
                tokens, MAX_TOKENS
            )))
        } else {
            Ok(())
        }
    }

    /// Validates identifier format.
    pub fn validate_identifier(id: &str) -> AdapterResult<()> {
        if id.is_empty() {
            return Err(AdapterError::ConfigurationError(
                "Identifier cannot be empty".into(),
            ));
        }
        if id.len() > 256 {
            return Err(AdapterError::ConfigurationError(
                "Identifier too long (max 256 chars)".into(),
            ));
        }
        if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(AdapterError::ConfigurationError(
                "Identifier contains invalid characters".into(),
            ));
        }
        Ok(())
    }
}

/// Translation metrics helper for tracking translation performance.
/// Sec 5.2: Metrics Utilities
#[derive(Debug, Clone)]
pub struct TranslationMetricsHelper {
    /// Translation operations tracked
    operations: BTreeMap<String, MetricSnapshot>,
}

/// Snapshot of a single metric.
#[derive(Debug, Clone)]
struct MetricSnapshot {
    count: u64,
    total_ns: u64,
    min_ns: u64,
    max_ns: u64,
}

impl TranslationMetricsHelper {
    /// Creates a new metrics helper.
    pub fn new() -> Self {
        TranslationMetricsHelper {
            operations: BTreeMap::new(),
        }
    }

    /// Records a translation operation duration.
    pub fn record_operation(&mut self, operation: String, duration_ns: u64) {
        self.operations
            .entry(operation)
            .and_modify(|m| {
                m.count += 1;
                m.total_ns += duration_ns;
                if duration_ns < m.min_ns {
                    m.min_ns = duration_ns;
                }
                if duration_ns > m.max_ns {
                    m.max_ns = duration_ns;
                }
            })
            .or_insert(MetricSnapshot {
                count: 1,
                total_ns: duration_ns,
                min_ns: duration_ns,
                max_ns: duration_ns,
            });
    }

    /// Gets average duration for operation.
    pub fn average_duration_ns(&self, operation: &str) -> Option<u64> {
        self.operations.get(operation).map(|m| m.total_ns / m.count)
    }

    /// Gets operation count.
    pub fn operation_count(&self, operation: &str) -> Option<u64> {
        self.operations.get(operation).map(|m| m.count)
    }

    /// Gets total operations tracked.
    pub fn total_operations(&self) -> u64 {
        self.operations.values().map(|m| m.count).sum()
    }

    /// Gets all operation names.
    pub fn operations(&self) -> Vec<String> {
        self.operations.keys().cloned().collect()
    }
}

impl Default for TranslationMetricsHelper {
    fn default() -> Self {
        Self::new()
    }
}

/// Error handling helper for adapter error recovery.
/// Sec 5.2: Error Handling Utilities
pub struct ErrorHandlingHelper;

impl ErrorHandlingHelper {
    /// Determines if an error is recoverable.
    pub fn is_recoverable(error: &AdapterError) -> bool {
        matches!(
            error,
            AdapterError::TranslationError(_)
                | AdapterError::MappingFailed { .. }
                | AdapterError::ConfigurationError(_)
                | AdapterError::KernelIpcError(_)
        )
    }

    /// Creates a detailed error report.
    pub fn create_error_report(error: &AdapterError) -> String {
        format!(
            "AdapterError: {}\nRecoverable: {}\nType: {}",
            error,
            Self::is_recoverable(error),
            match error {
                AdapterError::UnsupportedFramework(_) => "UnsupportedFramework",
                AdapterError::MappingFailed { .. } => "MappingFailed",
                AdapterError::TranslationError(_) => "TranslationError",
                AdapterError::IncompatibleVersion { .. } => "IncompatibleVersion",
                AdapterError::FidelityLoss { .. } => "FidelityLoss",
                AdapterError::MemoryMappingError(_) => "MemoryMappingError",
                AdapterError::ToolBindingError(_) => "ToolBindingError",
                AdapterError::ChannelMappingError(_) => "ChannelMappingError",
                AdapterError::FrameworkCompatibilityError(_) => "FrameworkCompatibilityError",
                AdapterError::KernelIpcError(_) => "KernelIpcError",
                AdapterError::AdapterStateError { .. } => "AdapterStateError",
                AdapterError::ConfigurationError(_) => "ConfigurationError",
                AdapterError::SerializationError(_) => "SerializationError",
                AdapterError::InvalidReference(_) => "InvalidReference",
                AdapterError::LockError(_) => "LockError",
                AdapterError::ValidationError(_) => "ValidationError",
                AdapterError::StateError(_) => "StateError",
                AdapterError::SyscallError(_) => "SyscallError",
                AdapterError::ConfigError(_) => "ConfigError",
                AdapterError::MemoryError(_) => "MemoryError",
                AdapterError::RetryExhausted(_) => "RetryExhausted",
                AdapterError::RetryableError(_) => "RetryableError",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_aggregator_success() {
        let mut agg = ResultAggregator::new(true);
        agg.add_result("result1".to_string());
        agg.add_result("result2".to_string());

        assert_eq!(agg.result_count(), 2);
        assert_eq!(agg.error_count(), 0);
        assert!(!agg.has_errors());
    }

    #[test]
    fn test_result_aggregator_with_errors() {
        let mut agg = ResultAggregator::new(false);
        agg.add_result("result1".to_string());
        agg.add_error(AdapterError::TranslationError("Error 1".into()));

        assert_eq!(agg.result_count(), 1);
        assert_eq!(agg.error_count(), 1);
        assert!(agg.has_errors());
    }

    #[test]
    fn test_result_aggregator_into_result() {
        let mut agg = ResultAggregator::new(true);
        agg.add_result(42);
        agg.add_result(43);

        let result = agg.into_result();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_serialization_helper_validate_json() {
        assert!(SerializationHelper::validate_json_string("{}").is_ok());
        assert!(SerializationHelper::validate_json_string("[]").is_ok());
        assert!(SerializationHelper::validate_json_string("null").is_ok());
        assert!(SerializationHelper::validate_json_string("true").is_ok());
        assert!(SerializationHelper::validate_json_string("false").is_ok());
        assert!(SerializationHelper::validate_json_string("123").is_ok());
        assert!(SerializationHelper::validate_json_string("\"string\"").is_ok());
        assert!(SerializationHelper::validate_json_string("invalid").is_err());
    }

    #[test]
    fn test_serialization_helper_escape_json() {
        let result = SerializationHelper::escape_json_string("test\"quote");
        assert!(result.contains("\\\""));

        let result = SerializationHelper::escape_json_string("test\\backslash");
        assert!(result.contains("\\\\"));
    }

    #[test]
    fn test_serialization_helper_extract_field() {
        let json = r#"{"name":"test","value":123}"#;
        let name = SerializationHelper::extract_json_field(json, "name");
        assert!(name.is_ok());

        let missing = SerializationHelper::extract_json_field(json, "missing");
        assert!(missing.is_err());
    }

    #[test]
    fn test_validation_helper_timeout() {
        assert!(ValidationHelper::validate_timeout(1000).is_ok());
        assert!(ValidationHelper::validate_timeout(50).is_err());
        assert!(ValidationHelper::validate_timeout(4_000_000).is_err());
    }

    #[test]
    fn test_validation_helper_memory() {
        assert!(ValidationHelper::validate_memory_capacity(100_000).is_ok());
        assert!(ValidationHelper::validate_memory_capacity(100).is_err());
        assert!(ValidationHelper::validate_memory_capacity(2_000_000_000).is_err());
    }

    #[test]
    fn test_validation_helper_identifier() {
        assert!(ValidationHelper::validate_identifier("valid-id_123").is_ok());
        assert!(ValidationHelper::validate_identifier("").is_err());
        assert!(ValidationHelper::validate_identifier("invalid@id").is_err());
    }

    #[test]
    fn test_metrics_helper() {
        let mut helper = TranslationMetricsHelper::new();
        helper.record_operation("translate".into(), 1000);
        helper.record_operation("translate".into(), 2000);

        assert_eq!(helper.operation_count("translate"), Some(2));
        assert_eq!(helper.average_duration_ns("translate"), Some(1500));
        assert_eq!(helper.total_operations(), 2);
    }

    #[test]
    fn test_error_handling_is_recoverable() {
        let recoverable = AdapterError::TranslationError("test".into());
        let not_recoverable =
            AdapterError::UnsupportedFramework("framework".into());

        assert!(ErrorHandlingHelper::is_recoverable(&recoverable));
        assert!(!ErrorHandlingHelper::is_recoverable(&not_recoverable));
    }

    #[test]
    fn test_error_handling_create_report() {
        let error = AdapterError::ConfigurationError("Missing field".into());
        let report = ErrorHandlingHelper::create_error_report(&error);
        assert!(report.contains("ConfigurationError"));
        assert!(report.contains("Recoverable"));
    }
}

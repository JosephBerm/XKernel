// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Adapter Error Types
//!
//! Comprehensive error handling for framework adapter operations.
//! Sec 4.2: Error Handling and Recovery
//! Sec 5.2: Extended Error Types for Week 5

use thiserror::Error;

/// Errors that can occur during framework adaptation and translation.
/// Sec 4.2: AdapterError Classification
/// Sec 5.2: Extended Error Types
#[derive(Error, Debug, Clone)]
pub enum AdapterError {
    /// The requested framework is not supported.
    /// Sec 4.2: Unsupported Framework Detection
    #[error("Unsupported framework: {0}")]
    UnsupportedFramework(String),

    /// Concept mapping failed (no direct mapping exists).
    /// Sec 4.3: Mapping Resolution Failure
    #[error("Failed to map {framework_concept} to CSCI entity: {reason}")]
    MappingFailed {
        /// The framework concept that couldn't be mapped
        framework_concept: String,
        /// Reason for mapping failure
        reason: String,
    },

    /// Translation of framework artifact to CSCI format failed.
    /// Sec 4.2: Translation Pipeline Failures
    /// Sec 5.2: Adapter Translation Error
    #[error("Translation error: {0}")]
    TranslationError(String),

    /// Framework version incompatibility detected.
    /// Sec 4.2: Version Compatibility Checking
    #[error("Incompatible framework version: {required} required, {found} found")]
    IncompatibleVersion {
        /// Required version specification
        required: String,
        /// Version that was found
        found: String,
    },

    /// Fidelity loss exceeds acceptable threshold.
    /// Sec 4.3: Fidelity Constraints
    #[error("Mapping fidelity degradation: {detail}")]
    FidelityLoss {
        /// Details about the fidelity loss
        detail: String,
    },

    /// Memory configuration is incompatible with CSCI semantics.
    /// Sec 4.2: Memory Mapping Constraints
    #[error("Memory mapping incompatibility: {0}")]
    MemoryMappingError(String),

    /// Tool binding configuration is invalid.
    /// Sec 4.2: Tool Binding Validation
    #[error("Tool binding error: {0}")]
    ToolBindingError(String),

    /// Communication channel configuration failed.
    /// Sec 4.2: Channel Configuration
    #[error("Channel mapping error: {0}")]
    ChannelMappingError(String),

    /// Framework compatibility issue detected during adapter initialization.
    /// Sec 5.2: Framework Compatibility Error
    #[error("Framework compatibility error: {0}")]
    FrameworkCompatibilityError(String),

    /// Kernel IPC communication failure.
    /// Sec 5.2: Kernel IPC Error
    #[error("Kernel IPC error: {0}")]
    KernelIpcError(String),

    /// Adapter state machine violation.
    /// Sec 5.2: Adapter State Error
    #[error("Adapter state error: {detail}")]
    AdapterStateError {
        /// Details about the state violation
        detail: String,
    },

    /// Configuration validation error.
    /// Sec 5.2: Configuration Error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Serialization/deserialization failure.
    /// Sec 5.2: Serialization Error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid adapter reference or lifecycle.
    /// Sec 5.2: Reference Error
    #[error("Invalid adapter reference: {0}")]
    InvalidReference(String),

    /// Lock acquisition failure (e.g., Mutex/RwLock poisoned).
    #[error("Lock error: {0}")]
    LockError(String),

    /// Validation error for adapter inputs or configurations.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// State machine transition error.
    #[error("State error: {0}")]
    StateError(String),

    /// Syscall invocation failure.
    #[error("Syscall error: {0}")]
    SyscallError(String),

    /// Configuration error (short alias).
    #[error("Config error: {0}")]
    ConfigError(String),

    /// Memory operation failure.
    #[error("Memory error: {0}")]
    MemoryError(String),

    /// Retry attempts exhausted.
    #[error("Retry exhausted: {0}")]
    RetryExhausted(String),

    /// Retryable transient error.
    #[error("Retryable error: {0}")]
    RetryableError(String),
}


/// Result type for adapter operations.
pub type AdapterResult<T> = Result<T, AdapterError>;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AdapterError::UnsupportedFramework("CustomFramework".into());
        assert!(err.to_string().contains("Unsupported framework"));
    }

    #[test]
    fn test_mapping_failed_error() {
        let err = AdapterError::MappingFailed {
            framework_concept: "Chain".into(),
            reason: "No CSCI entity exists for this pattern".into(),
        };
        assert!(err.to_string().contains("Chain"));
        assert!(err.to_string().contains("CSCI entity"));
    }

    #[test]
    fn test_incompatible_version_error() {
        let err = AdapterError::IncompatibleVersion {
            required: ">=0.1.0".into(),
            found: "0.0.1".into(),
        };
        assert!(err.to_string().contains("Incompatible"));
    }

    #[test]
    fn test_framework_compatibility_error() {
        let err = AdapterError::FrameworkCompatibilityError("GPU access required".into());
        assert!(err.to_string().contains("Framework compatibility"));
    }

    #[test]
    fn test_kernel_ipc_error() {
        let err = AdapterError::KernelIpcError("Channel creation failed".into());
        assert!(err.to_string().contains("Kernel IPC error"));
    }

    #[test]
    fn test_adapter_state_error() {
        let err = AdapterError::AdapterStateError {
            detail: "Cannot translate_chain in Initialized state".into(),
        };
        assert!(err.to_string().contains("state error"));
    }

    #[test]
    fn test_configuration_error() {
        let err = AdapterError::ConfigurationError("Missing timeout_ms".into());
        assert!(err.to_string().contains("Configuration error"));
    }

    #[test]
    fn test_serialization_error() {
        let err = AdapterError::SerializationError("Invalid JSON schema".into());
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_invalid_reference_error() {
        let err = AdapterError::InvalidReference("Adapter not initialized".into());
        assert!(err.to_string().contains("Invalid adapter reference"));
    }
}

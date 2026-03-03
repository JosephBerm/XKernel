// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Capability Engine error types.
//!
//! This module defines the error enum for all capability operations.
//! See Engineering Plan § 3.2.2: Error Handling & Recovery.

use thiserror::Error;

/// Errors that may occur during capability operations.
///
/// Per Engineering Plan § 3.2.2, all capability operations return `Result<T, CapError>`.
/// This enum provides typed, detailed error information for debugging and policy enforcement.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CapError {
    /// Capability was forged or tampered with (detected by validation).
    /// See § 3.1.3: Unforgeable Identity & Kernel Assignment.
    #[error("capability appears to be forged or tampered")]
    Forged,

    /// Capability has expired.
    /// See § 3.1.5: Time-Bounded Validity.
    #[error("capability has expired at {0}")]
    Expired(String),

    /// Rate limit has been exceeded for this capability.
    /// See § 3.1.6: Rate & Volume Constraints.
    #[error("rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Data volume limit has been exceeded.
    /// See § 3.1.6: Rate & Volume Constraints.
    #[error("volume limit exceeded: {0}")]
    VolumeExceeded(String),

    /// Delegation depth limit has been exceeded.
    /// See § 3.1.7: Delegation & Attenuation.
    #[error("delegation depth limit exceeded: {0}")]
    DepthExceeded(String),

    /// Capability does not contain required operations.
    /// See § 3.1.1: Discrete Operations & Composition.
    #[error("insufficient operations: required {required:?}, have {have:?}")]
    InsufficientOperations { required: String, have: String },

    /// Mandatory security policy has denied the operation.
    /// See § 3.1.4: Mandatory & Stateless.
    #[error("mandatory policy denied operation: {0}")]
    PolicyDenied(String),

    /// Revocation of this capability failed.
    /// See § 3.1.8: Revocation & Liveness.
    #[error("revocation failed: {0}")]
    RevocationFailed(String),

    /// Attenuation operation is invalid.
    /// See § 3.1.7: Delegation & Attenuation.
    #[error("invalid attenuation: {0}")]
    InvalidAttenuation(String),

    /// Invalid chain structure or operation.
    /// See § 3.1.2: Provenance & Chains.
    #[error("invalid chain operation: {0}")]
    InvalidChain(String),

    /// Invalid resource reference or type mismatch.
    /// See § 3.1.0: Core Domain Model.
    #[error("invalid resource reference: {0}")]
    InvalidResourceRef(String),

    /// Invalid policy structure or definition.
    /// See § 3.1.4: Mandatory & Stateless.
    #[error("invalid policy: {0}")]
    InvalidPolicy(String),

    /// Policy exception is invalid or malformed.
    /// See § 3.1.4: Mandatory & Stateless.
    #[error("invalid exception: {0}")]
    InvalidException(String),

    /// Interaction proof validation failed.
    /// See § 3.2.1: Verification & Proofs.
    #[error("interaction proof failed: {0}")]
    InteractionProofFailed(String),

    /// Generic capability error with context.
    #[error("capability error: {0}")]
    Other(String),
}

impl CapError {
    /// Returns true if this error is due to expiration.
    pub fn is_expired(&self) -> bool {
        matches!(self, CapError::Expired(_))
    }

    /// Returns true if this error is due to rate limiting.
    pub fn is_rate_limited(&self) -> bool {
        matches!(self, CapError::RateLimitExceeded(_))
    }

    /// Returns true if this error is due to a policy denial.
    pub fn is_policy_denied(&self) -> bool {
        matches!(self, CapError::PolicyDenied(_))
    }

    /// Returns true if this error is due to an invalid policy.
    /// See § 3.1.4: Mandatory & Stateless.
    pub fn is_invalid_policy(&self) -> bool {
        matches!(self, CapError::InvalidPolicy(_))
    }

    /// Returns true if this error is due to an invalid exception.
    /// See § 3.1.4: Mandatory & Stateless.
    pub fn is_invalid_exception(&self) -> bool {
        matches!(self, CapError::InvalidException(_))
    }

    /// Returns true if this error is due to an interaction proof failure.
    /// See § 3.2.1: Verification & Proofs.
    pub fn is_interaction_proof_failed(&self) -> bool {
        matches!(self, CapError::InteractionProofFailed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_error_display() {
        let err = CapError::Expired("2026-03-01T12:00:00Z".to_string());
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn test_error_is_expired() {
        let err = CapError::Expired("2026-03-01T12:00:00Z".to_string());
        assert!(err.is_expired());
        assert!(!err.is_rate_limited());
    }

    #[test]
    fn test_error_is_rate_limited() {
        let err = CapError::RateLimitExceeded("100 ops/sec exceeded".to_string());
        assert!(err.is_rate_limited());
        assert!(!err.is_policy_denied());
    }

    #[test]
    fn test_error_is_policy_denied() {
        let err = CapError::PolicyDenied("Agent not authorized".to_string());
        assert!(err.is_policy_denied());
    }

    #[test]
    fn test_error_invalid_policy() {
        let err = CapError::InvalidPolicy("policy ID cannot be empty".to_string());
        assert!(err.is_invalid_policy());
        assert!(err.to_string().contains("invalid policy"));
    }

    #[test]
    fn test_error_invalid_exception() {
        let err = CapError::InvalidException("exception pattern malformed".to_string());
        assert!(err.is_invalid_exception());
        assert!(err.to_string().contains("invalid exception"));
    }

    #[test]
    fn test_error_interaction_proof_failed() {
        let err = CapError::InteractionProofFailed("bypass detected".to_string());
        assert!(err.is_interaction_proof_failed());
        assert!(err.to_string().contains("interaction proof failed"));
    }

    #[test]
    fn test_error_invalid_policy_vs_others() {
        let err = CapError::InvalidPolicy("test".to_string());
        assert!(!err.is_expired());
        assert!(!err.is_rate_limited());
        assert!(!err.is_policy_denied());
        assert!(err.is_invalid_policy());
    }

    #[test]
    fn test_error_invalid_exception_vs_others() {
        let err = CapError::InvalidException("test".to_string());
        assert!(!err.is_expired());
        assert!(!err.is_invalid_policy());
        assert!(err.is_invalid_exception());
    }

    #[test]
    fn test_error_interaction_proof_failed_vs_others() {
        let err = CapError::InteractionProofFailed("test".to_string());
        assert!(!err.is_policy_denied());
        assert!(!err.is_invalid_policy());
        assert!(err.is_interaction_proof_failed());
    }
}

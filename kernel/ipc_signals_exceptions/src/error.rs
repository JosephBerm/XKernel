// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Error Types for IPC Subsystem
//!
//! This module defines all error types for channel operations, signal delivery,
//! and exception handling within the IPC subsystem.
//!
//! ## References
//!
//! - Engineering Plan § 5.2.6 (Error Handling)

use alloc::string::String;
use thiserror::Error;

/// IPC-specific error types.
///
/// Represents errors that can occur during semantic channel operations,
/// signal delivery, and exception handling.
///
/// See Engineering Plan § 5.2.6 (Error Handling)
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum IpcError {
    /// Invalid or nonexistent channel.
    ///
    /// The channel ID references a channel that does not exist or is invalid.
    #[error("Invalid channel")]
    InvalidChannel,

    /// Channel validation failed.
    ///
    /// The channel configuration violates an invariant. This can occur when:
    /// - exactly_once_local is used with distributed channels
    /// - distributed is Some(_) but capability_verification is false
    /// - distributed is Some(_) and delivery is exactly_once_local but idempotency_keys is false
    #[error("Channel validation failed: {0}")]
    ChannelValidationFailed(String),

    /// Message delivery failed.
    ///
    /// The message could not be delivered due to a transient or permanent error.
    /// This can occur due to network failures, timeout, or receiver unavailability.
    #[error("Delivery failed: {0}")]
    DeliveryFailed(String),

    /// Backpressure triggered (buffer full).
    ///
    /// The channel buffer is full and the backpressure policy is Drop.
    /// The message was discarded.
    #[error("Backpressure triggered: buffer full")]
    BackpressureTriggered,

    /// Distributed communication not supported for this channel configuration.
    ///
    /// The channel configuration does not support distributed communication,
    /// but a distributed operation was attempted.
    #[error("Distributed communication not supported")]
    DistributedNotSupported,

    /// Invalid protocol specification.
    ///
    /// The protocol specification is invalid or unknown.
    #[error("Invalid protocol: {0}")]
    InvalidProtocol(String),

    /// Capability verification failed.
    ///
    /// The sender does not have the capability to send to this receiver,
    /// particularly important for distributed channels.
    #[error("Capability verification failed")]
    CapabilityVerificationFailed,

    /// Endpoint not found or disconnected.
    ///
    /// The endpoint (sender or receiver) is not connected or no longer exists.
    #[error("Endpoint not found")]
    EndpointNotFound,

    /// Channel capacity exceeded.
    ///
    /// The message is too large for the channel's configured capacity.
    #[error("Channel capacity exceeded: message too large")]
    CapacityExceeded,

    /// Signal delivery failed.
    ///
    /// The signal could not be delivered to the target handler.
    #[error("Signal delivery failed: {0}")]
    SignalDeliveryFailed(String),

    /// Context sharing operation failed.
    ///
    /// The context sharing mode requested is not compatible with the channel
    /// configuration or the operation failed during context synchronization.
    #[error("Context sharing failed: {0}")]
    ContextSharingFailed(String),

    /// Encryption/decryption error.
    ///
    /// The message could not be encrypted or decrypted.
    #[error("Encryption error: {0}")]
    EncryptionError(String),

    /// Timeout during channel operation.
    ///
    /// A blocking operation (e.g., Suspend backpressure) timed out.
    #[error("Channel operation timeout")]
    Timeout,

    /// Generic IPC error.
    #[error("IPC error: {0}")]
    Other(String),
}

/// Cognitive Substrate error type - top-level error for the system.
///
/// This is the primary error type used throughout the Cognitive Substrate OS.
/// It includes IPC-specific errors as a variant.
#[derive(Clone, Debug, Error)]
pub enum CsError {
    /// IPC subsystem error
    #[error("IPC error: {0}")]
    Ipc(IpcError),

    /// Generic error with message
    #[error("{0}")]
    Other(String),
}

/// Result type alias for IPC operations.
///
/// Used throughout the IPC subsystem for consistent error handling.
pub type Result<T> = core::result::Result<T, CsError>;

impl From<IpcError> for CsError {
    fn from(err: IpcError) -> Self {
        CsError::Ipc(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_ipc_error_invalid_channel() {
        let err = IpcError::InvalidChannel;
        assert_eq!(err, IpcError::InvalidChannel);
    }

    #[test]
    fn test_ipc_error_display() {
        let err = IpcError::InvalidChannel;
        let msg = err.to_string();
        assert_eq!(msg, "Invalid channel");
    }

    #[test]
    fn test_ipc_error_channel_validation() {
        let err = IpcError::ChannelValidationFailed(
            alloc::string::String::from("exactly_once_local with distributed"),
        );
        assert!(err.to_string().contains("exactly_once_local"));
    }

    #[test]
    fn test_ipc_error_backpressure() {
        let err = IpcError::BackpressureTriggered;
        assert_eq!(err, IpcError::BackpressureTriggered);
    }

    #[test]
    fn test_cs_error_from_ipc_error() {
        let ipc_err = IpcError::InvalidChannel;
        let cs_err: CsError = ipc_err.into();
        match cs_err {
            CsError::Ipc(IpcError::InvalidChannel) => (),
            _ => panic!("Expected IpcError::InvalidChannel"),
        }
    }

    #[test]
    fn test_result_type() {
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());

        let err_result: Result<i32> = Err(CsError::Ipc(IpcError::InvalidChannel));
        assert!(err_result.is_err());
    }

    #[test]
    fn test_ipc_error_clone() {
        let err1 = IpcError::InvalidChannel;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_cs_error_other() {
        let err = CsError::Other(alloc::string::String::from("custom error"));
        let msg = err.to_string();
        assert_eq!(msg, "custom error");
    }
}

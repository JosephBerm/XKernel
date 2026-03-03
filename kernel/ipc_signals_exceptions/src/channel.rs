// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Semantic Channel Type System
//!
//! This module defines the SemanticChannel struct and related types for configurable,
//! type-safe inter-process communication. Semantic channels support configurable delivery
//! guarantees, backpressure handling, context sharing, and distributed communication.
//!
//! ## Validation Invariants
//!
//! Several invariants are enforced during channel validation:
//! - If `distributed = Some(_)`, then `delivery != ExactlyOnceLocal` OR `idempotency_keys = true`
//! - If `distributed = Some(_)`, then `capability_verification = true`
//! - Backpressure policy and delivery guarantee must be compatible
//!
//! ## References
//!
//! - Engineering Plan § 5.2 (SemanticChannel IPC Type System)

use crate::backpressure::BackpressurePolicy;
use crate::context::ContextMode;
use crate::delivery::DeliveryGuarantee;
use crate::distributed::DistributedConfig;
use crate::error::{CsError, IpcError, Result};
use crate::ids::{ChannelID, EndpointID};
use crate::protocol::ProtocolSpec;
use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Channel capacity specification.
///
/// Defines the maximum number of messages (or bytes) that can be buffered
/// in the channel before backpressure is triggered.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelCapacity {
    /// Maximum number of messages in the buffer.
    pub max_messages: u32,

    /// Maximum total size in bytes (0 = unlimited).
    pub max_bytes: u64,
}

impl ChannelCapacity {
    /// Create a new channel capacity specification.
    pub fn new(max_messages: u32, max_bytes: u64) -> Self {
        Self {
            max_messages,
            max_bytes,
        }
    }

    /// Create an unbounded capacity channel.
    pub fn unbounded() -> Self {
        Self {
            max_messages: u32::MAX,
            max_bytes: 0,
        }
    }

    /// Check if the capacity allows a message of the given size.
    pub fn can_fit(&self, message_size: u64) -> bool {
        if self.max_bytes == 0 {
            return true;
        }
        message_size <= self.max_bytes
    }
}

/// Endpoint pair representing a channel's sender and receiver.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EndpointPair {
    /// Sender endpoint ID.
    pub sender: EndpointID,

    /// Receiver endpoint ID.
    pub receiver: EndpointID,
}

impl EndpointPair {
    /// Create a new endpoint pair.
    pub fn new(sender: EndpointID, receiver: EndpointID) -> Self {
        Self { sender, receiver }
    }
}

/// Semantic channel for inter-process communication.
///
/// A semantic channel provides type-safe, configurable communication between
/// two endpoints (sender and receiver). It supports multiple protocols, delivery
/// guarantees, backpressure policies, context sharing modes, and optional
/// distributed communication.
///
/// See Engineering Plan § 5.2 (SemanticChannel IPC Type System)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticChannel {
    /// Unique channel identifier.
    pub id: ChannelID,

    /// Communication protocol specification.
    pub protocol: ProtocolSpec,

    /// Sender and receiver endpoint pair.
    pub endpoints: EndpointPair,

    /// Message delivery guarantee.
    pub delivery: DeliveryGuarantee,

    /// Backpressure handling policy.
    pub backpressure: BackpressurePolicy,

    /// Context sharing mode.
    pub context_sharing: ContextMode,

    /// Distributed communication configuration (None for local channels).
    pub distributed: Option<DistributedConfig>,

    /// Channel buffer capacity.
    pub capacity: ChannelCapacity,
}

impl SemanticChannel {
    /// Create a new semantic channel with default settings.
    ///
    /// Default configuration:
    /// - Local (no distributed config)
    /// - AtMostOnce delivery
    /// - Drop backpressure
    /// - No context sharing
    /// - Unbounded capacity
    pub fn new(
        id: ChannelID,
        protocol: ProtocolSpec,
        sender: EndpointID,
        receiver: EndpointID,
    ) -> Self {
        Self {
            id,
            protocol,
            endpoints: EndpointPair::new(sender, receiver),
            delivery: DeliveryGuarantee::AtMostOnce,
            backpressure: BackpressurePolicy::Drop,
            context_sharing: ContextMode::None,
            distributed: None,
            capacity: ChannelCapacity::unbounded(),
        }
    }

    /// Set the delivery guarantee.
    pub fn with_delivery(mut self, delivery: DeliveryGuarantee) -> Self {
        self.delivery = delivery;
        self
    }

    /// Set the backpressure policy.
    pub fn with_backpressure(mut self, backpressure: BackpressurePolicy) -> Self {
        self.backpressure = backpressure;
        self
    }

    /// Set the context sharing mode.
    pub fn with_context_sharing(mut self, context_sharing: ContextMode) -> Self {
        self.context_sharing = context_sharing;
        self
    }

    /// Enable distributed communication with the given configuration.
    pub fn with_distributed(mut self, config: DistributedConfig) -> Self {
        self.distributed = Some(config);
        self
    }

    /// Set the channel capacity.
    pub fn with_capacity(mut self, capacity: ChannelCapacity) -> Self {
        self.capacity = capacity;
        self
    }

    /// Validate the channel configuration.
    ///
    /// Enforces all channel invariants:
    /// 1. ExactlyOnceLocal cannot be used with distributed channels
    ///    (unless idempotency_keys are enabled)
    /// 2. Distributed channels MUST have capability_verification = true
    /// 3. Distributed + ExactlyOnceLocal requires idempotency_keys = true
    /// 4. Backpressure and delivery guarantee are compatible
    ///
    /// Returns an error if any invariant is violated.
    ///
    /// See Engineering Plan § 5.2 (Validation Rules)
    pub fn validate(&self) -> Result<()> {
        // Check distributed channel invariants
        if let Some(ref dist_cfg) = self.distributed {
            // Invariant 1: ExactlyOnceLocal with distributed requires idempotency_keys
            if self.delivery == DeliveryGuarantee::ExactlyOnceLocal {
                if !dist_cfg.idempotency_keys {
                    return Err(CsError::Ipc(IpcError::ChannelValidationFailed(
                        String::from(
                            "ExactlyOnceLocal delivery requires idempotency_keys \
                             when distributed",
                        ),
                    )));
                }
            }

            // Invariant 2: capability_verification must be true for distributed
            if !dist_cfg.capability_verification {
                return Err(CsError::Ipc(IpcError::ChannelValidationFailed(
                    String::from(
                        "Distributed channels MUST have capability_verification enabled",
                    ),
                )));
            }
        }

        // Check backpressure and delivery compatibility
        match (self.backpressure, self.delivery) {
            // Drop backpressure is compatible with all delivery guarantees
            (BackpressurePolicy::Drop, _) => (),

            // Suspend and SignalWarn require non-lossy delivery
            (BackpressurePolicy::Suspend, DeliveryGuarantee::AtMostOnce) => {
                return Err(CsError::Ipc(IpcError::ChannelValidationFailed(
                    String::from("Suspend backpressure incompatible with AtMostOnce delivery"),
                )));
            }
            (BackpressurePolicy::SignalWarn, DeliveryGuarantee::AtMostOnce) => {
                return Err(CsError::Ipc(IpcError::ChannelValidationFailed(
                    String::from(
                        "SignalWarn backpressure incompatible with AtMostOnce delivery",
                    ),
                )));
            }

            // All other combinations are valid
            _ => (),
        }

        Ok(())
    }

    /// Check if this is a local channel (not distributed).
    pub fn is_local(&self) -> bool {
        self.distributed.is_none()
    }

    /// Check if this channel guarantees exactly-once semantics.
    pub fn guarantees_exactly_once(&self) -> bool {
        self.delivery == DeliveryGuarantee::ExactlyOnceLocal
    }

    /// Check if this channel supports bidirectional context sharing.
    pub fn supports_bidirectional_context(&self) -> bool {
        self.context_sharing.is_bidirectional()
    }

    /// Check if this channel may drop messages under backpressure.
    pub fn may_drop_under_backpressure(&self) -> bool {
        self.backpressure.may_drop_messages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed::{DistributedConfig, EncryptionConfig, NetworkAddress};

    #[test]
    fn test_channel_capacity_new() {
        let cap = ChannelCapacity::new(100, 10000);
        assert_eq!(cap.max_messages, 100);
        assert_eq!(cap.max_bytes, 10000);
    }

    #[test]
    fn test_channel_capacity_unbounded() {
        let cap = ChannelCapacity::unbounded();
        assert_eq!(cap.max_messages, u32::MAX);
        assert_eq!(cap.max_bytes, 0);
    }

    #[test]
    fn test_channel_capacity_can_fit() {
        let cap = ChannelCapacity::new(100, 1000);
        assert!(cap.can_fit(500));
        assert!(!cap.can_fit(2000));
    }

    #[test]
    fn test_channel_capacity_unbounded_can_fit() {
        let cap = ChannelCapacity::unbounded();
        assert!(cap.can_fit(999999));
    }

    #[test]
    fn test_endpoint_pair_new() {
        let sender = EndpointID::new();
        let receiver = EndpointID::new();
        let pair = EndpointPair::new(sender, receiver);
        assert_eq!(pair.sender, sender);
        assert_eq!(pair.receiver, receiver);
    }

    #[test]
    fn test_semantic_channel_creation() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan =
            SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver);

        assert_eq!(chan.id, chan_id);
        assert_eq!(chan.delivery, DeliveryGuarantee::AtMostOnce);
        assert_eq!(chan.backpressure, BackpressurePolicy::Drop);
        assert_eq!(chan.context_sharing, ContextMode::None);
        assert!(chan.distributed.is_none());
    }

    #[test]
    fn test_semantic_channel_with_delivery() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_delivery(DeliveryGuarantee::AtLeastOnce);

        assert_eq!(chan.delivery, DeliveryGuarantee::AtLeastOnce);
    }

    #[test]
    fn test_semantic_channel_with_backpressure() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_backpressure(BackpressurePolicy::Suspend);

        assert_eq!(chan.backpressure, BackpressurePolicy::Suspend);
    }

    #[test]
    fn test_semantic_channel_validate_valid() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver);
        assert!(chan.validate().is_ok());
    }

    #[test]
    fn test_semantic_channel_validate_exactly_once_without_idempotency() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let addr = NetworkAddress::new(
            alloc::string::String::from("remote.example.com"),
            9000,
        );
        let dist_cfg = DistributedConfig::new(addr, EncryptionConfig::Aes256Gcm)
            .unwrap()
            .with_idempotency_keys(false);

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_delivery(DeliveryGuarantee::ExactlyOnceLocal)
            .with_distributed(dist_cfg);

        let result = chan.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_channel_validate_exactly_once_with_idempotency() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let addr = NetworkAddress::new(
            alloc::string::String::from("remote.example.com"),
            9000,
        );
        let dist_cfg = DistributedConfig::new(addr, EncryptionConfig::Aes256Gcm)
            .unwrap()
            .with_idempotency_keys(true);

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_delivery(DeliveryGuarantee::ExactlyOnceLocal)
            .with_distributed(dist_cfg);

        assert!(chan.validate().is_ok());
    }

    #[test]
    fn test_semantic_channel_validate_suspend_with_at_most_once() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_delivery(DeliveryGuarantee::AtMostOnce)
            .with_backpressure(BackpressurePolicy::Suspend);

        let result = chan.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_channel_validate_drop_with_any_delivery() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_delivery(DeliveryGuarantee::ExactlyOnceLocal)
            .with_backpressure(BackpressurePolicy::Drop);

        assert!(chan.validate().is_ok());
    }

    #[test]
    fn test_semantic_channel_is_local() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver);
        assert!(chan.is_local());
    }

    #[test]
    fn test_semantic_channel_guarantees_exactly_once() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_delivery(DeliveryGuarantee::ExactlyOnceLocal);

        assert!(chan.guarantees_exactly_once());
    }

    #[test]
    fn test_semantic_channel_supports_bidirectional_context() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_context_sharing(ContextMode::ReadWrite);

        assert!(chan.supports_bidirectional_context());
    }

    #[test]
    fn test_semantic_channel_may_drop_under_backpressure() {
        let chan_id = ChannelID::new();
        let sender = EndpointID::new();
        let receiver = EndpointID::new();

        let chan = SemanticChannel::new(chan_id, ProtocolSpec::ReAct, sender, receiver)
            .with_backpressure(BackpressurePolicy::Drop);

        assert!(chan.may_drop_under_backpressure());
    }
}

// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Protocol Specification Types
//!
//! This module defines the protocol types for semantic channels. Each protocol specifies
//! the message semantics and structure expectations for communication on a channel.
//!
//! ## References
//!
//! - Engineering Plan § 5.2 (SemanticChannel IPC Type System)

use serde::{Deserialize, Serialize};

/// Protocol specification for a semantic channel.
///
/// Defines the semantic contract and message structure for communication on a channel.
///
/// See Engineering Plan § 5.2 (SemanticChannel IPC Type System)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtocolSpec {
    /// ReAct protocol: Reasoning, Action, Observation loop.
    ///
    /// Used for multi-step agent reasoning and tool execution patterns.
    /// Enforces structured message ordering: Thought → Action → Observation cycles.
    ReAct,

    /// Structured data protocol: Schema-driven strongly-typed messaging.
    ///
    /// Messages must conform to a predefined schema. Type information enables
    /// validation, code generation, and optimization.
    StructuredData,

    /// Event stream protocol: Fire-and-forget event delivery.
    ///
    /// Unordered, best-effort event propagation. Suitable for telemetry,
    /// observability, and non-critical notifications.
    EventStream,

    /// Custom protocol: User-defined protocol specification.
    ///
    /// Allows application-specific protocol semantics. The string contains
    /// a protocol identifier or specification.
    Custom(alloc::string::String),
}

impl ProtocolSpec {
    /// Check if this protocol is fire-and-forget semantics.
    ///
    /// Fire-and-forget protocols (EventStream) do not guarantee ordering
    /// or delivery, suitable for non-critical messages.
    pub fn is_fire_and_forget(&self) -> bool {
        matches!(self, ProtocolSpec::EventStream)
    }

    /// Check if this protocol requires strict ordering.
    ///
    /// ReAct protocol enforces strict message ordering for the Thought-Action-Observation cycle.
    pub fn requires_strict_ordering(&self) -> bool {
        matches!(self, ProtocolSpec::ReAct)
    }

    /// Check if this protocol requires schema validation.
    ///
    /// StructuredData requires all messages conform to a predefined schema.
    pub fn requires_schema_validation(&self) -> bool {
        matches!(self, ProtocolSpec::StructuredData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;

    #[test]
    fn test_react_protocol_requires_ordering() {
        let proto = ProtocolSpec::ReAct;
        assert!(proto.requires_strict_ordering());
        assert!(!proto.is_fire_and_forget());
    }

    #[test]
    fn test_event_stream_is_fire_and_forget() {
        let proto = ProtocolSpec::EventStream;
        assert!(proto.is_fire_and_forget());
        assert!(!proto.requires_strict_ordering());
    }

    #[test]
    fn test_structured_data_requires_validation() {
        let proto = ProtocolSpec::StructuredData;
        assert!(proto.requires_schema_validation());
        assert!(!proto.is_fire_and_forget());
    }

    #[test]
    fn test_custom_protocol() {
        let proto = ProtocolSpec::Custom(alloc::string::String::from("my-proto"));
        assert!(!proto.is_fire_and_forget());
        assert!(!proto.requires_strict_ordering());
    }

    #[test]
    fn test_protocol_equality() {
        let proto1 = ProtocolSpec::ReAct;
        let proto2 = ProtocolSpec::ReAct;
        assert_eq!(proto1, proto2);

        let proto3 = ProtocolSpec::Custom(alloc::string::String::from("test"));
        let proto4 = ProtocolSpec::Custom(alloc::string::String::from("test"));
        assert_eq!(proto3, proto4);
    }
}

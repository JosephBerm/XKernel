// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Context Sharing Mode Types
//!
//! This module defines the context sharing modes available for semantic channels.
//! Context sharing allows channels to transparently carry execution context
//! (state, variables, reasoning state) between sender and receiver.
//!
//! ## References
//!
//! - Engineering Plan § 5.2.4 (Context Sharing)

use serde::{Deserialize, Serialize};

/// Context sharing mode for a semantic channel.
///
/// Defines how execution context is shared between sender and receiver endpoints.
/// Context sharing enables channels to carry state, variables, and other execution
/// information alongside messages.
///
/// See Engineering Plan § 5.2.4 (Context Sharing)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextMode {
    /// None: No context sharing.
    ///
    /// Messages are delivered without any additional context. Only the message
    /// payload is transmitted. This is the lowest-overhead mode, suitable for
    /// simple point-to-point communication where shared state is not needed.
    ///
    /// Characteristics:
    /// - No state transmission overhead
    /// - Receiver must have pre-existing context if needed
    /// - Suitable for stateless services
    /// - Lowest latency
    None,

    /// ReadOnly: Receiver can read sender's context, but cannot modify it.
    ///
    /// The receiver receives a snapshot of the sender's execution context (variables,
    /// reasoning state, etc.) in addition to the message. The receiver can read
    /// and use this context but cannot modify it. The sender's context is unaffected
    /// by the receiver's operations. This is useful for propagating information
    /// and state snapshots without full synchronization.
    ///
    /// Characteristics:
    /// - Receiver sees immutable sender context snapshot
    /// - No bidirectional state synchronization
    /// - Useful for telemetry, observation, and read-only distributed reasoning
    /// - Moderate overhead (snapshot transmission)
    ReadOnly,

    /// ReadWrite: Bidirectional context synchronization with CRDT conflict resolution.
    ///
    /// Full bidirectional context sharing with automatic conflict resolution using
    /// Conflict-free Replicated Data Types (CRDTs). The sender and receiver can both
    /// read and modify context, with conflicts automatically resolved according to
    /// CRDT semantics (e.g., last-write-wins, vector clock ordering).
    ///
    /// This is the highest-overhead mode, suitable for distributed reasoning and
    /// multi-party computation where strong eventual consistency is required.
    /// Context mutations are propagated bidirectionally.
    ///
    /// Characteristics:
    /// - Bidirectional context synchronization
    /// - Automatic CRDT-based conflict resolution
    /// - Strong eventual consistency semantics
    /// - Suitable for distributed multi-agent reasoning
    /// - Highest latency and resource overhead
    /// - Last-write-wins or vector-clock semantics for conflicts
    ReadWrite,
}

impl ContextMode {
    /// Check if this mode allows reading context.
    pub fn allows_read(&self) -> bool {
        !matches!(self, ContextMode::None)
    }

    /// Check if this mode allows writing context.
    pub fn allows_write(&self) -> bool {
        matches!(self, ContextMode::ReadWrite)
    }

    /// Check if this mode is bidirectional.
    pub fn is_bidirectional(&self) -> bool {
        matches!(self, ContextMode::ReadWrite)
    }

    /// Check if this mode requires CRDT conflict resolution.
    pub fn requires_crdt(&self) -> bool {
        matches!(self, ContextMode::ReadWrite)
    }

    /// Check if this mode is stateless (no context sharing).
    pub fn is_stateless(&self) -> bool {
        matches!(self, ContextMode::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_mode_properties() {
        let cm = ContextMode::None;
        assert!(!cm.allows_read());
        assert!(!cm.allows_write());
        assert!(!cm.is_bidirectional());
        assert!(!cm.requires_crdt());
        assert!(cm.is_stateless());
    }

    #[test]
    fn test_readonly_mode_properties() {
        let cm = ContextMode::ReadOnly;
        assert!(cm.allows_read());
        assert!(!cm.allows_write());
        assert!(!cm.is_bidirectional());
        assert!(!cm.requires_crdt());
        assert!(!cm.is_stateless());
    }

    #[test]
    fn test_readwrite_mode_properties() {
        let cm = ContextMode::ReadWrite;
        assert!(cm.allows_read());
        assert!(cm.allows_write());
        assert!(cm.is_bidirectional());
        assert!(cm.requires_crdt());
        assert!(!cm.is_stateless());
    }

    #[test]
    fn test_context_mode_equality() {
        let cm1 = ContextMode::ReadOnly;
        let cm2 = ContextMode::ReadOnly;
        assert_eq!(cm1, cm2);

        let cm3 = ContextMode::ReadWrite;
        assert_ne!(cm1, cm3);
    }

    #[test]
    fn test_context_mode_copy() {
        let cm = ContextMode::ReadWrite;
        let _cm_copy = cm;
        // Original still usable due to Copy trait
        assert!(cm.requires_crdt());
    }
}

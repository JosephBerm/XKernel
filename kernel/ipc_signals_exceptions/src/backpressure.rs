// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Backpressure Policy Types
//!
//! This module defines the backpressure handling strategies available for semantic channels.
//! Backpressure occurs when the receiver cannot keep up with the sender, causing the
//! channel buffer to fill. Different policies handle this situation in different ways.
//!
//! ## References
//!
//! - Engineering Plan § 5.2.3 (Backpressure Handling)

use serde::{Deserialize, Serialize};

/// Backpressure policy for a semantic channel.
///
/// Defines how the system responds when a channel's buffer reaches capacity
/// and the sender continues to produce messages faster than the receiver consumes them.
///
/// See Engineering Plan § 5.2.3 (Backpressure Handling)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackpressurePolicy {
    /// Drop: Discard newly arriving messages when buffer is full.
    ///
    /// When the channel buffer reaches capacity, incoming messages are silently dropped.
    /// The sender is not blocked and receives no indication of the drop.
    /// This is suitable for non-critical monitoring or telemetry where some data loss
    /// is acceptable in exchange for guaranteed non-blocking behavior.
    ///
    /// Characteristics:
    /// - Sender is never blocked
    /// - Oldest or newest messages may be dropped
    /// - Silent failure (no error signaling)
    /// - Suitable for observability, non-critical notifications
    /// - May cause data loss under high load
    Drop,

    /// Suspend: Block the sender until space becomes available in the buffer.
    ///
    /// When the channel buffer reaches capacity, the sender's next send() call
    /// will block until the receiver has consumed messages and freed space.
    /// This prevents message loss at the cost of potentially blocking the sender
    /// and requiring careful deadlock analysis.
    ///
    /// Characteristics:
    /// - Sender is blocked until buffer has space
    /// - No message loss
    /// - May increase latency and cause priority inversion
    /// - Requires careful deadlock prevention
    /// - Suitable for critical messages where loss is unacceptable
    Suspend,

    /// SignalWarn: Emit a SigContextPressure signal to the sender when buffer fills.
    ///
    /// When the channel buffer reaches capacity, the system sends a SigContextPressure
    /// signal to the sender instead of blocking. The sender can respond by slowing down,
    /// adjusting its cognitive workload, or taking other mitigation actions.
    /// This combines the benefits of non-blocking semantics with explicit awareness
    /// and adaptive behavior.
    ///
    /// Characteristics:
    /// - Sender receives explicit SigContextPressure signal
    /// - Sender can implement adaptive response
    /// - Non-blocking (sender continues if signal ignored)
    /// - Allows for graceful degradation under load
    /// - Suitable for critical interactive workloads
    SignalWarn,
}

impl BackpressurePolicy {
    /// Check if this policy may drop messages.
    pub fn may_drop_messages(&self) -> bool {
        matches!(self, BackpressurePolicy::Drop)
    }

    /// Check if this policy blocks the sender.
    pub fn blocks_sender(&self) -> bool {
        matches!(self, BackpressurePolicy::Suspend)
    }

    /// Check if this policy emits signals to the sender.
    pub fn emits_signals(&self) -> bool {
        matches!(self, BackpressurePolicy::SignalWarn)
    }

    /// Check if this policy guarantees message delivery.
    pub fn guarantees_delivery(&self) -> bool {
        !self.may_drop_messages()
    }

    /// Check if this policy is non-blocking.
    pub fn is_non_blocking(&self) -> bool {
        !self.blocks_sender()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drop_policy_properties() {
        let bp = BackpressurePolicy::Drop;
        assert!(bp.may_drop_messages());
        assert!(!bp.blocks_sender());
        assert!(!bp.emits_signals());
        assert!(!bp.guarantees_delivery());
        assert!(bp.is_non_blocking());
    }

    #[test]
    fn test_suspend_policy_properties() {
        let bp = BackpressurePolicy::Suspend;
        assert!(!bp.may_drop_messages());
        assert!(bp.blocks_sender());
        assert!(!bp.emits_signals());
        assert!(bp.guarantees_delivery());
        assert!(!bp.is_non_blocking());
    }

    #[test]
    fn test_signal_warn_policy_properties() {
        let bp = BackpressurePolicy::SignalWarn;
        assert!(!bp.may_drop_messages());
        assert!(!bp.blocks_sender());
        assert!(bp.emits_signals());
        assert!(bp.guarantees_delivery());
        assert!(bp.is_non_blocking());
    }

    #[test]
    fn test_backpressure_equality() {
        let bp1 = BackpressurePolicy::Suspend;
        let bp2 = BackpressurePolicy::Suspend;
        assert_eq!(bp1, bp2);

        let bp3 = BackpressurePolicy::Drop;
        assert_ne!(bp1, bp3);
    }

    #[test]
    fn test_backpressure_copy() {
        let bp = BackpressurePolicy::SignalWarn;
        let _bp_copy = bp;
        // Original still usable due to Copy trait
        assert!(bp.emits_signals());
    }
}

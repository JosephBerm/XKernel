// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Delivery Guarantee Types
//!
//! This module defines the message delivery guarantees available for semantic channels.
//! The choice of delivery guarantee affects latency, memory consumption, and system
//! behavior under congestion or failure conditions.
//!
//! ## References
//!
//! - Engineering Plan § 5.2.2 (Delivery Guarantees)

use serde::{Deserialize, Serialize};

/// Message delivery guarantee specification for a semantic channel.
///
/// Defines the delivery semantics and reliability guarantees for messages
/// transmitted on a channel.
///
/// See Engineering Plan § 5.2.2 (Delivery Guarantees)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeliveryGuarantee {
    /// At-Most-Once delivery: No duplicates, but messages may be lost.
    ///
    /// Messages are delivered at most one time. If the sender crashes or
    /// the network fails, messages may be lost without recovery. This is
    /// the lowest latency guarantee, suitable for non-critical messages
    /// where loss is acceptable (e.g., telemetry, monitoring).
    ///
    /// Characteristics:
    /// - Lowest latency and resource overhead
    /// - No duplicate suppression required
    /// - Messages may be silently lost
    /// - Suitable for fire-and-forget or best-effort semantics
    AtMostOnce,

    /// At-Least-Once delivery: Duplicates possible, but no message loss.
    ///
    /// Messages are guaranteed to be delivered at least once. The receiver
    /// may see the same message multiple times if the sender crashes or
    /// retransmits. Receivers should implement deduplication if exactly-once
    /// semantics are required. This offers a balance between reliability
    /// and performance.
    ///
    /// Characteristics:
    /// - Moderate latency and resource overhead
    /// - Sender retransmits on timeout/failure
    /// - Receiver must handle potential duplicates
    /// - Suitable when message loss is unacceptable but duplication is tolerable
    AtLeastOnce,

    /// Exactly-Once-Local delivery: No duplicates, no loss (within local system).
    ///
    /// Messages are delivered exactly once within a single cognitive substrate
    /// system. This requires persistent logging and deduplication, increasing
    /// latency and memory consumption. Cannot be used with distributed channels,
    /// as cross-system exactly-once semantics require additional infrastructure
    /// (idempotency keys, distributed consensus).
    ///
    /// Characteristics:
    /// - Highest latency and resource overhead (local only)
    /// - Sender maintains persistent log
    /// - Receiver deduplicates via message ID
    /// - Incompatible with distributed channels (see validation rules)
    /// - Suitable when exactly-once semantics are critical (financial transfers, state mutations)
    ExactlyOnceLocal,
}

impl DeliveryGuarantee {
    /// Check if this guarantee allows message loss.
    pub fn allows_loss(&self) -> bool {
        matches!(self, DeliveryGuarantee::AtMostOnce)
    }

    /// Check if this guarantee allows duplicate delivery.
    pub fn allows_duplicates(&self) -> bool {
        matches!(self, DeliveryGuarantee::AtLeastOnce)
    }

    /// Check if this guarantee is local-only (cannot be used with distributed).
    pub fn is_local_only(&self) -> bool {
        matches!(self, DeliveryGuarantee::ExactlyOnceLocal)
    }

    /// Check if deduplication is required for this guarantee.
    pub fn requires_deduplication(&self) -> bool {
        matches!(
            self,
            DeliveryGuarantee::AtLeastOnce | DeliveryGuarantee::ExactlyOnceLocal
        )
    }

    /// Check if sender-side retransmission is required.
    pub fn requires_retransmission(&self) -> bool {
        !matches!(self, DeliveryGuarantee::AtMostOnce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_most_once_properties() {
        let dg = DeliveryGuarantee::AtMostOnce;
        assert!(dg.allows_loss());
        assert!(!dg.allows_duplicates());
        assert!(!dg.is_local_only());
        assert!(!dg.requires_deduplication());
        assert!(!dg.requires_retransmission());
    }

    #[test]
    fn test_at_least_once_properties() {
        let dg = DeliveryGuarantee::AtLeastOnce;
        assert!(!dg.allows_loss());
        assert!(dg.allows_duplicates());
        assert!(!dg.is_local_only());
        assert!(dg.requires_deduplication());
        assert!(dg.requires_retransmission());
    }

    #[test]
    fn test_exactly_once_local_properties() {
        let dg = DeliveryGuarantee::ExactlyOnceLocal;
        assert!(!dg.allows_loss());
        assert!(!dg.allows_duplicates());
        assert!(dg.is_local_only());
        assert!(dg.requires_deduplication());
        assert!(dg.requires_retransmission());
    }

    #[test]
    fn test_delivery_guarantee_equality() {
        let dg1 = DeliveryGuarantee::AtLeastOnce;
        let dg2 = DeliveryGuarantee::AtLeastOnce;
        assert_eq!(dg1, dg2);

        let dg3 = DeliveryGuarantee::ExactlyOnceLocal;
        assert_ne!(dg1, dg3);
    }

    #[test]
    fn test_delivery_guarantee_copy() {
        let dg = DeliveryGuarantee::AtMostOnce;
        let _dg_copy = dg;
        // Original still usable due to Copy trait
        assert!(dg.allows_loss());
    }
}

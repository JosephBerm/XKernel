// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory system error types for Semantic Memory operations.
//!
//! This module defines all error conditions that can occur across the 3-tier
//! memory hierarchy. Errors are designed for deterministic handling without panics.
//!
//! See Engineering Plan § 4.1.0 (Error Handling) and § 4.1.1 (Operations).

use alloc::string::String;
use core::fmt;

/// Result type alias for semantic memory operations.
///
/// All public memory operations return `Result<T>` using this type.
/// See Engineering Plan § 4.1.0: Error Handling & Recovery.
pub type Result<T> = core::result::Result<T, MemoryError>;

/// Errors that may occur during semantic memory operations.
///
/// This enum represents all failure modes across L1, L2, and L3 tiers.
/// Per Engineering Plan § 4.1.0, all errors must be recoverable without panics.
///
/// # Variants
///
/// The error variants are organized by operation category and tier:
/// - Allocation and capacity errors
/// - Eviction and tier migration failures
/// - Capability and isolation violations
/// - Data consistency and replication errors
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryError {
    /// Allocation failed due to insufficient capacity in target tier.
    ///
    /// Occurs when requesting memory allocation and the tier cannot satisfy the request.
    /// See Engineering Plan § 4.1.1: Allocation Operations.
    #[error("allocation failed: requested {requested} bytes, available {available}")]
    AllocationFailed {
        /// Bytes requested
        requested: u64,
        /// Bytes available in tier
        available: u64,
    },

    /// Memory region is full and cannot accept more data.
    ///
    /// Occurs when a region has reached capacity and eviction cannot free space.
    /// See Engineering Plan § 4.1.1: Capacity Management.
    #[error("region full: {region_id} at {used}/{capacity} bytes")]
    RegionFull {
        /// ID of the full region
        region_id: String,
        /// Current bytes used
        used: u64,
        /// Total capacity
        capacity: u64,
    },

    /// Eviction operation failed.
    ///
    /// Occurs when the eviction policy cannot make progress in freeing memory.
    /// This may indicate a memory pressure emergency.
    /// See Engineering Plan § 4.1.2: Eviction & Tier Migration.
    #[error("eviction failed: {reason}")]
    EvictionFailed {
        /// Explanation of why eviction could not proceed
        reason: String,
    },

    /// Operation attempted on invalid or mismatched tier.
    ///
    /// Occurs when attempting to access or migrate between incompatible tiers.
    /// See Engineering Plan § 4.1.2: Tier Migration.
    #[error("invalid tier for operation: {operation}, got {tier}")]
    InvalidTier {
        /// Operation attempted
        operation: String,
        /// Tier that was involved
        tier: String,
    },

    /// Capability check failed for the requested operation.
    ///
    /// Occurs when an agent lacks the capability to perform the memory operation.
    /// See Engineering Plan § 4.1.3: Isolation & Capabilities.
    #[error("capability denied for {operation} on {resource}")]
    CapabilityDenied {
        /// Operation being denied
        operation: String,
        /// Resource or region being protected
        resource: String,
    },

    /// Memory region mounting failed.
    ///
    /// Occurs when mounting knowledge sources or attaching external storage fails.
    /// See Engineering Plan § 4.1.4: L3 Long-Term Operations.
    #[error("mount failed: {reason}")]
    MountFailed {
        /// Details of the mount failure
        reason: String,
    },

    /// Replication operation failed.
    ///
    /// Occurs when replicating data across crew members or CRDT updates fail.
    /// See Engineering Plan § 4.1.4: Replication & CRDT.
    #[error("replication failed: {reason}")]
    ReplicationFailed {
        /// Details of the replication failure
        reason: String,
    },

    /// Compaction or maintenance operation failed.
    ///
    /// Occurs when garbage collection, index maintenance, or tier compaction fails.
    /// See Engineering Plan § 4.1.1 and § 4.1.4: Maintenance Operations.
    #[error("compaction failed: {reason}")]
    CompactionFailed {
        /// Details of the compaction failure
        reason: String,
    },

    /// Invalid reference or identifier.
    ///
    /// Occurs when a memory reference (L1Ref, L2Ref, L3Ref) is invalid or stale.
    #[error("invalid reference: {reason}")]
    InvalidReference {
        /// Why the reference is invalid
        reason: String,
    },

    /// Data consistency violation detected.
    ///
    /// Occurs when CRDT or consistency checks detect conflicts or anomalies.
    /// See Engineering Plan § 4.1.4: CRDT Consistency.
    #[error("consistency violation: {reason}")]
    ConsistencyViolation {
        /// Details of the consistency issue
        reason: String,
    },

    /// Generic memory operation error.
    #[error("memory operation failed: {0}")]
    Other(String),
}

impl MemoryError {
    /// Returns true if this error is due to capacity exhaustion.
    pub fn is_capacity_error(&self) -> bool {
        matches!(
            self,
            MemoryError::AllocationFailed { .. }
                | MemoryError::RegionFull { .. }
                | MemoryError::EvictionFailed { .. }
        )
    }

    /// Returns true if this error is due to capability violation.
    pub fn is_capability_error(&self) -> bool {
        matches!(self, MemoryError::CapabilityDenied { .. })
    }

    /// Returns true if this error is due to isolation or tier mismatch.
    pub fn is_tier_error(&self) -> bool {
        matches!(self, MemoryError::InvalidTier { .. })
    }

    /// Returns true if this error is due to data consistency issues.
    pub fn is_consistency_error(&self) -> bool {
        matches!(self, MemoryError::ConsistencyViolation { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_allocation_failed_display() {
        let err = MemoryError::AllocationFailed {
            requested: 1024,
            available: 512,
        };
        let msg = err.to_string();
        assert!(msg.contains("allocation failed"));
        assert!(msg.contains("1024"));
        assert!(msg.contains("512"));
    }

    #[test]
    fn test_region_full_display() {
        let err = MemoryError::RegionFull {
            region_id: "l1-gpu-0".to_string(),
            used: 8192,
            capacity: 8192,
        };
        let msg = err.to_string();
        assert!(msg.contains("region full"));
        assert!(msg.contains("l1-gpu-0"));
    }

    #[test]
    fn test_eviction_failed_display() {
        let err = MemoryError::EvictionFailed {
            reason: "all entries pinned".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("eviction failed"));
        assert!(msg.contains("pinned"));
    }

    #[test]
    fn test_capability_denied_display() {
        let err = MemoryError::CapabilityDenied {
            operation: "write".to_string(),
            resource: "l3-shared".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("capability denied"));
        assert!(msg.contains("write"));
    }

    #[test]
    fn test_mount_failed_display() {
        let err = MemoryError::MountFailed {
            reason: "device not found".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("mount failed"));
    }

    #[test]
    fn test_replication_failed_display() {
        let err = MemoryError::ReplicationFailed {
            reason: "crew member unreachable".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("replication failed"));
    }

    #[test]
    fn test_compaction_failed_display() {
        let err = MemoryError::CompactionFailed {
            reason: "index corruption detected".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("compaction failed"));
    }

    #[test]
    fn test_invalid_reference_display() {
        let err = MemoryError::InvalidReference {
            reason: "reference expired".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid reference"));
    }

    #[test]
    fn test_consistency_violation_display() {
        let err = MemoryError::ConsistencyViolation {
            reason: "CRDT conflict unresolvable".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("consistency violation"));
    }

    #[test]
    fn test_is_capacity_error() {
        let err = MemoryError::AllocationFailed {
            requested: 1024,
            available: 512,
        };
        assert!(err.is_capacity_error());

        let err = MemoryError::CapabilityDenied {
            operation: "read".to_string(),
            resource: "l3".to_string(),
        };
        assert!(!err.is_capacity_error());
    }

    #[test]
    fn test_is_capability_error() {
        let err = MemoryError::CapabilityDenied {
            operation: "write".to_string(),
            resource: "l3-shared".to_string(),
        };
        assert!(err.is_capability_error());

        let err = MemoryError::AllocationFailed {
            requested: 1024,
            available: 512,
        };
        assert!(!err.is_capability_error());
    }

    #[test]
    fn test_is_tier_error() {
        let err = MemoryError::InvalidTier {
            operation: "prefetch".to_string(),
            tier: "L3".to_string(),
        };
        assert!(err.is_tier_error());
    }

    #[test]
    fn test_is_consistency_error() {
        let err = MemoryError::ConsistencyViolation {
            reason: "test".to_string(),
        };
        assert!(err.is_consistency_error());
    }

    #[test]
    fn test_error_equality() {
        let err1 = MemoryError::AllocationFailed {
            requested: 1024,
            available: 512,
        };
        let err2 = MemoryError::AllocationFailed {
            requested: 1024,
            available: 512,
        };
        assert_eq!(err1, err2);
    }
}

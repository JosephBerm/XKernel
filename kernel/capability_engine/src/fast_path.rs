// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Fast-path capability check: O(1) hash lookup achieving <50ns latency.
//!
//! This module implements the hot-path capability check that handles ~99% of requests.
//! Input: agent_id, capid, operation
//! Output: success/error immediately with <10 instructions
//! See Engineering Plan § 3.2.0 and Week 6 § 2.

#![forbid(unsafe_code)]

use core::fmt::{self, Debug};

use crate::capability::Capability;
use crate::capability_hash_table::CapabilityHashTable;
use crate::error::CapError;
use crate::ids::{AgentID, CapID};
use crate::operations::OperationSet;

/// Result of a fast-path capability check.
/// See Week 6 § 2: Fast-Path Capability Check.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FastPathResult {
    /// Capability found, operations validated, access granted.
    Granted {
        /// The capability that was checked
        capability: Capability,
    },

    /// Capability not found in table (fall through to slow path).
    NotFound,

    /// Capability exists but operation is not permitted.
    Denied {
        /// Reason for denial
        reason: FastPathDenyReason,
    },

    /// Error during lookup (e.g., concurrent write detected).
    Error(CapError),
}

impl FastPathResult {
    /// Returns true if access was granted.
    pub fn is_granted(&self) -> bool {
        matches!(self, FastPathResult::Granted { .. })
    }

    /// Returns true if capability was not found.
    pub fn is_not_found(&self) -> bool {
        matches!(self, FastPathResult::NotFound)
    }

    /// Returns true if access was denied.
    pub fn is_denied(&self) -> bool {
        matches!(self, FastPathResult::Denied { .. })
    }

    /// Returns true if an error occurred.
    pub fn is_error(&self) -> bool {
        matches!(self, FastPathResult::Error(_))
    }
}

impl fmt::Display for FastPathResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FastPathResult::Granted { .. } => write!(f, "FastPathResult::Granted"),
            FastPathResult::NotFound => write!(f, "FastPathResult::NotFound"),
            FastPathResult::Denied { reason } => write!(f, "FastPathResult::Denied({})", reason),
            FastPathResult::Error(e) => write!(f, "FastPathResult::Error({})", e),
        }
    }
}

/// Reason for fast-path capability denial.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FastPathDenyReason {
    /// Agent does not match capability's target agent.
    AgentMismatch,

    /// Operation is not permitted by this capability.
    InsufficientOperations,

    /// Capability has expired (basic time check).
    Expired,
}

impl fmt::Display for FastPathDenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FastPathDenyReason::AgentMismatch => write!(f, "agent mismatch"),
            FastPathDenyReason::InsufficientOperations => write!(f, "insufficient operations"),
            FastPathDenyReason::Expired => write!(f, "capability expired"),
        }
    }
}

/// Fast-path capability check context.
/// Input to the fast-path function.
/// See Week 6 § 2: Fast-Path Capability Check.
#[derive(Clone, Debug)]
pub struct FastPathCheckContext {
    /// Agent performing the operation
    pub agent_id: AgentID,

    /// Capability ID to check
    pub cap_id: CapID,

    /// Operation being performed (READ, WRITE, EXECUTE, INVOKE, SUBSCRIBE)
    pub operation: u8,

    /// Current time for expiration checks (if available, None = skip expiry check)
    pub current_time_ns: Option<u64>,
}

impl FastPathCheckContext {
    /// Creates a new fast-path check context.
    pub fn new(agent_id: AgentID, cap_id: CapID, operation: u8) -> Self {
        FastPathCheckContext {
            agent_id,
            cap_id,
            operation,
            current_time_ns: None,
        }
    }

    /// Creates a context with time check enabled.
    pub fn with_time(agent_id: AgentID, cap_id: CapID, operation: u8, time_ns: u64) -> Self {
        FastPathCheckContext {
            agent_id,
            cap_id,
            operation,
            current_time_ns: Some(time_ns),
        }
    }
}

/// Fast-path capability check: <50ns latency.
///
/// # Algorithm
///
/// 1. Hash lookup: CapID → Capability (O(1), ~10ns)
/// 2. Agent match check: O(1), ~5ns
/// 3. Operation check: bitwise AND (O(1), ~2ns)
/// 4. Expiry check (optional): comparison (O(1), ~3ns)
/// 5. Return result: ~5ns
///
/// Total: <50ns on modern CPUs
///
/// # Design
///
/// - No allocations (Result<> uses stack only)
/// - No system calls
/// - Lock-free reads via seqlock (caller must retry on contention)
/// - Minimal branching (processor-friendly)
///
/// See Week 6 § 2.
pub fn fast_path_check(
    hash_table: &CapabilityHashTable,
    context: &FastPathCheckContext,
) -> FastPathResult {
    // Step 1: Hash table lookup (O(1) with seqlock)
    let cap_result = match hash_table.lookup(&context.cap_id) {
        Ok(opt_cap) => opt_cap,
        Err(e) => return FastPathResult::Error(e),
    };

    let cap = match cap_result {
        Some(c) => c,
        None => return FastPathResult::NotFound,
    };

    // Step 2: Agent match check (early exit if mismatch)
    if cap.target_agent != context.agent_id {
        return FastPathResult::Denied {
            reason: FastPathDenyReason::AgentMismatch,
        };
    }

    // Step 3: Operation check (bitwise AND)
    if !cap.operations.contains(context.operation) {
        return FastPathResult::Denied {
            reason: FastPathDenyReason::InsufficientOperations,
        };
    }

    // Step 4: Expiry check (optional, if time is provided)
    if let Some(current_time) = context.current_time_ns {
        if let Some(expires_at) = cap.expires_at {
            if current_time >= expires_at.nanos() {
                return FastPathResult::Denied {
                    reason: FastPathDenyReason::Expired,
                };
            }
        }
    }

    // Step 5: All checks passed, grant access
    FastPathResult::Granted { capability: cap }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::Timestamp;
    use crate::ids::ResourceID;
    use crate::ids::ResourceType;
use alloc::string::ToString;

    #[test]
    fn test_fast_path_granted() {
        let mut table = CapabilityHashTable::new(256).expect("table creation");
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        let cap_id = cap.id.clone();

        table.insert(cap).expect("insert");

        let context = FastPathCheckContext::new(
            AgentID::new("agent-a"),
            cap_id,
            OperationSet::READ,
        );

        let result = fast_path_check(&table, &context);
        assert!(result.is_granted());
    }

    #[test]
    fn test_fast_path_agent_mismatch() {
        let mut table = CapabilityHashTable::new(256).expect("table creation");
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        let cap_id = cap.id.clone();

        table.insert(cap).expect("insert");

        let context = FastPathCheckContext::new(
            AgentID::new("agent-b"), // Different agent
            cap_id,
            OperationSet::READ,
        );

        let result = fast_path_check(&table, &context);
        assert!(result.is_denied());
        match result {
            FastPathResult::Denied {
                reason: FastPathDenyReason::AgentMismatch,
            } => {},
            _ => panic!("Expected agent mismatch"),
        }
    }

    #[test]
    fn test_fast_path_insufficient_operations() {
        let mut table = CapabilityHashTable::new(256).expect("table creation");
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        let cap_id = cap.id.clone();

        table.insert(cap).expect("insert");

        let context = FastPathCheckContext::new(
            AgentID::new("agent-a"),
            cap_id,
            OperationSet::WRITE, // Request WRITE but cap only has READ
        );

        let result = fast_path_check(&table, &context);
        assert!(result.is_denied());
        match result {
            FastPathResult::Denied {
                reason: FastPathDenyReason::InsufficientOperations,
            } => {},
            _ => panic!("Expected insufficient operations"),
        }
    }

    #[test]
    fn test_fast_path_not_found() {
        let table = CapabilityHashTable::new(256).expect("table creation");
        let cap_id = CapID::from_bytes([99u8; 32]);

        let context = FastPathCheckContext::new(
            AgentID::new("agent-a"),
            cap_id,
            OperationSet::READ,
        );

        let result = fast_path_check(&table, &context);
        assert!(result.is_not_found());
    }

    #[test]
    fn test_fast_path_expired() {
        let mut table = CapabilityHashTable::new(256).expect("table creation");
        let mut cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cap.expires_at = Some(Timestamp::new(2000));
        let cap_id = cap.id.clone();

        table.insert(cap).expect("insert");

        // Check at time 3000 (after expiry at 2000)
        let context = FastPathCheckContext::with_time(
            AgentID::new("agent-a"),
            cap_id,
            OperationSet::READ,
            3000,
        );

        let result = fast_path_check(&table, &context);
        assert!(result.is_denied());
        match result {
            FastPathResult::Denied {
                reason: FastPathDenyReason::Expired,
            } => {},
            _ => panic!("Expected expired"),
        }
    }

    #[test]
    fn test_fast_path_not_expired() {
        let mut table = CapabilityHashTable::new(256).expect("table creation");
        let mut cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cap.expires_at = Some(Timestamp::new(3000));
        let cap_id = cap.id.clone();

        table.insert(cap).expect("insert");

        // Check at time 2000 (before expiry at 3000)
        let context = FastPathCheckContext::with_time(
            AgentID::new("agent-a"),
            cap_id,
            OperationSet::READ,
            2000,
        );

        let result = fast_path_check(&table, &context);
        assert!(result.is_granted());
    }

    #[test]
    fn test_fast_path_result_display() {
        let granted = FastPathResult::Granted {
            capability: Capability::new(
                CapID::from_bytes([1u8; 32]),
                AgentID::new("agent-a"),
                ResourceType::file(),
                ResourceID::new("file-001"),
                OperationSet::read(),
                Timestamp::new(1000),
            ),
        };
        assert!(granted.to_string().contains("Granted"));

        let not_found = FastPathResult::NotFound;
        assert!(not_found.to_string().contains("NotFound"));

        let denied = FastPathResult::Denied {
            reason: FastPathDenyReason::AgentMismatch,
        };
        assert!(denied.to_string().contains("Denied"));
    }
}

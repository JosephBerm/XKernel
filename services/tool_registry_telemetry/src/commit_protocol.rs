// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Commit protocol configuration for tool bindings.
//!
//! Defines the transactional behavior of tools that modify state,
//! specifying how prepare/commit phases work and rollback strategies.
//!
//! See Engineering Plan § 2.11.6: Commit Protocol.

use core::fmt;

/// Commit protocol type enumeration.
///
/// See Engineering Plan § 2.11.6: Commit Protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommitType {
    /// Two-phase commit (prepare + commit).
    ///
    /// Tool invocation is divided into:
    /// 1. Prepare phase: validate and plan changes (no state modification)
    /// 2. Commit phase: apply prepared changes atomically
    ///
    /// Enables rollback during prepare phase before committing.
    PrepareCommit,
}

impl fmt::Display for CommitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommitType::PrepareCommit => write!(f, "PrepareCommit"),
        }
    }
}

/// Rollback strategy for failed commit operations.
///
/// Determines how the system behaves when a tool invocation fails
/// during prepare or commit phases.
///
/// See Engineering Plan § 2.11.6: Commit Protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RollbackStrategy {
    /// Automatic rollback on failure.
    ///
    /// System automatically reverts any changes made during preparation.
    /// No manual intervention required.
    ///
    /// Suitable for tools where automatic reversion is safe and deterministic.
    Automatic,

    /// Manual rollback required.
    ///
    /// On failure, the system leaves changes in place and requires
    /// manual intervention to roll back. The agent or operator
    /// must explicitly authorize rollback.
    ///
    /// Suitable for high-impact operations where automatic reversion
    /// could cause problems.
    Manual,

    /// Rollback via compensating transaction.
    ///
    /// Instead of reverting changes, the system executes an inverse
    /// transaction to compensate. For example, debit a credit for a failed deposit.
    ///
    /// Suitable for operations where reversion is not available
    /// but compensation is possible.
    Compensating,
}

impl fmt::Display for RollbackStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RollbackStrategy::Automatic => write!(f, "Automatic"),
            RollbackStrategy::Manual => write!(f, "Manual"),
            RollbackStrategy::Compensating => write!(f, "Compensating"),
        }
    }
}

/// Commit protocol configuration for a tool binding.
///
/// Specifies how tool invocations are prepared, committed, and rolled back.
/// Used for tools that modify state and require transactional guarantees.
///
/// See Engineering Plan § 2.11.6: Commit Protocol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitProtocol {
    /// Type of commit protocol (currently PrepareCommit only).
    pub protocol_type: CommitType,

    /// Maximum milliseconds allowed for prepare phase.
    ///
    /// If prepare phase exceeds this timeout, the entire operation
    /// is aborted and rolled back.
    pub prepare_timeout_ms: u64,

    /// Maximum milliseconds allowed for commit phase.
    ///
    /// If commit phase exceeds this timeout, the operation fails
    /// and rollback is triggered according to rollback_strategy.
    pub commit_timeout_ms: u64,

    /// Strategy for rolling back failed operations.
    ///
    /// Determines what happens when prepare or commit fails.
    pub rollback_strategy: RollbackStrategy,
}

impl CommitProtocol {
    /// Creates a new commit protocol with PrepareCommit type.
    pub fn new(
        prepare_timeout_ms: u64,
        commit_timeout_ms: u64,
        rollback_strategy: RollbackStrategy,
    ) -> Self {
        CommitProtocol {
            protocol_type: CommitType::PrepareCommit,
            prepare_timeout_ms,
            commit_timeout_ms,
            rollback_strategy,
        }
    }

    /// Returns total timeout (prepare + commit).
    pub fn total_timeout_ms(&self) -> u64 {
        self.prepare_timeout_ms.saturating_add(self.commit_timeout_ms)
    }

    /// Returns true if both prepare and commit have reasonable timeouts (>0).
    pub fn is_valid(&self) -> bool {
        self.prepare_timeout_ms > 0 && self.commit_timeout_ms > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_type_display() {
        assert_eq!(CommitType::PrepareCommit.to_string(), "PrepareCommit");
    }

    #[test]
    fn test_rollback_strategy_display() {
        assert_eq!(RollbackStrategy::Automatic.to_string(), "Automatic");
        assert_eq!(RollbackStrategy::Manual.to_string(), "Manual");
        assert_eq!(RollbackStrategy::Compensating.to_string(), "Compensating");
    }

    #[test]
    fn test_commit_protocol_creation() {
        let protocol = CommitProtocol::new(5000, 10000, RollbackStrategy::Automatic);
        assert_eq!(protocol.protocol_type, CommitType::PrepareCommit);
        assert_eq!(protocol.prepare_timeout_ms, 5000);
        assert_eq!(protocol.commit_timeout_ms, 10000);
        assert_eq!(protocol.rollback_strategy, RollbackStrategy::Automatic);
    }

    #[test]
    fn test_total_timeout_ms() {
        let protocol = CommitProtocol::new(3000, 7000, RollbackStrategy::Automatic);
        assert_eq!(protocol.total_timeout_ms(), 10000);
    }

    #[test]
    fn test_total_timeout_overflow() {
        let protocol = CommitProtocol::new(u64::MAX, 1000, RollbackStrategy::Automatic);
        // saturating_add should cap at u64::MAX
        assert_eq!(protocol.total_timeout_ms(), u64::MAX);
    }

    #[test]
    fn test_is_valid() {
        let valid = CommitProtocol::new(1000, 2000, RollbackStrategy::Automatic);
        assert!(valid.is_valid());

        let invalid_prepare = CommitProtocol::new(0, 2000, RollbackStrategy::Automatic);
        assert!(!invalid_prepare.is_valid());

        let invalid_commit = CommitProtocol::new(1000, 0, RollbackStrategy::Automatic);
        assert!(!invalid_commit.is_valid());

        let both_zero = CommitProtocol::new(0, 0, RollbackStrategy::Automatic);
        assert!(!both_zero.is_valid());
    }

    #[test]
    fn test_commit_protocol_with_different_strategies() {
        let auto = CommitProtocol::new(1000, 2000, RollbackStrategy::Automatic);
        let manual = CommitProtocol::new(1000, 2000, RollbackStrategy::Manual);
        let compensating =
            CommitProtocol::new(1000, 2000, RollbackStrategy::Compensating);

        assert_eq!(auto.rollback_strategy, RollbackStrategy::Automatic);
        assert_eq!(manual.rollback_strategy, RollbackStrategy::Manual);
        assert_eq!(
            compensating.rollback_strategy,
            RollbackStrategy::Compensating
        );
    }

    #[test]
    fn test_commit_protocol_equality() {
        let p1 = CommitProtocol::new(1000, 2000, RollbackStrategy::Automatic);
        let p2 = CommitProtocol::new(1000, 2000, RollbackStrategy::Automatic);
        let p3 = CommitProtocol::new(1000, 3000, RollbackStrategy::Automatic);

        assert_eq!(p1, p2);
        assert_ne!(p1, p3);
    }

    #[test]
    fn test_rollback_strategy_equality() {
        assert_eq!(RollbackStrategy::Automatic, RollbackStrategy::Automatic);
        assert_ne!(RollbackStrategy::Automatic, RollbackStrategy::Manual);
        assert_ne!(
            RollbackStrategy::Manual,
            RollbackStrategy::Compensating
        );
    }

    #[test]
    fn test_rollback_strategy_hash() {
        use std::collections::hash_map::DefaultHasher;
        use core::hash::{Hash, Hasher};
use alloc::string::ToString;

        let mut h1 = DefaultHasher::new();
        RollbackStrategy::Automatic.hash(&mut h1);
        let hash1 = h1.finish();

        let mut h2 = DefaultHasher::new();
        RollbackStrategy::Automatic.hash(&mut h2);
        let hash2 = h2.finish();

        let mut h3 = DefaultHasher::new();
        RollbackStrategy::Manual.hash(&mut h3);
        let hash3 = h3.finish();

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}

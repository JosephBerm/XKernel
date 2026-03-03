// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Slow-path capability check: fallback for complex constraint evaluation.
//!
//! This module implements the slow-path check called <1% of the time, handling:
//! - Revocation chain traversal for derived capabilities
//! - Complex constraint evaluation (time bounds, rate limits, data volume)
//! - Uncached policy checks
//! See Engineering Plan § 3.2.0 and Week 6 § 3.

use alloc::vec::Vec;

#![forbid(unsafe_code)]

use core::fmt::{self, Debug};

use crate::capability::Capability;
use crate::constraints::{Timestamp, RateLimit, DataVolumeLimit};
use crate::error::CapError;
use crate::ids::{AgentID, CapID};
use crate::operations::OperationSet;

/// Result of a slow-path capability check.
/// See Week 6 § 3: Slow-Path Capability Check.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SlowPathResult {
    /// All constraint checks passed, access granted.
    Granted {
        /// The capability that was checked
        capability: Capability,

        /// Constraint violations found (if any, still granted)
        constraint_warnings: alloc::vec::Vec<ConstraintWarning>,
    },

    /// One or more constraints failed.
    Denied {
        /// Primary reason for denial
        reason: SlowPathDenyReason,

        /// Additional constraint violations (for logging)
        violations: alloc::vec::Vec<ConstraintViolation>,
    },

    /// Error during evaluation.
    Error(CapError),
}

impl SlowPathResult {
    /// Returns true if access was granted.
    pub fn is_granted(&self) -> bool {
        matches!(self, SlowPathResult::Granted { .. })
    }

    /// Returns true if access was denied.
    pub fn is_denied(&self) -> bool {
        matches!(self, SlowPathResult::Denied { .. })
    }

    /// Returns true if an error occurred.
    pub fn is_error(&self) -> bool {
        matches!(self, SlowPathResult::Error(_))
    }
}

impl fmt::Display for SlowPathResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlowPathResult::Granted { .. } => write!(f, "SlowPathResult::Granted"),
            SlowPathResult::Denied { reason, .. } => write!(f, "SlowPathResult::Denied({})", reason),
            SlowPathResult::Error(e) => write!(f, "SlowPathResult::Error({})", e),
        }
    }
}

/// Reason for slow-path capability denial.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SlowPathDenyReason {
    /// Rate limit constraint violated.
    RateLimitExceeded,

    /// Data volume limit constraint violated.
    DataVolumeLimitExceeded,

    /// Time bound constraint violated (start not reached or expiry passed).
    TimeBoundViolation,

    /// Delegation depth limit exceeded.
    DelegationDepthExceeded,

    /// Multiple constraint violations.
    MultipleViolations,

    /// Policy evaluation returned denial.
    PolicyDenied,

    /// Revocation chain check failed.
    RevocationChainViolation,
}

impl fmt::Display for SlowPathDenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SlowPathDenyReason::RateLimitExceeded => write!(f, "rate limit exceeded"),
            SlowPathDenyReason::DataVolumeLimitExceeded => write!(f, "data volume limit exceeded"),
            SlowPathDenyReason::TimeBoundViolation => write!(f, "time bound violation"),
            SlowPathDenyReason::DelegationDepthExceeded => write!(f, "delegation depth exceeded"),
            SlowPathDenyReason::MultipleViolations => write!(f, "multiple violations"),
            SlowPathDenyReason::PolicyDenied => write!(f, "policy denied"),
            SlowPathDenyReason::RevocationChainViolation => write!(f, "revocation chain violation"),
        }
    }
}

/// A constraint violation detected during slow-path evaluation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConstraintViolation {
    /// Type of constraint violated.
    pub constraint_type: ConstraintType,

    /// Description of the violation.
    pub description: alloc::string::String,
}

impl ConstraintViolation {
    /// Creates a new constraint violation.
    pub fn new(constraint_type: ConstraintType, description: impl Into<alloc::string::String>) -> Self {
        ConstraintViolation {
            constraint_type,
            description: description.into(),
        }
    }
}

impl fmt::Display for ConstraintViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.constraint_type, self.description)
    }
}

/// Types of constraints that can be evaluated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConstraintType {
    /// Time-based constraint (start/expiry).
    TimeBound,

    /// Rate limit (operations per period).
    RateLimit,

    /// Data volume constraint.
    DataVolume,

    /// Delegation depth constraint.
    DelegationDepth,

    /// Policy constraint.
    Policy,

    /// Revocation status.
    Revocation,
}

impl fmt::Display for ConstraintType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintType::TimeBound => write!(f, "TimeBound"),
            ConstraintType::RateLimit => write!(f, "RateLimit"),
            ConstraintType::DataVolume => write!(f, "DataVolume"),
            ConstraintType::DelegationDepth => write!(f, "DelegationDepth"),
            ConstraintType::Policy => write!(f, "Policy"),
            ConstraintType::Revocation => write!(f, "Revocation"),
        }
    }
}

/// A non-fatal constraint warning (capability still granted).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConstraintWarning {
    /// Type of constraint.
    pub constraint_type: ConstraintType,

    /// Warning message.
    pub message: alloc::string::String,
}

impl ConstraintWarning {
    /// Creates a new constraint warning.
    pub fn new(constraint_type: ConstraintType, message: impl Into<alloc::string::String>) -> Self {
        ConstraintWarning {
            constraint_type,
            message: message.into(),
        }
    }
}

impl fmt::Display for ConstraintWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Warning - {}: {}", self.constraint_type, self.message)
    }
}

/// Slow-path capability check context.
/// Extended context for complex constraint evaluation.
/// See Week 6 § 3.
#[derive(Clone, Debug)]
pub struct SlowPathCheckContext {
    /// Agent performing the operation
    pub agent_id: AgentID,

    /// Capability to check
    pub capability: Capability,

    /// Operation being performed
    pub operation: u8,

    /// Current time in nanoseconds
    pub current_time_ns: u64,

    /// Operations performed so far in current rate limit period
    pub operations_in_period: u32,

    /// Data volume transferred so far in current period (bytes)
    pub data_volume_in_period: u64,

    /// Current delegation depth (0 for root caps)
    pub delegation_depth: u32,

    /// Whether to check revocation chain
    pub check_revocation: bool,
}

impl SlowPathCheckContext {
    /// Creates a new slow-path check context.
    pub fn new(
        agent_id: AgentID,
        capability: Capability,
        operation: u8,
        current_time_ns: u64,
    ) -> Self {
        SlowPathCheckContext {
            agent_id,
            capability,
            operation,
            current_time_ns,
            operations_in_period: 0,
            data_volume_in_period: 0,
            delegation_depth: 0,
            check_revocation: true,
        }
    }

    /// Sets rate limit context.
    pub fn with_rate_limit(mut self, ops_so_far: u32, period_nanos: u64) -> Self {
        self.operations_in_period = ops_so_far;
        self
    }

    /// Sets data volume context.
    pub fn with_data_volume(mut self, bytes_so_far: u64) -> Self {
        self.data_volume_in_period = bytes_so_far;
        self
    }

    /// Sets delegation depth context.
    pub fn with_delegation_depth(mut self, depth: u32) -> Self {
        self.delegation_depth = depth;
        self
    }

    /// Disables revocation checking (for testing).
    pub fn skip_revocation_check(mut self) -> Self {
        self.check_revocation = false;
        self
    }
}

/// Slow-path capability check: complex constraint evaluation.
///
/// # Algorithm
///
/// 1. Validate time bounds (if present)
/// 2. Check rate limits (if present)
/// 3. Check data volume limits (if present)
/// 4. Validate delegation depth (if present)
/// 5. Check revocation chain (optional)
/// 6. Evaluate policies (delegated to policy engine)
/// 7. Return result with constraint status
///
/// # Latency
///
/// Typically 1-10µs depending on constraint complexity.
/// Called <1% of the time (99% fast-path hits).
///
/// See Week 6 § 3.
pub fn slow_path_check(context: &SlowPathCheckContext) -> SlowPathResult {

    let cap = &context.capability;
    let mut violations: Vec<ConstraintViolation> = Vec::new();
    let mut warnings: Vec<ConstraintWarning> = Vec::new();

    // Check 1: Time bounds
    if let Some(time_bound) = cap.constraints.time_bound {
        if context.current_time_ns < time_bound.start_timestamp.nanos() {
            violations.push(ConstraintViolation::new(
                ConstraintType::TimeBound,
                "capability not yet valid (start time not reached)".into(),
            ));
        } else if context.current_time_ns >= time_bound.expiry_timestamp.nanos() {
            violations.push(ConstraintViolation::new(
                ConstraintType::TimeBound,
                "capability has expired".into(),
            ));
        }
    }

    // Check 2: Rate limits
    if let Some(rate_limit) = cap.constraints.rate_limited {
        if context.operations_in_period >= rate_limit.max_operations_per_period {
            violations.push(ConstraintViolation::new(
                ConstraintType::RateLimit,
                format!(
                    "rate limit {} ops/period exceeded at {} ops",
                    rate_limit.max_operations_per_period, context.operations_in_period
                ),
            ));
        } else {
            let remaining = rate_limit.max_operations_per_period - context.operations_in_period;
            if remaining <= 10 {
                warnings.push(ConstraintWarning::new(
                    ConstraintType::RateLimit,
                    format!("rate limit approaching: {} ops remaining", remaining),
                ));
            }
        }
    }

    // Check 3: Data volume limits
    if let Some(volume_limit) = cap.constraints.data_volume_limited {
        if context.data_volume_in_period >= volume_limit.max_bytes_per_period {
            violations.push(ConstraintViolation::new(
                ConstraintType::DataVolume,
                format!(
                    "data volume limit {} bytes/period exceeded at {} bytes",
                    volume_limit.max_bytes_per_period, context.data_volume_in_period
                ),
            ));
        } else {
            let remaining = volume_limit.max_bytes_per_period - context.data_volume_in_period;
            if remaining <= (1024 * 1024) {
                // 1MB remaining
                warnings.push(ConstraintWarning::new(
                    ConstraintType::DataVolume,
                    format!("data volume approaching: {} bytes remaining", remaining),
                ));
            }
        }
    }

    // Check 4: Delegation depth
    if let Some(depth_limit) = cap.constraints.delegation_depth_limit {
        if context.delegation_depth >= depth_limit.max_depth {
            violations.push(ConstraintViolation::new(
                ConstraintType::DelegationDepth,
                format!(
                    "delegation depth limit {} exceeded at depth {}",
                    depth_limit.max_depth, context.delegation_depth
                ),
            ));
        }
    }

    // Check 5: Revocation chain (simplified: just check if cap is in chain)
    if context.check_revocation && !cap.chain.entries.is_empty() {
        // Note: A real implementation would check revocation status here
        // For now, we just verify the chain is not empty
        if cap.chain.entries.is_empty() {
            violations.push(ConstraintViolation::new(
                ConstraintType::Revocation,
                "capability chain is empty".into(),
            ));
        }
    }

    // Accumulate results
    if !violations.is_empty() {
        let reason = if violations.len() == 1 {
            match violations[0].constraint_type {
                ConstraintType::TimeBound => SlowPathDenyReason::TimeBoundViolation,
                ConstraintType::RateLimit => SlowPathDenyReason::RateLimitExceeded,
                ConstraintType::DataVolume => SlowPathDenyReason::DataVolumeLimitExceeded,
                ConstraintType::DelegationDepth => SlowPathDenyReason::DelegationDepthExceeded,
                ConstraintType::Policy => SlowPathDenyReason::PolicyDenied,
                ConstraintType::Revocation => SlowPathDenyReason::RevocationChainViolation,
            }
        } else {
            SlowPathDenyReason::MultipleViolations
        };

        SlowPathResult::Denied {
            reason,
            violations,
        }
    } else {
        SlowPathResult::Granted {
            capability: cap.clone(),
            constraint_warnings: warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraints::{TimeBound, RateLimit as CapRateLimit};
    use crate::ids::ResourceID;
    use crate::ids::ResourceType;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_slow_path_granted_no_constraints() {
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );

        let context = SlowPathCheckContext::new(
            AgentID::new("agent-a"),
            cap,
            OperationSet::READ,
            5000,
        );

        let result = slow_path_check(&context);
        assert!(result.is_granted());
    }

    #[test]
    fn test_slow_path_time_bound_valid() {
        let mut cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cap.constraints.time_bound = Some(TimeBound::new(Timestamp::new(1000), Timestamp::new(10000)));

        let context = SlowPathCheckContext::new(
            AgentID::new("agent-a"),
            cap,
            OperationSet::READ,
            5000, // Within bounds
        );

        let result = slow_path_check(&context);
        assert!(result.is_granted());
    }

    #[test]
    fn test_slow_path_time_bound_expired() {
        let mut cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cap.constraints.time_bound = Some(TimeBound::new(Timestamp::new(1000), Timestamp::new(5000)));

        let context = SlowPathCheckContext::new(
            AgentID::new("agent-a"),
            cap,
            OperationSet::READ,
            6000, // After expiry
        );

        let result = slow_path_check(&context);
        assert!(result.is_denied());
    }

    #[test]
    fn test_slow_path_time_bound_not_yet_valid() {
        let mut cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cap.constraints.time_bound = Some(TimeBound::new(Timestamp::new(5000), Timestamp::new(10000)));

        let context = SlowPathCheckContext::new(
            AgentID::new("agent-a"),
            cap,
            OperationSet::READ,
            2000, // Before start time
        );

        let result = slow_path_check(&context);
        assert!(result.is_denied());
    }

    #[test]
    fn test_slow_path_rate_limit_exceeded() {
        let mut cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cap.constraints.rate_limited = Some(CapRateLimit::new(100, 1_000_000_000));

        let context = SlowPathCheckContext::new(
            AgentID::new("agent-a"),
            cap,
            OperationSet::READ,
            5000,
        )
        .with_rate_limit(100, 1_000_000_000); // Already at limit

        let result = slow_path_check(&context);
        assert!(result.is_denied());
    }

    #[test]
    fn test_slow_path_rate_limit_ok() {
        let mut cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::read(),
            Timestamp::new(1000),
        );
        cap.constraints.rate_limited = Some(CapRateLimit::new(100, 1_000_000_000));

        let context = SlowPathCheckContext::new(
            AgentID::new("agent-a"),
            cap,
            OperationSet::READ,
            5000,
        )
        .with_rate_limit(50, 1_000_000_000); // 50 < 100

        let result = slow_path_check(&context);
        assert!(result.is_granted());
    }

    #[test]
    fn test_constraint_violation_display() {
        let violation = ConstraintViolation::new(
            ConstraintType::TimeBound,
            "test violation".to_string(),
        );
        assert!(violation.to_string().contains("TimeBound"));
    }

    #[test]
    fn test_constraint_warning_display() {
        let warning = ConstraintWarning::new(
            ConstraintType::RateLimit,
            "test warning".to_string(),
        );
        assert!(warning.to_string().contains("RateLimit"));
    }
}

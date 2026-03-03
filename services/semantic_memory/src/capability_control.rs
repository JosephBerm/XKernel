// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Capability model for cross-tier memory access control.
//!
//! This module implements a capability-based security model that controls
//! which agents can access which memory tiers and perform which operations.
//!
//! See Engineering Plan § 3.1: Capability-Based Security & § 4.1.3: Access Control.

use alloc::collections::BTreeSet;
use alloc::string::String;
use crate::error::{MemoryError, Result};
use crate::concurrency::{MemoryTier, MemoryOperation};

/// Represents a capability that grants access to memory tiers and operations.
///
/// Capabilities are unforgeable tokens that authorize specific operations
/// on specific tiers for a bounded scope.
///
/// See Engineering Plan § 3.1: Capability-Based Security.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCapability {
    /// Unique capability identifier
    cap_id: String,

    /// Set of memory tiers this capability grants access to
    tier_access: BTreeSet<MemoryTier>,

    /// Set of operations this capability authorizes
    operations: BTreeSet<MemoryOperation>,

    /// Optional region scope restriction (e.g., "l2-agent-001")
    region_scope: Option<String>,

    /// Size limit in bytes (None = unlimited)
    size_limit_bytes: Option<u64>,

    /// Whether this capability has been revoked
    revoked: bool,
}

impl MemoryCapability {
    /// Creates a new memory capability.
    ///
    /// # Arguments
    ///
    /// * `cap_id` - Unique identifier for this capability
    /// * `tier_access` - Set of tiers accessible via this capability
    /// * `operations` - Set of operations allowed
    ///
    /// # See
    ///
    /// Engineering Plan § 3.1: Capability Creation.
    pub fn new(
        cap_id: impl Into<String>,
        tier_access: BTreeSet<MemoryTier>,
        operations: BTreeSet<MemoryOperation>,
    ) -> Self {
        MemoryCapability {
            cap_id: cap_id.into(),
            tier_access,
            operations,
            region_scope: None,
            size_limit_bytes: None,
            revoked: false,
        }
    }

    /// Creates a read-only capability for a specific tier.
    pub fn read_only(tier: MemoryTier) -> Self {
        let mut tiers = BTreeSet::new();
        tiers.insert(tier);

        let mut ops = BTreeSet::new();
        ops.insert(MemoryOperation::Read);

        MemoryCapability {
            cap_id: format!("cap-ro-{}", tier.name()),
            tier_access: tiers,
            operations: ops,
            region_scope: None,
            size_limit_bytes: None,
            revoked: false,
        }
    }

    /// Creates a read-write capability for a specific tier.
    pub fn read_write(tier: MemoryTier) -> Self {
        let mut tiers = BTreeSet::new();
        tiers.insert(tier);

        let mut ops = BTreeSet::new();
        ops.insert(MemoryOperation::Read);
        ops.insert(MemoryOperation::Write);

        MemoryCapability {
            cap_id: format!("cap-rw-{}", tier.name()),
            tier_access: tiers,
            operations: ops,
            region_scope: None,
            size_limit_bytes: None,
            revoked: false,
        }
    }

    /// Returns the capability ID.
    pub fn cap_id(&self) -> &str {
        &self.cap_id
    }

    /// Returns the set of accessible tiers.
    pub fn tier_access(&self) -> &BTreeSet<MemoryTier> {
        &self.tier_access
    }

    /// Returns the set of allowed operations.
    pub fn operations(&self) -> &BTreeSet<MemoryOperation> {
        &self.operations
    }

    /// Returns the region scope if set.
    pub fn region_scope(&self) -> Option<&str> {
        self.region_scope.as_deref()
    }

    /// Returns the size limit if set.
    pub fn size_limit_bytes(&self) -> Option<u64> {
        self.size_limit_bytes
    }

    /// Returns whether this capability has been revoked.
    pub fn is_revoked(&self) -> bool {
        self.revoked
    }

    /// Sets the region scope restriction.
    pub fn with_region_scope(mut self, scope: impl Into<String>) -> Self {
        self.region_scope = Some(scope.into());
        self
    }

    /// Sets the size limit.
    pub fn with_size_limit(mut self, limit: u64) -> Self {
        self.size_limit_bytes = Some(limit);
        self
    }

    /// Revokes this capability.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Checks if a capability grants access for a specific operation on a tier.
    pub fn grants_access(&self, tier: MemoryTier, op: MemoryOperation) -> bool {
        !self.revoked && self.tier_access.contains(&tier) && self.operations.contains(&op)
    }
}

/// Access rule for cross-tier operations.
///
/// Defines what operations are allowed between tiers.
#[derive(Clone, Debug)]
pub struct TierAccessRule {
    /// The tier being accessed
    tier: MemoryTier,

    /// Allowed operations on this tier
    allowed_operations: BTreeSet<MemoryOperation>,

    /// Size limit in bytes for allocations in this tier
    size_limit_bytes: u64,

    /// Rate limit in operations per second
    rate_limit_ops_per_sec: u32,
}

impl TierAccessRule {
    /// Creates a new tier access rule.
    pub fn new(
        tier: MemoryTier,
        allowed_operations: BTreeSet<MemoryOperation>,
        size_limit_bytes: u64,
        rate_limit_ops_per_sec: u32,
    ) -> Self {
        TierAccessRule {
            tier,
            allowed_operations,
            size_limit_bytes,
            rate_limit_ops_per_sec,
        }
    }

    /// Returns the tier this rule applies to.
    pub fn tier(&self) -> MemoryTier {
        self.tier
    }

    /// Returns whether an operation is allowed.
    pub fn allows_operation(&self, op: MemoryOperation) -> bool {
        self.allowed_operations.contains(&op)
    }

    /// Returns the size limit.
    pub fn size_limit_bytes(&self) -> u64 {
        self.size_limit_bytes
    }

    /// Returns the rate limit.
    pub fn rate_limit_ops_per_sec(&self) -> u32 {
        self.rate_limit_ops_per_sec
    }
}

/// Cross-tier migration policy.
///
/// Defines what migrations are allowed between tiers.
#[derive(Clone, Debug)]
pub struct CrossTierPolicy {
    /// Source tier
    source_tier: MemoryTier,

    /// Destination tier
    target_tier: MemoryTier,

    /// Whether migration is allowed
    allowed: bool,

    /// Whether a capability is required for migration
    requires_capability: bool,

    /// Migration mode/strategy
    migration_mode: String,
}

impl CrossTierPolicy {
    /// Creates a new cross-tier policy.
    pub fn new(
        source_tier: MemoryTier,
        target_tier: MemoryTier,
        allowed: bool,
        requires_capability: bool,
        migration_mode: impl Into<String>,
    ) -> Self {
        CrossTierPolicy {
            source_tier,
            target_tier,
            allowed,
            requires_capability,
            migration_mode: migration_mode.into(),
        }
    }

    /// Returns the source tier.
    pub fn source_tier(&self) -> MemoryTier {
        self.source_tier
    }

    /// Returns the target tier.
    pub fn target_tier(&self) -> MemoryTier {
        self.target_tier
    }

    /// Returns whether the migration is allowed.
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Returns whether a capability is required.
    pub fn requires_capability(&self) -> bool {
        self.requires_capability
    }

    /// Returns the migration mode.
    pub fn migration_mode(&self) -> &str {
        &self.migration_mode
    }

    /// Returns the default L1 -> L2 migration policy (allowed, no capability).
    pub fn l1_to_l2() -> Self {
        CrossTierPolicy::new(
            MemoryTier::L1,
            MemoryTier::L2,
            true,
            false,
            "eviction",
        )
    }

    /// Returns the default L2 -> L3 migration policy (allowed, no capability).
    pub fn l2_to_l3() -> Self {
        CrossTierPolicy::new(
            MemoryTier::L2,
            MemoryTier::L3,
            true,
            false,
            "spill_and_compact",
        )
    }

    /// Returns the L3 -> L2 prefetch policy (allowed, requires capability).
    pub fn l3_to_l2_prefetch() -> Self {
        CrossTierPolicy::new(
            MemoryTier::L3,
            MemoryTier::L2,
            true,
            true,
            "prefetch",
        )
    }

    /// Returns the L2 -> L1 prefetch policy (allowed, requires capability).
    pub fn l2_to_l1_prefetch() -> Self {
        CrossTierPolicy::new(
            MemoryTier::L2,
            MemoryTier::L1,
            true,
            true,
            "prefetch",
        )
    }
}

/// Trait for validating memory access based on capabilities.
///
/// Implementations of this trait enforce capability-based access control.
pub trait MemoryCapabilityValidator {
    /// Validates whether an operation is allowed.
    ///
    /// # Arguments
    ///
    /// * `cap` - The capability to validate
    /// * `tier` - The tier being accessed
    /// * `op` - The operation being performed
    ///
    /// # Returns
    ///
    /// Ok if access is allowed, CapabilityDenied error otherwise.
    ///
    /// # See
    ///
    /// Engineering Plan § 3.1: Capability Validation.
    fn validate_access(
        &self,
        cap: &MemoryCapability,
        tier: MemoryTier,
        op: MemoryOperation,
    ) -> Result<()>;
}

/// Default implementation of memory capability validation.
#[derive(Clone, Debug)]
pub struct DefaultMemoryCapabilityValidator;

impl MemoryCapabilityValidator for DefaultMemoryCapabilityValidator {
    fn validate_access(
        &self,
        cap: &MemoryCapability,
        tier: MemoryTier,
        op: MemoryOperation,
    ) -> Result<()> {
        if cap.is_revoked() {
            return Err(MemoryError::CapabilityDenied {
                operation: op.name().to_string(),
                resource: format!("tier-{}", tier.name()),
            });
        }

        if !cap.grants_access(tier, op) {
            return Err(MemoryError::CapabilityDenied {
                operation: op.name().to_string(),
                resource: format!("tier-{}", tier.name()),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_memory_capability_creation() {
        let mut tiers = BTreeSet::new();
        tiers.insert(MemoryTier::L1);

        let mut ops = BTreeSet::new();
        ops.insert(MemoryOperation::Read);

        let cap = MemoryCapability::new("cap-001", tiers, ops);

        assert_eq!(cap.cap_id(), "cap-001");
        assert!(cap.tier_access().contains(&MemoryTier::L1));
        assert!(cap.operations().contains(&MemoryOperation::Read));
        assert!(!cap.is_revoked());
    }

    #[test]
    fn test_memory_capability_read_only() {
        let cap = MemoryCapability::read_only(MemoryTier::L2);

        assert!(cap.tier_access().contains(&MemoryTier::L2));
        assert!(cap.operations().contains(&MemoryOperation::Read));
        assert!(!cap.operations().contains(&MemoryOperation::Write));
    }

    #[test]
    fn test_memory_capability_read_write() {
        let cap = MemoryCapability::read_write(MemoryTier::L2);

        assert!(cap.tier_access().contains(&MemoryTier::L2));
        assert!(cap.operations().contains(&MemoryOperation::Read));
        assert!(cap.operations().contains(&MemoryOperation::Write));
    }

    #[test]
    fn test_memory_capability_with_region_scope() {
        let cap = MemoryCapability::read_only(MemoryTier::L1)
            .with_region_scope("l1-agent-001");

        assert_eq!(cap.region_scope(), Some("l1-agent-001"));
    }

    #[test]
    fn test_memory_capability_with_size_limit() {
        let cap = MemoryCapability::read_write(MemoryTier::L2)
            .with_size_limit(1024 * 1024);

        assert_eq!(cap.size_limit_bytes(), Some(1024 * 1024));
    }

    #[test]
    fn test_memory_capability_revoke() {
        let mut cap = MemoryCapability::read_only(MemoryTier::L1);
        assert!(!cap.is_revoked());

        cap.revoke();
        assert!(cap.is_revoked());
    }

    #[test]
    fn test_memory_capability_grants_access() {
        let cap = MemoryCapability::read_write(MemoryTier::L1);

        assert!(cap.grants_access(MemoryTier::L1, MemoryOperation::Read));
        assert!(cap.grants_access(MemoryTier::L1, MemoryOperation::Write));
        assert!(!cap.grants_access(MemoryTier::L2, MemoryOperation::Read));
    }

    #[test]
    fn test_memory_capability_grants_access_revoked() {
        let mut cap = MemoryCapability::read_only(MemoryTier::L1);
        cap.revoke();

        assert!(!cap.grants_access(MemoryTier::L1, MemoryOperation::Read));
    }

    #[test]
    fn test_tier_access_rule_creation() {
        let mut ops = BTreeSet::new();
        ops.insert(MemoryOperation::Read);
        ops.insert(MemoryOperation::Write);

        let rule = TierAccessRule::new(MemoryTier::L2, ops, 1024 * 1024, 1000);

        assert_eq!(rule.tier(), MemoryTier::L2);
        assert!(rule.allows_operation(MemoryOperation::Read));
        assert!(rule.allows_operation(MemoryOperation::Write));
        assert!(!rule.allows_operation(MemoryOperation::Delete));
        assert_eq!(rule.size_limit_bytes(), 1024 * 1024);
        assert_eq!(rule.rate_limit_ops_per_sec(), 1000);
    }

    #[test]
    fn test_cross_tier_policy_creation() {
        let policy = CrossTierPolicy::new(
            MemoryTier::L1,
            MemoryTier::L2,
            true,
            false,
            "eviction",
        );

        assert_eq!(policy.source_tier(), MemoryTier::L1);
        assert_eq!(policy.target_tier(), MemoryTier::L2);
        assert!(policy.is_allowed());
        assert!(!policy.requires_capability());
        assert_eq!(policy.migration_mode(), "eviction");
    }

    #[test]
    fn test_cross_tier_policy_l1_to_l2() {
        let policy = CrossTierPolicy::l1_to_l2();

        assert_eq!(policy.source_tier(), MemoryTier::L1);
        assert_eq!(policy.target_tier(), MemoryTier::L2);
        assert!(policy.is_allowed());
        assert!(!policy.requires_capability());
    }

    #[test]
    fn test_cross_tier_policy_l2_to_l3() {
        let policy = CrossTierPolicy::l2_to_l3();

        assert_eq!(policy.source_tier(), MemoryTier::L2);
        assert_eq!(policy.target_tier(), MemoryTier::L3);
        assert!(policy.is_allowed());
        assert!(!policy.requires_capability());
    }

    #[test]
    fn test_cross_tier_policy_l3_to_l2_prefetch() {
        let policy = CrossTierPolicy::l3_to_l2_prefetch();

        assert_eq!(policy.source_tier(), MemoryTier::L3);
        assert_eq!(policy.target_tier(), MemoryTier::L2);
        assert!(policy.is_allowed());
        assert!(policy.requires_capability());
    }

    #[test]
    fn test_cross_tier_policy_l2_to_l1_prefetch() {
        let policy = CrossTierPolicy::l2_to_l1_prefetch();

        assert_eq!(policy.source_tier(), MemoryTier::L2);
        assert_eq!(policy.target_tier(), MemoryTier::L1);
        assert!(policy.is_allowed());
        assert!(policy.requires_capability());
    }

    #[test]
    fn test_default_memory_capability_validator_valid_access() {
        let validator = DefaultMemoryCapabilityValidator;
        let cap = MemoryCapability::read_write(MemoryTier::L1);

        let result = validator.validate_access(&cap, MemoryTier::L1, MemoryOperation::Read);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_memory_capability_validator_denied_access() {
        let validator = DefaultMemoryCapabilityValidator;
        let cap = MemoryCapability::read_only(MemoryTier::L1);

        let result = validator.validate_access(&cap, MemoryTier::L1, MemoryOperation::Write);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_memory_capability_validator_wrong_tier() {
        let validator = DefaultMemoryCapabilityValidator;
        let cap = MemoryCapability::read_write(MemoryTier::L1);

        let result = validator.validate_access(&cap, MemoryTier::L2, MemoryOperation::Read);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_memory_capability_validator_revoked() {
        let validator = DefaultMemoryCapabilityValidator;
        let mut cap = MemoryCapability::read_write(MemoryTier::L1);
        cap.revoke();

        let result = validator.validate_access(&cap, MemoryTier::L1, MemoryOperation::Read);
        assert!(result.is_err());
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Capability delegation chains and attenuation rules

use crate::capability::{Capability, CapabilityError, Permission, PermissionFlags};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub type Result<T> = core::result::Result<T, CapabilityError>;

/// Attenuation rule for capability delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttenuationRule {
    /// Permission to attenuate with
    pub permission: Permission,
    /// Whether this rule is mandatory
    pub mandatory: bool,
}

impl AttenuationRule {
    /// Create a new attenuation rule
    pub fn new(permission: Permission, mandatory: bool) -> Self {
        Self { permission, mandatory }
    }

    /// Apply this rule to a capability
    pub fn apply(&self, cap: &Capability) -> Result<Capability> {
        cap.attenuate(&self.permission)
    }
}

/// Revocation policy for delegated capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevocationPolicy {
    /// Revoke immediately
    Immediate,
    /// Revoke with a delay (in milliseconds)
    Delayed(u64),
    /// Revoke on specific condition
    Conditional,
    /// No revocation
    Never,
}

impl RevocationPolicy {
    /// Check if revocation should occur
    pub fn should_revoke(&self, elapsed_ms: u64) -> bool {
        match self {
            RevocationPolicy::Immediate => true,
            RevocationPolicy::Delayed(delay) => elapsed_ms >= *delay,
            RevocationPolicy::Conditional => false,
            RevocationPolicy::Never => false,
        }
    }
}

/// Entry in a delegation chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationEntry {
    /// Source capability
    pub from_cap: u64,
    /// Derived capability
    pub to_cap: u64,
    /// Attenuation applied
    pub attenuation: AttenuationRule,
    /// Revocation policy
    pub revocation_policy: RevocationPolicy,
    /// Timestamp of delegation
    pub timestamp: u64,
}

impl DelegationEntry {
    /// Create a new delegation entry
    pub fn new(
        from_cap: u64,
        to_cap: u64,
        attenuation: AttenuationRule,
        revocation_policy: RevocationPolicy,
    ) -> Self {
        Self {
            from_cap,
            to_cap,
            attenuation,
            revocation_policy,
            timestamp: 0,
        }
    }
}

/// Delegation chain tracking capability derivation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationChain {
    /// Entries in the chain
    entries: Vec<DelegationEntry>,
    /// Chain depth limit
    max_depth: usize,
}

impl DelegationChain {
    /// Create a new delegation chain
    pub fn new(max_depth: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_depth,
        }
    }

    /// Add a delegation entry
    pub fn add_entry(&mut self, entry: DelegationEntry) -> Result<()> {
        if self.entries.len() >= self.max_depth {
            return Err(CapabilityError::AttenuationViolation(
                "delegation chain depth exceeded".into(),
            ));
        }

        self.entries.push(entry);
        Ok(())
    }

    /// Get all entries in the chain
    pub fn entries(&self) -> &[DelegationEntry] {
        &self.entries
    }

    /// Get the chain depth
    pub fn depth(&self) -> usize {
        self.entries.len()
    }

    /// Verify capability ancestry
    pub fn verify_ancestry(&self, cap_id: u64) -> Result<bool> {
        Ok(self.entries.iter().any(|e| e.to_cap == cap_id))
    }

    /// Get the root capability
    pub fn root_cap(&self) -> Option<u64> {
        self.entries.first().map(|e| e.from_cap)
    }

    /// Get the leaf capability
    pub fn leaf_cap(&self) -> Option<u64> {
        self.entries.last().map(|e| e.to_cap)
    }

    /// Revoke all capabilities in the chain
    pub fn revoke_all(&mut self) {
        for entry in &mut self.entries {
            entry.revocation_policy = RevocationPolicy::Immediate;
        }
    }

    /// Find an entry by source capability
    pub fn find_entry(&self, from_cap: u64) -> Option<&DelegationEntry> {
        self.entries.iter().find(|e| e.from_cap == from_cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delegation_entry_creation() {
        let perm = Permission::new(1, PermissionFlags::read());
        let rule = AttenuationRule::new(perm, false);
        let entry = DelegationEntry::new(1, 2, rule, RevocationPolicy::Never);

        assert_eq!(entry.from_cap, 1);
        assert_eq!(entry.to_cap, 2);
    }

    #[test]
    fn test_delegation_chain() {
        let mut chain = DelegationChain::new(10);

        let perm = Permission::new(1, PermissionFlags::read());
        let rule = AttenuationRule::new(perm, false);

        chain
            .add_entry(DelegationEntry::new(1, 2, rule.clone(), RevocationPolicy::Never))
            .unwrap();

        assert_eq!(chain.depth(), 1);
        assert_eq!(chain.root_cap(), Some(1));
        assert_eq!(chain.leaf_cap(), Some(2));
    }

    #[test]
    fn test_chain_depth_limit() {
        let mut chain = DelegationChain::new(1);
        let perm = Permission::new(1, PermissionFlags::read());
        let rule = AttenuationRule::new(perm, false);

        chain
            .add_entry(DelegationEntry::new(1, 2, rule.clone(), RevocationPolicy::Never))
            .unwrap();

        assert!(chain
            .add_entry(DelegationEntry::new(2, 3, rule, RevocationPolicy::Never))
            .is_err());
    }

    #[test]
    fn test_revocation_policy() {
        let policy = RevocationPolicy::Delayed(1000);
        assert!(!policy.should_revoke(999));
        assert!(policy.should_revoke(1000));
    }
}

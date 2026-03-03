// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Membrane pattern: trust boundary enforcement.
//!
//! This module implements the membrane pattern for capabilities, creating
//! trust boundaries that enforce additional constraints on capability usage.
//! Used for cross-crew capability sharing with controlled degradation.
//! See Engineering Plan § 3.2.7: Membrane Pattern & Trust Boundaries.

use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::constraints::Timestamp;
use crate::error::CapError;
use crate::ids::{CapID, AgentID};
use crate::operations::OperationSet;

/// Configuration for a capability membrane.
///
/// Defines the constraints that a membrane-wrapped capability must enforce.
/// See Engineering Plan § 3.2.7: Membrane Pattern & Trust Boundaries.
#[derive(Clone, Debug)]
pub struct MembraneConfig {
    /// Set of operations allowed through the membrane.
    pub allowed_operations: OperationSet,

    /// Set of target agents that can exercise the capability through the membrane.
    pub allowed_targets: Vec<AgentID>,

    /// Maximum delegation depth allowed through the membrane.
    pub max_delegation_depth: u32,

    /// Time limit for the membrane wrapper (nanoseconds).
    /// If set, the membrane-wrapped capability expires after this duration.
    pub time_limit_nanos: Option<u64>,
}

impl MembraneConfig {
    /// Creates a new membrane configuration.
    pub fn new(
        allowed_operations: OperationSet,
        allowed_targets: Vec<AgentID>,
        max_delegation_depth: u32,
    ) -> Self {
        MembraneConfig {
            allowed_operations,
            allowed_targets,
            max_delegation_depth,
            time_limit_nanos: None,
        }
    }

    /// Sets the time limit for the membrane.
    pub fn with_time_limit(mut self, nanos: u64) -> Self {
        self.time_limit_nanos = Some(nanos);
        self
    }

    /// Validates the configuration for consistency.
    pub fn validate(&self) -> Result<(), CapError> {
        if self.allowed_targets.is_empty() {
            return Err(CapError::Other(
                "membrane must have at least one allowed target".to_string(),
            ));
        }

        if self.max_delegation_depth == 0 {
            return Err(CapError::Other(
                "membrane max_delegation_depth must be > 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Display for MembraneConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MembraneConfig(ops={}, targets={}, depth={}, time_limit={:?})",
            self.allowed_operations.bits(),
            self.allowed_targets.len(),
            self.max_delegation_depth,
            self.time_limit_nanos
        )
    }
}

/// A membrane-wrapped capability.
///
/// Enforces additional constraints at a trust boundary, allowing controlled
/// sharing of capabilities across security domains.
/// See Engineering Plan § 3.2.7: Membrane Pattern & Trust Boundaries.
#[derive(Clone, Debug)]
pub struct MembraneWrappedCapability {
    /// The underlying capability.
    pub capability: Capability,

    /// The membrane configuration enforcing the boundary.
    pub membrane: MembraneConfig,

    /// When the membrane was created.
    pub created_at: Timestamp,

    /// How many times this membrane has been used/crossed.
    pub usage_count: u64,

    /// Whether this membrane is currently active.
    pub is_active: bool,
}

impl MembraneWrappedCapability {
    /// Creates a new membrane-wrapped capability.
    pub fn new(
        capability: Capability,
        membrane: MembraneConfig,
        now: Timestamp,
    ) -> Result<Self, CapError> {
        membrane.validate()?;

        Ok(MembraneWrappedCapability {
            capability,
            membrane,
            created_at: now,
            usage_count: 0,
            is_active: true,
        })
    }

    /// Checks if an agent is allowed to use this capability through the membrane.
    pub fn is_target_allowed(&self, agent: &AgentID) -> bool {
        self.is_active && self.membrane.allowed_targets.contains(agent)
    }

    /// Checks if the operations are allowed through the membrane.
    pub fn are_operations_allowed(&self, ops: OperationSet) -> bool {
        self.is_active && ops.is_subset_of(self.membrane.allowed_operations)
    }

    /// Records a usage of the membrane-wrapped capability.
    pub fn record_usage(&mut self) {
        if self.is_active {
            self.usage_count += 1;
        }
    }

    /// Checks if the membrane has been exceeded in delegation depth.
    pub fn check_delegation_depth(&self) -> Result<(), CapError> {
        if self.capability.chain.len() as u32 >= self.membrane.max_delegation_depth {
            return Err(CapError::DepthExceeded(format!(
                "membrane delegation depth limit {} exceeded",
                self.membrane.max_delegation_depth
            )));
        }
        Ok(())
    }

    /// Checks if the membrane has expired (if time limit is set).
    pub fn check_expiry(&self, now: Timestamp) -> Result<(), CapError> {
        if let Some(limit_nanos) = self.membrane.time_limit_nanos {
            let elapsed = now.nanos().saturating_sub(self.created_at.nanos());
            if elapsed > limit_nanos {
                self.deactivate_internal();
                return Err(CapError::Expired(
                    "membrane wrapper has expired".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Deactivates the membrane (disables all access).
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Internal deactivation for immutable references.
    fn deactivate_internal(&self) {
        // Note: This is a placeholder; in real code with interior mutability,
        // would use Cell<bool> or similar for the is_active field.
    }

    /// Returns the membrane's usage count.
    pub fn get_usage_count(&self) -> u64 {
        self.usage_count
    }
}

impl Display for MembraneWrappedCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MembraneWrappedCap(cap={}, active={}, usage={}, created={})",
            self.capability.id, self.is_active, self.usage_count, self.created_at
        )
    }
}

/// A collection of membrane-wrapped capabilities for a sandbox boundary.
///
/// Provides bulk attenuation and revocation operations for all capabilities
/// in the collection, with transparent enforcement to agents.
/// See Engineering Plan § 3.2.7: Membrane Pattern & Trust Boundaries.
#[derive(Clone, Debug)]
pub struct MembraneSet {
    /// All wrapped capabilities in this membrane set.
    caps: Vec<MembraneWrappedCapability>,

    /// The boundary identifier (e.g., crew name or agent group).
    pub boundary_id: String,

    /// When this membrane set was created.
    pub created_at: Timestamp,

    /// Whether the entire set is currently active.
    pub is_active: bool,
}

impl MembraneSet {
    /// Creates a new membrane set for a boundary.
    pub fn new(boundary_id: String, now: Timestamp) -> Self {
        MembraneSet {
            caps: Vec::new(),
            boundary_id,
            created_at: now,
            is_active: true,
        }
    }

    /// Adds a wrapped capability to the set.
    pub fn add_capability(&mut self, wrapped_cap: MembraneWrappedCapability) {
        if self.is_active {
            self.caps.push(wrapped_cap);
        }
    }

    /// Bulk attenuation: applies a constraint to all capabilities in the set.
    /// This attenuates the operations allowed through all membranes.
    pub fn bulk_attenuate_operations(&mut self, attenuate_to: OperationSet) -> Result<u32, CapError> {
        let mut count = 0;
        for wrapped_cap in self.caps.iter_mut() {
            if wrapped_cap.is_active {
                // Intersect the membrane's allowed operations with the attenuation
                wrapped_cap.membrane.allowed_operations =
                    wrapped_cap.membrane.allowed_operations.intersect(attenuate_to);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Bulk revocation: deactivates all capabilities in the set atomically.
    /// This is an atomic operation: all succeed or all fail together.
    pub fn bulk_revoke(&mut self) -> Result<u32, CapError> {
        if !self.is_active {
            return Err(CapError::RevocationFailed(
                "membrane set is already inactive".to_string(),
            ));
        }

        let mut count = 0;
        for wrapped_cap in self.caps.iter_mut() {
            if wrapped_cap.is_active {
                wrapped_cap.deactivate();
                count += 1;
            }
        }

        self.is_active = false;
        Ok(count)
    }

    /// Checks if all capabilities in the set are still active.
    pub fn all_active(&self) -> bool {
        self.is_active && self.caps.iter().all(|c| c.is_active)
    }

    /// Returns the number of capabilities in the set.
    pub fn len(&self) -> usize {
        self.caps.len()
    }

    /// Returns an immutable view of wrapped capabilities.
    pub fn iter(&self) -> impl Iterator<Item = &MembraneWrappedCapability> {
        self.caps.iter()
    }

    /// Returns a mutable view of wrapped capabilities.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut MembraneWrappedCapability> {
        self.caps.iter_mut()
    }
}

impl Display for MembraneSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MembraneSet(boundary={}, caps={}, active={}, created={})",
            self.boundary_id,
            self.caps.len(),
            self.is_active,
            self.created_at
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ResourceID, ResourceType};
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;

    fn make_test_capability(cap_id_byte: u8, agent: AgentID) -> Capability {
        let mut bytes = [0u8; 32];
        bytes[0] = cap_id_byte;
        Capability::new(
            CapID::from_bytes(bytes),
            agent,
            ResourceType::file(),
            ResourceID::new("test-resource"),
            OperationSet::all(),
            Timestamp::new(1000),
        )
    }

    #[test]
    fn test_membrane_config_creation() {
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a"), AgentID::new("agent-b")];
        let config = MembraneConfig::new(ops, targets, 5);

        assert_eq!(config.allowed_targets.len(), 2);
        assert_eq!(config.max_delegation_depth, 5);
    }

    #[test]
    fn test_membrane_config_with_time_limit() {
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let config = MembraneConfig::new(ops, targets, 5).with_time_limit(1_000_000);

        assert_eq!(config.time_limit_nanos, Some(1_000_000));
    }

    #[test]
    fn test_membrane_config_validates() {
        let ops = OperationSet::read();
        let empty_targets = vec![];

        let config = MembraneConfig::new(ops, empty_targets, 5);
        let result = config.validate();

        assert!(result.is_err());
    }

    #[test]
    fn test_membrane_wrapped_capability_creation() {
        let cap = make_test_capability(1, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let membrane = MembraneConfig::new(ops, targets, 5);
        let now = Timestamp::new(1000);

        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        assert!(wrapped.is_active);
        assert_eq!(wrapped.usage_count, 0);
    }

    #[test]
    fn test_membrane_target_allowed() {
        let cap = make_test_capability(2, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");
        let targets = vec![agent_a.clone()];
        let membrane = MembraneConfig::new(ops, targets, 5);
        let now = Timestamp::new(1000);

        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        assert!(wrapped.is_target_allowed(&agent_a));
        assert!(!wrapped.is_target_allowed(&agent_b));
    }

    #[test]
    fn test_membrane_operations_allowed() {
        let cap = make_test_capability(3, AgentID::new("agent-a"));
        let read_ops = OperationSet::read();
        let agent = AgentID::new("agent-a");
        let targets = vec![agent.clone()];
        let membrane = MembraneConfig::new(read_ops, targets, 5);
        let now = Timestamp::new(1000);

        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        assert!(wrapped.are_operations_allowed(OperationSet::read()));
        assert!(!wrapped.are_operations_allowed(OperationSet::write()));
    }

    #[test]
    fn test_membrane_usage_tracking() {
        let cap = make_test_capability(4, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let membrane = MembraneConfig::new(ops, targets, 5);
        let now = Timestamp::new(1000);

        let mut wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        assert_eq!(wrapped.get_usage_count(), 0);
        wrapped.record_usage();
        assert_eq!(wrapped.get_usage_count(), 1);
        wrapped.record_usage();
        assert_eq!(wrapped.get_usage_count(), 2);
    }

    #[test]
    fn test_membrane_deactivation() {
        let cap = make_test_capability(5, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let agent = AgentID::new("agent-a");
        let targets = vec![agent.clone()];
        let membrane = MembraneConfig::new(ops, targets, 5);
        let now = Timestamp::new(1000);

        let mut wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        assert!(wrapped.is_active);
        wrapped.deactivate();
        assert!(!wrapped.is_active);
        assert!(!wrapped.is_target_allowed(&agent));
    }

    #[test]
    fn test_membrane_set_creation() {
        let now = Timestamp::new(1000);
        let set = MembraneSet::new("boundary-1".to_string(), now);

        assert_eq!(set.boundary_id, "boundary-1");
        assert!(set.is_active);
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_membrane_set_add_capability() {
        let cap = make_test_capability(6, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let membrane = MembraneConfig::new(ops, targets, 5);
        let now = Timestamp::new(1000);

        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        let mut set = MembraneSet::new("boundary-1".to_string(), now);
        set.add_capability(wrapped);

        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_membrane_set_bulk_revoke() {
        let now = Timestamp::new(1000);

        let mut set = MembraneSet::new("boundary-1".to_string(), now);
        for i in 0..3 {
            let cap = make_test_capability(i, AgentID::new("agent-a"));
            let ops = OperationSet::read();
            let targets = vec![AgentID::new("agent-a")];
            let membrane = MembraneConfig::new(ops, targets, 5);
            let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();
            set.add_capability(wrapped);
        }

        assert_eq!(set.len(), 3);
        assert!(set.is_active);

        let revoked_count = set.bulk_revoke().unwrap();

        assert_eq!(revoked_count, 3);
        assert!(!set.is_active);
        assert!(!set.all_active());
    }

    #[test]
    fn test_membrane_set_bulk_attenuation() {
        let now = Timestamp::new(1000);

        let mut set = MembraneSet::new("boundary-1".to_string(), now);
        for i in 0..3 {
            let cap = make_test_capability(i, AgentID::new("agent-a"));
            let ops = OperationSet::all();
            let targets = vec![AgentID::new("agent-a")];
            let membrane = MembraneConfig::new(ops, targets, 5);
            let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();
            set.add_capability(wrapped);
        }

        let attenuated_count = set.bulk_attenuate_operations(OperationSet::read()).unwrap();

        assert_eq!(attenuated_count, 3);
        for cap in set.iter() {
            assert!(cap.are_operations_allowed(OperationSet::read()));
        }
    }

    #[test]
    fn test_membrane_set_all_active() {
        let now = Timestamp::new(1000);
        let mut set = MembraneSet::new("boundary-1".to_string(), now);

        let cap = make_test_capability(7, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let membrane = MembraneConfig::new(ops, targets, 5);
        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();
        set.add_capability(wrapped);

        assert!(set.all_active());

        set.bulk_revoke().unwrap();
        assert!(!set.all_active());
    }

    #[test]
    fn test_membrane_config_display() {
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let config = MembraneConfig::new(ops, targets, 5);

        let display = config.to_string();
        assert!(display.contains("MembraneConfig"));
    }

    #[test]
    fn test_membrane_wrapped_capability_display() {
        let cap = make_test_capability(8, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let membrane = MembraneConfig::new(ops, targets, 5);
        let now = Timestamp::new(1000);

        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();
        let display = wrapped.to_string();

        assert!(display.contains("MembraneWrappedCap"));
    }

    #[test]
    fn test_membrane_set_display() {
        let now = Timestamp::new(1000);
        let set = MembraneSet::new("boundary-1".to_string(), now);

        let display = set.to_string();
        assert!(display.contains("MembraneSet"));
        assert!(display.contains("boundary-1"));
    }

    #[test]
    fn test_membrane_set_bulk_revoke_when_inactive_fails() {
        let now = Timestamp::new(1000);
        let mut set = MembraneSet::new("boundary-1".to_string(), now);
        set.is_active = false;

        let result = set.bulk_revoke();
        assert!(result.is_err());
    }

    #[test]
    fn test_membrane_check_delegation_depth() {
        let cap = make_test_capability(9, AgentID::new("agent-a"));
        let ops = OperationSet::read();
        let targets = vec![AgentID::new("agent-a")];
        let membrane = MembraneConfig::new(ops, targets, 1);
        let now = Timestamp::new(1000);

        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        // Chain length should be < max_delegation_depth
        let result = wrapped.check_delegation_depth();
        assert!(result.is_ok());
    }

    #[test]
    fn test_membrane_wrapped_capability_operations_respected() {
        let cap = make_test_capability(10, AgentID::new("agent-a"));
        let read_write = OperationSet::read() | OperationSet::write();
        let agent = AgentID::new("agent-a");
        let targets = vec![agent.clone()];
        let membrane = MembraneConfig::new(read_write, targets, 5);
        let now = Timestamp::new(1000);

        let wrapped = MembraneWrappedCapability::new(cap, membrane, now).unwrap();

        assert!(wrapped.are_operations_allowed(OperationSet::read()));
        assert!(wrapped.are_operations_allowed(OperationSet::write()));
        assert!(!wrapped.are_operations_allowed(OperationSet::execute()));
    }
}

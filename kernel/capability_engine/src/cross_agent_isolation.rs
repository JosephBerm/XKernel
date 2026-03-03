// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Cross-agent isolation enforcement.
//!
//! This module implements strict isolation between agents:
//! **Agent A cannot access Agent B's unmapped pages.**
//!
//! Mechanisms:
//! 1. **Grant Validation**: When granting a capability, verify the agent owns the resource
//! 2. **Delegate Boundary Enforcement**: Only the capability holder can delegate
//! 3. **Page Table Separation**: Each agent has its own page table
//! 4. **Capability Binding**: Each PTE is bound to the agent who holds the capability
//!
//! See Engineering Plan § 5.0: MMU-backed capability enforcement integration,
//! specifically § 5.6: Cross-Agent Isolation.

use alloc::collections::BTreeMap;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::capability_page_binding::{CapabilityPageBinding, CapabilityPageBindingRegistry};
use crate::error::CapError;
use crate::ids::{CapID, AgentID, ResourceID, ResourceType};

/// A resource owner declaration.
///
/// Asserts that a specific agent owns a specific resource.
/// Used during grant validation to prevent one agent from granting
/// another agent's resources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceOwnership {
    /// The agent that owns the resource.
    pub owner: AgentID,

    /// The resource type.
    pub resource_type: ResourceType,

    /// The resource ID.
    pub resource_id: ResourceID,
}

impl ResourceOwnership {
    /// Creates a new resource ownership declaration.
    pub fn new(owner: AgentID, resource_type: ResourceType, resource_id: ResourceID) -> Self {
        ResourceOwnership {
            owner,
            resource_type,
            resource_id,
        }
    }

    /// Returns a unique key for this ownership.
    pub fn key(&self) -> alloc::string::String {
        format!("{}:{}:{}", self.owner, self.resource_type, self.resource_id)
    }
}

impl Display for ResourceOwnership {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({}, {})", self.owner, self.resource_type, self.resource_id)
    }
}

/// Isolation policy for cross-agent access control.
#[derive(Clone, Debug)]
pub enum IsolationPolicy {
    /// Strict isolation: each agent can only access resources they own.
    Strict,

    /// Shared isolation: agents can grant and delegate capabilities to other agents.
    Shared,

    /// Hierarchical isolation: subordinate agents can access superior agents' resources
    /// (e.g., parent process can access child process memory).
    Hierarchical {
        /// Mapping from agent to its superior (if any).
        superiors: BTreeMap<AgentID, AgentID>,
    },
}

impl Display for IsolationPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IsolationPolicy::Strict => write!(f, "Strict"),
            IsolationPolicy::Shared => write!(f, "Shared"),
            IsolationPolicy::Hierarchical { .. } => write!(f, "Hierarchical"),
        }
    }
}

/// Delegate validation result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DelegateValidationResult {
    /// Delegation is allowed.
    Allowed,

    /// Delegation is denied.
    Denied(alloc::string::String),
}

impl Display for DelegateValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DelegateValidationResult::Allowed => write!(f, "Allowed"),
            DelegateValidationResult::Denied(reason) => write!(f, "Denied({})", reason),
        }
    }
}

/// Cross-agent isolation enforcer.
///
/// Maintains resource ownership declarations and validates:
/// 1. Grants (only agents can grant their own resources)
/// 2. Delegates (only capability holders can delegate)
/// 3. Cross-agent access (prevents one agent from accessing another's memory)
#[derive(Debug)]
pub struct CrossAgentIsolationEnforcer {
    /// Resource ownership declarations.
    /// Key: "agent:type:id", Value: ResourceOwnership
    pub resource_ownerships: BTreeMap<alloc::string::String, ResourceOwnership>,

    /// Current isolation policy.
    pub policy: IsolationPolicy,

    /// Audit log of isolation violations.
    pub violation_log: alloc::vec::Vec<IsolationViolation>,
}

/// Records an isolation violation.
#[derive(Clone, Debug)]
pub struct IsolationViolation {
    /// Type of violation.
    pub violation_type: ViolationType,

    /// Agent that attempted the violation.
    pub agent: AgentID,

    /// The resource or capability involved.
    pub resource: alloc::string::String,

    /// Human-readable reason.
    pub reason: alloc::string::String,
}

impl Display for IsolationViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IsolationViolation({}, agent={}, resource={}, reason={})",
            self.violation_type, self.agent, self.resource, self.reason
        )
    }
}

/// Types of isolation violations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViolationType {
    /// Grant attempted on resource not owned by the granting agent.
    UnauthorizedGrant,

    /// Delegate attempted by non-holder of the capability.
    UnauthorizedDelegate,

    /// Cross-agent access attempted (Agent A accessing Agent B's memory).
    CrossAgentAccess,

    /// Privilege escalation attempt.
    PrivilegeEscalation,
}

impl Display for ViolationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViolationType::UnauthorizedGrant => write!(f, "UnauthorizedGrant"),
            ViolationType::UnauthorizedDelegate => write!(f, "UnauthorizedDelegate"),
            ViolationType::CrossAgentAccess => write!(f, "CrossAgentAccess"),
            ViolationType::PrivilegeEscalation => write!(f, "PrivilegeEscalation"),
        }
    }
}

impl CrossAgentIsolationEnforcer {
    /// Creates a new cross-agent isolation enforcer with the given policy.
    pub fn new(policy: IsolationPolicy) -> Self {
        CrossAgentIsolationEnforcer {
            resource_ownerships: BTreeMap::new(),
            policy,
            violation_log: alloc::vec::Vec::new(),
        }
    }

    /// Creates an enforcer with strict isolation policy.
    pub fn strict() -> Self {
        CrossAgentIsolationEnforcer::new(IsolationPolicy::Strict)
    }

    /// Declares that an agent owns a resource.
    ///
    /// This must be called before the agent can grant capabilities for the resource.
    pub fn declare_ownership(
        &mut self,
        ownership: ResourceOwnership,
    ) -> Result<(), CapError> {
        self.resource_ownerships.insert(ownership.key(), ownership);
        Ok(())
    }

    /// Validates a grant operation.
    ///
    /// Checks:
    /// 1. The agent making the grant owns the resource
    /// 2. The resource type matches the declared ownership
    ///
    /// Returns Ok(()) if the grant is allowed, or an error if denied.
    pub fn validate_grant(
        &mut self,
        granting_agent: &AgentID,
        resource_type: &ResourceType,
        resource_id: &ResourceID,
    ) -> Result<(), CapError> {
        // Look up the resource ownership
        let ownership_key = format!("{}:{}:{}", granting_agent, resource_type, resource_id);

        if !self.resource_ownerships.contains_key(&ownership_key) {
            let violation = IsolationViolation {
                violation_type: ViolationType::UnauthorizedGrant,
                agent: granting_agent.clone(),
                resource: format!("{}:{}", resource_type, resource_id),
                reason: format!(
                    "agent {} does not own resource {}:{}",
                    granting_agent, resource_type, resource_id
                ),
            };
            self.violation_log.push(violation);
            return Err(CapError::Other(
                format!(
                    "grant denied: {} does not own {}:{}",
                    granting_agent, resource_type, resource_id
                )
            ));
        }

        Ok(())
    }

    /// Validates a delegate operation.
    ///
    /// Checks:
    /// 1. The delegating agent holds (or can hold) the capability
    /// 2. The recipient is not attempting a privilege escalation
    ///
    /// Returns Ok(()) if the delegation is allowed, or an error if denied.
    pub fn validate_delegate(
        &mut self,
        delegating_agent: &AgentID,
        recipient_agent: &AgentID,
        capability: &Capability,
    ) -> Result<(), CapError> {
        // Check that delegating agent is the current holder
        if capability.target_agent != *delegating_agent {
            let violation = IsolationViolation {
                violation_type: ViolationType::UnauthorizedDelegate,
                agent: delegating_agent.clone(),
                resource: format!("{}", capability.id),
                reason: format!(
                    "agent {} is not the holder of capability {}",
                    delegating_agent, capability.id
                ),
            };
            self.violation_log.push(violation);
            return Err(CapError::Other(
                format!(
                    "delegate denied: {} is not the holder of capability {}",
                    delegating_agent, capability.id
                )
            ));
        }

        // Policy-specific checks
        match &self.policy {
            IsolationPolicy::Strict => {
                // No delegation allowed across agent boundaries
                if delegating_agent != recipient_agent {
                    let violation = IsolationViolation {
                        violation_type: ViolationType::UnauthorizedDelegate,
                        agent: delegating_agent.clone(),
                        resource: format!("{}", capability.id),
                        reason: "strict isolation policy forbids cross-agent delegation"
                            .to_string(),
                    };
                    self.violation_log.push(violation);
                    return Err(CapError::Other(
                        "strict isolation policy forbids cross-agent delegation".to_string()
                    ));
                }
            }
            IsolationPolicy::Shared => {
                // Any delegation is allowed
            }
            IsolationPolicy::Hierarchical { superiors } => {
                // Only allow delegation to subordinates or self
                if delegating_agent != recipient_agent {
                    if let Some(superior) = superiors.get(recipient_agent) {
                        if superior != delegating_agent {
                            let violation = IsolationViolation {
                                violation_type: ViolationType::PrivilegeEscalation,
                                agent: delegating_agent.clone(),
                                resource: format!("{}", capability.id),
                                reason: "hierarchical isolation forbids delegation to non-subordinates"
                                    .to_string(),
                            };
                            self.violation_log.push(violation);
                            return Err(CapError::Other(
                                "cannot delegate to non-subordinate".to_string()
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Checks if an agent can access a binding.
    ///
    /// Returns Ok(()) if the access is allowed, or an error if denied.
    pub fn validate_access(
        &mut self,
        accessing_agent: &AgentID,
        binding: &CapabilityPageBinding,
    ) -> Result<(), CapError> {
        // The accessing agent must be the owner of the binding
        if binding.page_table_entry.owner_agent != *accessing_agent {
            let violation = IsolationViolation {
                violation_type: ViolationType::CrossAgentAccess,
                agent: accessing_agent.clone(),
                resource: format!("0x{:x}", binding.page_table_entry.virtual_address),
                reason: format!(
                    "agent {} cannot access memory owned by {}",
                    accessing_agent, binding.page_table_entry.owner_agent
                ),
            };
            self.violation_log.push(violation);
            return Err(CapError::Other(
                format!(
                    "cross-agent access denied: {} cannot access {}'s memory",
                    accessing_agent, binding.page_table_entry.owner_agent
                )
            ));
        }

        Ok(())
    }

    /// Returns the number of violations recorded.
    pub fn violation_count(&self) -> usize {
        self.violation_log.len()
    }

    /// Returns the number of violations of a specific type.
    pub fn violation_count_by_type(&self, vtype: ViolationType) -> usize {
        self.violation_log
            .iter()
            .filter(|v| v.violation_type == vtype)
            .count()
    }

    /// Clears the violation log.
    pub fn clear_violations(&mut self) {
        self.violation_log.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_ownership_key() {
        let ownership = ResourceOwnership::new(
            AgentID::new("agent-a"),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
        );

        let key = ownership.key();
        assert!(key.contains("agent-a"));
        assert!(key.contains("memory"));
        assert!(key.contains("mem:0x1000"));
    }

    #[test]
    fn test_isolation_enforcer_creation() {
        let enforcer = CrossAgentIsolationEnforcer::strict();
        assert_eq!(enforcer.violation_count(), 0);
    }

    #[test]
    fn test_declare_ownership() {
        let mut enforcer = CrossAgentIsolationEnforcer::strict();
        let ownership = ResourceOwnership::new(
            AgentID::new("agent-a"),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
        );

        let result = enforcer.declare_ownership(ownership);
        assert!(result.is_ok());
        assert_eq!(enforcer.resource_ownerships.len(), 1);
    }

    #[test]
    fn test_validate_grant_authorized() {
        let mut enforcer = CrossAgentIsolationEnforcer::strict();
        let agent = AgentID::new("agent-a");
        let ownership = ResourceOwnership::new(
            agent.clone(),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
        );
        enforcer.declare_ownership(ownership).unwrap();

        let result = enforcer.validate_grant(
            &agent,
            &ResourceType::memory(),
            &ResourceID::new("mem:0x1000"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_grant_unauthorized() {
        let mut enforcer = CrossAgentIsolationEnforcer::strict();

        let result = enforcer.validate_grant(
            &AgentID::new("agent-a"),
            &ResourceType::memory(),
            &ResourceID::new("mem:0x1000"),
        );
        assert!(result.is_err());
        assert_eq!(enforcer.violation_count(), 1);
        assert_eq!(
            enforcer.violation_count_by_type(ViolationType::UnauthorizedGrant),
            1
        );
    }

    #[test]
    fn test_strict_isolation_no_cross_agent_delegate() {
        use crate::capability::Capability;
        use crate::constraints::Timestamp;
        use crate::ids::CapID;
        use crate::operations::OperationSet;

        let mut enforcer = CrossAgentIsolationEnforcer::strict();
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");

        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            agent_a.clone(),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::all(),
            Timestamp::now_nanos(),
        );

        let result = enforcer.validate_delegate(&agent_a, &agent_b, &cap);
        assert!(result.is_err());
        assert_eq!(enforcer.violation_count(), 1);
    }

    #[test]
    fn test_shared_isolation_allows_delegate() {
        use crate::capability::Capability;
        use crate::constraints::Timestamp;
        use crate::ids::CapID;
        use crate::operations::OperationSet;

        let mut enforcer = CrossAgentIsolationEnforcer::new(IsolationPolicy::Shared);
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");

        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            agent_a.clone(),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::all(),
            Timestamp::now_nanos(),
        );

        let result = enforcer.validate_delegate(&agent_a, &agent_b, &cap);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_access_same_agent() {
        use crate::capability::Capability;
        use crate::capability_page_binding::CapabilityPageBinding;
        use crate::constraints::Timestamp;
        use crate::ids::CapID;
        use crate::operations::OperationSet;

        let mut enforcer = CrossAgentIsolationEnforcer::strict();
        let agent = AgentID::new("agent-a");

        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            agent.clone(),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::all(),
            Timestamp::now_nanos(),
        );

        let binding = CapabilityPageBinding::new(
            cap,
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        let result = enforcer.validate_access(&agent, &binding);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_access_different_agent() {
        use crate::capability::Capability;
        use crate::capability_page_binding::CapabilityPageBinding;
        use crate::constraints::Timestamp;
        use crate::ids::CapID;
        use crate::operations::OperationSet;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

        let mut enforcer = CrossAgentIsolationEnforcer::strict();
        let agent_a = AgentID::new("agent-a");
        let agent_b = AgentID::new("agent-b");

        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            agent_a.clone(),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::all(),
            Timestamp::now_nanos(),
        );

        let binding = CapabilityPageBinding::new(
            cap,
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        let result = enforcer.validate_access(&agent_b, &binding);
        assert!(result.is_err());
        assert_eq!(enforcer.violation_count(), 1);
        assert_eq!(
            enforcer.violation_count_by_type(ViolationType::CrossAgentAccess),
            1
        );
    }
}

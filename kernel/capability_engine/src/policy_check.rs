// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Policy check: validation of capabilities against mandatory policies before page mapping.
//!
//! This module implements pre-mapping validation that ensures all capabilities
//! comply with mandatory policies before being granted page table access.
//! See Engineering Plan § 3.2.3: Grant Operations and § 3.1.4: Mandatory & Stateless.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::error::CapError;
use crate::ids::{AgentID, PolicyID};
use crate::mandatory_policy::{EnforcementMode, MandatoryCapabilityPolicy};

/// Result of a policy check.
/// See Engineering Plan § 3.2.3: Grant Operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyCheckResult {
    /// All policies approved the capability.
    Approved,

    /// A policy denied the capability.
    Denied {
        policy_id: PolicyID,
        reason: String,
    },

    /// Policies audited the capability (approved but logged).
    Audited {
        policy_ids: Vec<PolicyID>,
        message: String,
    },

    /// Policies warned about the capability (approved with notification).
    Warning {
        policy_ids: Vec<PolicyID>,
        message: String,
    },
}

impl Display for PolicyCheckResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyCheckResult::Approved => write!(f, "Approved"),
            PolicyCheckResult::Denied { policy_id, reason } => {
                write!(f, "Denied(policy={}, reason={})", policy_id, reason)
            }
            PolicyCheckResult::Audited { policy_ids, message } => {
                write!(f, "Audited({} policies, msg={})", policy_ids.len(), message)
            }
            PolicyCheckResult::Warning { policy_ids, message } => {
                write!(f, "Warning({} policies, msg={})", policy_ids.len(), message)
            }
        }
    }
}

impl PolicyCheckResult {
    /// Returns true if the check allows the capability.
    pub fn allows(&self) -> bool {
        !matches!(self, PolicyCheckResult::Denied { .. })
    }

    /// Returns true if the check denies the capability.
    pub fn denies(&self) -> bool {
        matches!(self, PolicyCheckResult::Denied { .. })
    }

    /// Returns true if the check requires auditing.
    pub fn requires_audit(&self) -> bool {
        matches!(self, PolicyCheckResult::Audited { .. })
    }

    /// Returns true if the check generated a warning.
    pub fn generates_warning(&self) -> bool {
        matches!(self, PolicyCheckResult::Warning { .. })
    }
}

/// Policy check context for mapping validation.
/// See Engineering Plan § 3.2.3: Grant Operations.
#[derive(Clone, Debug)]
pub struct PolicyCheckContext {
    /// Current timestamp (nanoseconds since epoch).
    pub now_nanos: u64,

    /// The agent requesting the capability grant.
    pub requesting_agent: AgentID,

    /// The agent receiving the capability.
    pub target_agent: AgentID,

    /// Whether this is a kernel operation (has elevated privilege).
    pub is_kernel_operation: bool,
}

impl PolicyCheckContext {
    /// Creates a new policy check context.
    pub fn new(
        now_nanos: u64,
        requesting_agent: AgentID,
        target_agent: AgentID,
        is_kernel: bool,
    ) -> Self {
        PolicyCheckContext {
            now_nanos,
            requesting_agent,
            target_agent,
            is_kernel_operation: is_kernel,
        }
    }
}

/// Policy check engine for pre-mapping validation.
///
/// Per Engineering Plan § 3.2.3, validates capabilities against all mandatory
/// policies before granting page table access. Three enforcement modes:
/// - Deny: block immediately on violation
/// - Audit: allow but log for compliance
/// - Warn: allow with notification
///
/// Latency target: <100ns amortized with caching.
/// See Engineering Plan § 3.2.3: Grant Operations.
pub fn check_capability_against_policies(
    capability: &Capability,
    policies: &[&MandatoryCapabilityPolicy],
    context: &PolicyCheckContext,
) -> PolicyCheckResult {
    let mut denied_results = Vec::new();
    let mut audit_policies = Vec::new();
    let mut warn_policies = Vec::new();

    for policy in policies {
        // Check if policy applies to this capability
        if !policy_applies(policy, capability, context) {
            continue;
        }

        // Evaluate the policy
        match evaluate_policy(policy, capability, context) {
            EvaluationOutcome::Violates => {
                match policy.enforcement_mode {
                    EnforcementMode::Deny => {
                        // Stop immediately on deny
                        denied_results.push((policy.id.clone(), "policy violation detected".to_string()));
                    }
                    EnforcementMode::Audit => {
                        // Collect for audit report
                        audit_policies.push(policy.id.clone());
                    }
                    EnforcementMode::Warn => {
                        // Collect for warning
                        warn_policies.push(policy.id.clone());
                    }
                }
            }
            EvaluationOutcome::Allowed => {
                // Policy is satisfied, continue
            }
            EvaluationOutcome::Exception => {
                // Exception applies, skip this policy
            }
        }
    }

    // Step 1: Return immediately if any policy denied
    if !denied_results.is_empty() {
        let (policy_id, reason) = denied_results.into_iter().next().unwrap();
        return PolicyCheckResult::Denied {
            policy_id,
            reason,
        };
    }

    // Step 2: Return audit if any policy requires it
    if !audit_policies.is_empty() {
        return PolicyCheckResult::Audited {
            policy_ids: audit_policies,
            message: "capability usage is audited".to_string(),
        };
    }

    // Step 3: Return warning if any policy generated one
    if !warn_policies.is_empty() {
        return PolicyCheckResult::Warning {
            policy_ids: warn_policies,
            message: "capability usage is subject to policy constraints".to_string(),
        };
    }

    // Step 4: All policies are satisfied
    PolicyCheckResult::Approved
}

/// Evaluation outcome of a single policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EvaluationOutcome {
    /// Capability violates the policy.
    Violates,

    /// Capability is allowed by the policy.
    Allowed,

    /// An exception applies to this capability.
    Exception,
}

/// Checks if a policy applies to a capability.
/// A policy applies if:
/// 1. The policy's scope matches the requesting/target agents, AND
/// 2. The policy's rule matches the capability's resource type or operations
fn policy_applies(
    policy: &MandatoryCapabilityPolicy,
    capability: &Capability,
    context: &PolicyCheckContext,
) -> bool {
    // Check scope
    match &policy.scope {
        crate::mandatory_policy::PolicyScope::SystemWide => {
            // System-wide policies always apply
        }
        crate::mandatory_policy::PolicyScope::AgentScoped(agent) => {
            // Agent-scoped policies apply only to that agent
            if context.target_agent != *agent && context.requesting_agent != *agent {
                return false;
            }
        }
        crate::mandatory_policy::PolicyScope::CrewScoped(_crew) => {
            // Crew-scoped policies would check crew membership
            // Placeholder: assumes crew membership validation elsewhere
        }
    }

    // Policy applies
    true
}

/// Evaluates a policy against a capability.
/// Returns whether the capability violates, is allowed by, or is exempted from the policy.
fn evaluate_policy(
    policy: &MandatoryCapabilityPolicy,
    capability: &Capability,
    _context: &PolicyCheckContext,
) -> EvaluationOutcome {
    // Check exceptions first
    let resource_pattern = format!(
        "{}:{}",
        capability.resource_type, capability.resource_id
    );

    for exception in policy.exceptions.iter() {
        if exception.matches_pattern(&resource_pattern) {
            return EvaluationOutcome::Exception;
        }
    }

    // Check policy rule
    // In a full implementation, would evaluate the predicate composition
    // For now, placeholder: assume policy rule is satisfied if not exempted
    EvaluationOutcome::Allowed
}

/// Validates a single capability against a set of policies.
/// Returns the first policy violation if any.
pub fn validate_single_capability(
    capability: &Capability,
    policies: &[&MandatoryCapabilityPolicy],
) -> Result<(), CapError> {
    let context = PolicyCheckContext::new(
        0, // Will be filled by caller
        AgentID::new("kernel"),
        capability.target_agent.clone(),
        true,
    );

    match check_capability_against_policies(capability, policies, &context) {
        PolicyCheckResult::Denied { policy_id, reason } => {
            Err(CapError::PolicyDenied(format!(
                "policy {} denied capability: {}",
                policy_id, reason
            )))
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ResourceID, ResourceType};
    use crate::operations::OperationSet;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_policy_check_result_approved() {
        let result = PolicyCheckResult::Approved;
        assert!(result.allows());
        assert!(!result.denies());
        assert!(!result.requires_audit());
        assert!(!result.generates_warning());
    }

    #[test]
    fn test_policy_check_result_denied() {
        let result = PolicyCheckResult::Denied {
            policy_id: PolicyID::new("policy-1"),
            reason: "violation".to_string(),
        };
        assert!(!result.allows());
        assert!(result.denies());
    }

    #[test]
    fn test_policy_check_result_audited() {
        let result = PolicyCheckResult::Audited {
            policy_ids: vec![PolicyID::new("policy-1")],
            message: "audit".to_string(),
        };
        assert!(result.allows());
        assert!(result.requires_audit());
        assert!(!result.denies());
    }

    #[test]
    fn test_policy_check_result_warning() {
        let result = PolicyCheckResult::Warning {
            policy_ids: vec![PolicyID::new("policy-1")],
            message: "warning".to_string(),
        };
        assert!(result.allows());
        assert!(result.generates_warning());
        assert!(!result.denies());
    }

    #[test]
    fn test_policy_check_context_creation() {
        let context = PolicyCheckContext::new(
            1000,
            AgentID::new("agent-a"),
            AgentID::new("agent-b"),
            true,
        );

        assert_eq!(context.now_nanos, 1000);
        assert!(context.is_kernel_operation);
    }

    #[test]
    fn test_check_capability_empty_policies() {
        let mut bytes = [0u8; 32];
        bytes[0] = 1;
        let cap = Capability::new(
            CapID::from_bytes(bytes),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("resource-1"),
            OperationSet::all(),
            crate::constraints::Timestamp::new(1000),
        );

        let context = PolicyCheckContext::new(
            1000,
            AgentID::new("kernel"),
            AgentID::new("agent-a"),
            true,
        );

        let result = check_capability_against_policies(&cap, &[], &context);
        assert!(result.allows());
        assert_eq!(result, PolicyCheckResult::Approved);
    }

    #[test]
    fn test_policy_check_result_display() {
        let approved = PolicyCheckResult::Approved;
        assert!(approved.to_string().contains("Approved"));

        let denied = PolicyCheckResult::Denied {
            policy_id: PolicyID::new("policy-1"),
            reason: "test".to_string(),
        };
        assert!(denied.to_string().contains("Denied"));

        let audited = PolicyCheckResult::Audited {
            policy_ids: vec![PolicyID::new("policy-1")],
            message: "test".to_string(),
        };
        assert!(audited.to_string().contains("Audited"));

        let warning = PolicyCheckResult::Warning {
            policy_ids: vec![PolicyID::new("policy-1")],
            message: "test".to_string(),
        };
        assert!(warning.to_string().contains("Warning"));
    }

    #[test]
    fn test_validate_single_capability_success() {
        let mut bytes = [0u8; 32];
        bytes[0] = 2;
        let cap = Capability::new(
            CapID::from_bytes(bytes),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("resource-1"),
            OperationSet::all(),
            crate::constraints::Timestamp::new(1000),
        );

        let result = validate_single_capability(&cap, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_check_result_multiple_policies() {
        let mut policies = vec![
            PolicyID::new("policy-1"),
            PolicyID::new("policy-2"),
            PolicyID::new("policy-3"),
        ];

        let result = PolicyCheckResult::Audited {
            policy_ids: policies.clone(),
            message: "multiple policies".to_string(),
        };

        assert_eq!(
            result,
            PolicyCheckResult::Audited {
                policy_ids,
                message: "multiple policies".to_string()
            }
        );
    }

    #[test]
    fn test_policy_check_context_kernel_operation() {
        let context_kernel = PolicyCheckContext::new(
            1000,
            AgentID::new("kernel"),
            AgentID::new("agent-a"),
            true,
        );
        assert!(context_kernel.is_kernel_operation);

        let context_user = PolicyCheckContext::new(
            1000,
            AgentID::new("agent-b"),
            AgentID::new("agent-a"),
            false,
        );
        assert!(!context_user.is_kernel_operation);
    }

    #[test]
    fn test_policy_check_different_agents() {
        let mut bytes = [0u8; 32];
        bytes[0] = 3;
        let cap = Capability::new(
            CapID::from_bytes(bytes),
            AgentID::new("target-agent"),
            ResourceType::file(),
            ResourceID::new("resource-1"),
            OperationSet::all(),
            crate::constraints::Timestamp::new(1000),
        );

        let context = PolicyCheckContext::new(
            1000,
            AgentID::new("requesting-agent"),
            AgentID::new("target-agent"),
            false,
        );

        let result = check_capability_against_policies(&cap, &[], &context);
        assert!(result.allows());
    }
}

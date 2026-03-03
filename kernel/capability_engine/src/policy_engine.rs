// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Policy enforcement engine for mandatory capability policies.
//!
//! This module implements policy evaluation, enforcement ordering, and decision making.
//! See Engineering Plan § 3.1.4: Mandatory & Stateless and Addendum v2.5.1: CPL Integration.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::error::CapError;
use crate::ids::AgentID;
use crate::mandatory_policy::{EnforcementMode, MandatoryCapabilityPolicy, PolicyScope};

/// A policy decision resulting from evaluation.
///
/// Determines whether an operation is allowed, denied, audited, or warned.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyDecision {
    /// The operation is allowed.
    Allow,

    /// The operation is denied with a reason.
    Deny(String),

    /// The operation proceeds but should be audited with an audit record.
    Audit(String),

    /// The operation proceeds but a warning is issued.
    Warn(String),
}

impl Display for PolicyDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyDecision::Allow => write!(f, "Allow"),
            PolicyDecision::Deny(reason) => write!(f, "Deny({})", reason),
            PolicyDecision::Audit(record) => write!(f, "Audit({})", record),
            PolicyDecision::Warn(msg) => write!(f, "Warn({})", msg),
        }
    }
}

impl PolicyDecision {
    /// Returns true if the decision allows the operation.
    pub fn allows_operation(&self) -> bool {
        matches!(self, PolicyDecision::Allow)
    }

    /// Returns true if the decision denies the operation.
    pub fn denies_operation(&self) -> bool {
        matches!(self, PolicyDecision::Deny(_))
    }

    /// Returns true if the decision requires audit logging.
    pub fn requires_audit(&self) -> bool {
        matches!(self, PolicyDecision::Audit(_))
    }

    /// Returns true if the decision generates a warning.
    pub fn generates_warning(&self) -> bool {
        matches!(self, PolicyDecision::Warn(_))
    }
}

/// Context information for policy evaluation.
///
/// Provides runtime context such as the current timestamp, requesting agent,
/// and target agent for policy decisions.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
#[derive(Clone, Debug)]
pub struct EvalContext {
    /// Current time in nanoseconds since epoch.
    pub now_nanos: u64,

    /// The agent requesting the operation (may differ from capability holder).
    pub requesting_agent: AgentID,

    /// The target agent affected by the operation.
    pub target_agent: AgentID,

    /// Operation being attempted (e.g., "grant", "delegate", "revoke", "use").
    pub operation: String,
}

impl EvalContext {
    /// Creates a new evaluation context.
    pub fn new(
        now_nanos: u64,
        requesting_agent: AgentID,
        target_agent: AgentID,
        operation: impl Into<String>,
    ) -> Self {
        EvalContext {
            now_nanos,
            requesting_agent,
            target_agent,
            operation: operation.into(),
        }
    }
}

/// A trait for policy evaluation engines.
///
/// Implementations evaluate capabilities against mandatory policies
/// and produce policy decisions.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
pub trait PolicyEvaluator: Debug {
    /// Evaluates a capability against policies.
    ///
    /// Returns a PolicyDecision indicating whether the operation is allowed,
    /// denied, audited, or warned.
    fn evaluate(
        &self,
        capability: &Capability,
        context: &EvalContext,
    ) -> PolicyDecision;

    /// Evaluates a delegation operation against policies.
    fn evaluate_delegation(
        &self,
        capability: &Capability,
        delegatee: &AgentID,
        context: &EvalContext,
    ) -> PolicyDecision {
        // Default: delegate to main evaluate
        self.evaluate(capability, context)
    }

    /// Evaluates a revocation operation against policies.
    fn evaluate_revocation(
        &self,
        capability: &Capability,
        revoker: &AgentID,
        context: &EvalContext,
    ) -> PolicyDecision {
        // Default: delegate to main evaluate
        self.evaluate(capability, context)
    }
}

/// Policy enforcement engine implementing mandatory capability policies.
///
/// Manages a set of policies and evaluates capabilities against them.
/// Policies are checked in order: Deny first, then Audit, then Warn.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
#[derive(Clone, Debug)]
pub struct MandatoryPolicyEngine {
    /// Deny policies (checked first).
    deny_policies: Vec<MandatoryCapabilityPolicy>,

    /// Audit policies (checked second).
    audit_policies: Vec<MandatoryCapabilityPolicy>,

    /// Warn policies (checked last).
    warn_policies: Vec<MandatoryCapabilityPolicy>,
}

impl MandatoryPolicyEngine {
    /// Creates a new empty mandatory policy engine.
    pub fn new() -> Self {
        MandatoryPolicyEngine {
            deny_policies: Vec::new(),
            audit_policies: Vec::new(),
            warn_policies: Vec::new(),
        }
    }

    /// Adds a policy to the engine.
    pub fn add_policy(&mut self, policy: MandatoryCapabilityPolicy) -> Result<(), CapError> {
        // Validate the policy
        policy.validate()?;

        match policy.enforcement {
            EnforcementMode::Deny => self.deny_policies.push(policy),
            EnforcementMode::Audit => self.audit_policies.push(policy),
            EnforcementMode::Warn => self.warn_policies.push(policy),
        }

        Ok(())
    }

    /// Returns the number of policies in the engine.
    pub fn policy_count(&self) -> usize {
        self.deny_policies.len() + self.audit_policies.len() + self.warn_policies.len()
    }

    /// Checks if a scope matches the evaluation context.
    fn scope_matches(&self, scope: &PolicyScope, context: &EvalContext) -> bool {
        match scope {
            PolicyScope::SystemWide => true,
            PolicyScope::AgentScoped(agent) => {
                agent == &context.target_agent || agent == &context.requesting_agent
            }
            PolicyScope::CrewScoped(_crew) => {
                // Crew matching would require crew context information
                // For now, we return true (can be extended with crew context)
                true
            }
        }
    }

    /// Evaluates capability against deny policies.
    fn evaluate_deny_policies(
        &self,
        capability: &Capability,
        context: &EvalContext,
    ) -> PolicyDecision {
        for policy in &self.deny_policies {
            // Check if policy scope matches
            if !self.scope_matches(&policy.scope, context) {
                continue;
            }

            // Check if capability is exempted
            let cap_pattern = alloc::format!(
                "{}:{}:{}",
                capability.target_resource_type,
                capability.target_resource_id,
                capability.target_agent
            );

            if policy.is_exempted(&cap_pattern) {
                continue;
            }

            // Policy matched and not exempted: deny
            return PolicyDecision::Deny(alloc::format!(
                "policy {} denied operation: {}",
                policy.id, context.operation
            ));
        }

        PolicyDecision::Allow
    }

    /// Evaluates capability against audit policies.
    fn evaluate_audit_policies(
        &self,
        capability: &Capability,
        context: &EvalContext,
    ) -> PolicyDecision {
        for policy in &self.audit_policies {
            // Check if policy scope matches
            if !self.scope_matches(&policy.scope, context) {
                continue;
            }

            // Check if capability is exempted
            let cap_pattern = alloc::format!(
                "{}:{}:{}",
                capability.target_resource_type,
                capability.target_resource_id,
                capability.target_agent
            );

            if policy.is_exempted(&cap_pattern) {
                continue;
            }

            // Policy matched and not exempted: audit
            let audit_record = alloc::format!(
                "policy={}, agent={}, op={}, resource={}, timestamp={}",
                policy.id,
                context.requesting_agent,
                context.operation,
                capability.target_resource_id,
                context.now_nanos
            );

            return PolicyDecision::Audit(audit_record);
        }

        PolicyDecision::Allow
    }

    /// Evaluates capability against warn policies.
    fn evaluate_warn_policies(
        &self,
        capability: &Capability,
        context: &EvalContext,
    ) -> PolicyDecision {
        for policy in &self.warn_policies {
            // Check if policy scope matches
            if !self.scope_matches(&policy.scope, context) {
                continue;
            }

            // Check if capability is exempted
            let cap_pattern = alloc::format!(
                "{}:{}:{}",
                capability.target_resource_type,
                capability.target_resource_id,
                capability.target_agent
            );

            if policy.is_exempted(&cap_pattern) {
                continue;
            }

            // Policy matched and not exempted: warn
            let warning = alloc::format!(
                "warning: policy {} may be violated by operation {}",
                policy.id, context.operation
            );

            return PolicyDecision::Warn(warning);
        }

        PolicyDecision::Allow
    }
}

impl Default for MandatoryPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEvaluator for MandatoryPolicyEngine {
    fn evaluate(
        &self,
        capability: &Capability,
        context: &EvalContext,
    ) -> PolicyDecision {
        // Check deny policies first
        let deny_decision = self.evaluate_deny_policies(capability, context);
        if !deny_decision.allows_operation() {
            return deny_decision;
        }

        // Check audit policies second
        let audit_decision = self.evaluate_audit_policies(capability, context);
        if audit_decision.requires_audit() {
            return audit_decision;
        }

        // Check warn policies last
        self.evaluate_warn_policies(capability, context)
    }

    fn evaluate_delegation(
        &self,
        capability: &Capability,
        delegatee: &AgentID,
        context: &EvalContext,
    ) -> PolicyDecision {
        // For delegation, we evaluate with the delegatee as the target
        let mut delegation_context = context.clone();
        delegation_context.target_agent = delegatee.clone();
        delegation_context.operation = "delegate".to_string();

        self.evaluate(capability, &delegation_context)
    }

    fn evaluate_revocation(
        &self,
        capability: &Capability,
        revoker: &AgentID,
        context: &EvalContext,
    ) -> PolicyDecision {
        // For revocation, we evaluate with the revoker as the requesting agent
        let mut revocation_context = context.clone();
        revocation_context.requesting_agent = revoker.clone();
        revocation_context.operation = "revoke".to_string();

        self.evaluate(capability, &revocation_context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::constraints::Timestamp;
    use crate::ids::{CapID, ResourceID, ResourceType};
    use crate::mandatory_policy::{PolicyRule, PredicateComposition};
    use crate::operations::OperationSet;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

    fn create_test_cap() -> Capability {
        Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("agent-a"),
            ResourceType::file(),
            ResourceID::new("file-001"),
            OperationSet::all(),
            Timestamp::new(1000),
        )
    }

    fn create_test_context() -> EvalContext {
        EvalContext::new(
            5000,
            AgentID::new("requester"),
            AgentID::new("agent-a"),
            "use",
        )
    }

    #[test]
    fn test_policy_decision_allow() {
        let decision = PolicyDecision::Allow;
        assert!(decision.allows_operation());
        assert!(!decision.denies_operation());
        assert!(!decision.requires_audit());
        assert!(!decision.generates_warning());
    }

    #[test]
    fn test_policy_decision_deny() {
        let decision = PolicyDecision::Deny("access denied".to_string());
        assert!(!decision.allows_operation());
        assert!(decision.denies_operation());
        assert!(!decision.requires_audit());
        assert!(!decision.generates_warning());
    }

    #[test]
    fn test_policy_decision_audit() {
        let decision = PolicyDecision::Audit("audit record".to_string());
        assert!(!decision.allows_operation());
        assert!(!decision.denies_operation());
        assert!(decision.requires_audit());
        assert!(!decision.generates_warning());
    }

    #[test]
    fn test_policy_decision_warn() {
        let decision = PolicyDecision::Warn("warning message".to_string());
        assert!(!decision.allows_operation());
        assert!(!decision.denies_operation());
        assert!(!decision.requires_audit());
        assert!(decision.generates_warning());
    }

    #[test]
    fn test_policy_decision_display() {
        assert_eq!(PolicyDecision::Allow.to_string(), "Allow");
        assert!(PolicyDecision::Deny("reason".to_string()).to_string().contains("Deny"));
        assert!(PolicyDecision::Audit("record".to_string()).to_string().contains("Audit"));
        assert!(PolicyDecision::Warn("msg".to_string()).to_string().contains("Warn"));
    }

    #[test]
    fn test_eval_context_creation() {
        let ctx = EvalContext::new(
            1000,
            AgentID::new("requester"),
            AgentID::new("target"),
            "delegate",
        );
        assert_eq!(ctx.now_nanos, 1000);
        assert_eq!(ctx.operation, "delegate");
    }

    #[test]
    fn test_mandatory_policy_engine_creation() {
        let engine = MandatoryPolicyEngine::new();
        assert_eq!(engine.policy_count(), 0);
    }

    #[test]
    fn test_mandatory_policy_engine_add_policy() {
        let mut engine = MandatoryPolicyEngine::new();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        assert!(engine.add_policy(policy).is_ok());
        assert_eq!(engine.policy_count(), 1);
    }

    #[test]
    fn test_mandatory_policy_engine_deny_policy() {
        let mut engine = MandatoryPolicyEngine::new();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("deny-file-write"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        engine.add_policy(policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        assert!(decision.denies_operation());
    }

    #[test]
    fn test_mandatory_policy_engine_allow_unmatched() {
        let mut engine = MandatoryPolicyEngine::new();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("deny-memory-write"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        engine.add_policy(policy).unwrap();

        let cap = create_test_cap(); // File resource, not memory
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        assert!(decision.allows_operation());
    }

    #[test]
    fn test_mandatory_policy_engine_audit_policy() {
        let mut engine = MandatoryPolicyEngine::new();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::read());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("audit-file-read"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Audit,
            1000,
            1_000_000_000,
        );

        engine.add_policy(policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        assert!(decision.requires_audit());
    }

    #[test]
    fn test_mandatory_policy_engine_warn_policy() {
        let mut engine = MandatoryPolicyEngine::new();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::all());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("warn-file-access"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Warn,
            1000,
            1_000_000_000,
        );

        engine.add_policy(policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        assert!(decision.generates_warning());
    }

    #[test]
    fn test_mandatory_policy_engine_deny_precedence() {
        let mut engine = MandatoryPolicyEngine::new();

        // Add a deny policy
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let deny_policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("deny-policy"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        // Add an audit policy for the same resource
        let target2 = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule2 = PolicyRule::new(target2, OperationSet::write());
        let audit_policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("audit-policy"),
            rule2,
            PolicyScope::SystemWide,
            EnforcementMode::Audit,
            1000,
            1_000_000_000,
        );

        engine.add_policy(deny_policy).unwrap();
        engine.add_policy(audit_policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        // Deny should take precedence over Audit
        assert!(decision.denies_operation());
    }

    #[test]
    fn test_mandatory_policy_engine_audit_precedence() {
        let mut engine = MandatoryPolicyEngine::new();

        // Add only an audit policy
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let audit_policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("audit-policy"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Audit,
            1000,
            1_000_000_000,
        );

        // Add a warn policy for the same resource
        let target2 = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule2 = PolicyRule::new(target2, OperationSet::write());
        let warn_policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("warn-policy"),
            rule2,
            PolicyScope::SystemWide,
            EnforcementMode::Warn,
            1000,
            1_000_000_000,
        );

        engine.add_policy(audit_policy).unwrap();
        engine.add_policy(warn_policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        // Audit should take precedence over Warn
        assert!(decision.requires_audit());
    }

    #[test]
    fn test_mandatory_policy_engine_exception_exemption() {
        let mut engine = MandatoryPolicyEngine::new();

        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let mut policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("deny-file-write"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        // Add an exception for logs
        policy.add_exception("file:*:*");

        engine.add_policy(policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        // Should be allowed due to exception
        assert!(decision.allows_operation());
    }

    #[test]
    fn test_mandatory_policy_engine_agent_scoped() {
        let mut engine = MandatoryPolicyEngine::new();

        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("deny-agent-a"),
            rule,
            PolicyScope::AgentScoped(AgentID::new("agent-a")),
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        engine.add_policy(policy).unwrap();

        // Test with matching agent
        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        assert!(decision.denies_operation());

        // Test with different agent
        let mut cap2 = create_test_cap();
        cap2.target_agent = AgentID::new("agent-b");
        let mut ctx2 = create_test_context();
        ctx2.target_agent = AgentID::new("agent-b");

        let decision2 = engine.evaluate(&cap2, &ctx2);
        // Policy doesn't match agent-b, so should allow
        assert!(decision2.allows_operation());
    }

    #[test]
    fn test_mandatory_policy_engine_delegation() {
        let mut engine = MandatoryPolicyEngine::new();

        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("deny-delegation"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        engine.add_policy(policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let delegatee = AgentID::new("agent-b");

        let decision = engine.evaluate_delegation(&cap, &delegatee, &ctx);
        assert!(decision.denies_operation());
    }

    #[test]
    fn test_mandatory_policy_engine_revocation() {
        let mut engine = MandatoryPolicyEngine::new();

        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            crate::ids::PolicyID::new("deny-revocation"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        engine.add_policy(policy).unwrap();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let revoker = AgentID::new("admin");

        let decision = engine.evaluate_revocation(&cap, &revoker, &ctx);
        assert!(decision.denies_operation());
    }

    #[test]
    fn test_mandatory_policy_engine_multiple_policies() {
        let mut engine = MandatoryPolicyEngine::new();

        // Add 3 different deny policies
        for i in 1..=3 {
            let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
            let rule = PolicyRule::new(target, OperationSet::write());
            let policy = MandatoryCapabilityPolicy::new(
                crate::ids::PolicyID::new(&format!("policy-{}", i)),
                rule,
                PolicyScope::SystemWide,
                EnforcementMode::Deny,
                1000,
                1_000_000_000,
            );
            engine.add_policy(policy).unwrap();
        }

        assert_eq!(engine.policy_count(), 3);

        let cap = create_test_cap();
        let ctx = create_test_context();

        let decision = engine.evaluate(&cap, &ctx);
        assert!(decision.denies_operation());
    }

    #[test]
    fn test_policy_evaluator_trait() {
        let engine = MandatoryPolicyEngine::new();
        let cap = create_test_cap();
        let ctx = create_test_context();

        // Trait method should work
        let decision = engine.evaluate(&cap, &ctx);
        assert!(decision.allows_operation()); // No policies, should allow
    }

    #[test]
    fn test_default_mandatory_policy_engine() {
        let engine = MandatoryPolicyEngine::default();
        assert_eq!(engine.policy_count(), 0);
    }

    #[test]
    fn test_eval_context_cloning() {
        let ctx1 = EvalContext::new(
            1000,
            AgentID::new("agent-a"),
            AgentID::new("agent-b"),
            "use",
        );
        let ctx2 = ctx1.clone();

        assert_eq!(ctx1.now_nanos, ctx2.now_nanos);
        assert_eq!(ctx1.requesting_agent, ctx2.requesting_agent);
        assert_eq!(ctx1.target_agent, ctx2.target_agent);
    }

    #[test]
    fn test_scope_matching_system_wide() {
        let engine = MandatoryPolicyEngine::new();
        let scope = PolicyScope::SystemWide;
        let ctx = create_test_context();

        assert!(engine.scope_matches(&scope, &ctx));
    }

    #[test]
    fn test_scope_matching_agent_scoped_match() {
        let engine = MandatoryPolicyEngine::new();
        let scope = PolicyScope::AgentScoped(AgentID::new("agent-a"));
        let ctx = create_test_context();

        assert!(engine.scope_matches(&scope, &ctx));
    }

    #[test]
    fn test_scope_matching_agent_scoped_no_match() {
        let engine = MandatoryPolicyEngine::new();
        let scope = PolicyScope::AgentScoped(AgentID::new("agent-c"));
        let ctx = create_test_context();

        assert!(!engine.scope_matches(&scope, &ctx));
    }

    #[test]
    fn test_scope_matching_crew_scoped() {
        let engine = MandatoryPolicyEngine::new();
        let scope = PolicyScope::CrewScoped(crate::mandatory_policy::CrewID::new("crew-a"));
        let ctx = create_test_context();

        // Currently crew matching always returns true
        assert!(engine.scope_matches(&scope, &ctx));
    }
}

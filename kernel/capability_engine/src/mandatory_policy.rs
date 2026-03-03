// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Mandatory capability policies with compositional enforcement.
//!
//! This module defines the MandatoryCapabilityPolicy entity with policy rules,
//! scope definitions, enforcement modes, and exception handling.
//! See Engineering Plan § 3.1.4: Mandatory & Stateless and Addendum v2.5.1: CPL Integration.

use alloc::collections::BTreeSet;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::error::CapError;
use crate::ids::{AgentID, PolicyID, ResourceID, ResourceType};
use crate::operations::OperationSet;

/// A strongly-typed crew identifier for policy scoping.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CrewID(String);

impl CrewID {
    /// Creates a new crew ID.
    pub fn new(id: impl Into<String>) -> Self {
        CrewID(id.into())
    }

    /// Returns a reference to the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for CrewID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Crew({})", self.0)
    }
}

/// Scope of a mandatory capability policy.
///
/// Policies can be enforced at different scopes:
/// - SystemWide: applies to all agents and resources
/// - AgentScoped: applies only to operations involving a specific agent
/// - CrewScoped: applies only to operations within a cognitive task crew
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolicyScope {
    /// Policy applies system-wide to all agents and resources.
    /// See § 3.1.4: Mandatory & Stateless.
    SystemWide,

    /// Policy applies only to operations involving a specific agent.
    /// See § 3.1.4: Mandatory & Stateless.
    AgentScoped(AgentID),

    /// Policy applies only to operations within a specific crew context.
    /// See § 3.1.4: Mandatory & Stateless.
    CrewScoped(CrewID),
}

impl Display for PolicyScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolicyScope::SystemWide => write!(f, "SystemWide"),
            PolicyScope::AgentScoped(agent) => write!(f, "AgentScoped({})", agent),
            PolicyScope::CrewScoped(crew) => write!(f, "CrewScoped({})", crew),
        }
    }
}

/// Enforcement mode for mandatory capability policies.
///
/// Determines how policy violations are handled.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnforcementMode {
    /// Deny: Violating operations are blocked immediately.
    Deny,

    /// Audit: Operations proceed but are logged for compliance review.
    Audit,

    /// Warn: Operations proceed but generate a warning message.
    Warn,
}

impl Display for EnforcementMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnforcementMode::Deny => write!(f, "Deny"),
            EnforcementMode::Audit => write!(f, "Audit"),
            EnforcementMode::Warn => write!(f, "Warn"),
        }
    }
}

/// An exception path for policy exemptions.
///
/// Represents an agent or resource that is exempted from a policy rule,
/// with optional temporal constraints and authorization tracking.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
#[derive(Clone, Debug)]
pub struct ExceptionPath {
    /// The policy ID this exception is for.
    pub policy_id: PolicyID,

    /// Capability pattern that is exempted (e.g., "file:logs/*", "*:memory:*").
    pub cap_pattern: String,

    /// Reason for the exemption.
    pub exemption_reason: String,

    /// Agent who authorized this exception.
    pub authorized_by: AgentID,

    /// When the exception expires (nanoseconds since epoch).
    /// None means the exception never expires.
    pub expiry_timestamp: Option<u64>,
}

impl ExceptionPath {
    /// Creates a new exception path.
    pub fn new(
        policy_id: PolicyID,
        cap_pattern: impl Into<String>,
        exemption_reason: impl Into<String>,
        authorized_by: AgentID,
    ) -> Self {
        ExceptionPath {
            policy_id,
            cap_pattern: cap_pattern.into(),
            exemption_reason: exemption_reason.into(),
            authorized_by,
            expiry_timestamp: None,
        }
    }

    /// Sets the expiry timestamp for this exception.
    pub fn with_expiry(mut self, timestamp_nanos: u64) -> Self {
        self.expiry_timestamp = Some(timestamp_nanos);
        self
    }

    /// Checks if this exception is still valid at a given timestamp.
    pub fn is_valid_at(&self, now_nanos: u64) -> bool {
        if let Some(expiry) = self.expiry_timestamp {
            now_nanos < expiry
        } else {
            true
        }
    }

    /// Checks if a capability pattern matches this exception path.
    /// Uses simple wildcard matching: * matches any single segment.
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        Self::pattern_match(&self.cap_pattern, pattern)
    }

    fn pattern_match(policy_pattern: &str, test_pattern: &str) -> bool {
        let policy_parts: Vec<&str> = policy_pattern.split(':').collect();
        let test_parts: Vec<&str> = test_pattern.split(':').collect();

        if policy_parts.len() != test_parts.len() {
            return false;
        }

        for (policy_part, test_part) in policy_parts.iter().zip(test_parts.iter()) {
            if *policy_part != "*" && *policy_part != test_part {
                return false;
            }
        }
        true
    }
}

impl Display for ExceptionPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExceptionPath{{policy={}, pattern={}, auth_by={}, expires={:?}}}",
            self.policy_id, self.cap_pattern, self.authorized_by, self.expiry_timestamp
        )
    }
}

/// A compositional predicate for policy rules.
///
/// Allows combining simple predicates with AND, OR, NOT operations
/// to create complex policy rules.
/// See Addendum v2.5.1: CPL Integration.
#[derive(Clone, Debug)]
pub enum PredicateComposition {
    /// Allow operations matching a specific resource type.
    ResourceTypePredicate(ResourceType),

    /// Allow operations matching a specific agent.
    AgentPredicate(AgentID),

    /// Allow operations containing specific operations.
    OperationPredicate(OperationSet),

    /// Logical AND: both predicates must be true.
    And(Box<PredicateComposition>, Box<PredicateComposition>),

    /// Logical OR: either predicate may be true.
    Or(Box<PredicateComposition>, Box<PredicateComposition>),

    /// Logical NOT: predicate must be false.
    Not(Box<PredicateComposition>),
}

impl PredicateComposition {
    /// Combines two predicates with AND.
    pub fn and(self, other: PredicateComposition) -> Self {
        PredicateComposition::And(Box::new(self), Box::new(other))
    }

    /// Combines two predicates with OR.
    pub fn or(self, other: PredicateComposition) -> Self {
        PredicateComposition::Or(Box::new(self), Box::new(other))
    }

    /// Negates a predicate.
    pub fn not(self) -> Self {
        PredicateComposition::Not(Box::new(self))
    }
}

impl Display for PredicateComposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PredicateComposition::ResourceTypePredicate(rt) => write!(f, "type={}", rt),
            PredicateComposition::AgentPredicate(agent) => write!(f, "agent={}", agent),
            PredicateComposition::OperationPredicate(ops) => write!(f, "ops={}", ops),
            PredicateComposition::And(left, right) => write!(f, "({} AND {})", left, right),
            PredicateComposition::Or(left, right) => write!(f, "({} OR {})", left, right),
            PredicateComposition::Not(pred) => write!(f, "NOT({})", pred),
        }
    }
}

/// A policy rule defining target type, operation restrictions, constraint rules, and identity rules.
///
/// Combines compositional predicates with enforcement directives.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
#[derive(Clone, Debug)]
pub struct PolicyRule {
    /// Target type predicate (e.g., "file", "memory", or composed predicates).
    pub target_type: PredicateComposition,

    /// Operations that are restricted or allowed.
    pub operation_restrictions: OperationSet,

    /// Additional constraint rules (e.g., time bounds, rate limits).
    pub constraint_rules: String,

    /// Identity-based rules (e.g., who can delegate, who can revoke).
    pub identity_rules: String,
}

impl PolicyRule {
    /// Creates a new policy rule with the given target type and operation restrictions.
    pub fn new(target_type: PredicateComposition, operation_restrictions: OperationSet) -> Self {
        PolicyRule {
            target_type,
            operation_restrictions,
            constraint_rules: String::new(),
            identity_rules: String::new(),
        }
    }

    /// Sets constraint rules for this policy rule.
    pub fn with_constraint_rules(mut self, rules: impl Into<String>) -> Self {
        self.constraint_rules = rules.into();
        self
    }

    /// Sets identity rules for this policy rule.
    pub fn with_identity_rules(mut self, rules: impl Into<String>) -> Self {
        self.identity_rules = rules.into();
        self
    }
}

impl Display for PolicyRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PolicyRule{{target={}, ops={}, constraints={}, identity={}}}",
            self.target_type,
            self.operation_restrictions,
            if self.constraint_rules.is_empty() {
                "(none)".to_string()
            } else {
                self.constraint_rules.clone()
            },
            if self.identity_rules.is_empty() {
                "(none)".to_string()
            } else {
                self.identity_rules.clone()
            }
        )
    }
}

/// A mandatory capability policy entity.
///
/// Represents a system-wide, agent-scoped, or crew-scoped security policy
/// that enforces restrictions on capability operations.
/// See Engineering Plan § 3.1.4: Mandatory & Stateless.
///
/// # Fields
/// - id: Unique policy identifier
/// - rule: Compositional predicate rules
/// - scope: SystemWide, AgentScoped, or CrewScoped
/// - enforcement: Deny, Audit, or Warn
/// - exceptions: Set of capability patterns that are exempted
/// - created_timestamp: When the policy was created (nanoseconds since epoch)
/// - audit_retention_period_ns: How long audit records are retained
#[derive(Clone, Debug)]
pub struct MandatoryCapabilityPolicy {
    /// Unique policy identifier.
    pub id: PolicyID,

    /// Compositional policy rule.
    pub rule: PolicyRule,

    /// Scope of enforcement.
    pub scope: PolicyScope,

    /// Enforcement mode: Deny, Audit, or Warn.
    pub enforcement: EnforcementMode,

    /// Exemptions from this policy.
    pub exceptions: BTreeSet<String>,

    /// When this policy was created (nanoseconds since epoch).
    pub created_timestamp: u64,

    /// How long audit records are retained (nanoseconds).
    pub audit_retention_period_ns: u64,
}

impl MandatoryCapabilityPolicy {
    /// Creates a new mandatory capability policy.
    pub fn new(
        id: PolicyID,
        rule: PolicyRule,
        scope: PolicyScope,
        enforcement: EnforcementMode,
        created_timestamp: u64,
        audit_retention_period_ns: u64,
    ) -> Self {
        MandatoryCapabilityPolicy {
            id,
            rule,
            scope,
            enforcement,
            exceptions: BTreeSet::new(),
            created_timestamp,
            audit_retention_period_ns,
        }
    }

    /// Adds an exception path to this policy.
    pub fn add_exception(&mut self, pattern: impl Into<String>) {
        self.exceptions.insert(pattern.into());
    }

    /// Checks if a capability pattern is exempted from this policy.
    pub fn is_exempted(&self, pattern: &str) -> bool {
        self.exceptions.iter().any(|exc| {
            ExceptionPath::pattern_match(exc, pattern)
        })
    }

    /// Validates that the policy structure is well-formed.
    pub fn validate(&self) -> Result<(), CapError> {
        // Check that policy ID is not empty
        if self.id.as_str().is_empty() {
            return Err(CapError::InvalidPolicy("policy ID cannot be empty".to_string()));
        }

        // Check that created_timestamp is reasonable
        if self.created_timestamp == 0 {
            return Err(CapError::InvalidPolicy("created_timestamp cannot be zero".to_string()));
        }

        // Check that audit_retention_period_ns is positive
        if self.audit_retention_period_ns == 0 {
            return Err(CapError::InvalidPolicy(
                "audit_retention_period_ns must be positive".to_string(),
            ));
        }

        Ok(())
    }

    /// Returns a human-readable summary of this policy.
    pub fn summary(&self) -> String {
        alloc::format!(
            "Policy[id={}, scope={}, enforcement={}, exceptions={}]",
            self.id,
            self.scope,
            self.enforcement,
            self.exceptions.len()
        )
    }
}

impl Display for MandatoryCapabilityPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MandatoryCapabilityPolicy{{id={}, scope={}, enforcement={}, rule={}, created={}, audit_retention={}}}",
            self.id,
            self.scope,
            self.enforcement,
            self.rule,
            self.created_timestamp,
            self.audit_retention_period_ns
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_crew_id_creation() {
        let crew = CrewID::new("crew-001");
        assert_eq!(crew.as_str(), "crew-001");
    }

    #[test]
    fn test_crew_id_display() {
        let crew = CrewID::new("main-crew");
        assert_eq!(crew.to_string(), "Crew(main-crew)");
    }

    #[test]
    fn test_policy_scope_system_wide() {
        let scope = PolicyScope::SystemWide;
        assert_eq!(scope.to_string(), "SystemWide");
    }

    #[test]
    fn test_policy_scope_agent_scoped() {
        let scope = PolicyScope::AgentScoped(AgentID::new("agent-a"));
        assert!(scope.to_string().contains("AgentScoped"));
    }

    #[test]
    fn test_policy_scope_crew_scoped() {
        let scope = PolicyScope::CrewScoped(CrewID::new("crew-a"));
        assert!(scope.to_string().contains("CrewScoped"));
    }

    #[test]
    fn test_enforcement_mode_display() {
        assert_eq!(EnforcementMode::Deny.to_string(), "Deny");
        assert_eq!(EnforcementMode::Audit.to_string(), "Audit");
        assert_eq!(EnforcementMode::Warn.to_string(), "Warn");
    }

    #[test]
    fn test_exception_path_creation() {
        let exc = ExceptionPath::new(
            PolicyID::new("policy-001"),
            "file:logs/*",
            "Logging exemption",
            AgentID::new("admin"),
        );
        assert_eq!(exc.cap_pattern, "file:logs/*");
        assert!(exc.is_valid_at(u64::MAX));
    }

    #[test]
    fn test_exception_path_with_expiry() {
        let exc = ExceptionPath::new(
            PolicyID::new("policy-001"),
            "file:temp/*",
            "Temporary exemption",
            AgentID::new("admin"),
        )
        .with_expiry(1000);

        assert!(exc.is_valid_at(500));
        assert!(!exc.is_valid_at(1000));
        assert!(!exc.is_valid_at(2000));
    }

    #[test]
    fn test_exception_path_pattern_matching() {
        let exc = ExceptionPath::new(
            PolicyID::new("policy-001"),
            "file:logs/*",
            "Logging exemption",
            AgentID::new("admin"),
        );

        assert!(exc.matches_pattern("file:logs/app.log"));
        assert!(exc.matches_pattern("file:logs/debug.log"));
        assert!(!exc.matches_pattern("file:data/user.txt"));
    }

    #[test]
    fn test_exception_path_wildcard_patterns() {
        let exc = ExceptionPath::new(
            PolicyID::new("policy-001"),
            "*:memory:*",
            "Memory exemption",
            AgentID::new("admin"),
        );

        assert!(exc.matches_pattern("service:memory:buffer-001"));
        assert!(exc.matches_pattern("agent:memory:cache"));
        assert!(!exc.matches_pattern("file:disk:storage"));
    }

    #[test]
    fn test_predicate_composition_resource_type() {
        let pred = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        assert!(pred.to_string().contains("type="));
    }

    #[test]
    fn test_predicate_composition_agent() {
        let pred = PredicateComposition::AgentPredicate(AgentID::new("agent-a"));
        assert!(pred.to_string().contains("agent="));
    }

    #[test]
    fn test_predicate_composition_and() {
        let left = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let right = PredicateComposition::AgentPredicate(AgentID::new("agent-a"));
        let combined = left.and(right);
        assert!(combined.to_string().contains("AND"));
    }

    #[test]
    fn test_predicate_composition_or() {
        let left = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let right = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let combined = left.or(right);
        assert!(combined.to_string().contains("OR"));
    }

    #[test]
    fn test_predicate_composition_not() {
        let pred = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let negated = pred.not();
        assert!(negated.to_string().contains("NOT"));
    }

    #[test]
    fn test_policy_rule_creation() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        assert!(rule.constraint_rules.is_empty());
    }

    #[test]
    fn test_policy_rule_with_constraints() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::all())
            .with_constraint_rules("max_ops_per_sec=100");
        assert_eq!(rule.constraint_rules, "max_ops_per_sec=100");
    }

    #[test]
    fn test_policy_rule_with_identity_rules() {
        let target = PredicateComposition::AgentPredicate(AgentID::new("admin"));
        let rule = PolicyRule::new(target, OperationSet::all())
            .with_identity_rules("only_system_admin_can_delegate");
        assert_eq!(rule.identity_rules, "only_system_admin_can_delegate");
    }

    #[test]
    fn test_mandatory_policy_creation() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("deny-mem-write"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );
        assert_eq!(policy.id.as_str(), "deny-mem-write");
        assert_eq!(policy.enforcement, EnforcementMode::Deny);
    }

    #[test]
    fn test_mandatory_policy_add_exception() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let mut policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        policy.add_exception("*:memory:kernel-buf");
        assert!(policy.is_exempted("*:memory:kernel-buf"));
    }

    #[test]
    fn test_mandatory_policy_exception_matching() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::all());
        let mut policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        policy.add_exception("file:logs/*");
        policy.add_exception("file:temp/*");

        assert!(policy.is_exempted("file:logs/app.log"));
        assert!(policy.is_exempted("file:temp/cache.tmp"));
        assert!(!policy.is_exempted("file:data/user.txt"));
    }

    #[test]
    fn test_mandatory_policy_validate() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("valid-policy"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_mandatory_policy_validate_zero_timestamp() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            0, // Invalid: zero timestamp
            1_000_000_000,
        );

        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_mandatory_policy_validate_zero_retention() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            0, // Invalid: zero retention
        );

        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_mandatory_policy_summary() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let summary = policy.summary();
        assert!(summary.contains("policy-001"));
        assert!(summary.contains("SystemWide"));
        assert!(summary.contains("Deny"));
    }

    #[test]
    fn test_mandatory_policy_display() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::read());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::AgentScoped(AgentID::new("agent-a")),
            EnforcementMode::Audit,
            2000,
            2_000_000_000,
        );

        let display = policy.to_string();
        assert!(display.contains("policy-001"));
        assert!(display.contains("Audit"));
    }

    #[test]
    fn test_mandatory_policy_agent_scope() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("agent-policy"),
            rule,
            PolicyScope::AgentScoped(AgentID::new("restricted-agent")),
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        match policy.scope {
            PolicyScope::AgentScoped(agent) => {
                assert_eq!(agent.as_str(), "restricted-agent");
            }
            _ => panic!("Expected AgentScoped"),
        }
    }

    #[test]
    fn test_mandatory_policy_crew_scope() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::service());
        let rule = PolicyRule::new(target, OperationSet::invoke());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("crew-policy"),
            rule,
            PolicyScope::CrewScoped(CrewID::new("analytics-crew")),
            EnforcementMode::Warn,
            1000,
            1_000_000_000,
        );

        match policy.scope {
            PolicyScope::CrewScoped(crew) => {
                assert_eq!(crew.as_str(), "analytics-crew");
            }
            _ => panic!("Expected CrewScoped"),
        }
    }

    #[test]
    fn test_multiple_exception_patterns() {
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::all());
        let mut policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        policy.add_exception("file:logs/*");
        policy.add_exception("file:temp/*");
        policy.add_exception("file:cache/*");

        assert_eq!(policy.exceptions.len(), 3);
        assert!(policy.is_exempted("file:logs/debug.log"));
        assert!(policy.is_exempted("file:temp/session.tmp"));
        assert!(policy.is_exempted("file:cache/index.bin"));
    }
}

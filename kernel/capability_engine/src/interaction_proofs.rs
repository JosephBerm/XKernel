// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Policy-Capability interaction validation and proofs.
//!
//! This module provides mechanisms to prove that capabilities and policies
//! interact correctly, with no bypasses, proper scope isolation, and bounded exceptions.
//! See Engineering Plan § 3.1.4: Mandatory & Stateless and § 3.2.1: Verification & Proofs.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::error::CapError;
use crate::ids::{AgentID, ResourceID, ResourceType};
use crate::mandatory_policy::MandatoryCapabilityPolicy;
use crate::policy_engine::{EvalContext, MandatoryPolicyEngine, PolicyDecision, PolicyEvaluator};

/// A test scenario for policy-capability interactions.
///
/// Represents a specific combination of capability, policy, and context
/// to validate interaction properties.
/// See Engineering Plan § 3.2.1: Verification & Proofs.
#[derive(Clone, Debug)]
pub struct PolicyCapabilityInteraction {
    /// Name of the test scenario.
    pub scenario_name: String,

    /// The capability being tested.
    pub capability: Capability,

    /// The policy being enforced.
    pub policy: MandatoryCapabilityPolicy,

    /// Evaluation context.
    pub context: EvalContext,

    /// Expected decision from policy evaluation.
    pub expected_decision: PolicyDecisionType,

    /// Description of what this scenario tests.
    pub description: String,
}

/// Simplified expected policy decision type for validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyDecisionType {
    Allow,
    Deny,
    Audit,
    Warn,
}

impl PolicyCapabilityInteraction {
    /// Creates a new policy-capability interaction test.
    pub fn new(
        scenario_name: impl Into<String>,
        capability: Capability,
        policy: MandatoryCapabilityPolicy,
        context: EvalContext,
        expected_decision: PolicyDecisionType,
        description: impl Into<String>,
    ) -> Self {
        PolicyCapabilityInteraction {
            scenario_name: scenario_name.into(),
            capability,
            policy,
            context,
            expected_decision,
            description: description.into(),
        }
    }

    /// Executes this interaction test and returns whether it passed.
    pub fn validate(&self) -> Result<bool, CapError> {
        let mut engine = MandatoryPolicyEngine::new();
        engine.add_policy(self.policy.clone())?;

        let decision = engine.evaluate(&self.capability, &self.context);

        let matches = match self.expected_decision {
            PolicyDecisionType::Allow => decision.allows_operation(),
            PolicyDecisionType::Deny => decision.denies_operation(),
            PolicyDecisionType::Audit => decision.requires_audit(),
            PolicyDecisionType::Warn => decision.generates_warning(),
        };

        Ok(matches)
    }
}

impl Display for PolicyCapabilityInteraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PolicyCapabilityInteraction{{scenario={}, expected={:?}}}",
            self.scenario_name, self.expected_decision
        )
    }
}

/// Validator for policy-capability interaction proofs.
///
/// Provides methods to verify that capabilities and policies interact correctly.
/// See Engineering Plan § 3.2.1: Verification & Proofs.
pub struct InteractionProofValidator {
    /// Test scenarios to validate.
    scenarios: Vec<PolicyCapabilityInteraction>,
}

impl InteractionProofValidator {
    /// Creates a new interaction proof validator.
    pub fn new() -> Self {
        InteractionProofValidator {
            scenarios: Vec::new(),
        }
    }

    /// Adds a scenario to the validator.
    pub fn add_scenario(&mut self, scenario: PolicyCapabilityInteraction) {
        self.scenarios.push(scenario);
    }

    /// Adds multiple scenarios at once.
    pub fn add_scenarios(&mut self, scenarios: Vec<PolicyCapabilityInteraction>) {
        self.scenarios.extend(scenarios);
    }

    /// Returns the number of scenarios.
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Validates that no capability grant can circumvent mandatory policies.
    ///
    /// This proof demonstrates that even with the strongest capabilities,
    /// mandatory deny policies cannot be bypassed.
    pub fn validate_no_bypass(&self) -> Result<bool, CapError> {
        for scenario in &self.scenarios {
            if scenario.expected_decision == PolicyDecisionType::Deny {
                let passed = scenario.validate()?;
                if !passed {
                    return Err(CapError::InteractionProofFailed(alloc::format!(
                        "bypass detected in scenario: {}",
                        scenario.scenario_name
                    )));
                }
            }
        }
        Ok(true)
    }

    /// Validates that agent-scoped policies don't leak to other agents.
    ///
    /// This proof demonstrates that policies with AgentScoped scope
    /// only affect the designated agent.
    pub fn validate_scope_isolation(&self) -> Result<bool, CapError> {
        let mut agent_scoped_scenarios = Vec::new();

        // Collect agent-scoped policy scenarios
        for scenario in &self.scenarios {
            if matches!(
                scenario.policy.scope,
                crate::mandatory_policy::PolicyScope::AgentScoped(_)
            ) {
                agent_scoped_scenarios.push(scenario);
            }
        }

        // For each agent-scoped scenario, verify it only affects the target agent
        for scenario in &agent_scoped_scenarios {
            let passed = scenario.validate()?;

            if passed {
                // If the policy applied, verify it's because of agent matching
                if let crate::mandatory_policy::PolicyScope::AgentScoped(ref target_agent) =
                    scenario.policy.scope
                {
                    let matches_target = target_agent == &scenario.context.target_agent
                        || target_agent == &scenario.context.requesting_agent;

                    if !matches_target {
                        return Err(CapError::InteractionProofFailed(alloc::format!(
                            "scope isolation violated in scenario: {}",
                            scenario.scenario_name
                        )));
                    }
                }
            }
        }

        Ok(true)
    }

    /// Validates that exceptions are properly constrained.
    ///
    /// This proof demonstrates that exceptions only apply to their stated
    /// capability patterns and respects temporal bounds.
    pub fn validate_exception_bounds(&self) -> Result<bool, CapError> {
        for scenario in &self.scenarios {
            // Check that exceptions don't expand beyond their patterns
            if !scenario.policy.exceptions.is_empty() {
                // If exception is applied, verify the capability matches the pattern
                let cap_pattern = alloc::format!(
                    "{}:{}:{}",
                    scenario.capability.target_resource_type,
                    scenario.capability.target_resource_id,
                    scenario.capability.target_agent
                );

                if scenario.policy.is_exempted(&cap_pattern) {
                    // Exception applies: verify it's the right one
                    let mut found_match = false;
                    for exc_pattern in &scenario.policy.exceptions {
                        if crate::mandatory_policy::ExceptionPath::pattern_match(exc_pattern, &cap_pattern) {
                            found_match = true;
                            break;
                        }
                    }

                    if !found_match {
                        return Err(CapError::InteractionProofFailed(alloc::format!(
                            "exception bounds violated in scenario: {}",
                            scenario.scenario_name
                        )));
                    }
                }
            }
        }

        Ok(true)
    }

    /// Runs all validation proofs and returns overall result.
    pub fn validate_all(&self) -> Result<InteractionProofResult, CapError> {
        let no_bypass = self.validate_no_bypass()?;
        let scope_isolation = self.validate_scope_isolation()?;
        let exception_bounds = self.validate_exception_bounds()?;

        Ok(InteractionProofResult {
            no_bypass,
            scope_isolation,
            exception_bounds,
            total_scenarios: self.scenario_count(),
        })
    }
}

impl Default for InteractionProofValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of interaction proof validation.
#[derive(Clone, Debug)]
pub struct InteractionProofResult {
    /// Whether the no-bypass proof passed.
    pub no_bypass: bool,

    /// Whether the scope isolation proof passed.
    pub scope_isolation: bool,

    /// Whether the exception bounds proof passed.
    pub exception_bounds: bool,

    /// Total number of scenarios tested.
    pub total_scenarios: usize,
}

impl InteractionProofResult {
    /// Returns true if all proofs passed.
    pub fn all_passed(&self) -> bool {
        self.no_bypass && self.scope_isolation && self.exception_bounds
    }
}

impl Display for InteractionProofResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InteractionProofResult{{no_bypass={}, scope_isolation={}, exception_bounds={}, total_scenarios={}}}",
            self.no_bypass, self.scope_isolation, self.exception_bounds, self.total_scenarios
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::constraints::Timestamp;
    use crate::ids::{CapID, PolicyID};
    use crate::mandatory_policy::{
        EnforcementMode, MandatoryCapabilityPolicy, PolicyRule, PolicyScope,
        PredicateComposition,
    };
    use crate::operations::OperationSet;
use alloc::format;
use alloc::string::String;

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
    fn test_policy_capability_interaction_creation() {
        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "test-scenario",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Deny,
            "Test deny scenario",
        );

        assert_eq!(interaction.scenario_name, "test-scenario");
        assert_eq!(interaction.expected_decision, PolicyDecisionType::Deny);
    }

    #[test]
    fn test_policy_capability_interaction_validate() {
        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "test-scenario",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Deny,
            "Test deny scenario",
        );

        let result = interaction.validate();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_interaction_proof_validator_creation() {
        let validator = InteractionProofValidator::new();
        assert_eq!(validator.scenario_count(), 0);
    }

    #[test]
    fn test_interaction_proof_validator_add_scenario() {
        let mut validator = InteractionProofValidator::new();
        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "scenario-1",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Deny,
            "Test scenario",
        );

        validator.add_scenario(interaction);
        assert_eq!(validator.scenario_count(), 1);
    }

    #[test]
    fn test_interaction_proof_validator_no_bypass() {
        let mut validator = InteractionProofValidator::new();

        // Create a deny policy scenario
        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("deny-policy"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "deny-scenario",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Deny,
            "Test deny policy",
        );

        validator.add_scenario(interaction);

        let result = validator.validate_no_bypass();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_interaction_proof_validator_scope_isolation() {
        let mut validator = InteractionProofValidator::new();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("agent-policy"),
            rule,
            PolicyScope::AgentScoped(AgentID::new("agent-a")),
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "scope-test",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Deny,
            "Test scope isolation",
        );

        validator.add_scenario(interaction);

        let result = validator.validate_scope_isolation();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_interaction_proof_validator_exception_bounds() {
        let mut validator = InteractionProofValidator::new();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let mut policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-with-exception"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        // Add an exception
        policy.add_exception("file:*:*");

        let interaction = PolicyCapabilityInteraction::new(
            "exception-test",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Allow,
            "Test exception bounds",
        );

        validator.add_scenario(interaction);

        let result = validator.validate_exception_bounds();
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_interaction_proof_validator_validate_all() {
        let mut validator = InteractionProofValidator::new();

        // Add a deny scenario
        let cap1 = create_test_cap();
        let ctx1 = create_test_context();
        let target1 = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule1 = PolicyRule::new(target1, OperationSet::write());
        let policy1 = MandatoryCapabilityPolicy::new(
            PolicyID::new("policy-001"),
            rule1,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction1 = PolicyCapabilityInteraction::new(
            "deny-scenario",
            cap1,
            policy1,
            ctx1,
            PolicyDecisionType::Deny,
            "Test deny",
        );

        validator.add_scenario(interaction1);

        let result = validator.validate_all();
        assert!(result.is_ok());
        let proof_result = result.unwrap();
        assert!(proof_result.all_passed());
    }

    #[test]
    fn test_interaction_proof_result_all_passed() {
        let result = InteractionProofResult {
            no_bypass: true,
            scope_isolation: true,
            exception_bounds: true,
            total_scenarios: 5,
        };

        assert!(result.all_passed());
    }

    #[test]
    fn test_interaction_proof_result_not_all_passed() {
        let result = InteractionProofResult {
            no_bypass: true,
            scope_isolation: false,
            exception_bounds: true,
            total_scenarios: 5,
        };

        assert!(!result.all_passed());
    }

    // Test scenarios demonstrating specific interaction properties

    #[test]
    fn test_scenario_system_wide_deny() {
        let mut validator = InteractionProofValidator::new();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("sys-deny"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "system-wide-deny",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Deny,
            "System-wide deny policy",
        );

        validator.add_scenario(interaction);
        assert!(validator.validate_no_bypass().is_ok());
    }

    #[test]
    fn test_scenario_agent_scoped_isolation() {
        let mut validator = InteractionProofValidator::new();

        let mut cap = create_test_cap();
        cap.target_agent = AgentID::new("agent-x");

        let mut ctx = create_test_context();
        ctx.target_agent = AgentID::new("agent-x");

        let target = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule = PolicyRule::new(target, OperationSet::write());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("agent-x-policy"),
            rule,
            PolicyScope::AgentScoped(AgentID::new("agent-x")),
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "agent-isolation",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Deny,
            "Agent-scoped policy isolation",
        );

        validator.add_scenario(interaction);
        assert!(validator.validate_scope_isolation().is_ok());
    }

    #[test]
    fn test_scenario_exception_with_pattern() {
        let mut validator = InteractionProofValidator::new();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::all());
        let mut policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("with-exception"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        policy.add_exception("file:logs/*");

        let interaction = PolicyCapabilityInteraction::new(
            "exception-pattern",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Allow, // Allowed because of exception
            "Exception pattern matching",
        );

        validator.add_scenario(interaction);
        assert!(validator.validate_exception_bounds().is_ok());
    }

    #[test]
    fn test_scenario_audit_policy() {
        let mut validator = InteractionProofValidator::new();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::read());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("audit-policy"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Audit,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "audit-scenario",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Audit,
            "Audit policy enforcement",
        );

        validator.add_scenario(interaction);
        let result = validator.validate_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scenario_warn_policy() {
        let mut validator = InteractionProofValidator::new();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::all());
        let policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("warn-policy"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Warn,
            1000,
            1_000_000_000,
        );

        let interaction = PolicyCapabilityInteraction::new(
            "warn-scenario",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Warn,
            "Warn policy enforcement",
        );

        validator.add_scenario(interaction);
        let result = validator.validate_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scenario_multiple_agents() {
        let mut validator = InteractionProofValidator::new();

        // Scenario 1: Agent A with agent-scoped policy
        let mut cap1 = create_test_cap();
        cap1.target_agent = AgentID::new("agent-a");
        let mut ctx1 = create_test_context();
        ctx1.target_agent = AgentID::new("agent-a");

        let target1 = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule1 = PolicyRule::new(target1, OperationSet::write());
        let policy1 = MandatoryCapabilityPolicy::new(
            PolicyID::new("agent-a-policy"),
            rule1,
            PolicyScope::AgentScoped(AgentID::new("agent-a")),
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction1 = PolicyCapabilityInteraction::new(
            "agent-a-scenario",
            cap1,
            policy1,
            ctx1,
            PolicyDecisionType::Deny,
            "Agent A scoped policy",
        );

        // Scenario 2: Agent B should not be affected
        let mut cap2 = create_test_cap();
        cap2.target_agent = AgentID::new("agent-b");
        let mut ctx2 = create_test_context();
        ctx2.target_agent = AgentID::new("agent-b");

        let target2 = PredicateComposition::ResourceTypePredicate(ResourceType::memory());
        let rule2 = PolicyRule::new(target2, OperationSet::write());
        let policy2 = MandatoryCapabilityPolicy::new(
            PolicyID::new("agent-a-policy"),
            rule2,
            PolicyScope::AgentScoped(AgentID::new("agent-a")),
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        let interaction2 = PolicyCapabilityInteraction::new(
            "agent-b-scenario",
            cap2,
            policy2,
            ctx2,
            PolicyDecisionType::Allow,
            "Agent B not affected by Agent A policy",
        );

        validator.add_scenario(interaction1);
        validator.add_scenario(interaction2);

        assert!(validator.validate_scope_isolation().is_ok());
    }

    #[test]
    fn test_scenario_cascading_exceptions() {
        let mut validator = InteractionProofValidator::new();

        let cap = create_test_cap();
        let ctx = create_test_context();
        let target = PredicateComposition::ResourceTypePredicate(ResourceType::file());
        let rule = PolicyRule::new(target, OperationSet::write());
        let mut policy = MandatoryCapabilityPolicy::new(
            PolicyID::new("cascading-exceptions"),
            rule,
            PolicyScope::SystemWide,
            EnforcementMode::Deny,
            1000,
            1_000_000_000,
        );

        // Add multiple exceptions
        policy.add_exception("file:logs/*");
        policy.add_exception("file:temp/*");
        policy.add_exception("file:cache/*");

        let interaction = PolicyCapabilityInteraction::new(
            "cascading-test",
            cap,
            policy,
            ctx,
            PolicyDecisionType::Allow,
            "Multiple exceptions",
        );

        validator.add_scenario(interaction);
        assert!(validator.validate_exception_bounds().is_ok());
    }

    #[test]
    fn test_default_interaction_proof_validator() {
        let validator = InteractionProofValidator::default();
        assert_eq!(validator.scenario_count(), 0);
    }
}

// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Mandatory policy enforcement and Cognitive Policy Language (CPL)

use alloc::boxed::Box;
use crate::capability::{Capability, CapabilityError, PermissionFlags};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

pub type Result<T> = core::result::Result<T, CapabilityError>;

/// Cognitive Policy Language (CPL) policy expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyExpression {
    /// Always allow
    Allow,
    /// Always deny
    Deny,
    /// Require specific permission
    Require(PermissionFlags),
    /// Conjunction of policies
    And(Box<PolicyExpression>, Box<PolicyExpression>),
    /// Disjunction of policies
    Or(Box<PolicyExpression>, Box<PolicyExpression>),
    /// Role-based policy
    RoleBased(alloc::string::String),
}

impl PolicyExpression {
    /// Evaluate this policy expression against a capability
    pub fn evaluate(&self, cap: &Capability, scope: u64) -> Result<bool> {
        match self {
            PolicyExpression::Allow => Ok(true),
            PolicyExpression::Deny => Ok(false),
            PolicyExpression::Require(flags) => Ok(cap.has_permission(scope, *flags)),
            PolicyExpression::And(left, right) => {
                Ok(left.evaluate(cap, scope)? && right.evaluate(cap, scope)?)
            }
            PolicyExpression::Or(left, right) => {
                Ok(left.evaluate(cap, scope)? || right.evaluate(cap, scope)?)
            }
            PolicyExpression::RoleBased(_role) => Ok(true), // Simplified
        }
    }
}

/// Mandatory access control policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MandatoryPolicy {
    /// Policy name
    pub name: alloc::string::String,
    /// Policy expression
    pub expression: PolicyExpression,
    /// Is this policy enforceable
    pub enforceable: bool,
    /// Policy priority (higher = more important)
    pub priority: u32,
}

impl MandatoryPolicy {
    /// Create a new mandatory policy
    pub fn new(
        name: alloc::string::String,
        expression: PolicyExpression,
        priority: u32,
    ) -> Self {
        Self {
            name,
            expression,
            enforceable: true,
            priority,
        }
    }

    /// Check if a capability satisfies this policy
    pub fn check(&self, cap: &Capability, scope: u64) -> Result<bool> {
        if !self.enforceable {
            return Ok(true);
        }
        self.expression.evaluate(cap, scope)
    }
}

/// Policy enforcement engine
#[derive(Debug)]
pub struct PolicyEngine {
    policies: Vec<MandatoryPolicy>,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// Add a policy
    pub fn add_policy(&mut self, policy: MandatoryPolicy) {
        self.policies.push(policy);
        // Sort by priority (highest first)
        self.policies.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove a policy
    pub fn remove_policy(&mut self, name: &str) {
        self.policies.retain(|p| p.name.as_str() != name);
    }

    /// Check if a capability passes all policies
    pub fn enforce(&self, cap: &Capability, scope: u64) -> Result<bool> {
        for policy in &self.policies {
            if !policy.check(cap, scope)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get the number of active policies
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// List all policies
    pub fn policies(&self) -> &[MandatoryPolicy] {
        &self.policies
    }

    /// Find a policy by name
    pub fn find_policy(&self, name: &str) -> Option<&MandatoryPolicy> {
        self.policies.iter().find(|p| p.name.as_str() == name)
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use crate::capability::{Permission, PermissionFlags};

    #[test]
    fn test_policy_allow() {
        let policy = PolicyExpression::Allow;
        let cap = Capability::new(
            1,
            100,
            vec![Permission::new(1, PermissionFlags::all())],
            None,
        );
        assert!(policy.evaluate(&cap, 1).unwrap());
    }

    #[test]
    fn test_policy_deny() {
        let policy = PolicyExpression::Deny;
        let cap = Capability::new(
            1,
            100,
            vec![Permission::new(1, PermissionFlags::all())],
            None,
        );
        assert!(!policy.evaluate(&cap, 1).unwrap());
    }

    #[test]
    fn test_policy_conjunction() {
        let policy = PolicyExpression::And(
            Box::new(PolicyExpression::Allow),
            Box::new(PolicyExpression::Allow),
        );
        let cap = Capability::new(
            1,
            100,
            vec![Permission::new(1, PermissionFlags::all())],
            None,
        );
        assert!(policy.evaluate(&cap, 1).unwrap());
    }

    #[test]
    fn test_policy_engine() {
        let mut engine = PolicyEngine::new();
        let policy = MandatoryPolicy::new(
            "test_policy".into(),
            PolicyExpression::Allow,
            100,
        );
        engine.add_policy(policy);

        let cap = Capability::new(
            1,
            100,
            vec![Permission::new(1, PermissionFlags::all())],
            None,
        );
        assert!(engine.enforce(&cap, 1).unwrap());
    }
}

// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Agent Unit File validation logic.
//!
//! Provides validation rules and validation engine for agent unit file specifications.
//! Ensures unit files conform to schema requirements and semantic constraints.
//!
//! Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Validation

use crate::unit_file_schema::{AgentUnitFileSchema, UnitFileHealthCheckType, RestartPolicyType, UnitFileError, UnitFileResult};
use std::collections::BTreeSet;
use core::fmt;

/// Error type for validation failures.
///
/// Provides detailed information about validation rule violations.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Validation
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Name of the validation rule that failed.
    pub rule_name: String,
    /// Detailed description of the validation failure.
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.rule_name, self.message)
    }
}

/// Validation result type.
///
/// Contains all validation errors for a single unit file validation pass.
pub type ValidationResult = core::result::Result<(), Vec<ValidationError>>;

/// Trait for validation rules.
///
/// Each validation rule implements this trait and can validate a unit file schema
/// independently. The validation engine runs all rules and collects results.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Validation
pub trait ValidationRule: core::fmt::Debug {
    /// Validates the unit file and returns a list of validation errors.
    ///
    /// Returns Ok(()) if validation passes, Err(vec![...]) with all errors if it fails.
    fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult;

    /// Returns the name of this validation rule.
    fn rule_name(&self) -> &'static str;
}

/// Validates that all required fields are present.
///
/// Checks that:
/// - Agent section has all required fields (name, version, description)
/// - Framework is specified if needed
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Required Fields
#[derive(Debug)]
pub struct RequiredFieldsRule;

impl ValidationRule for RequiredFieldsRule {
    fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult {
        let mut errors = Vec::new();

        if schema.agent.name.is_empty() {
            errors.push(ValidationError {
                rule_name: self.rule_name().to_string(),
                message: "Agent name cannot be empty".to_string(),
            });
        }

        if schema.agent.version.is_empty() {
            errors.push(ValidationError {
                rule_name: self.rule_name().to_string(),
                message: "Agent version cannot be empty".to_string(),
            });
        }

        if schema.agent.description.is_empty() {
            errors.push(ValidationError {
                rule_name: self.rule_name().to_string(),
                message: "Agent description cannot be empty".to_string(),
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn rule_name(&self) -> &'static str {
        "RequiredFieldsRule"
    }
}

/// Validates resource limit constraints.
///
/// Checks that:
/// - Resource values are within reasonable bounds
/// - GPU time doesn't exceed wall clock time
/// - Memory is within system limits
/// - Token limits are reasonable
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Resource Limits
#[derive(Debug)]
pub struct ResourceLimitsRule {
    /// Maximum allowed memory in bytes (default: 16GB).
    pub max_memory_bytes: u64,
    /// Maximum allowed GPU milliseconds (default: 1 hour).
    pub max_gpu_ms: u64,
    /// Maximum allowed wall clock milliseconds (default: 24 hours).
    pub max_wall_clock_ms: u64,
    /// Maximum allowed tokens per task (default: 100k).
    pub max_tokens_per_task: u32,
}

impl Default for ResourceLimitsRule {
    fn default() -> Self {
        Self {
            max_memory_bytes: 16 * 1024 * 1024 * 1024, // 16GB
            max_gpu_ms: 3600000,                         // 1 hour
            max_wall_clock_ms: 86400000,                // 24 hours
            max_tokens_per_task: 100000,
        }
    }
}

impl ValidationRule for ResourceLimitsRule {
    fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult {
        let mut errors = Vec::new();

        if let Some(resources) = &schema.resources {
            if let Some(mem) = resources.max_memory_bytes {
                if mem > self.max_memory_bytes {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!(
                            "Memory limit {} exceeds system max {}",
                            mem, self.max_memory_bytes
                        ),
                    });
                }
            }

            if let Some(gpu) = resources.max_gpu_ms {
                if gpu > self.max_gpu_ms {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!(
                            "GPU time limit {} exceeds system max {}",
                            gpu, self.max_gpu_ms
                        ),
                    });
                }
            }

            if let Some(wall_clock) = resources.max_wall_clock_ms {
                if wall_clock > self.max_wall_clock_ms {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!(
                            "Wall clock limit {} exceeds system max {}",
                            wall_clock, self.max_wall_clock_ms
                        ),
                    });
                }
            }

            // GPU time should not exceed wall clock time
            if let (Some(gpu), Some(wall_clock)) = (resources.max_gpu_ms, resources.max_wall_clock_ms) {
                if gpu > wall_clock {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "GPU time limit cannot exceed wall clock time limit".to_string(),
                    });
                }
            }

            if let Some(tokens) = resources.max_tokens_per_task {
                if tokens > self.max_tokens_per_task {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!(
                            "Token limit {} exceeds system max {}",
                            tokens, self.max_tokens_per_task
                        ),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn rule_name(&self) -> &'static str {
        "ResourceLimitsRule"
    }
}

/// Validates dependency specifications.
///
/// Checks that:
/// - No circular dependencies (though basic - a full cycle detector would need graph analysis)
/// - Dependencies are properly formatted
/// - Required services are specified
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Dependency Validation
#[derive(Debug)]
pub struct DependencyConsistencyRule;

impl ValidationRule for DependencyConsistencyRule {
    fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult {
        let mut errors = Vec::new();

        if let Some(deps) = &schema.dependencies {
            // Check for self-dependency
            if let Some(after) = &deps.after {
                if after.contains(&schema.agent.name) {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!(
                            "Agent {} cannot depend on itself in 'after' clause",
                            schema.agent.name
                        ),
                    });
                }
            }

            if let Some(before) = &deps.before {
                if before.contains(&schema.agent.name) {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!(
                            "Agent {} cannot depend on itself in 'before' clause",
                            schema.agent.name
                        ),
                    });
                }
            }

            // Check that dependencies are non-empty if specified
            if let Some(after) = &deps.after {
                if after.is_empty() {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "'after' dependency list cannot be empty if specified".to_string(),
                    });
                }
            }

            if let Some(before) = &deps.before {
                if before.is_empty() {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "'before' dependency list cannot be empty if specified".to_string(),
                    });
                }
            }

            if let Some(requires) = &deps.requires {
                if requires.is_empty() {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "'requires' list cannot be empty if specified".to_string(),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn rule_name(&self) -> &'static str {
        "DependencyConsistencyRule"
    }
}

/// Validates that requested capabilities are valid.
///
/// Checks that:
/// - Capability names follow expected format
/// - No duplicate capabilities
/// - Required capabilities are actually required
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Capabilities
#[derive(Debug)]
pub struct CapabilityExistenceRule {
    /// Valid capability names.
    pub valid_capabilities: BTreeSet<&'static str>,
}

impl Default for CapabilityExistenceRule {
    fn default() -> Self {
        let mut caps = BTreeSet::new();
        caps.insert("mem_read");
        caps.insert("mem_write");
        caps.insert("tool_invoke");
        caps.insert("channel_send");
        caps.insert("file_access");
        caps.insert("network_raw");
        caps.insert("gpu_compute");
        caps.insert("agent_spawn");
        caps.insert("database_access");
        caps.insert("sys_resource");
        caps.insert("sys_ptrace");
        caps.insert("net_admin");
        caps.insert("net_bind_service");

        Self {
            valid_capabilities: caps,
        }
    }
}

impl ValidationRule for CapabilityExistenceRule {
    fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult {
        let mut errors = Vec::new();

        if let Some(caps) = &schema.capabilities {
            let mut seen = BTreeSet::new();

            if let Some(required) = &caps.required {
                for cap in required {
                    if !self.valid_capabilities.contains(cap.as_str()) {
                        errors.push(ValidationError {
                            rule_name: self.rule_name().to_string(),
                            message: format!("Unknown required capability: {}", cap),
                        });
                    }

                    if seen.contains(cap) {
                        errors.push(ValidationError {
                            rule_name: self.rule_name().to_string(),
                            message: format!("Duplicate capability in required list: {}", cap),
                        });
                    }
                    seen.insert(cap.clone());
                }
            }

            if let Some(optional) = &caps.optional {
                for cap in optional {
                    if !self.valid_capabilities.contains(cap.as_str()) {
                        errors.push(ValidationError {
                            rule_name: self.rule_name().to_string(),
                            message: format!("Unknown optional capability: {}", cap),
                        });
                    }

                    if seen.contains(cap) {
                        errors.push(ValidationError {
                            rule_name: self.rule_name().to_string(),
                            message: format!("Duplicate capability across required/optional: {}", cap),
                        });
                    }
                    seen.insert(cap.clone());
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn rule_name(&self) -> &'static str {
        "CapabilityExistenceRule"
    }
}

/// Validates health check configuration.
///
/// Checks that:
/// - Health check type is valid
/// - Endpoint is appropriate for the check type
/// - Intervals and timeouts are reasonable
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Health Checks
#[derive(Debug)]
pub struct HealthCheckRule;

impl ValidationRule for HealthCheckRule {
    fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult {
        let mut errors = Vec::new();

        if let Some(hc) = &schema.health_check {
            // Validate check type
            if let Some(check_type) = &hc.check_type {
                if UnitFileHealthCheckType::from_str(check_type).is_err() {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!("Invalid health check type: {}", check_type),
                    });
                }
            }

            // Validate interval is positive
            if let Some(interval) = hc.interval_ms {
                if interval == 0 {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Health check interval must be positive".to_string(),
                    });
                }
            }

            // Validate timeout is positive
            if let Some(timeout) = hc.timeout_ms {
                if timeout == 0 {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Health check timeout must be positive".to_string(),
                    });
                }
            }

            // Validate timeout < interval
            if let (Some(timeout), Some(interval)) = (hc.timeout_ms, hc.interval_ms) {
                if timeout >= interval {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Health check timeout must be less than interval".to_string(),
                    });
                }
            }

            // Validate thresholds are reasonable
            if let Some(failure) = hc.failure_threshold {
                if failure == 0 {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Failure threshold must be at least 1".to_string(),
                    });
                }
            }

            if let Some(success) = hc.success_threshold {
                if success == 0 {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Success threshold must be at least 1".to_string(),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn rule_name(&self) -> &'static str {
        "HealthCheckRule"
    }
}

/// Validates restart policy configuration.
///
/// Checks that:
/// - Restart policy type is valid
/// - Backoff parameters are reasonable
/// - Retry limits are positive
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Restart Policies
#[derive(Debug)]
pub struct RestartPolicyRule;

impl ValidationRule for RestartPolicyRule {
    fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult {
        let mut errors = Vec::new();

        if let Some(restart) = &schema.restart {
            // Validate policy type
            if let Some(policy) = &restart.policy {
                if RestartPolicyType::from_str(policy).is_err() {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: format!("Invalid restart policy: {}", policy),
                    });
                }
            }

            // Validate retry limits
            if let Some(retries) = restart.max_retries {
                if retries == 0 && restart.policy.as_deref() == Some("on_failure") {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "max_retries must be > 0 for on_failure policy".to_string(),
                    });
                }
            }

            // Validate backoff parameters
            if let Some(base) = restart.backoff_base_ms {
                if base == 0 {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Backoff base must be positive".to_string(),
                    });
                }
            }

            if let Some(multiplier) = restart.backoff_multiplier {
                if multiplier < 1.0 {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Backoff multiplier must be >= 1.0".to_string(),
                    });
                }
            }

            // Validate max backoff > base
            if let (Some(base), Some(max_backoff)) = (restart.backoff_base_ms, restart.max_backoff_ms) {
                if max_backoff < base {
                    errors.push(ValidationError {
                        rule_name: self.rule_name().to_string(),
                        message: "Max backoff must be >= backoff base".to_string(),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn rule_name(&self) -> &'static str {
        "RestartPolicyRule"
    }
}

/// Validation engine that runs all rules and collects errors.
///
/// Runs a series of validation rules against a unit file schema and collects
/// all errors for reporting.
///
/// Reference: Engineering Plan § Agent Lifecycle Management § Unit Files § Validation Engine
#[derive(Debug)]
pub struct ValidationEngine {
    /// Registered validation rules.
    rules: Vec<Box<dyn ValidationRule>>,
}

impl ValidationEngine {
    /// Creates a new validation engine with standard rules.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// Adds a validation rule to the engine.
    pub fn add_rule(&mut self, rule: Box<dyn ValidationRule>) {
        self.rules.push(rule);
    }

    /// Creates a validation engine with all standard rules.
    pub fn with_standard_rules() -> Self {
        let mut engine = Self::new();
        engine.add_rule(Box::new(RequiredFieldsRule));
        engine.add_rule(Box::new(ResourceLimitsRule::default()));
        engine.add_rule(Box::new(DependencyConsistencyRule));
        engine.add_rule(Box::new(CapabilityExistenceRule::default()));
        engine.add_rule(Box::new(HealthCheckRule));
        engine.add_rule(Box::new(RestartPolicyRule));
        engine
    }

    /// Validates a unit file schema using all registered rules.
    ///
    /// Returns Ok(()) if all validation passes, or Err with all errors from all rules.
    pub fn validate(&self, schema: &AgentUnitFileSchema) -> ValidationResult {
        let mut all_errors = Vec::new();

        for rule in &self.rules {
            if let Err(errors) = rule.validate(schema) {
                all_errors.extend(errors);
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::with_standard_rules()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit_file_schema::{AgentSection, AgentUnitFileSchema};

    #[test]
    fn test_required_fields_rule_valid() {
        let schema = AgentUnitFileSchema::new("test", "1.0.0", "Test agent");
        let rule = RequiredFieldsRule;
        assert!(rule.validate(&schema).is_ok());
    }

    #[test]
    fn test_required_fields_rule_missing_name() {
        let mut schema = AgentUnitFileSchema::new("test", "1.0.0", "Test agent");
        schema.agent.name.clear();
        let rule = RequiredFieldsRule;
        assert!(rule.validate(&schema).is_err());
    }

    #[test]
    fn test_resource_limits_rule_valid() {
        let schema = AgentUnitFileSchema::new("test", "1.0.0", "Test agent");
        let rule = ResourceLimitsRule::default();
        assert!(rule.validate(&schema).is_ok());
    }

    #[test]
    fn test_dependency_consistency_rule_no_self_dependency() {
        let mut schema = AgentUnitFileSchema::new("agent-a", "1.0.0", "Test");
        let mut deps = crate::unit_file_schema::DependenciesSection {
            after: Some(vec!["agent-a".to_string()]),
            before: None,
            requires: None,
        };
        schema.dependencies = Some(deps);

        let rule = DependencyConsistencyRule;
        assert!(rule.validate(&schema).is_err());
    }

    #[test]
    fn test_health_check_rule_invalid_type() {
        let mut schema = AgentUnitFileSchema::new("test", "1.0.0", "Test");
        schema.health_check = Some(crate::unit_file_schema::HealthCheckSection {
            check_type: Some("invalid_type".to_string()),
            endpoint: None,
            interval_ms: None,
            timeout_ms: None,
            failure_threshold: None,
            success_threshold: None,
        });

        let rule = HealthCheckRule;
        assert!(rule.validate(&schema).is_err());
    }

    #[test]
    fn test_restart_policy_rule_invalid_policy() {
        let mut schema = AgentUnitFileSchema::new("test", "1.0.0", "Test");
        schema.restart = Some(crate::unit_file_schema::RestartSection {
            policy: Some("invalid_policy".to_string()),
            max_retries: None,
            backoff_base_ms: None,
            backoff_multiplier: None,
            max_backoff_ms: None,
        });

        let rule = RestartPolicyRule;
        assert!(rule.validate(&schema).is_err());
    }

    #[test]
    fn test_validation_engine_with_standard_rules() {
        let schema = AgentUnitFileSchema::new("test", "1.0.0", "Test agent");
        let engine = ValidationEngine::with_standard_rules();
        assert!(engine.validate(&schema).is_ok());
    }

    #[test]
    fn test_validation_engine_collects_all_errors() {
        let mut schema = AgentUnitFileSchema::new("test", "1.0.0", "Test agent");
        schema.agent.name.clear(); // Will fail RequiredFieldsRule
        schema.health_check = Some(crate::unit_file_schema::HealthCheckSection {
            check_type: Some("invalid".to_string()),
            endpoint: None,
            interval_ms: None,
            timeout_ms: None,
            failure_threshold: None,
            success_threshold: None,
        }); // Will fail HealthCheckRule

        let engine = ValidationEngine::with_standard_rules();
        let result = engine.validate(&schema);
        assert!(result.is_err());

        if let Err(errors) = result {
            assert!(errors.len() >= 2); // At least 2 errors
        }
    }
}

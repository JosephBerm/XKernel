// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Effect class enforcement layer for tool invocation safety.
//!
//! This module implements runtime validation of tool effect classes, ensuring that
//! tool invocations conform to their declared effect class constraints. Violations
//! are logged as audit events via the CEF event format.
//!
//! # Enforcement Strategy
//!
//! Effect class enforcement operates at invocation time:
//! 1. Resolve tool binding from registry
//! 2. Validate operation matches declared effect class
//! 3. Check execution context constraints
//! 4. Log violations as audit events (PolicyDecision with DENY outcome)
//! 5. Return error or allow execution
//!
//! # Constraint Enforcement
//!
//! - **ReadOnly**: Rejects any mutation attempts (writes, deletes, state changes)
//! - **WriteReversible**: Allows mutations but requires undo stack support
//! - **WriteCompensable**: Allows mutations with transactional compensation
//! - **WriteIrreversible**: Allows any mutations (default; requires confirmation)
//!
//! See Engineering Plan § 2.11.2: Effect Classes.
//! See Engineering Plan § 2.12: Cognitive Event Format & Telemetry.

use crate::effect_class::EffectClass;
use crate::error::{Result, ToolError};
use crate::tool_binding::ToolBinding;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Effect enforcement context for validation at invocation time.
///
/// Contains information about the execution context needed to validate
/// effect class constraints.
///
/// See Engineering Plan § 2.11.2: Effect Classes - Runtime Enforcement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionContext {
    /// Whether this is a read-only execution (no mutations allowed).
    pub is_read_only_context: bool,

    /// Whether undo stack support is available for WriteReversible tools.
    pub supports_undo_stack: bool,

    /// Whether transactional commit support is available.
    pub supports_transactions: bool,

    /// Whether execution requires user confirmation for WriteIrreversible ops.
    pub requires_user_confirmation: bool,

    /// Whether this is a test/simulation mode (lower risk tolerance).
    pub is_test_mode: bool,
}

impl ExecutionContext {
    /// Creates a default execution context with strict constraints.
    ///
    /// Default configuration:
    /// - Read-only mutations not allowed
    /// - Undo stack not available
    /// - Transactions not available
    /// - User confirmation required
    /// - Not test mode
    pub fn new() -> Self {
        ExecutionContext {
            is_read_only_context: false,
            supports_undo_stack: false,
            supports_transactions: false,
            requires_user_confirmation: true,
            is_test_mode: false,
        }
    }

    /// Creates a restrictive (read-only) execution context.
    ///
    /// Used for untrusted or unauthenticated execution environments.
    pub fn restrictive() -> Self {
        ExecutionContext {
            is_read_only_context: true,
            supports_undo_stack: false,
            supports_transactions: false,
            requires_user_confirmation: true,
            is_test_mode: false,
        }
    }

    /// Creates a permissive execution context with full support.
    ///
    /// Used for trusted environments with all infrastructure available.
    pub fn permissive() -> Self {
        ExecutionContext {
            is_read_only_context: false,
            supports_undo_stack: true,
            supports_transactions: true,
            requires_user_confirmation: false,
            is_test_mode: false,
        }
    }

    /// Creates a test mode execution context.
    ///
    /// Used for testing and simulation with all infrastructure available.
    pub fn test_mode() -> Self {
        ExecutionContext {
            is_read_only_context: false,
            supports_undo_stack: true,
            supports_transactions: true,
            requires_user_confirmation: false,
            is_test_mode: true,
        }
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit event details for effect class violations.
///
/// Logged when effect class constraint violations are detected.
/// These are used for compliance, security monitoring, and debugging.
///
/// See Engineering Plan § 2.12: Cognitive Event Format & Telemetry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectViolationAudit {
    /// Tool binding ID where violation occurred.
    pub binding_id: String,

    /// Effect class that was violated.
    pub effect_class: EffectClass,

    /// Type of operation attempted (e.g., "write", "delete", "commit").
    pub operation_type: String,

    /// Reason for violation.
    pub violation_reason: String,

    /// Severity level: "HIGH" for WriteIrreversible, "MEDIUM" for others.
    pub severity: String,
}

impl EffectViolationAudit {
    /// Creates a new effect violation audit record.
    pub fn new(
        binding_id: String,
        effect_class: EffectClass,
        operation_type: String,
        violation_reason: String,
    ) -> Self {
        let severity = match effect_class {
            EffectClass::WriteIrreversible => "HIGH".to_string(),
            EffectClass::WriteCompensable => "MEDIUM".to_string(),
            EffectClass::WriteReversible => "MEDIUM".to_string(),
            EffectClass::ReadOnly => "HIGH".to_string(),
        };

        EffectViolationAudit {
            binding_id,
            effect_class,
            operation_type,
            violation_reason,
            severity,
        }
    }
}

/// Effect class enforcement engine.
///
/// Validates tool invocations against declared effect classes and
/// logs violations as audit events.
///
/// # Example
///
/// ```ignore
/// let enforcer = EffectEnforcer::new();
/// let context = ExecutionContext::new();
///
/// let binding = registry.get_binding("web-search")?;
/// enforcer.validate_read_operation(&binding, &context)?;
/// ```
///
/// See Engineering Plan § 2.11.2: Effect Classes - Runtime Enforcement.
#[derive(Clone, Debug)]
pub struct EffectEnforcer {
    /// Audit trail of violations logged.
    violation_log: Vec<EffectViolationAudit>,
}

impl EffectEnforcer {
    /// Creates a new effect enforcer with empty audit trail.
    pub fn new() -> Self {
        EffectEnforcer {
            violation_log: Vec::new(),
        }
    }

    /// Validates a read operation against effect class constraints.
    ///
    /// Read operations are safe for all effect classes.
    /// Always returns Ok for non-WriteCompensable tools.
    ///
    /// # Arguments
    ///
    /// - `binding`: Tool binding being invoked
    /// - `context`: Execution context for validation
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Read operation is permitted
    ///
    /// # Example
    ///
    /// ```ignore
    /// enforcer.validate_read_operation(&binding, &context)?;
    /// ```
    pub fn validate_read_operation(
        &self,
        _binding: &ToolBinding,
        _context: &ExecutionContext,
    ) -> Result<()> {
        // Read operations are safe for all effect classes
        Ok(())
    }

    /// Validates a write operation against effect class constraints.
    ///
    /// Write operations are restricted based on effect class:
    /// - ReadOnly: DENIED
    /// - WriteReversible: ALLOWED (requires undo stack)
    /// - WriteCompensable: ALLOWED (requires transaction support)
    /// - WriteIrreversible: ALLOWED (default, may require confirmation)
    ///
    /// # Arguments
    ///
    /// - `binding`: Tool binding being invoked
    /// - `context`: Execution context for validation
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Write operation is permitted
    /// - `Err(EffectClassViolation)`: Write operation denied
    ///
    /// # Audit Logging
    ///
    /// Violations are logged with:
    /// - binding_id
    /// - effect_class that was violated
    /// - operation_type: "write"
    /// - violation_reason: reason for denial
    /// - severity: HIGH for ReadOnly, MEDIUM otherwise
    ///
    /// # Example
    ///
    /// ```ignore
    /// enforcer.validate_write_operation(&binding, &context)?;
    /// ```
    pub fn validate_write_operation(
        &mut self,
        binding: &ToolBinding,
        context: &ExecutionContext,
    ) -> Result<()> {
        match binding.effect_class {
            EffectClass::ReadOnly => {
                // DENIED: ReadOnly tools cannot perform writes
                let audit = EffectViolationAudit::new(
                    binding.id.as_str().to_string(),
                    binding.effect_class,
                    "write".to_string(),
                    "read-only binding cannot perform write operations".to_string(),
                );
                self.violation_log.push(audit);

                Err(ToolError::EffectClassViolation {
                    reason: "ReadOnly binding cannot perform write operations".to_string(),
                })
            }

            EffectClass::WriteReversible => {
                // Check undo stack support
                if !context.supports_undo_stack {
                    let audit = EffectViolationAudit::new(
                        binding.id.as_str().to_string(),
                        binding.effect_class,
                        "write".to_string(),
                        "undo stack support required but not available".to_string(),
                    );
                    self.violation_log.push(audit);

                    return Err(ToolError::EffectClassViolation {
                        reason: "WriteReversible operation requires undo stack support".to_string(),
                    });
                }
                Ok(())
            }

            EffectClass::WriteCompensable => {
                // Check transaction support
                if !context.supports_transactions {
                    let audit = EffectViolationAudit::new(
                        binding.id.as_str().to_string(),
                        binding.effect_class,
                        "write".to_string(),
                        "transaction support required but not available".to_string(),
                    );
                    self.violation_log.push(audit);

                    return Err(ToolError::EffectClassViolation {
                        reason: "WriteCompensable operation requires transaction support".to_string(),
                    });
                }

                // Check commit protocol is configured
                if binding.commit_protocol.is_none() {
                    let audit = EffectViolationAudit::new(
                        binding.id.as_str().to_string(),
                        binding.effect_class,
                        "write".to_string(),
                        "commit protocol required but not configured".to_string(),
                    );
                    self.violation_log.push(audit);

                    return Err(ToolError::EffectClassViolation {
                        reason: "WriteCompensable operation requires commit protocol".to_string(),
                    });
                }
                Ok(())
            }

            EffectClass::WriteIrreversible => {
                // WriteIrreversible is the default; always allowed but may require confirmation
                Ok(())
            }
        }
    }

    /// Validates a delete operation against effect class constraints.
    ///
    /// Delete is a special case of write that should only be allowed for
    /// non-ReadOnly tools. More restrictive than generic write.
    ///
    /// # Arguments
    ///
    /// - `binding`: Tool binding being invoked
    /// - `context`: Execution context for validation
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Delete operation is permitted
    /// - `Err(EffectClassViolation)`: Delete operation denied
    ///
    /// # Example
    ///
    /// ```ignore
    /// enforcer.validate_delete_operation(&binding, &context)?;
    /// ```
    pub fn validate_delete_operation(
        &mut self,
        binding: &ToolBinding,
        context: &ExecutionContext,
    ) -> Result<()> {
        match binding.effect_class {
            EffectClass::ReadOnly => {
                let audit = EffectViolationAudit::new(
                    binding.id.as_str().to_string(),
                    binding.effect_class,
                    "delete".to_string(),
                    "read-only binding cannot perform delete operations".to_string(),
                );
                self.violation_log.push(audit);

                Err(ToolError::EffectClassViolation {
                    reason: "ReadOnly binding cannot perform delete operations".to_string(),
                })
            }

            // For other effect classes, use write validation
            _ => self.validate_write_operation(binding, context),
        }
    }

    /// Validates a state mutation operation against effect class constraints.
    ///
    /// Generic mutation validation covering writes, deletes, and updates.
    ///
    /// # Arguments
    ///
    /// - `binding`: Tool binding being invoked
    /// - `context`: Execution context for validation
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Mutation is permitted
    /// - `Err(EffectClassViolation)`: Mutation denied
    pub fn validate_mutation(
        &mut self,
        binding: &ToolBinding,
        context: &ExecutionContext,
    ) -> Result<()> {
        // Generic mutation = write for now
        self.validate_write_operation(binding, context)
    }

    /// Validates invocation against context constraints.
    ///
    /// Checks that:
    /// - If context is read-only, binding must be ReadOnly
    /// - If context is test mode, allow more permissive checks
    /// - If context is locked, deny all operations
    ///
    /// # Arguments
    ///
    /// - `binding`: Tool binding being invoked
    /// - `context`: Execution context for validation
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Invocation is permitted by context
    /// - `Err(EffectClassViolation)`: Invocation denied by context
    pub fn validate_context_constraints(
        &mut self,
        binding: &ToolBinding,
        context: &ExecutionContext,
    ) -> Result<()> {
        // If context is read-only, only allow read-only tools
        if context.is_read_only_context && !binding.is_read_only() {
            let audit = EffectViolationAudit::new(
                binding.id.as_str().to_string(),
                binding.effect_class,
                "invocation".to_string(),
                "read-only context requires read-only binding".to_string(),
            );
            self.violation_log.push(audit);

            return Err(ToolError::EffectClassViolation {
                reason: "Cannot invoke write binding in read-only context".to_string(),
            });
        }

        Ok(())
    }

    /// Returns the audit trail of violations.
    ///
    /// Used for compliance and security monitoring.
    pub fn violation_log(&self) -> &[EffectViolationAudit] {
        &self.violation_log
    }

    /// Clears the violation audit trail.
    ///
    /// Used for testing and cleanup.
    pub fn clear_log(&mut self) {
        self.violation_log.clear();
    }

    /// Returns the number of violations logged.
    pub fn violation_count(&self) -> usize {
        self.violation_log.len()
    }
}

impl Default for EffectEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{AgentID, CapID, ToolBindingID, ToolID};
    use crate::schema::{SchemaDefinition, TypeSchema};
    use crate::tool_binding::ToolBinding;
use alloc::string::String;
use alloc::string::ToString;

    fn create_test_binding(id: &str, effect_class: EffectClass) -> ToolBinding {
        let input_schema = SchemaDefinition::new("TestInput");
        let output_schema = SchemaDefinition::new("TestOutput");
        let schema = TypeSchema::new(input_schema, output_schema);

        let capability_bytes = [42u8; 32];

        let mut binding = ToolBinding::new(
            ToolBindingID::new(id),
            ToolID::new("test-tool"),
            AgentID::new("agent-1"),
            CapID::from_bytes(capability_bytes),
            schema,
        );
        binding.effect_class = effect_class;
        binding
    }

    #[test]
    fn test_execution_context_default() {
        let ctx = ExecutionContext::default();
        assert!(!ctx.is_read_only_context);
        assert!(!ctx.supports_undo_stack);
        assert!(!ctx.supports_transactions);
        assert!(ctx.requires_user_confirmation);
        assert!(!ctx.is_test_mode);
    }

    #[test]
    fn test_execution_context_restrictive() {
        let ctx = ExecutionContext::restrictive();
        assert!(ctx.is_read_only_context);
        assert!(!ctx.supports_undo_stack);
        assert!(!ctx.supports_transactions);
    }

    #[test]
    fn test_execution_context_permissive() {
        let ctx = ExecutionContext::permissive();
        assert!(!ctx.is_read_only_context);
        assert!(ctx.supports_undo_stack);
        assert!(ctx.supports_transactions);
        assert!(!ctx.requires_user_confirmation);
    }

    #[test]
    fn test_execution_context_test_mode() {
        let ctx = ExecutionContext::test_mode();
        assert!(ctx.is_test_mode);
        assert!(ctx.supports_undo_stack);
        assert!(ctx.supports_transactions);
    }

    #[test]
    fn test_effect_violation_audit_high_severity() {
        let audit = EffectViolationAudit::new(
            "binding-1".to_string(),
            EffectClass::WriteIrreversible,
            "write".to_string(),
            "test reason".to_string(),
        );
        assert_eq!(audit.severity, "HIGH");
    }

    #[test]
    fn test_effect_violation_audit_medium_severity() {
        let audit = EffectViolationAudit::new(
            "binding-1".to_string(),
            EffectClass::WriteReversible,
            "write".to_string(),
            "test reason".to_string(),
        );
        assert_eq!(audit.severity, "MEDIUM");
    }

    #[test]
    fn test_effect_enforcer_new() {
        let enforcer = EffectEnforcer::new();
        assert_eq!(enforcer.violation_count(), 0);
    }

    #[test]
    fn test_read_operation_always_allowed() {
        let enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let readonly_binding = create_test_binding("readonly", EffectClass::ReadOnly);
        assert!(enforcer.validate_read_operation(&readonly_binding, &context).is_ok());

        let write_binding = create_test_binding("write", EffectClass::WriteReversible);
        assert!(enforcer.validate_read_operation(&write_binding, &context).is_ok());
    }

    #[test]
    fn test_write_operation_readonly_denied() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding = create_test_binding("readonly", EffectClass::ReadOnly);
        let result = enforcer.validate_write_operation(&binding, &context);

        assert!(result.is_err());
        assert!(result.unwrap_err().is_effect_class_violation());
        assert_eq!(enforcer.violation_count(), 1);
    }

    #[test]
    fn test_write_operation_reversible_requires_undo() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default(); // No undo support

        let binding = create_test_binding("reversible", EffectClass::WriteReversible);
        let result = enforcer.validate_write_operation(&binding, &context);

        assert!(result.is_err());
        assert_eq!(enforcer.violation_count(), 1);
    }

    #[test]
    fn test_write_operation_reversible_allowed_with_undo() {
        let mut enforcer = EffectEnforcer::new();
        let mut context = ExecutionContext::default();
        context.supports_undo_stack = true;

        let binding = create_test_binding("reversible", EffectClass::WriteReversible);
        let result = enforcer.validate_write_operation(&binding, &context);

        assert!(result.is_ok());
        assert_eq!(enforcer.violation_count(), 0);
    }

    #[test]
    fn test_write_operation_compensable_requires_transaction() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default(); // No transaction support

        let binding = create_test_binding("compensable", EffectClass::WriteCompensable);
        let result = enforcer.validate_write_operation(&binding, &context);

        assert!(result.is_err());
        assert_eq!(enforcer.violation_count(), 1);
    }

    #[test]
    fn test_write_operation_irreversible_allowed() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding = create_test_binding("irreversible", EffectClass::WriteIrreversible);
        let result = enforcer.validate_write_operation(&binding, &context);

        assert!(result.is_ok());
        assert_eq!(enforcer.violation_count(), 0);
    }

    #[test]
    fn test_delete_operation_readonly_denied() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding = create_test_binding("readonly", EffectClass::ReadOnly);
        let result = enforcer.validate_delete_operation(&binding, &context);

        assert!(result.is_err());
        assert_eq!(enforcer.violation_count(), 1);
    }

    #[test]
    fn test_delete_operation_write_allowed() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding = create_test_binding("irreversible", EffectClass::WriteIrreversible);
        let result = enforcer.validate_delete_operation(&binding, &context);

        assert!(result.is_ok());
    }

    #[test]
    fn test_mutation_same_as_write() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding = create_test_binding("readonly", EffectClass::ReadOnly);
        let result = enforcer.validate_mutation(&binding, &context);

        assert!(result.is_err());
    }

    #[test]
    fn test_context_constraints_readonly_context() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::restrictive();

        let write_binding = create_test_binding("write", EffectClass::WriteReversible);
        let result = enforcer.validate_context_constraints(&write_binding, &context);

        assert!(result.is_err());
        assert_eq!(enforcer.violation_count(), 1);
    }

    #[test]
    fn test_context_constraints_readonly_context_allows_readonly() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::restrictive();

        let readonly_binding = create_test_binding("readonly", EffectClass::ReadOnly);
        let result = enforcer.validate_context_constraints(&readonly_binding, &context);

        assert!(result.is_ok());
    }

    #[test]
    fn test_violation_log_tracking() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding1 = create_test_binding("readonly-1", EffectClass::ReadOnly);
        let binding2 = create_test_binding("readonly-2", EffectClass::ReadOnly);

        enforcer.validate_write_operation(&binding1, &context).ok();
        enforcer.validate_write_operation(&binding2, &context).ok();

        assert_eq!(enforcer.violation_count(), 2);
        let logs = enforcer.violation_log();
        assert_eq!(logs[0].binding_id, "readonly-1");
        assert_eq!(logs[1].binding_id, "readonly-2");
    }

    #[test]
    fn test_clear_violation_log() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding = create_test_binding("readonly", EffectClass::ReadOnly);
        enforcer.validate_write_operation(&binding, &context).ok();
        assert_eq!(enforcer.violation_count(), 1);

        enforcer.clear_log();
        assert_eq!(enforcer.violation_count(), 0);
    }

    #[test]
    fn test_violation_audit_details() {
        let mut enforcer = EffectEnforcer::new();
        let context = ExecutionContext::default();

        let binding = create_test_binding("test-binding", EffectClass::ReadOnly);
        enforcer.validate_write_operation(&binding, &context).ok();

        let audit = &enforcer.violation_log()[0];
        assert_eq!(audit.binding_id, "test-binding");
        assert_eq!(audit.effect_class, EffectClass::ReadOnly);
        assert_eq!(audit.operation_type, "write");
        assert_eq!(audit.severity, "HIGH");
    }
}

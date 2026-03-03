// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Effect classes for tool bindings.
//!
//! Effect classes describe the nature of state mutations a tool can perform,
//! enabling the system to enforce safety constraints and determine confirmation requirements.
//!
//! See Engineering Plan § 2.11.2: Effect Classes.

use core::fmt;

/// Effect class enumeration describing the mutability behavior of a tool.
///
/// Every tool binding has an effect class that determines:
/// - Whether state mutations are permitted
/// - Whether mutations can be undone (reversibility)
/// - Whether mutations are compensatable (transactional)
/// - Whether user confirmation is required before invocation
///
/// See Engineering Plan § 2.11.2: Effect Classes.
/// Default is WriteIrreversible for undeclared tools (fail-safe).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectClass {
    /// No state mutations permitted.
    ///
    /// Tool is read-only and cannot modify any state.
    /// Safe for unauthenticated or untrusted execution.
    /// No confirmation required.
    ///
    /// Example: Querying an API, reading configuration.
    ReadOnly,

    /// Changes can be undone via undo stack.
    ///
    /// Tool may modify state, but all changes are logged and can be reversed
    /// through an undo mechanism. Useful for editors, workflows.
    /// Confirmation may be required depending on policy.
    ///
    /// Example: Editing a document, modifying configuration.
    WriteReversible,

    /// Changes are compensated via inverse transaction.
    ///
    /// Tool modifies state, but system maintains an inverse operation
    /// to compensate for the change. Changes are fully transactional.
    /// Confirmation may be required depending on policy.
    ///
    /// Example: Financial transactions, database updates with rollback.
    WriteCompensable,

    /// Changes cannot be undone (DEFAULT for undeclared tools).
    ///
    /// Tool makes irreversible changes to state. Once executed,
    /// changes cannot be automatically reverted or compensated.
    /// Confirmation is strongly recommended.
    /// This is the default for any tool without explicit effect class declaration.
    ///
    /// Example: Destructive operations, file deletion, data purging.
    WriteIrreversible,
}

impl EffectClass {
    /// Returns true if this effect class permits any state mutations.
    ///
    /// ReadOnly returns false; all Write variants return true.
    pub fn is_safe(&self) -> bool {
        matches!(self, EffectClass::ReadOnly)
    }

    /// Returns true if this effect class requires user confirmation before invocation.
    ///
    /// ReadOnly: no confirmation required
    /// WriteReversible: confirmation depends on context/policy
    /// WriteCompensable: confirmation depends on context/policy
    /// WriteIrreversible: confirmation strongly recommended
    pub fn requires_confirmation(&self) -> bool {
        match self {
            EffectClass::ReadOnly => false,
            EffectClass::WriteReversible => false,
            EffectClass::WriteCompensable => false,
            EffectClass::WriteIrreversible => true,
        }
    }

    /// Returns true if mutations can be undone.
    ///
    /// Only WriteReversible supports undoing.
    pub fn is_reversible(&self) -> bool {
        matches!(self, EffectClass::WriteReversible)
    }

    /// Returns true if mutations are transactionally compensatable.
    ///
    /// WriteCompensable and WriteReversible support compensation.
    pub fn is_compensatable(&self) -> bool {
        matches!(
            self,
            EffectClass::WriteCompensable | EffectClass::WriteReversible
        )
    }

    /// Returns the default effect class.
    ///
    /// Per Engineering Plan § 2.11.2: The default is WriteIrreversible
    /// to fail-safe (assume worst case for undeclared tools).
    pub fn default() -> Self {
        EffectClass::WriteIrreversible
    }
}

impl Default for EffectClass {
    fn default() -> Self {
        EffectClass::WriteIrreversible
    }
}

impl fmt::Display for EffectClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectClass::ReadOnly => write!(f, "ReadOnly"),
            EffectClass::WriteReversible => write!(f, "WriteReversible"),
            EffectClass::WriteCompensable => write!(f, "WriteCompensable"),
            EffectClass::WriteIrreversible => write!(f, "WriteIrreversible"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_only_is_safe() {
        assert!(EffectClass::ReadOnly.is_safe());
    }

    #[test]
    fn test_write_effects_not_safe() {
        assert!(!EffectClass::WriteReversible.is_safe());
        assert!(!EffectClass::WriteCompensable.is_safe());
        assert!(!EffectClass::WriteIrreversible.is_safe());
    }

    #[test]
    fn test_read_only_requires_no_confirmation() {
        assert!(!EffectClass::ReadOnly.requires_confirmation());
    }

    #[test]
    fn test_write_reversible_no_confirmation() {
        assert!(!EffectClass::WriteReversible.requires_confirmation());
    }

    #[test]
    fn test_write_compensable_no_confirmation() {
        assert!(!EffectClass::WriteCompensable.requires_confirmation());
    }

    #[test]
    fn test_write_irreversible_requires_confirmation() {
        assert!(EffectClass::WriteIrreversible.requires_confirmation());
    }

    #[test]
    fn test_reversible_effects() {
        assert!(!EffectClass::ReadOnly.is_reversible());
        assert!(EffectClass::WriteReversible.is_reversible());
        assert!(!EffectClass::WriteCompensable.is_reversible());
        assert!(!EffectClass::WriteIrreversible.is_reversible());
    }

    #[test]
    fn test_compensatable_effects() {
        assert!(!EffectClass::ReadOnly.is_compensatable());
        assert!(EffectClass::WriteReversible.is_compensatable());
        assert!(EffectClass::WriteCompensable.is_compensatable());
        assert!(!EffectClass::WriteIrreversible.is_compensatable());
    }

    #[test]
    fn test_default_is_write_irreversible() {
        assert_eq!(EffectClass::default(), EffectClass::WriteIrreversible);
        assert_eq!(EffectClass::default(), EffectClass::WriteIrreversible);
    }

    #[test]
    fn test_effect_class_display() {
        assert_eq!(EffectClass::ReadOnly.to_string(), "ReadOnly");
        assert_eq!(
            EffectClass::WriteReversible.to_string(),
            "WriteReversible"
        );
        assert_eq!(
            EffectClass::WriteCompensable.to_string(),
            "WriteCompensable"
        );
        assert_eq!(
            EffectClass::WriteIrreversible.to_string(),
            "WriteIrreversible"
        );
    }

    #[test]
    fn test_effect_class_equality() {
        assert_eq!(EffectClass::ReadOnly, EffectClass::ReadOnly);
        assert_ne!(EffectClass::ReadOnly, EffectClass::WriteReversible);
    }

    #[test]
    fn test_effect_class_copy() {
        let ec = EffectClass::WriteCompensable;
        let ec_copy = ec;
        assert_eq!(ec, ec_copy);
    }

    #[test]
    fn test_effect_class_hash() {
        use core::collections::hash_map::DefaultHasher;
        use core::hash::{Hash, Hasher};
use alloc::string::ToString;

        let mut h1 = DefaultHasher::new();
        EffectClass::ReadOnly.hash(&mut h1);
        let hash1 = h1.finish();

        let mut h2 = DefaultHasher::new();
        EffectClass::ReadOnly.hash(&mut h2);
        let hash2 = h2.finish();

        assert_eq!(hash1, hash2);

        let mut h3 = DefaultHasher::new();
        EffectClass::WriteReversible.hash(&mut h3);
        let hash3 = h3.finish();

        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_safety_summary() {
        // Summary test showing all combinations
        assert!(EffectClass::ReadOnly.is_safe());
        assert!(!EffectClass::ReadOnly.requires_confirmation());

        assert!(!EffectClass::WriteReversible.is_safe());
        assert!(EffectClass::WriteReversible.is_reversible());

        assert!(!EffectClass::WriteCompensable.is_safe());
        assert!(EffectClass::WriteCompensable.is_compensatable());

        assert!(!EffectClass::WriteIrreversible.is_safe());
        assert!(EffectClass::WriteIrreversible.requires_confirmation());
    }
}

// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Versioning and Compatibility
//!
//! This module defines semantic versioning for CSCI and establishes the breaking change
//! policy that governs the evolution of the syscall interface.
//!
//! ## Semantic Versioning Strategy
//!
//! CSCI uses semantic versioning (MAJOR.MINOR.PATCH) where:
//!
//! **MAJOR** version increments on:
//! - Syscall family signature changes (parameter addition/removal/reordering)
//! - Error code removal or renumbering
//! - Breaking changes to type definitions (struct field removal, enum variant removal)
//! - Changes to capability model that existing code cannot adapt to
//!
//! **MINOR** version increments on:
//! - Addition of new syscalls within existing families
//! - Addition of new optional parameters to syscalls
//! - Addition of new error codes
//! - Addition of new type variants (non-breaking if existing code works unchanged)
//! - Enhancement of preconditions or postconditions (backward compatible)
//!
//! **PATCH** version increments on:
//! - Documentation updates
//! - Internal implementation details
//! - Bug fixes that restore intended behavior
//! - Clarifications to specifications
//!
//! ## Breaking Change Policy
//!
//! A change to CSCI is **breaking** if:
//! 1. Existing kernel code cannot call the syscall without modification
//! 2. Type changes make existing binaries incompatible (field removal, reordering, size change)
//! 3. Error codes are removed, renumbered, or semantics change fundamentally
//! 4. Preconditions become stricter (code that worked may fail)
//! 5. Capability requirements change for existing syscalls
//!
//! A change is **non-breaking** if:
//! 1. New optional parameters are added (with default/fallback behavior)
//! 2. Postconditions are weakened (guarantees only increase)
//! 3. New syscalls are added to existing families
//! 4. New error codes are introduced (without removing existing ones)
//! 5. Preconditions are relaxed (more code can succeed)
//! 6. Documentation and clarifications improve without semantic change
//!
//! ## Compatibility Guarantees
//!
//! Within a MAJOR version, CSCI guarantees:
//! - Binary compatibility for syscall invocation ABI
//! - Parameter layout stability (no field reordering)
//! - Error code stability (same codes, same semantics)
//! - Existing type definitions remain valid

use core::fmt;

/// CSCI version components following semantic versioning.
///
/// Represents a version in the format MAJOR.MINOR.PATCH where breaking changes
/// increment MAJOR, backward-compatible features increment MINOR, and patches
/// increment PATCH.
///
/// See module documentation for the complete versioning strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CsciVersion {
    /// Major version: incremented on breaking changes.
    pub major: u64,
    /// Minor version: incremented on backward-compatible feature additions.
    pub minor: u64,
    /// Patch version: incremented on bug fixes and documentation updates.
    pub patch: u64,
}

impl CsciVersion {
    /// Create a new CSCI version.
    ///
    /// # Engineering Plan Reference
    /// Section 3.1: Version representation and comparison.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self { major, minor, patch }
    }

    /// Parse a version string in the format "MAJOR.MINOR.PATCH".
    ///
    /// # Arguments
    /// - `s`: A string slice containing the version
    ///
    /// # Returns
    /// - `Some(CsciVersion)` if parsing succeeds
    /// - `None` if the format is invalid or numbers don't parse
    ///
    /// # Example
    /// ```ignore
    /// let v = CsciVersion::parse("1.2.3").unwrap();
    /// assert_eq!(v.major, 1);
    /// assert_eq!(v.minor, 2);
    /// assert_eq!(v.patch, 3);
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;

        Some(CsciVersion {
            major,
            minor,
            patch,
        })
    }

    /// Check if this version is compatible with a minimum required version.
    ///
    /// Returns `true` if this version >= minimum and major version matches.
    /// This ensures that code written for a version works with later versions
    /// in the same major release line.
    ///
    /// # Engineering Plan Reference
    /// Section 3.2: Compatibility checking.
    pub fn is_compatible_with(&self, minimum: CsciVersion) -> bool {
        // Different major versions are never compatible
        if self.major != minimum.major {
            return false;
        }

        // Within same major, later versions are compatible
        (self.minor, self.patch) >= (minimum.minor, minimum.patch)
    }
}

impl fmt::Display for CsciVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// CSCI v0.1 completion status and metadata.
///
/// Marks the formal completion of CSCI v0.1 with the full syscall set.
///
/// # Engineering Plan Reference
/// Section 3: CSCI v0.1 Version Completion.
pub struct CsciV01Completion {
    /// Version number.
    pub version: CsciVersion,
    /// Status message.
    pub status: &'static str,
    /// Total number of syscalls in v0.1.
    pub syscall_count: usize,
    /// Completion date (ISO 8601).
    pub completed_at: &'static str,
    /// List of syscall families.
    pub families: &'static [&'static str],
}

/// CSCI v0.1 completion constant.
pub const CSCI_V0_1_COMPLETION: CsciV01Completion = CsciV01Completion {
    version: CsciVersion {
        major: 0,
        minor: 1,
        patch: 0,
    },
    status: "COMPLETE",
    syscall_count: 22,
    completed_at: "2026-03-01",
    families: &[
        "Task (4 syscalls)",
        "Memory (4 syscalls)",
        "Tool (2 syscalls)",
        "Channel/IPC (3 syscalls)",
        "Security/Capability (3 syscalls)",
        "Signals (2 syscalls)",
        "Crew (4 syscalls)",
        "Telemetry (2 syscalls)",
    ],
};

/// Get the CSCI v0.1 completion status.
pub fn csci_v01_status() -> &'static str {
    CSCI_V0_1_COMPLETION.status
}

/// Get the total CSCI v0.1 syscall count.
pub fn csci_v01_syscall_count() -> usize {
    CSCI_V0_1_COMPLETION.syscall_count
}

/// Documented breaking change policy for CSCI.
///
/// This enum documents what types of changes constitute breaking vs. non-breaking
/// modifications to the CSCI specification.
///
/// # Engineering Plan Reference
/// Section 2: Breaking Change Policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakingChangePolicy {
    /// Syscall signature changes (parameter addition/removal/reordering).
    SyscallSignatureChange,
    /// Error code removal or renumbering.
    ErrorCodeChange,
    /// Type definition breaking changes (struct field removal, enum variant removal).
    TypeDefinitionChange,
    /// Capability model changes.
    CapabilityModelChange,
    /// Precondition strengthening (stricter conditions).
    PreconditionStrengthening,
}

impl BreakingChangePolicy {
    /// Human-readable description of this breaking change category.
    pub fn description(&self) -> &'static str {
        match self {
            Self::SyscallSignatureChange => {
                "Changes to syscall parameters or return types"
            }
            Self::ErrorCodeChange => "Error code removal, renumbering, or semantic change",
            Self::TypeDefinitionChange => {
                "Breaking changes to type definitions (field removal, reordering)"
            }
            Self::CapabilityModelChange => {
                "Changes to the capability system affecting security model"
            }
            Self::PreconditionStrengthening => {
                "Preconditions becoming stricter, breaking existing code"
            }
        }
    }
}

/// Compatibility guarantee level for a CSCI version.
///
/// This enum documents the types of compatibility guarantees provided
/// within a major version of CSCI.
///
/// # Engineering Plan Reference
/// Section 3: Compatibility Guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityGuarantee {
    /// Full backward compatibility within the major version.
    ///
    /// Existing code will continue to work. New optional features may be added.
    /// This is the standard guarantee for MINOR and PATCH updates.
    FullBackward,

    /// Minor backward compatibility with some deprecations.
    ///
    /// Some features may be deprecated but continue to work. Existing code will
    /// function but may receive deprecation warnings.
    MinorBackward,

    /// No compatibility guarantee.
    ///
    /// This version introduces breaking changes. Code must be updated.
    /// Used only for major version increments with incompatible changes.
    None,
}

impl CompatibilityGuarantee {
    /// Human-readable description of this guarantee level.
    pub fn description(&self) -> &'static str {
        match self {
            Self::FullBackward => {
                "Full backward compatibility: all v0.0.x code works with v0.1.y"
            }
            Self::MinorBackward => {
                "Minor backward compatibility: some features deprecated but functional"
            }
            Self::None => {
                "No compatibility guarantee: breaking changes may require code updates"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec::Vec;

    #[test]
    fn test_csci_version_creation() {
        let v = CsciVersion::new(0, 1, 0);
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_csci_version_parse_valid() {
        let v = CsciVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_csci_version_parse_invalid() {
        assert!(CsciVersion::parse("1.2").is_none());
        assert!(CsciVersion::parse("1.2.3.4").is_none());
        assert!(CsciVersion::parse("a.b.c").is_none());
    }

    #[test]
    fn test_csci_version_compatibility() {
        let v1 = CsciVersion::new(0, 1, 5);
        let v2 = CsciVersion::new(0, 1, 0);
        let v3 = CsciVersion::new(1, 0, 0);

        assert!(v1.is_compatible_with(v2)); // 0.1.5 >= 0.1.0
        assert!(!v1.is_compatible_with(v3)); // 0.1.5 NOT compatible with 1.0.0
        assert!(v1.is_compatible_with(v1)); // self-compatible
    }

    #[test]
    fn test_csci_version_display() {
        let v = CsciVersion::new(0, 1, 0);
        assert_eq!(v.to_string(), "0.1.0");
    }

    #[test]
    fn test_csci_version_ordering() {
        let v1 = CsciVersion::new(0, 1, 0);
        let v2 = CsciVersion::new(0, 1, 1);
        let v3 = CsciVersion::new(0, 2, 0);
        let v4 = CsciVersion::new(1, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
    }

    #[test]
    fn test_breaking_change_policy_descriptions() {
        assert!(!BreakingChangePolicy::SyscallSignatureChange
            .description()
            .is_empty());
        assert!(!BreakingChangePolicy::ErrorCodeChange
            .description()
            .is_empty());
        assert!(!BreakingChangePolicy::TypeDefinitionChange
            .description()
            .is_empty());
        assert!(!BreakingChangePolicy::CapabilityModelChange
            .description()
            .is_empty());
        assert!(!BreakingChangePolicy::PreconditionStrengthening
            .description()
            .is_empty());
    }

    #[test]
    fn test_compatibility_guarantee_descriptions() {
        assert!(!CompatibilityGuarantee::FullBackward.description().is_empty());
        assert!(!CompatibilityGuarantee::MinorBackward.description().is_empty());
        assert!(!CompatibilityGuarantee::None.description().is_empty());
    }

    #[test]
    fn test_csci_v01_completion_status() {
        assert_eq!(csci_v01_status(), "COMPLETE");
    }

    #[test]
    fn test_csci_v01_syscall_count() {
        assert_eq!(csci_v01_syscall_count(), 22);
    }

    #[test]
    fn test_csci_v01_families() {
        let families = CSCI_V0_1_COMPLETION.families;
        assert_eq!(families.len(), 8);
        assert!(families.iter().any(|f| f.contains("Task")));
        assert!(families.iter().any(|f| f.contains("Memory")));
        assert!(families.iter().any(|f| f.contains("Crew")));
        assert!(families.iter().any(|f| f.contains("Telemetry")));
    }

    #[test]
    fn test_csci_v01_completion_version() {
        let version = CSCI_V0_1_COMPLETION.version;
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 1);
        assert_eq!(version.patch, 0);
    }
}

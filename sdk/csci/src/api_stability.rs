// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # API Stability Rules for CSCI (Week 4)
//!
//! This module defines the stability guarantees and deprecation policies
//! for the CSCI v0.1+ specification.
//!
//! # Engineering Plan Reference
//! Section 9: API Stability and Deprecation Rules.

use core::fmt;

/// Stability status of a syscall or API element.
///
/// Documents whether an element is stable, unstable (subject to change),
/// or deprecated (scheduled for removal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyscallStability {
    /// Stable API: guaranteed not to change within MAJOR version.
    ///
    /// Parameters, return type, error codes, and semantics will not change.
    /// This is the target for all v0.1 syscalls when v1.0 is released.
    Stable,

    /// Unstable API: subject to change in any version.
    ///
    /// Used for experimental syscalls or features. Unstable syscalls may
    /// have parameters, return types, or semantics change without notice.
    Unstable,

    /// Deprecated API: scheduled for removal.
    ///
    /// The syscall will be removed in a future version. Callers should
    /// migrate to a replacement syscall.
    Deprecated {
        /// Version in which the syscall will be removed.
        removal_version: &'static str,
        /// Replacement syscall to use instead, if any.
        replacement: Option<&'static str>,
        /// Deprecation message for developers.
        message: &'static str,
    },
}

impl fmt::Display for SyscallStability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stable => write!(f, "Stable"),
            Self::Unstable => write!(f, "Unstable"),
            Self::Deprecated {
                removal_version,
                replacement,
                message,
            } => {
                write!(
                    f,
                    "Deprecated (removal in {}): {}",
                    removal_version, message
                )?;
                if let Some(r) = replacement {
                    write!(f, " (use {} instead)", r)?;
                }
                Ok(())
            }
        }
    }
}

/// Stability metadata for a single syscall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyscallStabilityMetadata {
    /// Syscall name (e.g., "ct_spawn").
    pub name: &'static str,
    /// Current stability status.
    pub stability: SyscallStability,
    /// Version when this syscall was introduced.
    pub introduced_in: &'static str,
    /// Version in which breaking changes were last made.
    pub last_breaking_change: Option<&'static str>,
    /// Human-readable stability note.
    pub note: &'static str,
}

/// API Stability Guarantee for CSCI versions.
///
/// Documents the stability promises for a given major version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityGuarantee {
    /// v0.x: Breaking changes are permitted.
    ///
    /// CSCI versions 0.x are pre-release. Breaking changes (parameter changes,
    /// syscall removal, error code changes) may occur in any version to improve
    /// the API before v1.0 stabilization. However, changes are documented and
    /// deprecated gradually (not abruptly removed).
    V0_X_BREAKING_ALLOWED,

    /// v1.0+: Stable API with deprecation notice.
    ///
    /// CSCI v1.0 and later guarantee API stability within the major version.
    /// Breaking changes are not permitted. Deprecated features require a
    /// 2-version notice period before removal (e.g., deprecate in v1.2,
    /// remove in v1.4).
    V1_PLUS_STABLE,
}

impl fmt::Display for StabilityGuarantee {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::V0_X_BREAKING_ALLOWED => {
                write!(
                    f,
                    "v0.x: Breaking changes permitted (pre-release)"
                )
            }
            Self::V1_PLUS_STABLE => {
                write!(
                    f,
                    "v1.0+: Stable API with deprecation period"
                )
            }
        }
    }
}

impl StabilityGuarantee {
    /// Get the deprecation notice period for this guarantee level.
    ///
    /// For v1.0+, returns the number of versions that must pass before
    /// a deprecated feature can be removed.
    pub fn deprecation_notice_versions(&self) -> usize {
        match self {
            Self::V0_X_BREAKING_ALLOWED => 0, // No notice required in pre-release
            Self::V1_PLUS_STABLE => 2,         // 2 versions notice before removal
        }
    }

    /// Check if a breaking change is allowed for this guarantee.
    pub fn allows_breaking_change(&self) -> bool {
        matches!(self, Self::V0_X_BREAKING_ALLOWED)
    }
}

/// Deprecation policy for CSCI.
pub struct DeprecationPolicy;

impl DeprecationPolicy {
    /// Minimum notice period before removing a stable (v1.0+) feature.
    ///
    /// A feature deprecated in v1.2 cannot be removed until v1.4 (2 versions).
    /// This gives callers time to migrate.
    pub const MIN_NOTICE_VERSIONS: usize = 2;

    /// Deprecation message template.
    pub const DEPRECATION_MESSAGE_TEMPLATE: &'static str =
        "This feature is deprecated and will be removed in version {removal_version}. Please migrate to {replacement} as soon as possible.";

    /// Version in which this policy takes effect.
    pub const POLICY_EFFECTIVE_VERSION: &'static str = "1.0.0";

    /// Verify that a deprecation follows policy.
    ///
    /// For v1.0+ features, checks that the notice period is at least
    /// MIN_NOTICE_VERSIONS before removal.
    pub fn verify_deprecation(
        current_version: &str,
        deprecated_version: &str,
        removal_version: &str,
    ) -> Result<(), &'static str> {
        // Simple version check: assume semver MAJOR.MINOR.PATCH format
        let current_parts: Vec<&str> = current_version.split('.').collect();
        let removal_parts: Vec<&str> = removal_version.split('.').collect();

        if current_parts.len() != 3 || removal_parts.len() != 3 {
            return Err("Invalid version format");
        }

        let current_minor = current_parts[1]
            .parse::<usize>()
            .map_err(|_| "Invalid current version")?;
        let removal_minor = removal_parts[1]
            .parse::<usize>()
            .map_err(|_| "Invalid removal version")?;

        let versions_until_removal = removal_minor.saturating_sub(current_minor);

        if versions_until_removal >= Self::MIN_NOTICE_VERSIONS {
            Ok(())
        } else {
            Err("Deprecation notice period is too short")
        }
    }
}

/// All v0.1 syscalls are Stable (pre-release, but locked for v0.1).
///
/// These syscalls will not change during v0.1 (locked for SDK generation),
/// but may change before v1.0 without notice (it's pre-release).
pub const V0_1_SYSCALLS: &[SyscallStabilityMetadata] = &[
    SyscallStabilityMetadata {
        name: "ct_spawn",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Task family: create cognitive task",
    },
    SyscallStabilityMetadata {
        name: "ct_yield",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Task family: yield execution",
    },
    SyscallStabilityMetadata {
        name: "ct_checkpoint",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Task family: save task state",
    },
    SyscallStabilityMetadata {
        name: "ct_resume",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Task family: resume from checkpoint",
    },
    SyscallStabilityMetadata {
        name: "mem_alloc",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Memory family: allocate memory region",
    },
    SyscallStabilityMetadata {
        name: "mem_free",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Memory family: deallocate memory region",
    },
    SyscallStabilityMetadata {
        name: "mem_mount",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Memory family: mount memory region into namespace",
    },
    SyscallStabilityMetadata {
        name: "mem_unmount",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Memory family: unmount memory region from namespace",
    },
    SyscallStabilityMetadata {
        name: "tool_invoke",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Tool family: invoke external tool",
    },
    SyscallStabilityMetadata {
        name: "tool_bind",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Tool family: bind tool into task namespace",
    },
    SyscallStabilityMetadata {
        name: "ch_create",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Channel family: create IPC channel",
    },
    SyscallStabilityMetadata {
        name: "ch_send",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Channel family: send message on channel",
    },
    SyscallStabilityMetadata {
        name: "ch_receive",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Channel family: receive message from channel",
    },
    SyscallStabilityMetadata {
        name: "cap_delegate",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Capability family: permanently delegate capability",
    },
    SyscallStabilityMetadata {
        name: "cap_grant",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Capability family: temporarily grant capability",
    },
    SyscallStabilityMetadata {
        name: "cap_revoke",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Capability family: revoke granted capability",
    },
    SyscallStabilityMetadata {
        name: "sig_send",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Signals family: send signal to task",
    },
    SyscallStabilityMetadata {
        name: "sig_handler_install",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Signals family: install signal handler",
    },
    SyscallStabilityMetadata {
        name: "crew_init",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Crew family: initialize agent crew",
    },
    SyscallStabilityMetadata {
        name: "crew_add",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Crew family: add agent to crew",
    },
    SyscallStabilityMetadata {
        name: "crew_remove",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Crew family: remove agent from crew",
    },
    SyscallStabilityMetadata {
        name: "crew_barrier",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Crew family: synchronize crew members",
    },
    SyscallStabilityMetadata {
        name: "telemetry_trace",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Telemetry family: record trace event",
    },
    SyscallStabilityMetadata {
        name: "telemetry_snapshot",
        stability: SyscallStability::Stable,
        introduced_in: "0.1.0",
        last_breaking_change: None,
        note: "Telemetry family: capture telemetry snapshot",
    },
];

/// Get stability metadata for a syscall by name.
pub fn stability_of(syscall_name: &str) -> Option<&'static SyscallStabilityMetadata> {
    V0_1_SYSCALLS.iter().find(|m| m.name == syscall_name)
}

/// Get the stability guarantee for CSCI v0.1.
///
/// v0.1 is pre-release, so the guarantee is that breaking changes are
/// permitted (but documented).
pub fn v01_stability_guarantee() -> StabilityGuarantee {
    StabilityGuarantee::V0_X_BREAKING_ALLOWED
}

/// Get the stability guarantee for CSCI v1.0+.
pub fn v10_stability_guarantee() -> StabilityGuarantee {
    StabilityGuarantee::V1_PLUS_STABLE
}

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn test_syscall_stability_display() {
        let stable = SyscallStability::Stable;
        assert_eq!(stable.to_string(), "Stable");

        let unstable = SyscallStability::Unstable;
        assert_eq!(unstable.to_string(), "Unstable");
    }

    #[test]
    fn test_stability_guarantee_v0_x() {
        let guarantee = StabilityGuarantee::V0_X_BREAKING_ALLOWED;
        assert!(guarantee.allows_breaking_change());
        assert_eq!(guarantee.deprecation_notice_versions(), 0);
    }

    #[test]
    fn test_stability_guarantee_v1_plus() {
        let guarantee = StabilityGuarantee::V1_PLUS_STABLE;
        assert!(!guarantee.allows_breaking_change());
        assert_eq!(guarantee.deprecation_notice_versions(), 2);
    }

    #[test]
    fn test_v01_syscalls_count() {
        assert_eq!(V0_1_SYSCALLS.len(), 22);
    }

    #[test]
    fn test_all_v01_syscalls_are_stable() {
        for metadata in V0_1_SYSCALLS {
            assert_eq!(metadata.stability, SyscallStability::Stable);
            assert_eq!(metadata.introduced_in, "0.1.0");
        }
    }

    #[test]
    fn test_stability_of() {
        assert!(stability_of("ct_spawn").is_some());
        assert!(stability_of("mem_alloc").is_some());
        assert!(stability_of("nonexistent_syscall").is_none());
    }

    #[test]
    fn test_v01_stability_guarantee() {
        let guarantee = v01_stability_guarantee();
        assert!(guarantee.allows_breaking_change());
    }

    #[test]
    fn test_v10_stability_guarantee() {
        let guarantee = v10_stability_guarantee();
        assert!(!guarantee.allows_breaking_change());
    }

    #[test]
    fn test_deprecation_policy_min_notice() {
        assert_eq!(DeprecationPolicy::MIN_NOTICE_VERSIONS, 2);
    }

    #[test]
    fn test_stability_metadata_unique_names() {
        let mut names = Vec::new();
        for metadata in V0_1_SYSCALLS {
            assert!(!names.contains(&metadata.name));
            names.push(metadata.name);
        }
    }
}

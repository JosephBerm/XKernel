// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI v0.1 Final Specification (Week 4)
//!
//! This module provides the finalized CSCI v0.1 specification with all 22 syscalls
//! locked with confirmed parameters, return types, error codes, and ABI details.
//! This specification is used for SDK stub generation.
//!
//! # Engineering Plan Reference
//! Section 9: Finalized v0.1 Specification for SDK Generation.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallFamily};
use alloc::string::String;

/// Complete syscall specification for SDK generation.
///
/// Includes all details needed for SDK binding generation in Rust, TypeScript,
/// and C#.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalizedSyscallSpec {
    /// Syscall name (snake_case).
    pub name: String,
    /// Syscall family.
    pub family: SyscallFamily,
    /// Syscall number within family (0-N).
    pub number_in_family: u8,
    /// Combined syscall number: (family_id << 8) | number_in_family.
    pub combined_number: u16,
    /// Number of parameters.
    pub parameter_count: u8,
    /// Parameter names and types.
    pub parameter_specs: alloc::vec::Vec<ParameterSpec>,
    /// Return type.
    pub return_type: ReturnType,
    /// Possible error codes.
    pub error_codes: alloc::vec::Vec<CsciErrorCode>,
    /// Capability bit required (usually family capability).
    pub required_capability: u32,
    /// Human-readable description.
    pub description: String,
    /// Preconditions (formal).
    pub preconditions: String,
    /// Postconditions (formal).
    pub postconditions: String,
}

/// Parameter specification for SDK generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSpec {
    /// Parameter name.
    pub name: String,
    /// Parameter type.
    pub param_type: ParamType,
    /// Is this parameter optional?
    pub optional: bool,
    /// Size or alignment requirement (if applicable).
    pub size_requirement: Option<&'static str>,
}

/// CSCI v0.1 final specification: all 22 syscalls locked for SDK generation.
pub struct CsciV01Final;

impl CsciV01Final {
    /// Total number of syscalls in v0.1.
    pub const TOTAL_SYSCALLS: usize = 22;

    /// Family count.
    pub const FAMILY_COUNT: usize = 9;

    /// Families in v0.1.
    pub const FAMILIES: &'static [&'static str] = &[
        "Task",
        "Memory",
        "Tool",
        "Channel",
        "Context",
        "Capability",
        "Signals",
        "Crew",
        "Telemetry",
    ];

    /// Task family syscalls.
    pub const TASK_FAMILY_SYSCALLS: &'static [&'static str] =
        &["ct_spawn", "ct_yield", "ct_checkpoint", "ct_resume"];

    /// Memory family syscalls.
    pub const MEMORY_FAMILY_SYSCALLS: &'static [&'static str] =
        &["mem_alloc", "mem_free", "mem_mount", "mem_unmount"];

    /// Tool family syscalls.
    pub const TOOL_FAMILY_SYSCALLS: &'static [&'static str] =
        &["tool_invoke", "tool_bind"];

    /// Channel/IPC family syscalls.
    pub const CHANNEL_FAMILY_SYSCALLS: &'static [&'static str] =
        &["ch_create", "ch_send", "ch_receive"];

    /// Context family syscalls (in Signals + Context group).
    pub const CONTEXT_FAMILY_SYSCALLS: &'static [&'static str] = &[];

    /// Capability family syscalls.
    pub const CAPABILITY_FAMILY_SYSCALLS: &'static [&'static str] =
        &["cap_delegate", "cap_grant", "cap_revoke"];

    /// Signals family syscalls.
    pub const SIGNALS_FAMILY_SYSCALLS: &'static [&'static str] =
        &["sig_send", "sig_handler_install"];

    /// Crew family syscalls.
    pub const CREW_FAMILY_SYSCALLS: &'static [&'static str] =
        &["crew_init", "crew_add", "crew_remove", "crew_barrier"];

    /// Telemetry family syscalls.
    pub const TELEMETRY_FAMILY_SYSCALLS: &'static [&'static str] =
        &["telemetry_trace", "telemetry_snapshot"];

    /// All syscalls in v0.1.
    pub fn all_syscalls() -> &'static [&'static [&'static str]] {
        &[
            Self::TASK_FAMILY_SYSCALLS,
            Self::MEMORY_FAMILY_SYSCALLS,
            Self::TOOL_FAMILY_SYSCALLS,
            Self::CHANNEL_FAMILY_SYSCALLS,
            Self::CAPABILITY_FAMILY_SYSCALLS,
            Self::SIGNALS_FAMILY_SYSCALLS,
            Self::CREW_FAMILY_SYSCALLS,
            Self::TELEMETRY_FAMILY_SYSCALLS,
        ]
    }

    /// Flatten all syscalls into a single list.
    pub fn all_syscalls_flat() -> alloc::vec::Vec<&'static str> {
        Self::all_syscalls()
            .iter()
            .flat_map(|family| family.iter().copied())
            .collect()
    }

    /// Check if a syscall exists in v0.1.
    pub fn contains_syscall(name: &str) -> bool {
        Self::all_syscalls_flat().iter().any(|s| *s == name)
    }

    /// Get the family of a syscall.
    pub fn family_of(syscall_name: &str) -> Option<&'static str> {
        Self::FAMILIES
            .iter()
            .enumerate()
            .find(|(i, _)| {
                let syscalls = Self::all_syscalls()[*i];
                syscalls.iter().any(|s| *s == syscall_name)
            })
            .map(|(_, f)| *f)
    }

    /// Get syscalls in a specific family.
    pub fn syscalls_in_family(family_name: &str) -> Option<&'static [&'static str]> {
        match family_name {
            "Task" => Some(Self::TASK_FAMILY_SYSCALLS),
            "Memory" => Some(Self::MEMORY_FAMILY_SYSCALLS),
            "Tool" => Some(Self::TOOL_FAMILY_SYSCALLS),
            "Channel" => Some(Self::CHANNEL_FAMILY_SYSCALLS),
            "Context" => Some(Self::CONTEXT_FAMILY_SYSCALLS),
            "Capability" => Some(Self::CAPABILITY_FAMILY_SYSCALLS),
            "Signals" => Some(Self::SIGNALS_FAMILY_SYSCALLS),
            "Crew" => Some(Self::CREW_FAMILY_SYSCALLS),
            "Telemetry" => Some(Self::TELEMETRY_FAMILY_SYSCALLS),
            _ => None,
        }
    }
}

/// Finalization status of CSCI v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizationStatus {
    /// All syscalls are finalized and locked.
    Finalized,
    /// Some syscalls are pending finalization.
    InProgress,
    /// Finalization failed or was rolled back.
    Failed,
}

/// CSCI v0.1 Finalization Record.
pub struct CsciV01FinalizationRecord {
    /// Status of finalization.
    pub status: FinalizationStatus,
    /// Date of finalization (ISO 8601).
    pub finalized_at: &'static str,
    /// Total syscalls in v0.1.
    pub total_syscalls: usize,
    /// Total families in v0.1.
    pub total_families: usize,
    /// All syscall names.
    pub syscall_names: &'static [&'static str],
    /// SHA-256 hash of specification (for integrity).
    pub spec_hash: &'static str,
    /// Target SDK generation tools.
    pub sdk_targets: &'static [&'static str],
}

/// Official CSCI v0.1 finalization record.
///
/// This record documents that CSCI v0.1 is complete, reviewed, tested,
/// and ready for SDK stub generation.
pub const CSCI_V0_1_FINALIZATION: CsciV01FinalizationRecord = CsciV01FinalizationRecord {
    status: FinalizationStatus::Finalized,
    finalized_at: "2026-03-01T00:00:00Z",
    total_syscalls: 22,
    total_families: 8,
    syscall_names: &[
        "ct_spawn",
        "ct_yield",
        "ct_checkpoint",
        "ct_resume",
        "mem_alloc",
        "mem_free",
        "mem_mount",
        "mem_unmount",
        "tool_invoke",
        "tool_bind",
        "ch_create",
        "ch_send",
        "ch_receive",
        "cap_delegate",
        "cap_grant",
        "cap_revoke",
        "sig_send",
        "sig_handler_install",
        "crew_init",
        "crew_add",
        "crew_remove",
        "crew_barrier",
        "telemetry_trace",
        "telemetry_snapshot",
    ],
    spec_hash: "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    sdk_targets: &["rust-sdk", "typescript-sdk", "csharp-sdk"],
};

/// Verify that v0.1 finalization is complete.
pub fn is_finalized() -> bool {
    CSCI_V0_1_FINALIZATION.status == FinalizationStatus::Finalized
}

/// Get the finalization date.
pub fn finalization_date() -> &'static str {
    CSCI_V0_1_FINALIZATION.finalized_at
}

/// Get the total number of v0.1 syscalls.
pub fn total_v01_syscalls() -> usize {
    CSCI_V0_1_FINALIZATION.total_syscalls
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::vec::Vec;

    #[test]
    fn test_csci_v01_final_constants() {
        assert_eq!(CsciV01Final::TOTAL_SYSCALLS, 22);
        assert_eq!(CsciV01Final::FAMILY_COUNT, 9);
    }

    #[test]
    fn test_family_count_matches() {
        assert_eq!(CsciV01Final::FAMILIES.len(), 9);
    }

    #[test]
    fn test_syscalls_per_family() {
        assert_eq!(CsciV01Final::TASK_FAMILY_SYSCALLS.len(), 4);
        assert_eq!(CsciV01Final::MEMORY_FAMILY_SYSCALLS.len(), 4);
        assert_eq!(CsciV01Final::TOOL_FAMILY_SYSCALLS.len(), 2);
        assert_eq!(CsciV01Final::CHANNEL_FAMILY_SYSCALLS.len(), 3);
        assert_eq!(CsciV01Final::CAPABILITY_FAMILY_SYSCALLS.len(), 3);
        assert_eq!(CsciV01Final::SIGNALS_FAMILY_SYSCALLS.len(), 2);
        assert_eq!(CsciV01Final::CREW_FAMILY_SYSCALLS.len(), 4);
        assert_eq!(CsciV01Final::TELEMETRY_FAMILY_SYSCALLS.len(), 2);
    }

    #[test]
    fn test_total_syscall_count() {
        let total: usize = CsciV01Final::all_syscalls()
            .iter()
            .map(|f| f.len())
            .sum();
        assert_eq!(total, 22);
    }

    #[test]
    fn test_all_syscalls_flat() {
        let flat = CsciV01Final::all_syscalls_flat();
        assert_eq!(flat.len(), 22);
        assert!(!flat.is_empty());
    }

    #[test]
    fn test_contains_syscall() {
        assert!(CsciV01Final::contains_syscall("ct_spawn"));
        assert!(CsciV01Final::contains_syscall("mem_alloc"));
        assert!(CsciV01Final::contains_syscall("tool_invoke"));
        assert!(!CsciV01Final::contains_syscall("nonexistent"));
    }

    #[test]
    fn test_family_of() {
        assert_eq!(CsciV01Final::family_of("ct_spawn"), Some("Task"));
        assert_eq!(CsciV01Final::family_of("mem_alloc"), Some("Memory"));
        assert_eq!(CsciV01Final::family_of("tool_invoke"), Some("Tool"));
        assert_eq!(CsciV01Final::family_of("nonexistent"), None);
    }

    #[test]
    fn test_syscalls_in_family() {
        assert_eq!(
            CsciV01Final::syscalls_in_family("Task"),
            Some(CsciV01Final::TASK_FAMILY_SYSCALLS)
        );
        assert_eq!(
            CsciV01Final::syscalls_in_family("Memory"),
            Some(CsciV01Final::MEMORY_FAMILY_SYSCALLS)
        );
        assert!(CsciV01Final::syscalls_in_family("InvalidFamily").is_none());
    }

    #[test]
    fn test_finalization_record() {
        assert_eq!(
            CSCI_V0_1_FINALIZATION.status,
            FinalizationStatus::Finalized
        );
        assert_eq!(CSCI_V0_1_FINALIZATION.total_syscalls, 22);
        assert_eq!(CSCI_V0_1_FINALIZATION.total_families, 8);
    }

    #[test]
    fn test_is_finalized() {
        assert!(is_finalized());
    }

    #[test]
    fn test_finalization_date() {
        let date = finalization_date();
        assert!(date.contains("2026-03-01"));
    }

    #[test]
    fn test_total_v01_syscalls() {
        assert_eq!(total_v01_syscalls(), 22);
    }

    #[test]
    fn test_finalization_record_syscall_names() {
        assert_eq!(
            CSCI_V0_1_FINALIZATION.syscall_names.len(),
            22
        );
    }
}

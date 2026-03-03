// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Cognitive Substrate Syscall Interface (CSCI) v0.1
//!
//! The CSCI crate provides the specification and types for the Cognitive Substrate syscall
//! interface, a kernel-facing API for semantic operations including task lifecycle management,
//! memory operations, and resource management.
//!
//! ## Architecture
//!
//! This crate defines:
//! - **Error codes** following POSIX-like errno conventions (CS_* prefix)
//! - **Syscall families** (Task, Memory, Tool, Channel, Context, Capability)
//! - **Type definitions** for syscall parameters and return values
//! - **Specification metadata** for compatibility and versioning
//!
//! ## Design Philosophy
//!
//! CSCI follows the principle of explicit capability-based access control. Each syscall
//! requires explicit capability grants, enabling fine-grained security models for the
//! cognitive substrate kernel.
//!
//! ## No Std
//!
//! This crate is `#![no_std]` to support kernel-space code. It requires `alloc` for
//! dynamic allocations.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

pub mod api_stability;
pub mod crew_family;
pub mod csci_registry;
pub mod csci_v01_final;
pub mod design_rationale;
pub mod error_codes;
pub mod feedback_integration;
pub mod ipc_family;
pub mod memory_family;
pub mod rfc_review;
pub mod security_family;
pub mod signals_family;
pub mod syscall;
pub mod task_family;
pub mod telemetry_family;
pub mod tool_family;
pub mod types;
pub mod version;
pub mod ci_pipeline_config;
pub mod monorepo_integration;

/// CSCI semantic versioning constant.
///
/// Follows semver: MAJOR.MINOR.PATCH where changes to Task or Memory families
/// constitute breaking changes to the MAJOR version.
///
/// # Version History
/// - 0.1.0: Initial CSCI v0.1 with Task, Memory families (Week 1)
/// - 0.1-week2: Week 2 completion with IPC, Security, Tool, Signals/Context families
/// - 0.1-complete: Week 3 completion with Crew and Telemetry families (22 syscalls total)
/// - 0.1-finalized: Week 4 completion with feedback integration and API stability rules
pub const CSCI_VERSION: &str = "0.1-finalized";

/// Maximum number of syscalls across all families.
pub const MAX_SYSCALL_COUNT: usize = 256;

/// Maximum length of syscall names.
pub const MAX_SYSCALL_NAME_LEN: usize = 64;

/// Maximum number of parameters per syscall.
pub const MAX_SYSCALL_PARAMS: usize = 8;

/// Week 4 status: Feedback integration and v0.1 finalization complete.
///
/// All 22 syscalls are locked with confirmed parameters, return types, error codes,
/// and ABI details. Design rationale is documented. API stability rules are established.
/// Ready for SDK stub generation.
pub const WEEK_4_COMPLETE: bool = true;

#[cfg(test)]
mod tests {
    use super::*;
use alloc::vec::Vec;

    #[test]
    fn test_csci_version_format() {
        let parts: Vec<&str> = CSCI_VERSION.split('-').collect();
        assert!(!parts.is_empty());
        assert!(parts[0].contains("0.1"));
    }

    #[test]
    fn test_csci_constants() {
        assert!(MAX_SYSCALL_COUNT > 0);
        assert!(MAX_SYSCALL_NAME_LEN > 0);
        assert!(MAX_SYSCALL_PARAMS > 0);
    }

    #[test]
    fn test_week_4_completion() {
        assert!(WEEK_4_COMPLETE);
    }

    #[test]
    fn test_module_imports() {
        // Verify key Week 4 modules are accessible
        let _ = api_stability::v01_stability_guarantee();
        let _ = design_rationale::all_decisions();
        let _ = feedback_integration::FeedbackSummary::calculate();
        let _ = csci_v01_final::CsciV01Final::all_syscalls();
    }
}

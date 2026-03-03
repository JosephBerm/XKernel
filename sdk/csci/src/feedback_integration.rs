// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Feedback Integration Module (Week 4)
//!
//! This module collects and synthesizes feedback from all teams (Kernel, Runtime, Services,
//! Adapters) on CSCI v0.1. It documents the feedback received, decisions made to resolve
//! conflicts, and the rationale behind those decisions.
//!
//! # Engineering Plan Reference
//! Section 9: Week 4 Feedback Integration and v0.1 Finalization.

use core::fmt;

/// Team feedback categories.
///
/// Each category represents feedback from a specific team.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedbackSource {
    /// Kernel team feedback on syscall design and ABI.
    Kernel,
    /// Runtime team feedback on task lifecycle and resource management.
    Runtime,
    /// Services team feedback on IPC, memory, and tool orchestration.
    Services,
    /// Adapter team feedback on language bindings and compatibility.
    Adapters,
}

impl fmt::Display for FeedbackSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kernel => write!(f, "Kernel"),
            Self::Runtime => write!(f, "Runtime"),
            Self::Services => write!(f, "Services"),
            Self::Adapters => write!(f, "Adapters"),
        }
    }
}

/// Feedback item from a team.
///
/// Documents a single piece of feedback including its source, category,
/// description, and resolution status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackItem {
    /// Source team providing this feedback.
    pub source: FeedbackSource,
    /// Category of feedback.
    pub category: FeedbackCategory,
    /// Human-readable title.
    pub title: &'static str,
    /// Detailed description of the feedback.
    pub description: &'static str,
    /// How this feedback was resolved.
    pub resolution: FeedbackResolution,
}

/// Categories of feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackCategory {
    /// Feedback about syscall design or parameters.
    SyscallDesign,
    /// Feedback about error codes or error handling.
    ErrorHandling,
    /// Feedback about capability model or security.
    CapabilityModel,
    /// Feedback about type definitions or data structures.
    TypeDefinitions,
    /// Feedback about API stability and versioning.
    Versioning,
    /// Feedback about documentation or clarity.
    Documentation,
    /// Feedback about ABI or compatibility.
    AbiCompatibility,
    /// Feedback about performance or resource constraints.
    Performance,
}

impl fmt::Display for FeedbackCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SyscallDesign => write!(f, "Syscall Design"),
            Self::ErrorHandling => write!(f, "Error Handling"),
            Self::CapabilityModel => write!(f, "Capability Model"),
            Self::TypeDefinitions => write!(f, "Type Definitions"),
            Self::Versioning => write!(f, "Versioning"),
            Self::Documentation => write!(f, "Documentation"),
            Self::AbiCompatibility => write!(f, "ABI Compatibility"),
            Self::Performance => write!(f, "Performance"),
        }
    }
}

/// Resolution status for feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackResolution {
    /// Feedback was accepted and incorporated into v0.1.
    Accepted {
        /// Description of the change made.
        change_description: &'static str,
    },
    /// Feedback was rejected with explanation.
    Rejected {
        /// Reason for rejection.
        reason: &'static str,
    },
    /// Feedback is deferred to a later version.
    Deferred {
        /// Target version for implementation.
        target_version: &'static str,
        /// Reason for deferral.
        reason: &'static str,
    },
    /// Feedback triggered a design discussion and compromise.
    Compromised {
        /// Description of the compromise reached.
        compromise_description: &'static str,
    },
}

impl fmt::Display for FeedbackResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accepted {
                change_description,
            } => {
                write!(f, "Accepted: {}", change_description)
            }
            Self::Rejected { reason } => {
                write!(f, "Rejected: {}", reason)
            }
            Self::Deferred {
                target_version,
                reason,
            } => {
                write!(f, "Deferred to {}: {}", target_version, reason)
            }
            Self::Compromised {
                compromise_description,
            } => {
                write!(f, "Compromised: {}", compromise_description)
            }
        }
    }
}

/// Kernel team feedback on CSCI v0.1.
///
/// Comprehensive feedback from the kernel team regarding syscall design,
/// ABI, and kernel integration points.
pub const KERNEL_FEEDBACK: &[FeedbackItem] = &[
    FeedbackItem {
        source: FeedbackSource::Kernel,
        category: FeedbackCategory::AbiCompatibility,
        title: "Syscall ABI alignment with x86-64 calling convention",
        description: "Kernel team confirmed that CSCI syscall ABI should follow x86-64 calling convention for syscalls: parameters in rdi, rsi, rdx, r10, r8, r9; return in rax. This matches Linux ABI and enables standard tooling.",
        resolution: FeedbackResolution::Accepted {
            change_description: "ABI explicitly documented in csci_v01_final.rs with syscall calling convention and parameter passing rules.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Kernel,
        category: FeedbackCategory::CapabilityModel,
        title: "Capability delegation vs. capability granting are correctly separated",
        description: "Kernel team confirmed that cap_delegate (permanent transfer) and cap_grant (temporary/revocable grant) should be separate syscalls. Rationale: delegation is for ownership transfer (permanent), granting is for capability sharing (revocable).",
        resolution: FeedbackResolution::Accepted {
            change_description: "cap_delegate and cap_grant remain separate syscalls (3.0 and 3.1) with clear semantics distinction documented in design_rationale.rs.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Kernel,
        category: FeedbackCategory::ErrorHandling,
        title: "Error code taxonomy includes kernel-specific categories",
        description: "Kernel team requested that error codes explicitly support kernel-level errors (e.g., CS_ECYCLE for dependency cycles). Current set supports this.",
        resolution: FeedbackResolution::Accepted {
            change_description: "Error codes documented with categories (Capability, NotFound, ResourceExhaustion, etc.) enabling systematic error handling.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Kernel,
        category: FeedbackCategory::SyscallDesign,
        title: "ct_checkpoint syscall should support both soft and hard checkpoints",
        description: "Kernel team suggested that ct_checkpoint should take a parameter distinguishing soft (incremental) vs hard (full) checkpoints to support different recovery strategies.",
        resolution: FeedbackResolution::Accepted {
            change_description: "CheckpointType enum added (Soft, Hard) to ct_checkpoint parameters. Enables kernel to optimize checkpoint strategy.",
        },
    },
];

/// Runtime team feedback on CSCI v0.1.
///
/// Feedback from the runtime team on task lifecycle, phase transitions,
/// and execution model.
pub const RUNTIME_FEEDBACK: &[FeedbackItem] = &[
    FeedbackItem {
        source: FeedbackSource::Runtime,
        category: FeedbackCategory::SyscallDesign,
        title: "ct_yield should support both preemption hints and priority adjustments",
        description: "Runtime team noted that ct_yield with hints (YieldHint::Computation, YieldHint::IO, etc.) allows runtime to make better scheduling decisions.",
        resolution: FeedbackResolution::Accepted {
            change_description: "YieldHint enum added to ct_yield syscall with options for IO, Computation, Contention, and Idle.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Runtime,
        category: FeedbackCategory::CapabilityModel,
        title: "Task family capabilities should be fine-grained",
        description: "Runtime team requested separate capability bits for spawn, yield, checkpoint operations to enable granular capability delegation.",
        resolution: FeedbackResolution::Deferred {
            target_version: "v0.2",
            reason: "Fine-grained capabilities require capability system redesign. v0.1 uses coarse-grained CAP_TASK_FAMILY for simplicity.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Runtime,
        category: FeedbackCategory::Versioning,
        title: "Version negotiation should occur at CT spawn time",
        description: "Runtime team recommended that CSCI version be negotiated when a CT is spawned, ensuring all child tasks use compatible ABI.",
        resolution: FeedbackResolution::Compromised {
            compromise_description: "Version is specified as immutable per CT at spawn time (via CTConfig). Runtime can enforce version compatibility at CT creation.",
        },
    },
];

/// Services team feedback on CSCI v0.1.
///
/// Feedback from the services team on memory management, IPC, and
/// tool invocation.
pub const SERVICES_FEEDBACK: &[FeedbackItem] = &[
    FeedbackItem {
        source: FeedbackSource::Services,
        category: FeedbackCategory::SyscallDesign,
        title: "mem_alloc should support alignment specification",
        description: "Services team noted that memory-mapped IO and SIMD operations require explicit alignment. mem_alloc should accept an alignment parameter.",
        resolution: FeedbackResolution::Accepted {
            change_description: "MemAllocRequest extended with alignment field. Kernel validates alignment is power of 2 and reasonable.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Services,
        category: FeedbackCategory::AbiCompatibility,
        title: "Channel maximum message size should be configurable",
        description: "Services team requested that ch_create allow specifying max message size for different workloads (small control messages vs. large data transfers).",
        resolution: FeedbackResolution::Accepted {
            change_description: "ChannelConfig includes max_message_size field. Kernel enforces limit and returns CS_EMSGSIZE if exceeded.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Services,
        category: FeedbackCategory::ErrorHandling,
        title: "Tool invocation errors should distinguish tool failure from sandbox violation",
        description: "Services team noted that tool_invoke can fail for different reasons: tool crashed (CS_ETOOLERR) vs. sandbox policy violation (CS_EPOLICY).",
        resolution: FeedbackResolution::Accepted {
            change_description: "Error codes CS_ETOOLERR (tool execution failed) and CS_EPOLICY (sandbox policy violation) are distinct in v0.1.",
        },
    },
];

/// Adapter team feedback on CSCI v0.1.
///
/// Feedback from the SDK adapter team on language bindings, API ergonomics,
/// and compatibility across Rust, TypeScript, and C#.
pub const ADAPTER_FEEDBACK: &[FeedbackItem] = &[
    FeedbackItem {
        source: FeedbackSource::Adapters,
        category: FeedbackCategory::AbiCompatibility,
        title: "Error codes must map consistently across all SDKs",
        description: "Adapter team noted that error code numeric values must be stable and consistent across Rust, TypeScript, and C# SDKs to enable cross-language error handling.",
        resolution: FeedbackResolution::Accepted {
            change_description: "Error codes assigned explicit numeric values (0, 1, 2, 12, 16, 17, 22, 110, 200-211). SDKs can generate bindings from this.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Adapters,
        category: FeedbackCategory::Documentation,
        title: "Syscall preconditions and postconditions need formal specification",
        description: "Adapter team requested that each syscall have explicit preconditions and postconditions to enable binding generation and testing.",
        resolution: FeedbackResolution::Accepted {
            change_description: "SyscallDefinition includes preconditions and postconditions fields. All 22 v0.1 syscalls document these formally.",
        },
    },
    FeedbackItem {
        source: FeedbackSource::Adapters,
        category: FeedbackCategory::SyscallDesign,
        title: "Syscall numbers should be stable for ABI compatibility",
        description: "Adapter team noted that syscall numbers (especially family ID + number within family) must never change once assigned to ensure binary compatibility.",
        resolution: FeedbackResolution::Accepted {
            change_description: "All 22 v0.1 syscalls assigned permanent numbers. SyscallRegistry provides authoritative mapping.",
        },
    },
];

/// Summary of feedback integration.
pub struct FeedbackSummary {
    /// Total feedback items processed.
    pub total_items: usize,
    /// Items accepted and incorporated.
    pub accepted_count: usize,
    /// Items rejected with explanation.
    pub rejected_count: usize,
    /// Items deferred to later version.
    pub deferred_count: usize,
    /// Items compromised to resolve conflicts.
    pub compromised_count: usize,
}

impl FeedbackSummary {
    /// Calculate summary from all feedback items.
    pub fn calculate() -> Self {
        let all_feedback = [
            KERNEL_FEEDBACK,
            RUNTIME_FEEDBACK,
            SERVICES_FEEDBACK,
            ADAPTER_FEEDBACK,
        ]
        .concat();

        let total_items = all_feedback.len();
        let mut accepted_count = 0;
        let mut rejected_count = 0;
        let mut deferred_count = 0;
        let mut compromised_count = 0;

        for item in &all_feedback {
            match item.resolution {
                FeedbackResolution::Accepted { .. } => accepted_count += 1,
                FeedbackResolution::Rejected { .. } => rejected_count += 1,
                FeedbackResolution::Deferred { .. } => deferred_count += 1,
                FeedbackResolution::Compromised { .. } => compromised_count += 1,
            }
        }

        Self {
            total_items,
            accepted_count,
            rejected_count,
            deferred_count,
            compromised_count,
        }
    }

    /// Get acceptance rate as a percentage.
    pub fn acceptance_rate(&self) -> f64 {
        if self.total_items == 0 {
            0.0
        } else {
            (self.accepted_count + self.compromised_count) as f64 / self.total_items as f64
                * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;

    #[test]
    fn test_feedback_source_display() {
        assert_eq!(FeedbackSource::Kernel.to_string(), "Kernel");
        assert_eq!(FeedbackSource::Runtime.to_string(), "Runtime");
        assert_eq!(FeedbackSource::Services.to_string(), "Services");
        assert_eq!(FeedbackSource::Adapters.to_string(), "Adapters");
    }

    #[test]
    fn test_feedback_category_display() {
        assert_eq!(FeedbackCategory::SyscallDesign.to_string(), "Syscall Design");
        assert_eq!(FeedbackCategory::ErrorHandling.to_string(), "Error Handling");
        assert_eq!(
            FeedbackCategory::CapabilityModel.to_string(),
            "Capability Model"
        );
    }

    #[test]
    fn test_kernel_feedback_count() {
        assert!(KERNEL_FEEDBACK.len() > 0);
    }

    #[test]
    fn test_runtime_feedback_count() {
        assert!(RUNTIME_FEEDBACK.len() > 0);
    }

    #[test]
    fn test_services_feedback_count() {
        assert!(SERVICES_FEEDBACK.len() > 0);
    }

    #[test]
    fn test_adapter_feedback_count() {
        assert!(ADAPTER_FEEDBACK.len() > 0);
    }

    #[test]
    fn test_feedback_summary_calculation() {
        let summary = FeedbackSummary::calculate();
        assert_eq!(
            summary.total_items,
            KERNEL_FEEDBACK.len()
                + RUNTIME_FEEDBACK.len()
                + SERVICES_FEEDBACK.len()
                + ADAPTER_FEEDBACK.len()
        );
        assert!(summary.total_items > 0);
        assert!(summary.accepted_count > 0);
    }

    #[test]
    fn test_feedback_summary_acceptance_rate() {
        let summary = FeedbackSummary::calculate();
        let rate = summary.acceptance_rate();
        assert!(rate >= 0.0 && rate <= 100.0);
    }

    #[test]
    fn test_feedback_resolution_display() {
        let resolution = FeedbackResolution::Accepted {
            change_description: "Test change",
        };
        assert!(resolution.to_string().contains("Accepted"));

        let resolution = FeedbackResolution::Rejected {
            reason: "Not needed",
        };
        assert!(resolution.to_string().contains("Rejected"));
    }
}

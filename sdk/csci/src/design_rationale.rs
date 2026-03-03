// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # Design Rationale for CSCI v0.1 (Week 4)
//!
//! This module documents the rationale behind key design decisions in CSCI v0.1,
//! including why certain syscalls were included, why capabilities are structured
//! as they are, and the philosophy guiding the specification.
//!
//! # Engineering Plan Reference
//! Section 9: Design Rationale and Finalization.

use core::fmt;

/// Design decision categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignDecisionCategory {
    /// Why certain syscalls were included or excluded.
    SyscallSelection,
    /// Why syscalls are organized into families.
    FamilyOrganization,
    /// Rationale for capability separation (e.g., cap_delegate vs. cap_grant).
    CapabilityStructure,
    /// Why error codes are numbered and categorized as they are.
    ErrorCodeTaxonomy,
    /// ABI and calling convention decisions.
    AbiConventions,
    /// Type and data structure design decisions.
    TypeDesign,
}

impl fmt::Display for DesignDecisionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SyscallSelection => write!(f, "Syscall Selection"),
            Self::FamilyOrganization => write!(f, "Family Organization"),
            Self::CapabilityStructure => write!(f, "Capability Structure"),
            Self::ErrorCodeTaxonomy => write!(f, "Error Code Taxonomy"),
            Self::AbiConventions => write!(f, "ABI Conventions"),
            Self::TypeDesign => write!(f, "Type Design"),
        }
    }
}

/// A documented design decision with rationale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignDecision {
    /// Category of this decision.
    pub category: DesignDecisionCategory,
    /// Title summarizing the decision.
    pub title: &'static str,
    /// The decision itself: what was chosen?
    pub decision: &'static str,
    /// Why was this choice made? Rationale including trade-offs.
    pub rationale: &'static str,
    /// Alternative approaches considered.
    pub alternatives_considered: &'static str,
    /// Design principles underlying this decision.
    pub underlying_principles: &'static [&'static str],
}

/// Decision: Capability delegation vs. granting are separate.
pub const CAPABILITY_SEPARATION_DECISION: DesignDecision = DesignDecision {
    category: DesignDecisionCategory::CapabilityStructure,
    title: "Separate cap_delegate and cap_grant syscalls",
    decision:
        "CSCI v0.1 provides two distinct syscalls: cap_delegate (permanent transfer) and cap_grant (temporary/revocable grant). They are NOT merged into a single syscall with a mode parameter.",
    rationale:
        "Separation clarifies intent and enables kernel to optimize each case differently. Delegation is ownership transfer (permanent, consumes grantor's capability), suitable for parent->child relationships. Granting is sharing (temporary, grantor retains capability), suitable for library bindings or temporary delegation. Merging would force all use cases to specify a mode, adding complexity without benefit. Separation follows the principle of 'explicit intent'.",
    alternatives_considered:
        "Single syscall with mode parameter (cap_grant_or_delegate with mode enum). Rejected because it obscures intent and creates confusion about when to use which mode.",
    underlying_principles: &[
        "Explicit intent: syscalls should make intent clear",
        "Semantic clarity: different operations should have different names",
        "Kernel optimization: allows kernel to implement each case efficiently",
        "Security by transparency: grantors and grantees understand the semantics",
    ],
};

/// Decision: Exactly 22 syscalls in v0.1.
pub const SYSCALL_COUNT_DECISION: DesignDecision = DesignDecision {
    category: DesignDecisionCategory::SyscallSelection,
    title: "CSCI v0.1 defines exactly 22 syscalls",
    decision:
        "The v0.1 specification includes exactly 22 syscalls across 9 families: Task (4), Memory (4), Tool (2), Channel/IPC (3), Security/Capability (3), Signals/Context (2), Crew (4), Telemetry (2).",
    rationale:
        "22 syscalls provides complete coverage of core OS functionality (task lifecycle, memory, IPC, tools, security) while remaining bounded and reviewable. The number is derived from necessity: each family includes only syscalls essential for the use case. Task family: spawn, yield, checkpoint, resume (4 necessary). Memory family: alloc, free, mount, unmount (4 necessary). This avoids both under-specification (missing essential syscalls) and over-specification (kitchen-sink APIs). 22 is implementable in ~3-6 weeks (empirically validated in Weeks 1-3).",
    alternatives_considered:
        "Fewer syscalls (15-18): Would require multiplexing operations into syscalls with mode parameters, reducing clarity. More syscalls (25-40): Would add nice-to-have operations (batch operations, etc.) not essential for v0.1, delaying stabilization.",
    underlying_principles: &[
        "Bounded completeness: specify what's essential, defer nice-to-haves",
        "Reviewability: 22 syscalls can be thoroughly reviewed in one design cycle",
        "Implementation-ready: empirically validated as implementable in 3-6 weeks",
        "Minimalism: each syscall must justify its inclusion",
    ],
};

/// Decision: Syscalls organized by family.
pub const FAMILY_ORGANIZATION_DECISION: DesignDecision = DesignDecision {
    category: DesignDecisionCategory::FamilyOrganization,
    title: "Syscalls organized into 9 families with shared capability bits",
    decision:
        "Instead of a flat namespace of 22 individually-numbered syscalls, CSCI v0.1 organizes syscalls into 9 families (Task, Memory, Tool, Channel, Context, Capability, Signals, Crew, Telemetry). Each family has its own capability bit (CAP_TASK_FAMILY, CAP_MEMORY_FAMILY, etc.) and syscalls are numbered 0-N within each family.",
    rationale:
        "Family organization achieves three goals: (1) Capability granularity: grantors can delegate capabilities by family (e.g., 'allow memory allocation but not tool invocation') without per-syscall capability bits. (2) Logical grouping: related syscalls live together, making the API easier to understand and document. (3) Extensibility: new syscalls can be added to families in future versions without reordering. The trade-off is that capabilities are coarse-grained (family-level). Future versions can introduce fine-grained capabilities if needed.",
    alternatives_considered:
        "Flat numbering: 22 globally unique syscall numbers. Rejected: loses logical grouping. Per-syscall capabilities: 22 capability bits. Rejected: wastes capability bits on rare operations, complicates capability model.",
    underlying_principles: &[
        "Logical organization: group related operations",
        "Capability granularity: balance specificity vs. simplicity",
        "Future extensibility: design for v0.2+ additions",
        "Clear API boundaries: families define API domains",
    ],
};

/// Decision: Error code numbering matches POSIX where applicable.
pub const ERROR_CODE_NUMBERING_DECISION: DesignDecision = DesignDecision {
    category: DesignDecisionCategory::ErrorCodeTaxonomy,
    title: "Error codes use POSIX errno values where applicable",
    decision:
        "CSCI error codes match standard POSIX errno values for common errors (EPERM=1, ENOENT=2, ENOMEM=12, EBUSY=16, EEXIST=17, EINVAL=22, ETIMEDOUT=110). CSCI-specific errors use values 200+ (EBUDGET=200, ECYCLE=201, EUNIMPL=202, etc.).",
    rationale:
        "Using POSIX values enables: (1) Familiar error codes for developers (EPERM, ENOMEM, etc. have well-known meanings), (2) Easier integration with libc and standard libraries, (3) Potential for kernel modules to translate CSCI errors to POSIX errnos for compatibility. CSCI-specific errors (200+) have numeric space for future additions. This choice reduces learning curve for developers familiar with Unix.",
    alternatives_considered:
        "Custom numbering (CS_SUCCESS=0, CS_ERROR_1=1, ...): Avoids POSIX assumptions, but requires custom error translation. CSCI-only namespace: Loses familiarity and compatibility benefit.",
    underlying_principles: &[
        "Familiar APIs reduce developer friction",
        "Compatibility with POSIX enables ecosystem reuse",
        "Predictable error codes enable recovery logic",
        "Clear categorization aids debugging",
    ],
};

/// Decision: ABI follows x86-64 System V calling convention.
pub const ABI_CONVENTION_DECISION: DesignDecision = DesignDecision {
    category: DesignDecisionCategory::AbiConventions,
    title: "CSCI syscalls follow x86-64 System V calling convention",
    decision:
        "Syscall parameters are passed in registers (rdi, rsi, rdx, r10, r8, r9 for the first 6 parameters). Return values are in rax and rdx (for 128-bit returns). Error codes are indicated by rax = -(error_code) for errors, positive values for success.",
    rationale:
        "System V calling convention is the standard for x86-64 Unix/Linux. This choice: (1) Enables direct integration with C libraries and standard ABIs, (2) Allows developers to use standard tools (strace, gdb, etc.) to debug syscall interactions, (3) Reduces runtime overhead vs. alternative calling conventions, (4) Matches expectations of systems programmers familiar with Linux. The -error_code convention matches Linux syscall semantics.",
    alternatives_considered:
        "Cap'n Proto RPC serialization: More flexibility, but adds marshaling overhead and requires Cap'n Proto runtime. Custom calling convention: Simpler in some ways, but incompatible with standard tools and libraries.",
    underlying_principles: &[
        "Integration with standard POSIX ABI",
        "Developer familiarity with Unix conventions",
        "Minimal runtime overhead",
        "Debuggability with standard tools",
    ],
};

/// Decision: Type definitions include structured config objects.
pub const TYPE_DESIGN_DECISION: DesignDecision = DesignDecision {
    category: DesignDecisionCategory::TypeDesign,
    title: "Complex syscall parameters use structured config types",
    decision:
        "Rather than passing many scalar parameters, CSCI defines structured types (CTConfig, MemAllocRequest, ChannelConfig, etc.) for complex syscalls. For example, ct_spawn takes a single CTConfig parameter rather than separate name, timeout, priority parameters.",
    rationale:
        "Structured types achieve: (1) Versioning flexibility: new fields can be added to config structures without changing ABI (via size/version fields), (2) Readability: named fields are more understandable than positional parameters, (3) Forward compatibility: unknown fields can be ignored by older kernels. This enables v0.2 to extend configs without breaking v0.1 code.",
    alternatives_considered:
        "Flat parameter list: Simpler initially, but harder to extend. Separate syscalls per configuration: Multiplicative explosion (e.g., ct_spawn_with_timeout, ct_spawn_with_priority, etc.).",
    underlying_principles: &[
        "Forward compatibility through versioning",
        "Semantic clarity via named fields",
        "Extensibility without ABI breakage",
        "Reduced parameter explosion",
    ],
};

/// Summary of all design decisions.
pub fn all_decisions() -> &'static [&'static DesignDecision] {
    &[
        &CAPABILITY_SEPARATION_DECISION,
        &SYSCALL_COUNT_DECISION,
        &FAMILY_ORGANIZATION_DECISION,
        &ERROR_CODE_NUMBERING_DECISION,
        &ABI_CONVENTION_DECISION,
        &TYPE_DESIGN_DECISION,
    ]
}

/// Philosophical principles guiding CSCI design.
pub struct DesignPhilosophy;

impl DesignPhilosophy {
    /// Explicit intent: syscalls should make intent clear, not require mode parameters.
    pub const EXPLICIT_INTENT: &'static str =
        "Syscalls should use separate operations for different intents (e.g., cap_delegate vs. cap_grant) rather than multiplexing via mode parameters.";

    /// Semantic clarity: names should convey meaning.
    pub const SEMANTIC_CLARITY: &'static str =
        "Syscall names, error codes, and types should clearly convey their semantics. Abbreviations and mnemonics should be consistent with Unix tradition.";

    /// Bounded completeness: specify what's essential, defer extensions.
    pub const BOUNDED_COMPLETENESS: &'static str =
        "v0.1 specifies exactly what's needed for core functionality. Extensions and optimizations are deferred to v0.2+.";

    /// Forward compatibility: design for evolution.
    pub const FORWARD_COMPATIBILITY: &'static str =
        "Type definitions and syscall interfaces should evolve gracefully. Version fields, size fields, and optional parameters enable forward compatibility.";

    /// Minimal overhead: trust the kernel to be efficient.
    pub const MINIMAL_OVERHEAD: &'static str =
        "CSCI should not add marshaling, serialization, or conversion overhead. Syscalls should map directly to kernel operations.";

    /// Developer familiarity: align with Unix traditions.
    pub const DEVELOPER_FAMILIARITY: &'static str =
        "Developers are familiar with POSIX, Linux, and x86-64 ABIs. Align CSCI with these conventions to reduce friction.";

    /// All philosophical principles.
    pub fn all_principles() -> &'static [&'static str] {
        &[
            Self::EXPLICIT_INTENT,
            Self::SEMANTIC_CLARITY,
            Self::BOUNDED_COMPLETENESS,
            Self::FORWARD_COMPATIBILITY,
            Self::MINIMAL_OVERHEAD,
            Self::DEVELOPER_FAMILIARITY,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_capability_separation_decision() {
        assert_eq!(
            CAPABILITY_SEPARATION_DECISION.category,
            DesignDecisionCategory::CapabilityStructure
        );
        assert!(!CAPABILITY_SEPARATION_DECISION.title.is_empty());
        assert!(!CAPABILITY_SEPARATION_DECISION.rationale.is_empty());
        assert!(!CAPABILITY_SEPARATION_DECISION
            .underlying_principles
            .is_empty());
    }

    #[test]
    fn test_syscall_count_decision() {
        assert_eq!(
            SYSCALL_COUNT_DECISION.category,
            DesignDecisionCategory::SyscallSelection
        );
        assert!(SYSCALL_COUNT_DECISION.title.contains("22"));
    }

    #[test]
    fn test_family_organization_decision() {
        assert_eq!(
            FAMILY_ORGANIZATION_DECISION.category,
            DesignDecisionCategory::FamilyOrganization
        );
        assert!(FAMILY_ORGANIZATION_DECISION.title.contains("9 families"));
    }

    #[test]
    fn test_error_code_numbering_decision() {
        assert_eq!(
            ERROR_CODE_NUMBERING_DECISION.category,
            DesignDecisionCategory::ErrorCodeTaxonomy
        );
        assert!(ERROR_CODE_NUMBERING_DECISION.title.contains("POSIX"));
    }

    #[test]
    fn test_abi_convention_decision() {
        assert_eq!(
            ABI_CONVENTION_DECISION.category,
            DesignDecisionCategory::AbiConventions
        );
        assert!(ABI_CONVENTION_DECISION.title.contains("x86-64"));
    }

    #[test]
    fn test_type_design_decision() {
        assert_eq!(
            TYPE_DESIGN_DECISION.category,
            DesignDecisionCategory::TypeDesign
        );
        assert!(TYPE_DESIGN_DECISION.title.contains("config"));
    }

    #[test]
    fn test_all_decisions() {
        let decisions = all_decisions();
        assert_eq!(decisions.len(), 6);
    }

    #[test]
    fn test_design_philosophy_principles() {
        let principles = DesignPhilosophy::all_principles();
        assert_eq!(principles.len(), 6);
        assert!(!principles[0].is_empty());
    }

    #[test]
    fn test_design_decision_category_display() {
        assert_eq!(
            DesignDecisionCategory::SyscallSelection.to_string(),
            "Syscall Selection"
        );
        assert_eq!(
            DesignDecisionCategory::CapabilityStructure.to_string(),
            "Capability Structure"
        );
    }
}

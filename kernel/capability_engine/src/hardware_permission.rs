// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Hardware permission fault handling.
//!
//! This module implements the kernel exception handler for hardware permission faults
//! (page faults, access violations). When a CPU detects an unauthorized memory access:
//!
//! 1. Hardware raises an exception (Page Fault on x86_64, Data Abort on ARM64)
//! 2. Kernel exception handler calls [handle_permission_fault]
//! 3. We check the capability-page binding registry
//! 4. If the access is denied, we signal the agent
//! 5. If allowed, we allow the access to proceed
//!
//! Fail-safe principle: No capability = No mapping = Page fault = Access denied.
//!
//! See Engineering Plan § 5.0: MMU-backed capability enforcement integration,
//! specifically § 5.5: Hardware Permission Enforcement.

use core::fmt::{self, Debug, Display};

use crate::capability_page_binding::CapabilityPageBindingRegistry;
use crate::error::CapError;
use crate::ids::AgentID;
use crate::mmu_abstraction::{VirtualAddr, PageTablePermissions};

/// Fault types from hardware exceptions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FaultType {
    /// Page is not present in page table (unmapped memory).
    /// Indicates no capability was ever granted for this address.
    PageNotPresent,

    /// Access was denied due to permission mismatch.
    /// Page is mapped but the requested operation (R/W/X) is not allowed.
    PermissionDenied,

    /// Write attempted to a read-only page.
    WriteToReadOnly,

    /// Execution attempted on a non-executable page.
    ExecutionNotAllowed,

    /// Privilege violation (e.g., kernel-only page accessed from user mode).
    PrivilegeViolation,

    /// Reserved bit set in page table entry.
    ReservedBitViolation,

    /// SMEP (Supervisor Mode Execution Prevention) violation.
    SmepViolation,

    /// SMAP (Supervisor Mode Access Prevention) violation.
    SmapViolation,

    /// Other unclassified fault.
    Other,
}

impl Display for FaultType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FaultType::PageNotPresent => write!(f, "PageNotPresent"),
            FaultType::PermissionDenied => write!(f, "PermissionDenied"),
            FaultType::WriteToReadOnly => write!(f, "WriteToReadOnly"),
            FaultType::ExecutionNotAllowed => write!(f, "ExecutionNotAllowed"),
            FaultType::PrivilegeViolation => write!(f, "PrivilegeViolation"),
            FaultType::ReservedBitViolation => write!(f, "ReservedBitViolation"),
            FaultType::SmepViolation => write!(f, "SmepViolation"),
            FaultType::SmapViolation => write!(f, "SmapViolation"),
            FaultType::Other => write!(f, "Other"),
        }
    }
}

/// Access type requested by the faulting instruction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccessType {
    /// Read access (instruction or data fetch).
    Read,

    /// Write access (store instruction).
    Write,

    /// Execute access (instruction fetch at fault address).
    Execute,

    /// Instruction fetch (implicit execute).
    InstructionFetch,
}

impl Display for AccessType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessType::Read => write!(f, "Read"),
            AccessType::Write => write!(f, "Write"),
            AccessType::Execute => write!(f, "Execute"),
            AccessType::InstructionFetch => write!(f, "InstructionFetch"),
        }
    }
}

/// A hardware permission fault (exception) from the CPU.
#[derive(Clone, Debug)]
pub struct PermissionFault {
    /// Virtual address being accessed when the fault occurred.
    pub virtual_address: VirtualAddr,

    /// Type of fault.
    pub fault_type: FaultType,

    /// Type of access attempted (read, write, execute).
    pub access_type: AccessType,

    /// The agent (process/domain) that made the access attempt.
    pub faulting_agent: AgentID,

    /// CPU/core number where the fault occurred.
    pub cpu_number: usize,

    /// Instruction pointer (IP) at fault time.
    pub instruction_pointer: VirtualAddr,

    /// Whether the faulting agent is in kernel mode (true) or user mode (false).
    pub is_kernel_mode: bool,

    /// Raw fault code from hardware (for debugging/logging).
    pub error_code: u32,
}

impl Display for PermissionFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PermissionFault({}, addr=0x{:x}, access={}, agent={}, cpu={}, error=0x{:x})",
            self.fault_type, self.virtual_address, self.access_type, self.faulting_agent, self.cpu_number, self.error_code
        )
    }
}

/// Decision about how to handle a permission fault.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FaultDecision {
    /// Allow the access to proceed (mapping exists and permissions allow it).
    Allow,

    /// Deny the access and signal the agent (capability not found or insufficient).
    Deny,

    /// Kill the agent (security violation detected).
    KillAgent,

    /// Panic the kernel (unrecoverable security failure).
    KernelPanic,
}

impl Display for FaultDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FaultDecision::Allow => write!(f, "Allow"),
            FaultDecision::Deny => write!(f, "Deny"),
            FaultDecision::KillAgent => write!(f, "KillAgent"),
            FaultDecision::KernelPanic => write!(f, "KernelPanic"),
        }
    }
}

/// The result of handling a permission fault.
#[derive(Clone, Debug)]
pub struct FaultHandlingResult {
    /// Decision about how to handle the fault.
    pub decision: FaultDecision,

    /// Reason for the decision (human-readable).
    pub reason: alloc::string::String,

    /// Audit log entry to record.
    pub audit_entry: alloc::string::String,
}

impl FaultHandlingResult {
    /// Creates a result allowing the access.
    pub fn allow(reason: &str) -> Self {
        FaultHandlingResult {
            decision: FaultDecision::Allow,
            reason: reason.to_string(),
            audit_entry: format!("FAULT_ALLOWED: {}", reason),
        }
    }

    /// Creates a result denying the access.
    pub fn deny(reason: &str) -> Self {
        FaultHandlingResult {
            decision: FaultDecision::Deny,
            reason: reason.to_string(),
            audit_entry: format!("FAULT_DENIED: {}", reason),
        }
    }

    /// Creates a result killing the agent.
    pub fn kill_agent(reason: &str) -> Self {
        FaultHandlingResult {
            decision: FaultDecision::KillAgent,
            reason: reason.to_string(),
            audit_entry: format!("FAULT_KILL_AGENT: {}", reason),
        }
    }

    /// Creates a result panicking the kernel.
    pub fn kernel_panic(reason: &str) -> Self {
        FaultHandlingResult {
            decision: FaultDecision::KernelPanic,
            reason: reason.to_string(),
            audit_entry: format!("FAULT_KERNEL_PANIC: {}", reason),
        }
    }
}

/// Permission fault handler.
///
/// This is called by the CPU exception handler whenever a permission fault occurs.
/// It consults the capability-page binding registry to determine if the access
/// should be allowed or denied.
#[derive(Debug)]
pub struct PermissionFaultHandler {
    /// The binding registry to check for valid mappings.
    pub binding_registry: CapabilityPageBindingRegistry,

    /// Audit log of handled faults.
    pub fault_log: alloc::vec::Vec<PermissionFault>,

    /// Decisions log.
    pub decision_log: alloc::vec::Vec<FaultHandlingResult>,
}

impl PermissionFaultHandler {
    /// Creates a new permission fault handler.
    pub fn new(binding_registry: CapabilityPageBindingRegistry) -> Self {
        PermissionFaultHandler {
            binding_registry,
            fault_log: alloc::vec::Vec::new(),
            decision_log: alloc::vec::Vec::new(),
        }
    }

    /// Handles a permission fault.
    ///
    /// Implements the fail-safe principle:
    /// 1. Check if the faulting address has a binding in the registry
    /// 2. If no binding → Deny (no capability granted)
    /// 3. If binding exists, check if permissions allow the access
    /// 4. If permissions allow → Allow
    /// 5. If permissions deny → Deny
    ///
    /// # Arguments
    /// * `fault` - The permission fault from hardware
    ///
    /// # Returns
    /// The decision on how to handle the fault.
    pub fn handle_fault(&mut self, fault: PermissionFault) -> FaultHandlingResult {
        // Step 1: Log the fault
        self.fault_log.push(fault.clone());

        // Step 2: Check if the address has a binding
        if let Some(binding) = self.binding_registry.lookup(fault.virtual_address) {
            // Check if binding is active
            if !binding.is_active {
                let result = FaultHandlingResult::deny(
                    &format!("binding at 0x{:x} has been revoked", fault.virtual_address)
                );
                self.decision_log.push(result.clone());
                return result;
            }

            // Step 3: Check if the access is permitted
            let required_perms = self.access_type_to_permission(fault.access_type);
            if binding.page_table_entry.permissions.contains(required_perms) {
                let result = FaultHandlingResult::allow(
                    &format!(
                        "access to 0x{:x} ({}) allowed by capability {}",
                        fault.virtual_address, fault.access_type, binding.page_table_entry.capability_id
                    )
                );
                self.decision_log.push(result.clone());
                return result;
            } else {
                let result = FaultHandlingResult::deny(
                    &format!(
                        "access to 0x{:x} ({}) denied: insufficient permissions in capability",
                        fault.virtual_address, fault.access_type
                    )
                );
                self.decision_log.push(result.clone());
                return result;
            }
        }

        // Step 4: No binding found → Deny (fail-safe)
        let result = FaultHandlingResult::deny(
            &format!(
                "no binding found for address 0x{:x}: no capability granted",
                fault.virtual_address
            )
        );
        self.decision_log.push(result.clone());
        result
    }

    /// Converts an access type to required page table permissions.
    fn access_type_to_permission(&self, access: AccessType) -> PageTablePermissions {
        match access {
            AccessType::Read | AccessType::InstructionFetch => PageTablePermissions {
                readable: true,
                writable: false,
                executable: false,
            },
            AccessType::Write => PageTablePermissions {
                readable: false,
                writable: true,
                executable: false,
            },
            AccessType::Execute => PageTablePermissions {
                readable: false,
                writable: false,
                executable: true,
            },
        }
    }

    /// Returns the number of faults handled.
    pub fn fault_count(&self) -> usize {
        self.fault_log.len()
    }

    /// Returns the number of denied faults.
    pub fn denied_count(&self) -> usize {
        self.decision_log
            .iter()
            .filter(|d| d.decision == FaultDecision::Deny)
            .count()
    }

    /// Returns the number of allowed faults.
    pub fn allowed_count(&self) -> usize {
        self.decision_log
            .iter()
            .filter(|d| d.decision == FaultDecision::Allow)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::Capability;
    use crate::capability_page_binding::CapabilityPageBinding;
    use crate::ids::{CapID, ResourceID, ResourceType};
    use crate::constraints::Timestamp;
    use crate::operations::OperationSet;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

    fn create_test_binding(vaddr: VirtualAddr) -> CapabilityPageBinding {
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("test-agent"),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::read().union(OperationSet::write()),
            Timestamp::now_nanos(),
        );

        CapabilityPageBinding::new(
            cap,
            vaddr,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap()
    }

    #[test]
    fn test_permission_fault_display() {
        let fault = PermissionFault {
            virtual_address: 0x10000,
            fault_type: FaultType::PermissionDenied,
            access_type: AccessType::Write,
            faulting_agent: AgentID::new("agent-a"),
            cpu_number: 0,
            instruction_pointer: 0x20000,
            is_kernel_mode: false,
            error_code: 0x3,
        };

        let display = format!("{}", fault);
        assert!(display.contains("PermissionDenied"));
        assert!(display.contains("0x10000"));
        assert!(display.contains("Write"));
    }

    #[test]
    fn test_fault_handler_creation() {
        let registry = CapabilityPageBindingRegistry::new();
        let handler = PermissionFaultHandler::new(registry);
        assert_eq!(handler.fault_count(), 0);
    }

    #[test]
    fn test_handle_fault_no_binding() {
        let registry = CapabilityPageBindingRegistry::new();
        let mut handler = PermissionFaultHandler::new(registry);

        let fault = PermissionFault {
            virtual_address: 0x10000,
            fault_type: FaultType::PageNotPresent,
            access_type: AccessType::Read,
            faulting_agent: AgentID::new("agent-a"),
            cpu_number: 0,
            instruction_pointer: 0x20000,
            is_kernel_mode: false,
            error_code: 0,
        };

        let result = handler.handle_fault(fault);
        assert_eq!(result.decision, FaultDecision::Deny);
        assert_eq!(handler.fault_count(), 1);
        assert_eq!(handler.denied_count(), 1);
    }

    #[test]
    fn test_handle_fault_with_binding_allowed() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let binding = create_test_binding(0x10000);
        registry.register(binding).unwrap();

        let mut handler = PermissionFaultHandler::new(registry);

        let fault = PermissionFault {
            virtual_address: 0x10000,
            fault_type: FaultType::PageNotPresent,
            access_type: AccessType::Read,
            faulting_agent: AgentID::new("test-agent"),
            cpu_number: 0,
            instruction_pointer: 0x20000,
            is_kernel_mode: false,
            error_code: 0,
        };

        let result = handler.handle_fault(fault);
        assert_eq!(result.decision, FaultDecision::Allow);
        assert_eq!(handler.allowed_count(), 1);
    }

    #[test]
    fn test_handle_fault_with_binding_insufficient_perms() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let cap = Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("test-agent"),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::read(), // Read only
            Timestamp::now_nanos(),
        );

        let binding = CapabilityPageBinding::new(
            cap,
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();
        registry.register(binding).unwrap();

        let mut handler = PermissionFaultHandler::new(registry);

        let fault = PermissionFault {
            virtual_address: 0x10000,
            fault_type: FaultType::PermissionDenied,
            access_type: AccessType::Write,
            faulting_agent: AgentID::new("test-agent"),
            cpu_number: 0,
            instruction_pointer: 0x20000,
            is_kernel_mode: false,
            error_code: 0x3,
        };

        let result = handler.handle_fault(fault);
        assert_eq!(result.decision, FaultDecision::Deny);
        assert!(result.reason.contains("insufficient"));
    }

    #[test]
    fn test_handle_fault_revoked_binding() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let mut binding = create_test_binding(0x10000);
        binding.revoke();
        registry.register(binding).unwrap();

        let mut handler = PermissionFaultHandler::new(registry);

        let fault = PermissionFault {
            virtual_address: 0x10000,
            fault_type: FaultType::PageNotPresent,
            access_type: AccessType::Read,
            faulting_agent: AgentID::new("test-agent"),
            cpu_number: 0,
            instruction_pointer: 0x20000,
            is_kernel_mode: false,
            error_code: 0,
        };

        let result = handler.handle_fault(fault);
        assert_eq!(result.decision, FaultDecision::Deny);
        assert!(result.reason.contains("revoked"));
    }

    #[test]
    fn test_fault_handling_result_allow() {
        let result = FaultHandlingResult::allow("test reason");
        assert_eq!(result.decision, FaultDecision::Allow);
        assert!(result.audit_entry.contains("FAULT_ALLOWED"));
    }

    #[test]
    fn test_fault_handling_result_deny() {
        let result = FaultHandlingResult::deny("test reason");
        assert_eq!(result.decision, FaultDecision::Deny);
        assert!(result.audit_entry.contains("FAULT_DENIED"));
    }

    #[test]
    fn test_access_type_to_permission() {
        let registry = CapabilityPageBindingRegistry::new();
        let handler = PermissionFaultHandler::new(registry);

        let read_perm = handler.access_type_to_permission(AccessType::Read);
        assert!(read_perm.readable);
        assert!(!read_perm.writable);
        assert!(!read_perm.executable);

        let write_perm = handler.access_type_to_permission(AccessType::Write);
        assert!(!write_perm.readable);
        assert!(write_perm.writable);
        assert!(!write_perm.executable);

        let exec_perm = handler.access_type_to_permission(AccessType::Execute);
        assert!(!exec_perm.readable);
        assert!(!exec_perm.writable);
        assert!(exec_perm.executable);
    }
}

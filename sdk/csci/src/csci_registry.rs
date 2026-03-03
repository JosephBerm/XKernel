// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Complete Syscall Registry
//!
//! The authoritative registry of all CSCI v0.1 syscalls with their definitions,
//! numbers, families, parameters, return types, and capability requirements.
//!
//! # Engineering Plan Reference
//! Section 8: CSCI Syscall Registry (Complete v0.1).

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily};
use crate::task_family;
use crate::memory_family;
use crate::tool_family;
use crate::ipc_family;
use crate::security_family;
use crate::signals_family;
use crate::crew_family;
use crate::telemetry_family;
use alloc::collections::BTreeMap;
use core::fmt;

/// Syscall number type: combination of family and number within family.
///
/// The complete syscall number is computed as: (family_id << 8) | (number within family)
///
/// # Engineering Plan Reference
/// Section 8.1: Syscall numbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SyscallNumber(pub u16);

impl SyscallNumber {
    /// Create a syscall number from family ID and number within family.
    pub const fn new(family_id: u8, number: u8) -> Self {
        Self(((family_id as u16) << 8) | (number as u16))
    }

    /// Get the family ID from this syscall number.
    pub const fn family_id(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    /// Get the number within the family.
    pub const fn number(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Convert to u16 for ABI compatibility.
    pub const fn as_u16(&self) -> u16 {
        self.0
    }
}

impl fmt::Display for SyscallNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:04x}", self.0)
    }
}

/// Entry in the syscall registry.
///
/// Complete information about a single CSCI syscall including definition,
/// number, and metadata.
///
/// # Engineering Plan Reference
/// Section 8.2: Syscall registry entries.
#[derive(Debug, Clone)]
pub struct SyscallEntry {
    /// Syscall number (family ID << 8 | number within family).
    pub number: SyscallNumber,
    /// Syscall name (e.g., "ct_spawn").
    pub name: &'static str,
    /// Syscall family classification.
    pub family: SyscallFamily,
    /// Number within family (0-255).
    pub family_number: u8,
    /// Return type classification.
    pub return_type: ReturnType,
    /// Required capability bit.
    pub required_capability: u32,
    /// Short description of the syscall.
    pub description: &'static str,
    /// Reference to full definition for detailed information.
    pub full_definition: fn() -> SyscallDefinition,
}

/// Complete CSCI v0.1 syscall registry.
///
/// The registry maps syscall numbers to their definitions and provides
/// lookup methods by number or name.
///
/// # Engineering Plan Reference
/// Section 8: Complete CSCI v0.1 Registry.
pub struct SyscallRegistry {
    /// Map of syscall number to registry entry.
    entries: BTreeMap<SyscallNumber, SyscallEntry>,
}

impl SyscallRegistry {
    /// Create a new empty syscall registry.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Add a syscall entry to the registry.
    pub fn insert(&mut self, entry: SyscallEntry) {
        self.entries.insert(entry.number, entry);
    }

    /// Look up a syscall by number.
    pub fn lookup_by_number(&self, number: SyscallNumber) -> Option<&SyscallEntry> {
        self.entries.get(&number)
    }

    /// Look up a syscall by name.
    pub fn lookup_by_name(&self, name: &str) -> Option<&SyscallEntry> {
        self.entries.values().find(|e| e.name == name)
    }

    /// Get the number of syscalls in the registry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all syscall entries.
    pub fn iter(&self) -> impl Iterator<Item = &SyscallEntry> {
        self.entries.values()
    }
}

impl Default for SyscallRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Family IDs for syscall numbering.
pub mod family_ids {
    /// Task family ID.
    pub const TASK: u8 = 0;
    /// Memory family ID.
    pub const MEMORY: u8 = 1;
    /// Tool family ID.
    pub const TOOL: u8 = 2;
    /// IPC/Channel family ID.
    pub const CHANNEL: u8 = 3;
    /// Security/Capability family ID.
    pub const SECURITY: u8 = 4;
    /// Signals/Context family ID.
    pub const SIGNALS: u8 = 5;
    /// Crew family ID.
    pub const CREW: u8 = 6;
    /// Telemetry family ID.
    pub const TELEMETRY: u8 = 7;
}

/// Build the complete CSCI v0.1 registry with all 22 syscalls.
///
/// # Engineering Plan Reference
/// Section 8: Complete CSCI v0.1 Syscall Registry.
pub fn build_registry() -> SyscallRegistry {
    let mut registry = SyscallRegistry::new();

    // Task Family (0x00xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TASK, task_family::number::CT_SPAWN),
        name: "ct_spawn",
        family: SyscallFamily::Task,
        family_number: task_family::number::CT_SPAWN,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_TASK_FAMILY,
        description: "Create a new cognitive task",
        full_definition: task_family::ct_spawn_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TASK, task_family::number::CT_YIELD),
        name: "ct_yield",
        family: SyscallFamily::Task,
        family_number: task_family::number::CT_YIELD,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_TASK_FAMILY,
        description: "Voluntarily yield task execution",
        full_definition: task_family::ct_yield_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TASK, task_family::number::CT_CHECKPOINT),
        name: "ct_checkpoint",
        family: SyscallFamily::Task,
        family_number: task_family::number::CT_CHECKPOINT,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_TASK_FAMILY,
        description: "Create a task state checkpoint",
        full_definition: task_family::ct_checkpoint_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TASK, task_family::number::CT_RESUME),
        name: "ct_resume",
        family: SyscallFamily::Task,
        family_number: task_family::number::CT_RESUME,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_TASK_FAMILY,
        description: "Resume task from checkpoint",
        full_definition: task_family::ct_resume_definition,
    });

    // Memory Family (0x01xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::MEMORY, memory_family::number::MEM_ALLOC),
        name: "mem_alloc",
        family: SyscallFamily::Memory,
        family_number: memory_family::number::MEM_ALLOC,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_MEMORY_FAMILY,
        description: "Allocate semantic memory region",
        full_definition: memory_family::mem_alloc_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::MEMORY, memory_family::number::MEM_MOUNT),
        name: "mem_mount",
        family: SyscallFamily::Memory,
        family_number: memory_family::number::MEM_MOUNT,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_MEMORY_FAMILY,
        description: "Mount knowledge source into memory",
        full_definition: memory_family::mem_mount_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::MEMORY, memory_family::number::MEM_READ),
        name: "mem_read",
        family: SyscallFamily::Memory,
        family_number: memory_family::number::MEM_READ,
        return_type: ReturnType::Memory,
        required_capability: crate::types::CapabilitySet::CAP_MEMORY_FAMILY,
        description: "Read from memory region",
        full_definition: memory_family::mem_read_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::MEMORY, memory_family::number::MEM_WRITE),
        name: "mem_write",
        family: SyscallFamily::Memory,
        family_number: memory_family::number::MEM_WRITE,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_MEMORY_FAMILY,
        description: "Write to memory region",
        full_definition: memory_family::mem_write_definition,
    });

    // Tool Family (0x02xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TOOL, tool_family::number::TOOL_BIND),
        name: "tool_bind",
        family: SyscallFamily::Tool,
        family_number: tool_family::number::TOOL_BIND,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_TOOL_FAMILY,
        description: "Bind a tool into the task environment",
        full_definition: tool_family::tool_bind_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TOOL, tool_family::number::TOOL_INVOKE),
        name: "tool_invoke",
        family: SyscallFamily::Tool,
        family_number: tool_family::number::TOOL_INVOKE,
        return_type: ReturnType::Memory,
        required_capability: crate::types::CapabilitySet::CAP_TOOL_FAMILY,
        description: "Invoke a bound tool",
        full_definition: tool_family::tool_invoke_definition,
    });

    // Channel Family (0x03xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::CHANNEL, ipc_family::number::CHAN_OPEN),
        name: "chan_open",
        family: SyscallFamily::Channel,
        family_number: ipc_family::number::CHAN_OPEN,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_CHANNEL_FAMILY,
        description: "Open a new IPC channel",
        full_definition: ipc_family::chan_open_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::CHANNEL, ipc_family::number::CHAN_SEND),
        name: "chan_send",
        family: SyscallFamily::Channel,
        family_number: ipc_family::number::CHAN_SEND,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_CHANNEL_FAMILY,
        description: "Send message on channel",
        full_definition: ipc_family::chan_send_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::CHANNEL, ipc_family::number::CHAN_RECV),
        name: "chan_recv",
        family: SyscallFamily::Channel,
        family_number: ipc_family::number::CHAN_RECV,
        return_type: ReturnType::Memory,
        required_capability: crate::types::CapabilitySet::CAP_CHANNEL_FAMILY,
        description: "Receive message from channel",
        full_definition: ipc_family::chan_recv_definition,
    });

    // Security/Capability Family (0x04xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::SECURITY, security_family::number::CAP_GRANT),
        name: "cap_grant",
        family: SyscallFamily::Capability,
        family_number: security_family::number::CAP_GRANT,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_CAPABILITY_FAMILY,
        description: "Grant capability to agent",
        full_definition: security_family::cap_grant_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::SECURITY, security_family::number::CAP_REVOKE),
        name: "cap_revoke",
        family: SyscallFamily::Capability,
        family_number: security_family::number::CAP_REVOKE,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_CAPABILITY_FAMILY,
        description: "Revoke capability from agent",
        full_definition: security_family::cap_revoke_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::SECURITY, security_family::number::CAP_DELEGATE),
        name: "cap_delegate",
        family: SyscallFamily::Capability,
        family_number: security_family::number::CAP_DELEGATE,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_CAPABILITY_FAMILY,
        description: "Delegate capability authority",
        full_definition: security_family::cap_delegate_definition,
    });

    // Signals Family (0x05xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::SIGNALS, signals_family::number::SIG_REGISTER),
        name: "sig_register",
        family: SyscallFamily::Signals,
        family_number: signals_family::number::SIG_REGISTER,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_SIGNALS_FAMILY,
        description: "Register signal handler",
        full_definition: signals_family::sig_register_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::SIGNALS, signals_family::number::EXC_REGISTER),
        name: "exc_register",
        family: SyscallFamily::Signals,
        family_number: signals_family::number::EXC_REGISTER,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_SIGNALS_FAMILY,
        description: "Register exception handler",
        full_definition: signals_family::exc_register_definition,
    });

    // Crew Family (0x06xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::CREW, crew_family::number::CREW_CREATE),
        name: "crew_create",
        family: SyscallFamily::Crew,
        family_number: crew_family::number::CREW_CREATE,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_CREW_FAMILY,
        description: "Create a new agent crew",
        full_definition: crew_family::crew_create_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::CREW, crew_family::number::CREW_JOIN),
        name: "crew_join",
        family: SyscallFamily::Crew,
        family_number: crew_family::number::CREW_JOIN,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_CREW_FAMILY,
        description: "Join an existing crew",
        full_definition: crew_family::crew_join_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::CREW, crew_family::number::CREW_LEAVE),
        name: "crew_leave",
        family: SyscallFamily::Crew,
        family_number: crew_family::number::CREW_LEAVE,
        return_type: ReturnType::Unit,
        required_capability: crate::types::CapabilitySet::CAP_CREW_FAMILY,
        description: "Leave a crew",
        full_definition: crew_family::crew_leave_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::CREW, crew_family::number::CREW_QUERY),
        name: "crew_query",
        family: SyscallFamily::Crew,
        family_number: crew_family::number::CREW_QUERY,
        return_type: ReturnType::Memory,
        required_capability: crate::types::CapabilitySet::CAP_CREW_FAMILY,
        description: "Query crew status",
        full_definition: crew_family::crew_query_definition,
    });

    // Telemetry Family (0x07xx)
    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TELEMETRY, telemetry_family::number::TRACE_EMIT),
        name: "trace_emit",
        family: SyscallFamily::Telemetry,
        family_number: telemetry_family::number::TRACE_EMIT,
        return_type: ReturnType::Identifier,
        required_capability: crate::types::CapabilitySet::CAP_TELEMETRY_FAMILY,
        description: "Emit a CEF trace event",
        full_definition: telemetry_family::trace_emit_definition,
    });

    registry.insert(SyscallEntry {
        number: SyscallNumber::new(family_ids::TELEMETRY, telemetry_family::number::TRACE_QUERY),
        name: "trace_query",
        family: SyscallFamily::Telemetry,
        family_number: telemetry_family::number::TRACE_QUERY,
        return_type: ReturnType::Memory,
        required_capability: crate::types::CapabilitySet::CAP_TELEMETRY_FAMILY,
        description: "Query trace events",
        full_definition: telemetry_family::trace_query_definition,
    });

    registry
}

/// Global constant syscall registry for CSCI v0.1.
///
/// This is the authoritative registry containing all 22 CSCI v0.1 syscalls.
pub fn get_registry() -> SyscallRegistry {
    build_registry()
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec::Vec;

    #[test]
    fn test_syscall_number_creation() {
        let num = SyscallNumber::new(0, 5);
        assert_eq!(num.family_id(), 0);
        assert_eq!(num.number(), 5);
        assert_eq!(num.as_u16(), 0x0005);
    }

    #[test]
    fn test_syscall_number_display() {
        let num = SyscallNumber::new(3, 10);
        assert_eq!(num.to_string(), "0x030a");
    }

    #[test]
    fn test_syscall_number_ordering() {
        let num1 = SyscallNumber::new(0, 1);
        let num2 = SyscallNumber::new(0, 2);
        let num3 = SyscallNumber::new(1, 0);
        assert!(num1 < num2);
        assert!(num2 < num3);
    }

    #[test]
    fn test_registry_creation() {
        let registry = build_registry();
        assert_eq!(registry.len(), 22);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_registry_lookup_by_name() {
        let registry = build_registry();

        let ct_spawn = registry.lookup_by_name("ct_spawn");
        assert!(ct_spawn.is_some());
        assert_eq!(ct_spawn.unwrap().name, "ct_spawn");

        let crew_create = registry.lookup_by_name("crew_create");
        assert!(crew_create.is_some());
        assert_eq!(crew_create.unwrap().name, "crew_create");

        let trace_emit = registry.lookup_by_name("trace_emit");
        assert!(trace_emit.is_some());
        assert_eq!(trace_emit.unwrap().name, "trace_emit");
    }

    #[test]
    fn test_registry_lookup_by_number() {
        let registry = build_registry();

        let num = SyscallNumber::new(family_ids::TASK, task_family::number::CT_SPAWN);
        let entry = registry.lookup_by_number(num);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().name, "ct_spawn");
    }

    #[test]
    fn test_registry_complete_syscall_list() {
        let registry = build_registry();

        let names: Vec<&str> = registry.iter().map(|e| e.name).collect();
        assert!(names.contains(&"ct_spawn"));
        assert!(names.contains(&"ct_yield"));
        assert!(names.contains(&"ct_checkpoint"));
        assert!(names.contains(&"ct_resume"));
        assert!(names.contains(&"mem_alloc"));
        assert!(names.contains(&"mem_mount"));
        assert!(names.contains(&"mem_read"));
        assert!(names.contains(&"mem_write"));
        assert!(names.contains(&"tool_bind"));
        assert!(names.contains(&"tool_invoke"));
        assert!(names.contains(&"chan_open"));
        assert!(names.contains(&"chan_send"));
        assert!(names.contains(&"chan_recv"));
        assert!(names.contains(&"cap_grant"));
        assert!(names.contains(&"cap_revoke"));
        assert!(names.contains(&"cap_delegate"));
        assert!(names.contains(&"sig_register"));
        assert!(names.contains(&"exc_register"));
        assert!(names.contains(&"crew_create"));
        assert!(names.contains(&"crew_join"));
        assert!(names.contains(&"crew_leave"));
        assert!(names.contains(&"crew_query"));
        assert!(names.contains(&"trace_emit"));
        assert!(names.contains(&"trace_query"));
    }

    #[test]
    fn test_registry_entry_consistency() {
        let registry = build_registry();

        for entry in registry.iter() {
            let def = (entry.full_definition)();
            assert_eq!(entry.name, def.name);
            assert_eq!(entry.family, def.family);
        }
    }

    #[test]
    fn test_registry_crew_family_syscalls() {
        let registry = build_registry();

        let crew_syscalls: Vec<&str> = registry
            .iter()
            .filter(|e| e.family == SyscallFamily::Crew)
            .map(|e| e.name)
            .collect();

        assert_eq!(crew_syscalls.len(), 4);
        assert!(crew_syscalls.contains(&"crew_create"));
        assert!(crew_syscalls.contains(&"crew_join"));
        assert!(crew_syscalls.contains(&"crew_leave"));
        assert!(crew_syscalls.contains(&"crew_query"));
    }

    #[test]
    fn test_registry_telemetry_family_syscalls() {
        let registry = build_registry();

        let telemetry_syscalls: Vec<&str> = registry
            .iter()
            .filter(|e| e.family == SyscallFamily::Telemetry)
            .map(|e| e.name)
            .collect();

        assert_eq!(telemetry_syscalls.len(), 2);
        assert!(telemetry_syscalls.contains(&"trace_emit"));
        assert!(telemetry_syscalls.contains(&"trace_query"));
    }

    #[test]
    fn test_registry_syscall_numbers_unique() {
        let registry = build_registry();
        let mut numbers = Vec::new();

        for entry in registry.iter() {
            assert!(!numbers.contains(&entry.number));
            numbers.push(entry.number);
        }

        assert_eq!(numbers.len(), 22);
    }
}

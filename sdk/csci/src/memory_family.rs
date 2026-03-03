// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Memory Family Syscalls
//!
//! Memory family syscalls manage semantic memory operations:
//! - **mem_alloc**: Allocate a memory region
//! - **mem_read**: Read from a memory region
//! - **mem_write**: Write to a memory region
//! - **mem_mount**: Mount external knowledge source
//!
//! # Engineering Plan Reference
//! Section 8: Memory Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{AccessMode, AgentID, CapabilitySet, KnowledgeSourceRef, MemoryRegionID, MemoryTier, MountPoint};

/// Memory family syscall numbers.
pub mod number {
    /// mem_alloc syscall number within Memory family.
    pub const MEM_ALLOC: u8 = 0;
    /// mem_read syscall number within Memory family.
    pub const MEM_READ: u8 = 1;
    /// mem_write syscall number within Memory family.
    pub const MEM_WRITE: u8 = 2;
    /// mem_mount syscall number within Memory family.
    pub const MEM_MOUNT: u8 = 3;
}

/// Get the definition of the mem_alloc syscall.
///
/// **mem_alloc**: Allocate a semantic memory region.
///
/// Allocates a contiguous region of semantic memory at the specified tier.
/// Memory tiers provide different performance/capacity tradeoffs:
/// - L1: Fast, small (working memory)
/// - L2: Medium speed and capacity (task-local memory)
/// - L3: Large capacity, persistent (global semantic memory)
///
/// # Parameters
/// - `tier`: (MemoryTier) Memory tier to allocate from (L1, L2, L3)
/// - `size_bytes`: (Numeric) Size of region in bytes
/// - `agent`: (AgentID) Agent on whose behalf to allocate
///
/// # Returns
/// - Success: MemoryRegionID of the allocated region
/// - Error: CS_EPERM (no memory capability), CS_ENOMEM (insufficient space in tier),
///          CS_EINVAL (invalid size), CS_EBUDGET (would exceed quota)
///
/// # Preconditions
/// - Caller must have Memory family capability (CAP_MEMORY_FAMILY)
/// - `agent` must be valid and existing
/// - `size_bytes` must be > 0 and align to tier page size
/// - Allocation must not exceed agent's memory quota
/// - Allocation must not exceed tier capacity
///
/// # Postconditions
/// - MemoryRegionID allocated and unique
/// - Region initialized to zero
/// - Region accessible only by agent (unless granted to others)
/// - Agent's memory quota reduced by size_bytes
///
/// # Engineering Plan Reference
/// Section 8.1: mem_alloc specification.
pub fn mem_alloc_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "mem_alloc",
        SyscallFamily::Memory,
        number::MEM_ALLOC,
        ReturnType::Identifier,
        CapabilitySet::CAP_MEMORY_FAMILY,
        "Allocate semantic memory region",
    )
    .with_param(SyscallParam::new(
        "tier",
        ParamType::Enum,
        "Memory tier to allocate from (L1, L2, L3)",
        false,
    ))
    .with_param(SyscallParam::new(
        "size_bytes",
        ParamType::Numeric,
        "Size of region in bytes",
        false,
    ))
    .with_param(SyscallParam::new(
        "agent",
        ParamType::Identifier,
        "Agent on whose behalf to allocate",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnomem)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEbudget)
    .with_preconditions(
        "Caller has Memory capability; agent valid; size > 0 and aligned; within quota and tier capacity",
    )
    .with_postconditions(
        "MemoryRegionID allocated and unique; region initialized to zero; quota reduced",
    )
}

/// Get the definition of the mem_read syscall.
///
/// **mem_read**: Read from a memory region.
///
/// Reads a slice of data from a previously allocated memory region.
/// The read operation is constrained by:
/// - Caller's capability to read from this region
/// - Offset and length must be within region bounds
/// - Read is atomic at the region level
///
/// # Parameters
/// - `region`: (MemoryRegionID) Memory region to read from
/// - `offset`: (Numeric) Offset in bytes within region
/// - `length`: (Numeric) Number of bytes to read
///
/// # Returns
/// - Success: MemorySlice containing the read data
/// - Error: CS_EPERM (no read capability), CS_ENOENT (region not found),
///          CS_EINVAL (offset/length out of bounds), CS_EBUSY (region locked)
///
/// # Preconditions
/// - Caller must have read capability for this region
/// - Region must exist and be valid
/// - offset + length must be <= region size
/// - Region must not be locked for exclusive write
///
/// # Postconditions
/// - Data returned as MemorySlice
/// - Region state unchanged
/// - Region reference count incremented (for lock tracking)
///
/// # Engineering Plan Reference
/// Section 8.2: mem_read specification.
pub fn mem_read_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "mem_read",
        SyscallFamily::Memory,
        number::MEM_READ,
        ReturnType::Memory,
        CapabilitySet::CAP_MEMORY_FAMILY,
        "Read from memory region",
    )
    .with_param(SyscallParam::new(
        "region",
        ParamType::Identifier,
        "Memory region to read from",
        false,
    ))
    .with_param(SyscallParam::new(
        "offset",
        ParamType::Numeric,
        "Offset in bytes within region",
        false,
    ))
    .with_param(SyscallParam::new(
        "length",
        ParamType::Numeric,
        "Number of bytes to read",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEbusy)
    .with_preconditions(
        "Caller has read capability; region exists; offset + length <= region size; region not locked",
    )
    .with_postconditions("Data returned as MemorySlice; region unchanged; reference count incremented")
}

/// Get the definition of the mem_write syscall.
///
/// **mem_write**: Write to a memory region.
///
/// Writes data to a previously allocated memory region at the specified offset.
/// Write operations acquire exclusive locks to ensure consistency.
///
/// # Parameters
/// - `region`: (MemoryRegionID) Memory region to write to
/// - `offset`: (Numeric) Offset in bytes within region
/// - `data`: (MemorySlice) Data to write
///
/// # Returns
/// - Success: Numeric (bytes written)
/// - Error: CS_EPERM (no write capability), CS_ENOENT (region not found),
///          CS_EINVAL (offset/length out of bounds), CS_EBUSY (region locked for read)
///
/// # Preconditions
/// - Caller must have write capability for this region
/// - Region must exist and be valid
/// - offset + data.len() must be <= region size
/// - Region must not be locked for exclusive read or write
///
/// # Postconditions
/// - Data written atomically to region
/// - Bytes written returned
/// - Region reference count decremented
/// - Timestamp updated on region metadata
///
/// # Engineering Plan Reference
/// Section 8.3: mem_write specification.
pub fn mem_write_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "mem_write",
        SyscallFamily::Memory,
        number::MEM_WRITE,
        ReturnType::Numeric,
        CapabilitySet::CAP_MEMORY_FAMILY,
        "Write to memory region",
    )
    .with_param(SyscallParam::new(
        "region",
        ParamType::Identifier,
        "Memory region to write to",
        false,
    ))
    .with_param(SyscallParam::new(
        "offset",
        ParamType::Numeric,
        "Offset in bytes within region",
        false,
    ))
    .with_param(SyscallParam::new(
        "data",
        ParamType::Memory,
        "Data to write",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEbusy)
    .with_preconditions(
        "Caller has write capability; region exists; offset + data.len() <= region size; region unlocked",
    )
    .with_postconditions("Data written atomically; bytes written returned; reference count decremented; metadata updated")
}

/// Get the definition of the mem_mount syscall.
///
/// **mem_mount**: Mount external knowledge source.
///
/// Mounts an external knowledge source (embedding database, vector store, etc.)
/// into the memory namespace at a specified mount point. The mounted source
/// becomes addressable through the memory access interface.
///
/// # Parameters
/// - `source`: (KnowledgeSourceRef) Reference to knowledge source
/// - `mount_point`: (MountPoint) Path where to mount (e.g., "/knowledge/embeddings")
/// - `access_mode`: (AccessMode) Read-only, write-only, or read-write
///
/// # Returns
/// - Success: MemoryRegionID representing the mount
/// - Error: CS_EPERM (no mount capability), CS_ENOENT (source not found/accessible),
///          CS_EEXIST (mount point already occupied), CS_EINVAL (invalid access mode)
///
/// # Preconditions
/// - Caller must have Memory family capability
/// - Knowledge source must exist and be accessible
/// - Mount point must not already be in use
/// - Caller must have permission to access this knowledge source
/// - Access mode must be valid (read, write, or read-write)
///
/// # Postconditions
/// - Mount created at mount_point with unique MemoryRegionID
/// - Mount is read-only by default (unless RW specified)
/// - Source indexed in memory namespace
/// - Query operations available on mounted source
///
/// # Engineering Plan Reference
/// Section 8.4: mem_mount specification.
pub fn mem_mount_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "mem_mount",
        SyscallFamily::Memory,
        number::MEM_MOUNT,
        ReturnType::Identifier,
        CapabilitySet::CAP_MEMORY_FAMILY,
        "Mount external knowledge source",
    )
    .with_param(SyscallParam::new(
        "source",
        ParamType::Config,
        "Reference to external knowledge source",
        false,
    ))
    .with_param(SyscallParam::new(
        "mount_point",
        ParamType::Config,
        "Path where to mount the source",
        false,
    ))
    .with_param(SyscallParam::new(
        "access_mode",
        ParamType::Enum,
        "Access permissions (ReadOnly, WriteOnly, ReadWrite)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEexist)
    .with_error(CsciErrorCode::CsEinval)
    .with_preconditions(
        "Caller has Memory capability; source exists and accessible; mount_point free; valid access mode",
    )
    .with_postconditions("Mount created at mount_point; indexed in memory namespace; query operations available")
}

/// Get all Memory family syscall definitions.
///
/// Returns a vector of all four syscall definitions in the Memory family.
pub fn all_definitions() -> alloc::vec::Vec<SyscallDefinition> {
    alloc::vec![
        mem_alloc_definition(),
        mem_read_definition(),
        mem_write_definition(),
        mem_mount_definition(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_mem_alloc_definition() {
        let def = mem_alloc_definition();
        assert_eq!(def.name, "mem_alloc");
        assert_eq!(def.family, SyscallFamily::Memory);
        assert_eq!(def.number, number::MEM_ALLOC);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 3);
        assert!(def.error_codes.len() > 0);
    }

    #[test]
    fn test_mem_read_definition() {
        let def = mem_read_definition();
        assert_eq!(def.name, "mem_read");
        assert_eq!(def.family, SyscallFamily::Memory);
        assert_eq!(def.number, number::MEM_READ);
        assert_eq!(def.return_type, ReturnType::Memory);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_mem_write_definition() {
        let def = mem_write_definition();
        assert_eq!(def.name, "mem_write");
        assert_eq!(def.family, SyscallFamily::Memory);
        assert_eq!(def.number, number::MEM_WRITE);
        assert_eq!(def.return_type, ReturnType::Numeric);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_mem_mount_definition() {
        let def = mem_mount_definition();
        assert_eq!(def.name, "mem_mount");
        assert_eq!(def.family, SyscallFamily::Memory);
        assert_eq!(def.number, number::MEM_MOUNT);
        assert_eq!(def.return_type, ReturnType::Identifier);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_all_memory_definitions() {
        let defs = all_definitions();
        assert_eq!(defs.len(), 4);
        assert_eq!(defs[0].name, "mem_alloc");
        assert_eq!(defs[1].name, "mem_read");
        assert_eq!(defs[2].name, "mem_write");
        assert_eq!(defs[3].name, "mem_mount");
    }

    #[test]
    fn test_mem_alloc_parameters() {
        let def = mem_alloc_definition();
        assert_eq!(def.parameters[0].name, "tier");
        assert_eq!(def.parameters[1].name, "size_bytes");
        assert_eq!(def.parameters[2].name, "agent");
    }

    #[test]
    fn test_mem_alloc_errors() {
        let def = mem_alloc_definition();
        let error_codes = &def.error_codes;
        assert!(error_codes.contains(&CsciErrorCode::CsEperm));
        assert!(error_codes.contains(&CsciErrorCode::CsEnomem));
    }

    #[test]
    fn test_syscall_definitions_have_preconditions() {
        let defs = all_definitions();
        for def in defs {
            assert!(!def.preconditions.is_empty());
            assert!(!def.postconditions.is_empty());
        }
    }
}

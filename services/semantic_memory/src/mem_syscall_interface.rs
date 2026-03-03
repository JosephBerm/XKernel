// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Memory Manager CSCI syscall interface definitions.
//!
//! This module defines the four core syscall specifications for memory operations:
//! mem_alloc, mem_read, mem_write, and mem_mount. These syscalls provide the
//! primary interface through which Cognitive Threads interact with the Memory Manager.
//!
//! See Engineering Plan § 4.1.1: CSCI Syscall Definitions (Week 5).

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use crate::error::{MemoryError, Result};

/// Allocation flags for mem_alloc syscall.
///
/// Controls allocation behavior and memory region properties.
/// See Engineering Plan § 4.1.1: Allocation Operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AllocFlags(u32);

impl AllocFlags {
    /// No special flags.
    pub const NONE: Self = AllocFlags(0);

    /// Memory region is read-only after creation.
    pub const READ_ONLY: Self = AllocFlags(1 << 0);

    /// Memory region should be eagerly zeroed.
    pub const ZERO_INIT: Self = AllocFlags(1 << 1);

    /// Allocation should fail rather than trigger eviction.
    pub const NO_EVICT: Self = AllocFlags(1 << 2);

    /// Memory region should be replicated across crew members.
    pub const REPLICATE: Self = AllocFlags(1 << 3);

    /// Allocation is guaranteed to be durable (L3-backed).
    pub const DURABLE: Self = AllocFlags(1 << 4);

    /// Memory region is shared across processes.
    pub const SHARED: Self = AllocFlags(1 << 5);

    /// Creates flags from a raw u32 value.
    pub fn from_bits(bits: u32) -> Self {
        AllocFlags(bits)
    }

    /// Returns the raw bit representation.
    pub fn bits(&self) -> u32 {
        self.0
    }

    /// Checks if a specific flag is set.
    pub fn contains(&self, other: AllocFlags) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// Mount flags for mem_mount syscall.
///
/// Controls filesystem-like mounting behavior for external knowledge sources.
/// See Engineering Plan § 4.1.4: L3 Long-Term Operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MountFlags(u32);

impl MountFlags {
    /// No special flags.
    pub const NONE: Self = MountFlags(0);

    /// Mount source is read-only.
    pub const READ_ONLY: Self = MountFlags(1 << 0);

    /// Mount source should be indexed for semantic search.
    pub const INDEXED: Self = MountFlags(1 << 1);

    /// Mount operation should succeed immediately (lazy-load content).
    pub const LAZY: Self = MountFlags(1 << 2);

    /// Mount source is replicated; sync with crew nodes.
    pub const SYNC_REPLICAS: Self = MountFlags(1 << 3);

    /// Creates flags from a raw u32 value.
    pub fn from_bits(bits: u32) -> Self {
        MountFlags(bits)
    }

    /// Returns the raw bit representation.
    pub fn bits(&self) -> u32 {
        self.0
    }

    /// Checks if a specific flag is set.
    pub fn contains(&self, other: MountFlags) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for MountFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        MountFlags(self.0 | rhs.0)
    }
}

/// Opaque handle to an allocated memory region.
///
/// This strongly-typed handle prevents confusion with other reference types.
/// Handles are created by mem_alloc and used in subsequent mem_read/write operations.
///
/// See Engineering Plan § 4.1.0: Core Memory Model & § 4.1.5: Typed References.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MemHandle(u64);

impl MemHandle {
    /// Creates a new memory handle from a raw u64.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique 64-bit handle identifier
    pub fn new(id: u64) -> Self {
        MemHandle(id)
    }

    /// Returns the raw handle value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Creates an invalid/null handle for testing.
    #[cfg(test)]
    pub fn null() -> Self {
        MemHandle(0)
    }
}

impl fmt::Display for MemHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MemHandle({})", self.0)
    }
}

/// Opaque handle to a mounted memory region.
///
/// Returned by mem_mount syscall, used to reference mounted storage sources.
/// See Engineering Plan § 4.1.4: L3 Long-Term Operations.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MountHandle(u64);

impl MountHandle {
    /// Creates a new mount handle from a raw u64.
    pub fn new(id: u64) -> Self {
        MountHandle(id)
    }

    /// Returns the raw handle value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Creates an invalid/null handle for testing.
    #[cfg(test)]
    pub fn null() -> Self {
        MountHandle(0)
    }
}

impl fmt::Display for MountHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MountHandle({})", self.0)
    }
}

/// Source specification for mem_mount syscall.
///
/// Describes where data is coming from (local path, remote URL, shared region, etc.)
/// See Engineering Plan § 4.1.4: L3 Long-Term Operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MountSource {
    /// Local filesystem path.
    LocalPath(String),
    /// Remote URL (HTTP, S3, etc.)
    RemoteUrl(String),
    /// Shared memory region ID.
    SharedRegion(String),
    /// CRDT replica endpoint.
    CrewReplica(String),
}

impl MountSource {
    /// Returns a human-readable description of this mount source.
    pub fn description(&self) -> &str {
        match self {
            MountSource::LocalPath(_) => "local path",
            MountSource::RemoteUrl(_) => "remote URL",
            MountSource::SharedRegion(_) => "shared region",
            MountSource::CrewReplica(_) => "crew replica",
        }
    }

    /// Returns the source identifier as a string.
    pub fn as_str(&self) -> &str {
        match self {
            MountSource::LocalPath(s) | MountSource::RemoteUrl(s) |
            MountSource::SharedRegion(s) | MountSource::CrewReplica(s) => s,
        }
    }
}

/// mem_alloc syscall specification.
///
/// Allocates a memory region of the specified size and alignment.
/// Returns an opaque MemHandle for subsequent read/write operations.
///
/// # Syscall Signature
///
/// ```text
/// mem_alloc(size: usize, alignment: usize, flags: AllocFlags) -> Result<MemHandle>
/// ```
///
/// # Parameters
///
/// - `size`: Number of bytes to allocate (must be > 0)
/// - `alignment`: Byte boundary alignment requirement (must be power of 2, >= 1)
/// - `flags`: Allocation flags controlling behavior (AllocFlags::*)
///
/// # Returns
///
/// - `Ok(MemHandle)`: Successfully allocated region; handle valid until evicted
/// - `Err(MemoryError::AllocationFailed)`: Insufficient capacity in tier
/// - `Err(MemoryError::CapabilityDenied)`: Caller lacks allocation capability
/// - `Err(MemoryError::InvalidReference)`: Invalid alignment or size value
///
/// # Error Codes (POSIX-compatible)
///
/// - ENOMEM (12): Allocation failed, insufficient memory
/// - EINVAL (22): Invalid size or alignment parameter
/// - EACCES (13): Capability denied
///
/// # Guarantees
///
/// - Allocated memory is zero-initialized if AllocFlags::ZERO_INIT is set
/// - Memory address will satisfy the alignment requirement
/// - If AllocFlags::REPLICATE is set, region is replicated across crew
/// - If AllocFlags::DURABLE is set, region is backed by L3 storage
///
/// See Engineering Plan § 4.1.1: Allocation Operations.
pub mod mem_alloc {
    use super::*;

    /// Allocates a memory region.
    ///
    /// # Arguments
    ///
    /// * `size` - Bytes to allocate (must be > 0)
    /// * `alignment` - Alignment requirement (must be power of 2)
    /// * `flags` - Allocation control flags
    ///
    /// # Returns
    ///
    /// Memory handle or error
    pub fn syscall(size: usize, alignment: usize, flags: AllocFlags) -> Result<MemHandle> {
        // This is the interface definition; implementation is in handlers
        if size == 0 {
            return Err(MemoryError::Other("allocation size must be > 0".into()));
        }

        if alignment == 0 || (alignment & (alignment - 1)) != 0 {
            return Err(MemoryError::Other("alignment must be power of 2".into()));
        }

        // Stub: actual allocation happens in handlers
        Ok(MemHandle::new(0))
    }
}

/// mem_read syscall specification.
///
/// Reads data from a previously allocated memory region.
/// Supports bounded reads with offset and size parameters.
///
/// # Syscall Signature
///
/// ```text
/// mem_read(handle: MemHandle, offset: usize, size: usize) -> Result<Vec<u8>>
/// ```
///
/// # Parameters
///
/// - `handle`: Memory handle returned by mem_alloc
/// - `offset`: Byte offset within the region (0-based)
/// - `size`: Number of bytes to read (must be > 0)
///
/// # Returns
///
/// - `Ok(Vec<u8>)`: Successfully read data (len == size)
/// - `Err(MemoryError::InvalidReference)`: Handle invalid or stale
/// - `Err(MemoryError::CapabilityDenied)`: Caller lacks read capability
/// - `Err(MemoryError::Other("bounds"))`: Read exceeds region bounds
///
/// # Error Codes (POSIX-compatible)
///
/// - EIO (5): I/O error reading data
/// - EACCES (13): Capability denied
/// - EINVAL (22): Invalid handle or offset/size
///
/// # Guarantees
///
/// - If successful, returned buffer is exactly `size` bytes
/// - Partial reads are not supported; read fully or return error
/// - Read is atomic from the perspective of the caller
/// - If region was evicted to slower tier, read transparently pulls from there
///
/// See Engineering Plan § 4.1.1: Memory Operations.
pub mod mem_read {
    use super::*;

    /// Reads data from an allocated region.
    ///
    /// # Arguments
    ///
    /// * `handle` - Memory handle from mem_alloc
    /// * `offset` - Starting byte offset
    /// * `size` - Bytes to read
    ///
    /// # Returns
    ///
    /// Buffer of read data or error
    pub fn syscall(handle: MemHandle, offset: usize, size: usize) -> Result<Vec<u8>> {
        if size == 0 {
            return Ok(Vec::new());
        }

        // Stub: actual read happens in handlers
        Ok(Vec::new())
    }
}

/// mem_write syscall specification.
///
/// Writes data to a previously allocated memory region.
/// Supports bounded writes with offset and size parameters.
///
/// # Syscall Signature
///
/// ```text
/// mem_write(handle: MemHandle, offset: usize, size: usize, buffer: &[u8]) -> Result<()>
/// ```
///
/// # Parameters
///
/// - `handle`: Memory handle returned by mem_alloc
/// - `offset`: Byte offset within the region (0-based)
/// - `size`: Number of bytes to write (must be > 0, <= buffer.len())
/// - `buffer`: Data buffer to write (may be larger than size)
///
/// # Returns
///
/// - `Ok(())`: Successfully wrote data
/// - `Err(MemoryError::InvalidReference)`: Handle invalid or stale
/// - `Err(MemoryError::CapabilityDenied)`: Caller lacks write capability
/// - `Err(MemoryError::Other("read-only"))`: Region is read-only
/// - `Err(MemoryError::Other("bounds"))`: Write exceeds region bounds
///
/// # Error Codes (POSIX-compatible)
///
/// - EIO (5): I/O error writing data
/// - EACCES (13): Capability denied or read-only region
/// - EINVAL (22): Invalid handle, offset, or size
///
/// # Guarantees
///
/// - If successful, exactly `size` bytes are written
/// - Write is atomic from the perspective of the caller
/// - Writes are eventually persisted if region is durable
/// - If region has AllocFlags::REPLICATE, write is replicated to crew
///
/// See Engineering Plan § 4.1.1: Memory Operations.
pub mod mem_write {
    use super::*;

    /// Writes data to an allocated region.
    ///
    /// # Arguments
    ///
    /// * `handle` - Memory handle from mem_alloc
    /// * `offset` - Starting byte offset
    /// * `size` - Bytes to write
    /// * `buffer` - Data to write
    ///
    /// # Returns
    ///
    /// Success or error
    pub fn syscall(handle: MemHandle, offset: usize, size: usize, buffer: &[u8]) -> Result<()> {
        if size == 0 {
            return Ok(());
        }

        if size > buffer.len() {
            return Err(MemoryError::Other("buffer too small".into()));
        }

        // Stub: actual write happens in handlers
        Ok(())
    }
}

/// mem_mount syscall specification.
///
/// Mounts an external data source (filesystem, remote storage, shared region, etc.)
/// into the virtual address space of the Memory Manager, making it accessible
/// to other memory operations (particularly semantic search on L3).
///
/// # Syscall Signature
///
/// ```text
/// mem_mount(source: MountSource, mount_point: &str, flags: MountFlags) -> Result<MountHandle>
/// ```
///
/// # Parameters
///
/// - `source`: Source of data to mount (local path, remote URL, shared region, crew replica)
/// - `mount_point`: Virtual mount path (e.g., "/mnt/knowledge", "/mnt/corpus")
/// - `flags`: Mount control flags (read-only, indexed, lazy-load, sync replicas)
///
/// # Returns
///
/// - `Ok(MountHandle)`: Successfully mounted source; handle for later reference
/// - `Err(MemoryError::MountFailed)`: Mount operation failed
/// - `Err(MemoryError::CapabilityDenied)`: Caller lacks mount capability
/// - `Err(MemoryError::InvalidReference)`: Invalid source or mount point
///
/// # Error Codes (POSIX-compatible)
///
/// - EACCES (13): Capability denied
/// - EINVAL (22): Invalid source or mount point
/// - EIO (5): Mount I/O failed (source unreachable, corrupted, etc.)
///
/// # Guarantees
///
/// - If successful, mount persists until explicitly unmounted
/// - Data from mounted source is accessible via L3 read operations
/// - If MountFlags::INDEXED is set, semantic index is built asynchronously
/// - If MountFlags::SYNC_REPLICAS is set, mount is synchronized across crew
/// - If MountFlags::READ_ONLY is set, no modifications allowed to mounted data
///
/// # Notable Behaviors
///
/// - Mounting does NOT load all data immediately (on-demand access)
/// - Semantic search on L3 can query mounted sources
/// - Mount sources can be hot-swapped if same mount point is re-mounted
/// - Unmounting is implicit when process terminates
///
/// See Engineering Plan § 4.1.4: L3 Long-Term Operations.
pub mod mem_mount {
    use super::*;

    /// Mounts an external source into the memory hierarchy.
    ///
    /// # Arguments
    ///
    /// * `source` - Source to mount
    /// * `mount_point` - Virtual path for mount
    /// * `flags` - Mount options
    ///
    /// # Returns
    ///
    /// Mount handle or error
    pub fn syscall(
        source: MountSource,
        mount_point: &str,
        flags: MountFlags,
    ) -> Result<MountHandle> {
        if mount_point.is_empty() {
            return Err(MemoryError::Other("mount point must not be empty".into()));
        }

        // Stub: actual mount happens in handlers
        Ok(MountHandle::new(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc_flags_creation() {
        let flags = AllocFlags::ZERO_INIT;
        assert!(flags.contains(AllocFlags::ZERO_INIT));
        assert!(!flags.contains(AllocFlags::READ_ONLY));
    }

    #[test]
    fn test_alloc_flags_combined() {
        let flags = AllocFlags::from_bits(
            AllocFlags::ZERO_INIT.bits() | AllocFlags::REPLICATE.bits()
        );
        assert!(flags.contains(AllocFlags::ZERO_INIT));
        assert!(flags.contains(AllocFlags::REPLICATE));
        assert!(!flags.contains(AllocFlags::DURABLE));
    }

    #[test]
    fn test_mount_flags() {
        let flags = MountFlags::READ_ONLY | MountFlags::INDEXED;
        assert!(flags.contains(MountFlags::READ_ONLY));
        assert!(flags.contains(MountFlags::INDEXED));
    }

    #[test]
    fn test_mem_handle() {
        let handle = MemHandle::new(42);
        assert_eq!(handle.as_u64(), 42);
    }

    #[test]
    fn test_mount_handle() {
        let handle = MountHandle::new(100);
        assert_eq!(handle.as_u64(), 100);
    }

    #[test]
    fn test_mount_source_description() {
        assert_eq!(MountSource::LocalPath("test".into()).description(), "local path");
        assert_eq!(MountSource::RemoteUrl("test".into()).description(), "remote URL");
        assert_eq!(MountSource::SharedRegion("test".into()).description(), "shared region");
        assert_eq!(MountSource::CrewReplica("test".into()).description(), "crew replica");
    }

    #[test]
    fn test_mem_alloc_zero_size_invalid() {
        let result = mem_alloc::syscall(0, 8, AllocFlags::NONE);
        assert!(result.is_err());
    }

    #[test]
    fn test_mem_alloc_bad_alignment() {
        let result = mem_alloc::syscall(100, 3, AllocFlags::NONE);
        assert!(result.is_err());

        let result = mem_alloc::syscall(100, 0, AllocFlags::NONE);
        assert!(result.is_err());
    }

    #[test]
    fn test_mem_read_zero_size() {
        let result = mem_read::syscall(MemHandle::null(), 0, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_mem_write_zero_size() {
        let result = mem_write::syscall(MemHandle::null(), 0, 0, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mem_write_buffer_too_small() {
        let result = mem_write::syscall(MemHandle::null(), 0, 100, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_mem_mount_empty_path() {
        let result = mem_mount::syscall(
            MountSource::LocalPath("/test".into()),
            "",
            MountFlags::NONE,
        );
        assert!(result.is_err());
    }
}

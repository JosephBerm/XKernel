// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Platform-independent MMU (Memory Management Unit) abstraction.
//!
//! This module provides a hardware-independent interface to MMU operations,
//! with concrete implementations for x86_64 and ARM64 architectures.
//! See Engineering Plan § 5.0: MMU-backed capability enforcement integration.
//!
//! # Design Principles
//!
//! 1. **Platform Independence**: The [MmuAbstraction] trait defines operations
//!    that work on any architecture with multi-level page tables.
//!
//! 2. **Atomic Updates**: Page table updates via [allocate_pagetable], [map_page],
//!    [unmap_page] are atomic—either fully succeed or fully fail.
//!
//! 3. **TLB Synchronization**: All mapping changes trigger TLB invalidation
//!    via [invalidate_tlb] to maintain consistency.
//!
//! 4. **No Capability = No Mapping**: Pages can only be mapped if the
//!    holder possesses a valid capability (enforced by caller).

use alloc::vec::Vec;
use core::fmt::{self, Debug, Display};

use crate::error::CapError;
use crate::ids::{CapID, AgentID};

/// Physical address type (64-bit).
pub type PhysicalAddr = u64;

/// Virtual address type (64-bit).
pub type VirtualAddr = u64;

/// Page size constant: 4096 bytes (standard for most architectures).
pub const PAGE_SIZE: u64 = 4096;

/// Page table entry permission flags.
///
/// Derived from Capability.operations:
/// - Read (0x1) → PAGE_READABLE
/// - Write (0x2) → PAGE_WRITABLE
/// - Execute (0x4) → PAGE_EXECUTABLE
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PageTablePermissions {
    /// Read permission from capability.operations & 0x1
    pub readable: bool,

    /// Write permission from capability.operations & 0x2
    pub writable: bool,

    /// Execute permission from capability.operations & 0x4
    pub executable: bool,
}

impl PageTablePermissions {
    /// Creates permissions from operation bits.
    ///
    /// Maps Capability.operations to page table permissions:
    /// - bit 0 (READ=0x01) → readable
    /// - bit 1 (WRITE=0x02) → writable
    /// - bit 2 (EXECUTE=0x04) → executable
    pub fn from_operation_bits(bits: u8) -> Self {
        PageTablePermissions {
            readable: (bits & 0x01) != 0,
            writable: (bits & 0x02) != 0,
            executable: (bits & 0x04) != 0,
        }
    }

    /// Converts permissions back to operation bits.
    pub fn to_operation_bits(&self) -> u8 {
        let mut bits = 0u8;
        if self.readable {
            bits |= 0x01;
        }
        if self.writable {
            bits |= 0x02;
        }
        if self.executable {
            bits |= 0x04;
        }
        bits
    }

    /// Checks if all required permissions are satisfied.
    pub fn contains(&self, required: PageTablePermissions) -> bool {
        (self.readable || !required.readable)
            && (self.writable || !required.writable)
            && (self.executable || !required.executable)
    }

    /// Returns true if any permission is granted.
    pub fn is_empty(&self) -> bool {
        !self.readable && !self.writable && !self.executable
    }
}

impl Display for PageTablePermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut perms = String::new();
        if self.readable {
            perms.push('R');
        }
        if self.writable {
            perms.push('W');
        }
        if self.executable {
            perms.push('X');
        }
        if perms.is_empty() {
            perms = "---".to_string();
        }
        write!(f, "{}", perms)
    }
}

/// A single page table entry (PTE) in the MMU abstraction.
///
/// Per Engineering Plan § 5.0, each PTE includes:
/// - physical_address: phys addr of the mapped page
/// - permissions: permission bits (from Capability.operations)
/// - capability_id: the CapID authorizing this mapping
/// - owner_agent: the AgentID who holds the capability
#[derive(Clone, Debug)]
pub struct PageTableEntry {
    /// Physical address of the mapped memory region.
    pub physical_address: PhysicalAddr,

    /// Virtual address where this mapping is installed.
    pub virtual_address: VirtualAddr,

    /// Permission bits (R/W/X) derived from capability operations.
    pub permissions: PageTablePermissions,

    /// The capability ID that authorized this mapping.
    pub capability_id: CapID,

    /// The agent who holds the authorizing capability.
    pub owner_agent: AgentID,

    /// Size of this mapping in bytes (typically PAGE_SIZE).
    pub size: u64,
}

impl PageTableEntry {
    /// Creates a new page table entry.
    pub fn new(
        physical_address: PhysicalAddr,
        virtual_address: VirtualAddr,
        permissions: PageTablePermissions,
        capability_id: CapID,
        owner_agent: AgentID,
        size: u64,
    ) -> Self {
        PageTableEntry {
            physical_address,
            virtual_address,
            permissions,
            capability_id,
            owner_agent,
            size,
        }
    }

    /// Checks if a virtual address is within this mapping's range.
    pub fn contains_vaddr(&self, vaddr: VirtualAddr) -> bool {
        vaddr >= self.virtual_address && vaddr < self.virtual_address + self.size
    }

    /// Checks if a physical address is within this mapping's range.
    pub fn contains_paddr(&self, paddr: PhysicalAddr) -> bool {
        paddr >= self.physical_address && paddr < self.physical_address + self.size
    }
}

impl Display for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PTE(virt=0x{:x}, phys=0x{:x}, perms={}, cap={}, agent={}, size=0x{:x})",
            self.virtual_address,
            self.physical_address,
            self.permissions,
            self.capability_id,
            self.owner_agent,
            self.size
        )
    }
}

/// Abstract MMU interface for page table operations.
///
/// Implementations must support both x86_64 and ARM64 architectures.
/// All operations are platform-independent at this level.
/// See Engineering Plan § 5.0: MMU-backed capability enforcement integration.
pub trait MmuAbstraction: Send + Sync {
    /// Allocates a new page table structure.
    ///
    /// This is typically a Level 1 page table (PML4 on x86_64, TTBR on ARM64).
    /// Returns an error if allocation fails.
    ///
    /// # Arguments
    /// * `owner_agent` - The agent that will use this page table
    ///
    /// # Returns
    /// A handle/ID for the allocated page table, used in subsequent operations.
    fn allocate_pagetable(&mut self, owner_agent: &AgentID) -> Result<u64, CapError>;

    /// Maps a single page into the page table at the given virtual address.
    ///
    /// Per § 5.0, this creates a PTE with:
    /// - physical_address: the actual phys addr
    /// - permissions: from capability.operations
    /// - capability_id: the authorizing cap
    /// - owner_agent: the cap holder
    ///
    /// The operation is atomic: either fully succeeds or fully fails.
    /// Multi-level page tables are created as needed.
    ///
    /// # Arguments
    /// * `pt_handle` - Handle returned from allocate_pagetable
    /// * `entry` - The PTE to install
    ///
    /// # Returns
    /// Ok(()) if mapping succeeds, or an error describing the failure.
    fn map_page(
        &mut self,
        pt_handle: u64,
        entry: PageTableEntry,
    ) -> Result<(), CapError>;

    /// Unmaps a page from the page table at the given virtual address.
    ///
    /// Removes the PTE for the specified virtual address, effectively
    /// revoking access to that memory region. The operation is atomic.
    ///
    /// # Arguments
    /// * `pt_handle` - Handle returned from allocate_pagetable
    /// * `virtual_addr` - Virtual address to unmap
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if the address is not mapped.
    fn unmap_page(&mut self, pt_handle: u64, virtual_addr: VirtualAddr) -> Result<(), CapError>;

    /// Invalidates the TLB (Translation Lookaside Buffer) for a virtual address.
    ///
    /// On x86_64: issues INVLPG instruction for the given virtual address.
    /// On ARM64: issues TLBI VAAE1IS for virtual address invalidation.
    ///
    /// # Arguments
    /// * `virtual_addr` - Virtual address to invalidate in the TLB
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if invalidation fails.
    fn invalidate_tlb(&self, virtual_addr: VirtualAddr) -> Result<(), CapError>;

    /// Invalidates the entire TLB for a specific page table.
    ///
    /// Used after major page table changes. On multi-core systems,
    /// this typically broadcasts an IPI to all cores.
    ///
    /// # Arguments
    /// * `pt_handle` - Handle of the page table to invalidate
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if invalidation fails.
    fn invalidate_tlb_all(&self, pt_handle: u64) -> Result<(), CapError>;

    /// Looks up a page table entry by virtual address.
    ///
    /// Returns the PTE if it exists and is valid.
    ///
    /// # Arguments
    /// * `pt_handle` - Handle of the page table to query
    /// * `virtual_addr` - Virtual address to look up
    ///
    /// # Returns
    /// Ok(Some(entry)) if the page is mapped, Ok(None) if not mapped, or an error.
    fn lookup_page(
        &self,
        pt_handle: u64,
        virtual_addr: VirtualAddr,
    ) -> Result<Option<PageTableEntry>, CapError>;

    /// Checks if a virtual address is accessible with the given permissions.
    ///
    /// Used for hardware permission fault handling (see hardware_permission.rs).
    /// Returns Ok(()) if the access is allowed, or an error describing the violation.
    ///
    /// # Arguments
    /// * `pt_handle` - Handle of the page table
    /// * `virtual_addr` - Virtual address being accessed
    /// * `required_perms` - Required permissions
    ///
    /// # Returns
    /// Ok(()) if access is allowed, CapError if denied.
    fn check_access(
        &self,
        pt_handle: u64,
        virtual_addr: VirtualAddr,
        required_perms: PageTablePermissions,
    ) -> Result<(), CapError>;

    /// Deallocates a page table structure.
    ///
    /// Removes all mappings and frees the page table memory.
    /// After this call, the pt_handle is invalid.
    ///
    /// # Arguments
    /// * `pt_handle` - Handle of the page table to deallocate
    ///
    /// # Returns
    /// Ok(()) if successful, or an error if deallocation fails.
    fn deallocate_pagetable(&mut self, pt_handle: u64) -> Result<(), CapError>;

    /// Returns the architecture name (e.g., "x86_64" or "arm64").
    fn architecture(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::String;
use alloc::string::ToString;

    #[test]
    fn test_permissions_from_operation_bits() {
        // Test READ (0x01)
        let perms = PageTablePermissions::from_operation_bits(0x01);
        assert!(perms.readable);
        assert!(!perms.writable);
        assert!(!perms.executable);

        // Test WRITE (0x02)
        let perms = PageTablePermissions::from_operation_bits(0x02);
        assert!(!perms.readable);
        assert!(perms.writable);
        assert!(!perms.executable);

        // Test EXECUTE (0x04)
        let perms = PageTablePermissions::from_operation_bits(0x04);
        assert!(!perms.readable);
        assert!(!perms.writable);
        assert!(perms.executable);

        // Test combined (0x07 = READ | WRITE | EXECUTE)
        let perms = PageTablePermissions::from_operation_bits(0x07);
        assert!(perms.readable);
        assert!(perms.writable);
        assert!(perms.executable);
    }

    #[test]
    fn test_permissions_to_operation_bits() {
        let perms = PageTablePermissions {
            readable: true,
            writable: false,
            executable: false,
        };
        assert_eq!(perms.to_operation_bits(), 0x01);

        let perms = PageTablePermissions {
            readable: true,
            writable: true,
            executable: true,
        };
        assert_eq!(perms.to_operation_bits(), 0x07);

        let perms = PageTablePermissions {
            readable: false,
            writable: false,
            executable: false,
        };
        assert_eq!(perms.to_operation_bits(), 0x00);
    }

    #[test]
    fn test_permissions_contains() {
        let all_perms = PageTablePermissions {
            readable: true,
            writable: true,
            executable: true,
        };

        let read_only = PageTablePermissions {
            readable: true,
            writable: false,
            executable: false,
        };

        assert!(all_perms.contains(read_only));
        assert!(!read_only.contains(all_perms));
    }

    #[test]
    fn test_permissions_is_empty() {
        let empty = PageTablePermissions {
            readable: false,
            writable: false,
            executable: false,
        };
        assert!(empty.is_empty());

        let with_read = PageTablePermissions {
            readable: true,
            writable: false,
            executable: false,
        };
        assert!(!with_read.is_empty());
    }

    #[test]
    fn test_page_table_entry_contains_vaddr() {
        let entry = PageTableEntry::new(
            0x1000,
            0x10000,
            PageTablePermissions {
                readable: true,
                writable: false,
                executable: false,
            },
            CapID::from_bytes([1u8; 32]),
            AgentID::new("test-agent"),
            PAGE_SIZE,
        );

        assert!(entry.contains_vaddr(0x10000));
        assert!(entry.contains_vaddr(0x10500));
        assert!(!entry.contains_vaddr(0x11000)); // beyond size
        assert!(!entry.contains_vaddr(0x0f000)); // before start
    }

    #[test]
    fn test_page_table_entry_contains_paddr() {
        let entry = PageTableEntry::new(
            0x1000,
            0x10000,
            PageTablePermissions {
                readable: true,
                writable: false,
                executable: false,
            },
            CapID::from_bytes([1u8; 32]),
            AgentID::new("test-agent"),
            PAGE_SIZE,
        );

        assert!(entry.contains_paddr(0x1000));
        assert!(entry.contains_paddr(0x1500));
        assert!(!entry.contains_paddr(0x2000));
        assert!(!entry.contains_paddr(0x0000));
    }

    #[test]
    fn test_permissions_display() {
        let perms_r = PageTablePermissions {
            readable: true,
            writable: false,
            executable: false,
        };
        assert_eq!(perms_r.to_string(), "R");

        let perms_rw = PageTablePermissions {
            readable: true,
            writable: true,
            executable: false,
        };
        assert_eq!(perms_rw.to_string(), "RW");

        let perms_rwx = PageTablePermissions {
            readable: true,
            writable: true,
            executable: true,
        };
        assert_eq!(perms_rwx.to_string(), "RWX");

        let perms_empty = PageTablePermissions {
            readable: false,
            writable: false,
            executable: false,
        };
        assert_eq!(perms_empty.to_string(), "---");
    }
}

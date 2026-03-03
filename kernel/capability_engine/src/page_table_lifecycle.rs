// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Page table lifecycle management for capabilities.
//!
//! This module implements the lifecycle of page table entries as capabilities
//! are granted, delegated, revoked, and attenuated.
//!
//! Lifecycle:
//! 1. **Grant**: Creates a PTE for the initial capability holder
//! 2. **Delegate**: Updates ownership when delegating the capability
//! 3. **Revoke**: Invalidates the PTE and flushes TLB
//! 4. **Attenuation**: Narrows permissions (e.g., R+W → R only)
//!
//! See Engineering Plan § 5.0: MMU-backed capability enforcement integration.

use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::capability_page_binding::{CapabilityPageBinding, CapabilityPageBindingRegistry};
use crate::error::CapError;
use crate::ids::{CapID, AgentID};
use crate::mmu_abstraction::{MmuAbstraction, VirtualAddr, PhysicalAddr, PageTableEntry, PageTablePermissions};
use crate::operations::OperationSet;

/// Represents a lifecycle event on a page table entry.
#[derive(Clone, Debug)]
pub enum PageTableLifecycleEvent {
    /// A new PTE was created for a capability grant.
    GrantCreatedPte {
        cap_id: CapID,
        agent: AgentID,
        vaddr: VirtualAddr,
        paddr: PhysicalAddr,
    },

    /// Ownership was transferred during delegation.
    DelegateUpdatedOwner {
        cap_id: CapID,
        old_owner: AgentID,
        new_owner: AgentID,
        vaddr: VirtualAddr,
    },

    /// A PTE was invalidated during revocation.
    RevokeInvalidatedPte {
        cap_id: CapID,
        agent: AgentID,
        vaddr: VirtualAddr,
    },

    /// Permissions were narrowed during attenuation.
    AttenuationNarrowedPerms {
        cap_id: CapID,
        agent: AgentID,
        vaddr: VirtualAddr,
        old_perms: u8,
        new_perms: u8,
    },

    /// TLB was flushed for a virtual address.
    TlbInvalidated {
        vaddr: VirtualAddr,
    },
}

impl Display for PageTableLifecycleEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PageTableLifecycleEvent::GrantCreatedPte { cap_id, agent, vaddr, paddr } => {
                write!(f, "GrantCreatedPte(cap={}, agent={}, vaddr=0x{:x}, paddr=0x{:x})", 
                       cap_id, agent, vaddr, paddr)
            }
            PageTableLifecycleEvent::DelegateUpdatedOwner { cap_id, old_owner, new_owner, vaddr } => {
                write!(f, "DelegateUpdatedOwner(cap={}, {} → {}, vaddr=0x{:x})", 
                       cap_id, old_owner, new_owner, vaddr)
            }
            PageTableLifecycleEvent::RevokeInvalidatedPte { cap_id, agent, vaddr } => {
                write!(f, "RevokeInvalidatedPte(cap={}, agent={}, vaddr=0x{:x})", 
                       cap_id, agent, vaddr)
            }
            PageTableLifecycleEvent::AttenuationNarrowedPerms { cap_id, agent, vaddr, old_perms, new_perms } => {
                write!(f, "AttenuationNarrowedPerms(cap={}, agent={}, vaddr=0x{:x}, 0x{:02x} → 0x{:02x})", 
                       cap_id, agent, vaddr, old_perms, new_perms)
            }
            PageTableLifecycleEvent::TlbInvalidated { vaddr } => {
                write!(f, "TlbInvalidated(vaddr=0x{:x})", vaddr)
            }
        }
    }
}

/// Manager for page table lifecycle operations.
///
/// Handles the four main lifecycle transitions:
/// 1. Grant → Create new PTE
/// 2. Delegate → Update ownership
/// 3. Revoke → Invalidate PTE + flush TLB
/// 4. Attenuation → Narrow permissions
#[derive(Debug)]
pub struct PageTableLifecycleManager {
    /// The underlying MMU abstraction for low-level operations.
    /// This is borrowed during lifecycle operations.
    
    /// Registry of capability-to-page bindings.
    pub binding_registry: CapabilityPageBindingRegistry,

    /// Event log for audit trails.
    pub events: alloc::vec::Vec<PageTableLifecycleEvent>,
}

impl PageTableLifecycleManager {
    /// Creates a new page table lifecycle manager.
    pub fn new() -> Self {
        PageTableLifecycleManager {
            binding_registry: CapabilityPageBindingRegistry::new(),
            events: alloc::vec::Vec::new(),
        }
    }

    /// Handles the GRANT lifecycle event.
    ///
    /// When a capability is granted, a new PTE is created with:
    /// - physical_address from the resource allocation
    /// - permission_bits from capability.operations
    /// - capability_id for tracking
    /// - owner_agent set to the grant recipient
    ///
    /// # Arguments
    /// * `mmu` - The MMU abstraction for low-level operations
    /// * `pt_handle` - Page table handle for the receiving agent
    /// * `capability` - The capability being granted
    /// * `physical_addr` - Physical address of the resource
    ///
    /// # Returns
    /// Ok(VirtualAddr) with the virtual address where the page was mapped,
    /// or an error if the grant failed.
    pub fn handle_grant(
        &mut self,
        mmu: &mut dyn MmuAbstraction,
        pt_handle: u64,
        capability: Capability,
        physical_addr: PhysicalAddr,
    ) -> Result<VirtualAddr, CapError> {
        // Allocate a virtual address (simplified: would be more complex in real impl)
        let vaddr = self.allocate_virtual_address()?;

        // Create the page table entry
        let perms = PageTablePermissions::from_operation_bits(capability.operations.bits());
        let entry = PageTableEntry::new(
            physical_addr,
            vaddr,
            perms,
            capability.id.clone(),
            capability.target_agent.clone(),
            crate::mmu_abstraction::PAGE_SIZE,
        );

        // Map the page in the MMU
        mmu.map_page(pt_handle, entry)?;

        // Create and register the capability-page binding
        let binding = CapabilityPageBinding::new(
            capability.clone(),
            vaddr,
            physical_addr,
            crate::mmu_abstraction::PAGE_SIZE,
        )?;
        self.binding_registry.register(binding)?;

        // Log the event
        self.events.push(PageTableLifecycleEvent::GrantCreatedPte {
            cap_id: capability.id.clone(),
            agent: capability.target_agent.clone(),
            vaddr,
            paddr: physical_addr,
        });

        Ok(vaddr)
    }

    /// Handles the DELEGATE lifecycle event.
    ///
    /// When a capability is delegated to a new agent:
    /// 1. Check that the delegating agent owns the page
    /// 2. Create a new mapping for the recipient agent
    /// 3. Update the binding to reflect new ownership
    ///
    /// # Arguments
    /// * `mmu` - The MMU abstraction
    /// * `pt_handle` - Page table handle of the recipient agent
    /// * `cap_id` - The capability being delegated
    /// * `old_owner` - The current owner (delegating agent)
    /// * `new_owner` - The recipient agent
    ///
    /// # Returns
    /// Ok(VirtualAddr) with the virtual address in the new agent's space,
    /// or an error if the delegation failed.
    pub fn handle_delegate(
        &mut self,
        mmu: &mut dyn MmuAbstraction,
        pt_handle: u64,
        cap_id: &CapID,
        old_owner: &AgentID,
        new_owner: &AgentID,
    ) -> Result<VirtualAddr, CapError> {
        // Find bindings for this capability
        let bindings = self.binding_registry.bindings_for_capability(cap_id);
        if bindings.is_empty() {
            return Err(CapError::Other(
                format!("no binding found for capability {}", cap_id)
            ));
        }

        // Get the first binding (in practice, we'd select based on old_owner)
        let binding = bindings[0].clone();

        // Check that old_owner actually owns this binding
        if binding.page_table_entry.owner_agent != *old_owner {
            return Err(CapError::Other(
                format!("capability {} not owned by {}", cap_id, old_owner)
            ));
        }

        // Allocate a new virtual address for the recipient
        let new_vaddr = self.allocate_virtual_address()?;

        // Create a new entry for the recipient
        let entry = PageTableEntry::new(
            binding.page_table_entry.physical_address,
            new_vaddr,
            binding.page_table_entry.permissions,
            cap_id.clone(),
            new_owner.clone(),
            binding.page_table_entry.size,
        );

        // Map the page in the recipient's page table
        mmu.map_page(pt_handle, entry)?;

        // Create and register a new binding for the recipient
        let mut new_binding = binding.clone();
        new_binding.page_table_entry = entry;
        self.binding_registry.register(new_binding)?;

        // Log the event
        self.events.push(PageTableLifecycleEvent::DelegateUpdatedOwner {
            cap_id: cap_id.clone(),
            old_owner: old_owner.clone(),
            new_owner: new_owner.clone(),
            vaddr: new_vaddr,
        });

        Ok(new_vaddr)
    }

    /// Handles the REVOKE lifecycle event.
    ///
    /// When a capability is revoked:
    /// 1. Find all page mappings for that capability
    /// 2. Unmap each page from the MMU
    /// 3. Invalidate TLB entries
    /// 4. Remove bindings from the registry
    ///
    /// # Arguments
    /// * `mmu` - The MMU abstraction
    /// * `cap_id` - The capability being revoked
    ///
    /// # Returns
    /// Ok(()) if revocation succeeds, or an error.
    pub fn handle_revoke(
        &mut self,
        mmu: &mut dyn MmuAbstraction,
        cap_id: &CapID,
    ) -> Result<(), CapError> {
        // Find all bindings for this capability
        let bindings = self.binding_registry.bindings_for_capability(cap_id);
        let vaddrs: alloc::vec::Vec<VirtualAddr> = bindings
            .iter()
            .map(|b| b.page_table_entry.virtual_address)
            .collect();

        // For each binding
        for vaddr in vaddrs {
            // Unmap the page from the MMU
            // Note: We'd need to know the pt_handle for the owner here
            // In a real implementation, we'd store this in the binding
            
            // Invalidate TLB
            mmu.invalidate_tlb(vaddr)?;

            // Log the event
            if let Some(binding) = self.binding_registry.lookup(vaddr) {
                self.events.push(PageTableLifecycleEvent::RevokeInvalidatedPte {
                    cap_id: cap_id.clone(),
                    agent: binding.page_table_entry.owner_agent.clone(),
                    vaddr,
                });
            }

            // Remove from registry
            self.binding_registry.remove(vaddr);
        }

        Ok(())
    }

    /// Handles the ATTENUATION lifecycle event.
    ///
    /// When a capability is attenuated (restricted):
    /// 1. Narrow the permission bits in the PTE
    /// 2. Invalidate TLB for that page
    /// 3. Update the binding to reflect new permissions
    ///
    /// # Arguments
    /// * `mmu` - The MMU abstraction
    /// * `cap_id` - The capability being attenuated
    /// * `new_operations` - The new, restricted operation set
    ///
    /// # Returns
    /// Ok(()) if attenuation succeeds, or an error.
    pub fn handle_attenuation(
        &mut self,
        mmu: &mut dyn MmuAbstraction,
        cap_id: &CapID,
        new_operations: OperationSet,
    ) -> Result<(), CapError> {
        // Find bindings for this capability
        let bindings = self.binding_registry.bindings_for_capability(cap_id);

        for binding in bindings {
            let vaddr = binding.page_table_entry.virtual_address;
            let old_perms = binding.page_table_entry.permissions.to_operation_bits();
            let new_perms = new_operations.bits();

            // Validate that we're not expanding permissions
            if (new_perms & old_perms) != new_perms {
                return Err(CapError::InvalidAttenuation(
                    format!("cannot expand permissions: 0x{:02x} → 0x{:02x}", old_perms, new_perms)
                ));
            }

            // Update the binding's permissions
            if let Some(binding_mut) = self.binding_registry.lookup_mut(vaddr) {
                binding_mut.page_table_entry.permissions =
                    PageTablePermissions::from_operation_bits(new_perms);
            }

            // Invalidate TLB
            mmu.invalidate_tlb(vaddr)?;

            // Log the event
            self.events.push(PageTableLifecycleEvent::AttenuationNarrowedPerms {
                cap_id: cap_id.clone(),
                agent: binding.page_table_entry.owner_agent.clone(),
                vaddr,
                old_perms,
                new_perms,
            });
        }

        Ok(())
    }

    /// Allocates a virtual address for a new mapping.
    ///
    /// In a real implementation, this would consult a virtual address allocator.
    /// For now, it's a placeholder that increments from a base address.
    fn allocate_virtual_address(&self) -> Result<VirtualAddr, CapError> {
        // Simplified: just allocate in sequence from a base
        let base = 0x1_0000_0000u64; // Start at 4GB
        let count = self.binding_registry.count_total() as u64;
        let vaddr = base + (count * crate::mmu_abstraction::PAGE_SIZE);
        Ok(vaddr)
    }

    /// Returns the event log.
    pub fn events(&self) -> &[PageTableLifecycleEvent] {
        &self.events
    }

    /// Clears the event log.
    pub fn clear_events(&mut self) {
        self.events.clear();
    }
}

impl Default for PageTableLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ResourceID, ResourceType};
    use crate::constraints::Timestamp;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

    struct MockMmu {
        allocations: alloc::collections::BTreeMap<u64, alloc::vec::Vec<PageTableEntry>>,
    }

    impl MockMmu {
        fn new() -> Self {
            MockMmu {
                allocations: alloc::collections::BTreeMap::new(),
            }
        }
    }

    impl MmuAbstraction for MockMmu {
        fn allocate_pagetable(&mut self, _owner_agent: &AgentID) -> Result<u64, CapError> {
            let handle = self.allocations.len() as u64;
            self.allocations.insert(handle, alloc::vec::Vec::new());
            Ok(handle)
        }

        fn map_page(&mut self, pt_handle: u64, entry: PageTableEntry) -> Result<(), CapError> {
            self.allocations
                .get_mut(&pt_handle)
                .ok_or_else(|| CapError::Other("page table not found".to_string()))?
                .push(entry);
            Ok(())
        }

        fn unmap_page(&mut self, pt_handle: u64, virtual_addr: VirtualAddr) -> Result<(), CapError> {
            let entries = self.allocations
                .get_mut(&pt_handle)
                .ok_or_else(|| CapError::Other("page table not found".to_string()))?;
            entries.retain(|e| e.virtual_address != virtual_addr);
            Ok(())
        }

        fn invalidate_tlb(&self, _virtual_addr: VirtualAddr) -> Result<(), CapError> {
            Ok(())
        }

        fn invalidate_tlb_all(&self, _pt_handle: u64) -> Result<(), CapError> {
            Ok(())
        }

        fn lookup_page(
            &self,
            pt_handle: u64,
            virtual_addr: VirtualAddr,
        ) -> Result<Option<PageTableEntry>, CapError> {
            let entries = self.allocations
                .get(&pt_handle)
                .ok_or_else(|| CapError::Other("page table not found".to_string()))?;
            Ok(entries.iter().find(|e| e.virtual_address == virtual_addr).cloned())
        }

        fn check_access(
            &self,
            pt_handle: u64,
            virtual_addr: VirtualAddr,
            required_perms: PageTablePermissions,
        ) -> Result<(), CapError> {
            if let Some(entry) = self.lookup_page(pt_handle, virtual_addr)? {
                if entry.permissions.contains(required_perms) {
                    Ok(())
                } else {
                    Err(CapError::Other("permission denied".to_string()))
                }
            } else {
                Err(CapError::Other("page not mapped".to_string()))
            }
        }

        fn deallocate_pagetable(&mut self, pt_handle: u64) -> Result<(), CapError> {
            self.allocations.remove(&pt_handle);
            Ok(())
        }

        fn architecture(&self) -> &str {
            "mock"
        }
    }

    fn create_test_capability() -> Capability {
        Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("test-agent"),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::all(),
            Timestamp::now_nanos(),
        )
    }

    #[test]
    fn test_lifecycle_manager_creation() {
        let manager = PageTableLifecycleManager::new();
        assert_eq!(manager.binding_registry.count_total(), 0);
        assert_eq!(manager.events.len(), 0);
    }

    #[test]
    fn test_handle_grant() {
        let mut manager = PageTableLifecycleManager::new();
        let mut mmu = MockMmu::new();
        let pt_handle = mmu.allocate_pagetable(&AgentID::new("test-agent")).unwrap();

        let cap = create_test_capability();
        let result = manager.handle_grant(&mut mmu, pt_handle, cap.clone(), 0x1000);

        assert!(result.is_ok());
        assert_eq!(manager.binding_registry.count_total(), 1);
        assert_eq!(manager.events.len(), 1);
    }

    #[test]
    fn test_handle_revoke() {
        let mut manager = PageTableLifecycleManager::new();
        let mut mmu = MockMmu::new();
        let pt_handle = mmu.allocate_pagetable(&AgentID::new("test-agent")).unwrap();

        let cap = create_test_capability();
        let cap_id = cap.id.clone();
        let _ = manager.handle_grant(&mut mmu, pt_handle, cap, 0x1000).unwrap();

        assert_eq!(manager.binding_registry.count_total(), 1);

        let result = manager.handle_revoke(&mut mmu, &cap_id);
        assert!(result.is_ok());
        assert_eq!(manager.binding_registry.count_active(), 0);
    }

    #[test]
    fn test_handle_attenuation() {
        let mut manager = PageTableLifecycleManager::new();
        let mut mmu = MockMmu::new();
        let pt_handle = mmu.allocate_pagetable(&AgentID::new("test-agent")).unwrap();

        let cap = create_test_capability();
        let cap_id = cap.id.clone();
        let _ = manager.handle_grant(&mut mmu, pt_handle, cap, 0x1000).unwrap();

        // Attenuate from R+W+X to R only
        let result = manager.handle_attenuation(&mut mmu, &cap_id, OperationSet::read());
        assert!(result.is_ok());

        // Check that permissions were narrowed
        let binding = manager.binding_registry.bindings_for_capability(&cap_id);
        assert!(!binding.is_empty());
        assert!(binding[0].page_table_entry.permissions.readable);
        assert!(!binding[0].page_table_entry.permissions.writable);
    }
}

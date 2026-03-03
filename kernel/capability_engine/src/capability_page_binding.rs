// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Capability-to-page-table binding and enforcement.
//!
//! This module implements the core enforcement rule:
//! **No capability = No PTE = No access (fail-safe).**
//!
//! Each page table entry (PTE) is bound to a capability ID and includes:
//! - physical_address: the actual phys addr
//! - permission_bits: derived from Capability.operations (read, write, execute)
//! - capability_id: the CapID authorizing the mapping
//! - owner_agent: the AgentID who holds the capability
//!
//! See Engineering Plan § 5.0: MMU-backed capability enforcement integration.

use alloc::collections::BTreeMap;
use core::fmt::{self, Debug, Display};

use crate::capability::Capability;
use crate::error::CapError;
use crate::ids::{CapID, AgentID};
use crate::mmu_abstraction::{PageTableEntry, PageTablePermissions, VirtualAddr, PhysicalAddr};
use crate::operations::OperationSet;

/// A binding between a capability and a page table mapping.
///
/// This structure ensures that:
/// 1. Every page table entry corresponds to a valid capability
/// 2. Permission bits are derived from Capability.operations
/// 3. Access is only permitted if the capability exists and is valid
/// 4. Revoking the capability immediately invalidates the mapping
#[derive(Clone, Debug)]
pub struct CapabilityPageBinding {
    /// The capability that authorizes this page mapping.
    pub capability: Capability,

    /// The page table entry that was created from this capability.
    pub page_table_entry: PageTableEntry,

    /// Whether this binding is currently active (not revoked).
    pub is_active: bool,
}

impl CapabilityPageBinding {
    /// Creates a new capability-page binding.
    ///
    /// Validates that:
    /// 1. The capability exists and is valid
    /// 2. Permission bits are correctly derived from capability.operations
    /// 3. The virtual/physical addresses are properly aligned
    ///
    /// # Arguments
    /// * `capability` - The capability authorizing the mapping
    /// * `virtual_addr` - Virtual address to map the page at
    /// * `physical_addr` - Physical address of the actual memory
    /// * `size` - Size of the mapping (typically PAGE_SIZE)
    ///
    /// # Returns
    /// A new binding if validation succeeds, or an error.
    pub fn new(
        capability: Capability,
        virtual_addr: VirtualAddr,
        physical_addr: PhysicalAddr,
        size: u64,
    ) -> Result<Self, CapError> {
        // Validate alignment
        if virtual_addr % crate::mmu_abstraction::PAGE_SIZE != 0 {
            return Err(CapError::Other(
                format!("virtual address not aligned: 0x{:x}", virtual_addr)
            ));
        }

        if physical_addr % crate::mmu_abstraction::PAGE_SIZE != 0 {
            return Err(CapError::Other(
                format!("physical address not aligned: 0x{:x}", physical_addr)
            ));
        }

        // Derive permissions from capability.operations
        let perms = PageTablePermissions::from_operation_bits(capability.operations.bits());

        // Create page table entry bound to this capability
        let pte = PageTableEntry::new(
            physical_addr,
            virtual_addr,
            perms,
            capability.id.clone(),
            capability.target_agent.clone(),
            size,
        );

        Ok(CapabilityPageBinding {
            capability,
            page_table_entry: pte,
            is_active: true,
        })
    }

    /// Checks if the capability for this binding is still valid.
    ///
    /// Returns Ok(()) if the binding is active and the capability is valid,
    /// or an error if the binding has been revoked or the capability is invalid.
    pub fn validate_binding(&self, now: u64) -> Result<(), CapError> {
        if !self.is_active {
            return Err(CapError::Other(
                "binding has been revoked".to_string()
            ));
        }

        // Check if the capability itself is still valid
        self.capability.is_valid_at(now)
    }

    /// Revokes this binding, preventing further access through this mapping.
    ///
    /// Sets is_active to false and invalidates the PTE.
    /// The page table entry becomes useless and should be removed from the MMU.
    pub fn revoke(&mut self) {
        self.is_active = false;
    }

    /// Checks if a specific operation is allowed by this binding.
    ///
    /// Returns Ok(()) if the permission is granted in the PTE.
    pub fn check_permission(&self, operation: u8) -> Result<(), CapError> {
        if !self.is_active {
            return Err(CapError::Other(
                "binding has been revoked".to_string()
            ));
        }

        if !self.capability.allows_operation(operation) {
            return Err(CapError::InsufficientOperations {
                required: format!("operation 0x{:02x}", operation),
                have: format!("0x{:02x}", self.capability.operations.bits()),
            });
        }

        Ok(())
    }

    /// Checks if the page is accessible for reading.
    pub fn check_read_permission(&self) -> Result<(), CapError> {
        self.check_permission(OperationSet::READ)
    }

    /// Checks if the page is accessible for writing.
    pub fn check_write_permission(&self) -> Result<(), CapError> {
        self.check_permission(OperationSet::WRITE)
    }

    /// Checks if the page is accessible for execution.
    pub fn check_execute_permission(&self) -> Result<(), CapError> {
        self.check_permission(OperationSet::EXECUTE)
    }
}

impl Display for CapabilityPageBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CapPageBinding(cap={}, vaddr=0x{:x}, active={})",
            self.capability.id, self.page_table_entry.virtual_address, self.is_active
        )
    }
}

/// A registry of capability-to-page bindings.
///
/// This is the core data structure that enforces the rule:
/// **No capability = No PTE = No access.**
///
/// Each virtual address is mapped to a binding, which is only valid if:
/// 1. The binding exists in this registry
/// 2. The binding is marked as active (not revoked)
/// 3. The underlying capability is valid
#[derive(Clone, Debug, Default)]
pub struct CapabilityPageBindingRegistry {
    /// Map from virtual address to capability-page binding.
    /// Using virtual address as key ensures O(log n) lookup on fault.
    bindings: BTreeMap<VirtualAddr, CapabilityPageBinding>,
}

impl CapabilityPageBindingRegistry {
    /// Creates a new, empty binding registry.
    pub fn new() -> Self {
        CapabilityPageBindingRegistry {
            bindings: BTreeMap::new(),
        }
    }

    /// Registers a new capability-page binding.
    ///
    /// If a binding already exists at this virtual address, it is replaced.
    pub fn register(&mut self, binding: CapabilityPageBinding) -> Result<(), CapError> {
        self.bindings.insert(binding.page_table_entry.virtual_address, binding);
        Ok(())
    }

    /// Looks up a binding by virtual address.
    ///
    /// Returns a reference to the binding if it exists.
    pub fn lookup(&self, vaddr: VirtualAddr) -> Option<&CapabilityPageBinding> {
        self.bindings.get(&vaddr)
    }

    /// Mutably looks up a binding by virtual address.
    pub fn lookup_mut(&mut self, vaddr: VirtualAddr) -> Option<&mut CapabilityPageBinding> {
        self.bindings.get_mut(&vaddr)
    }

    /// Revokes a binding at the given virtual address.
    ///
    /// Returns Ok(()) if the binding was found and revoked, or an error otherwise.
    pub fn revoke(&mut self, vaddr: VirtualAddr) -> Result<(), CapError> {
        if let Some(binding) = self.bindings.get_mut(&vaddr) {
            binding.revoke();
            Ok(())
        } else {
            Err(CapError::Other(
                format!("no binding found at vaddr 0x{:x}", vaddr)
            ))
        }
    }

    /// Removes a binding from the registry entirely.
    ///
    /// Used when unmapping pages during revocation.
    pub fn remove(&mut self, vaddr: VirtualAddr) -> Option<CapabilityPageBinding> {
        self.bindings.remove(&vaddr)
    }

    /// Returns the number of active bindings in the registry.
    pub fn count_active(&self) -> usize {
        self.bindings.values().filter(|b| b.is_active).count()
    }

    /// Returns the total number of bindings (active and revoked).
    pub fn count_total(&self) -> usize {
        self.bindings.len()
    }

    /// Checks if a virtual address has an active binding.
    pub fn has_active_binding(&self, vaddr: VirtualAddr) -> bool {
        self.bindings
            .get(&vaddr)
            .map(|b| b.is_active)
            .unwrap_or(false)
    }

    /// Validates all bindings against a given timestamp.
    ///
    /// Returns a count of invalid bindings that should be revoked.
    pub fn count_invalid_bindings(&self, now: u64) -> usize {
        self.bindings
            .values()
            .filter(|b| b.validate_binding(now).is_err())
            .count()
    }

    /// Returns all active bindings for a specific agent.
    pub fn bindings_for_agent(&self, agent: &AgentID) -> Vec<&CapabilityPageBinding> {
        self.bindings
            .values()
            .filter(|b| {
                b.is_active && b.page_table_entry.owner_agent == *agent
            })
            .collect()
    }

    /// Returns all bindings for a specific capability.
    pub fn bindings_for_capability(&self, cap_id: &CapID) -> Vec<&CapabilityPageBinding> {
        self.bindings
            .values()
            .filter(|b| b.page_table_entry.capability_id == *cap_id)
            .collect()
    }

    /// Revokes all bindings for a specific capability.
    ///
    /// Used during capability revocation to ensure all mappings are removed.
    pub fn revoke_capability(&mut self, cap_id: &CapID) -> Result<(), CapError> {
        let vaddrs: Vec<VirtualAddr> = self.bindings
            .values()
            .filter(|b| b.page_table_entry.capability_id == *cap_id)
            .map(|b| b.page_table_entry.virtual_address)
            .collect();

        for vaddr in vaddrs {
            self.revoke(vaddr)?;
        }

        Ok(())
    }
}

impl Display for CapabilityPageBindingRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CapPageBindingRegistry(active={}, total={})",
            self.count_active(),
            self.count_total()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ResourceID, ResourceType};
    use crate::constraints::Timestamp;
use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

    fn create_test_capability() -> Capability {
        Capability::new(
            CapID::from_bytes([1u8; 32]),
            AgentID::new("test-agent"),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::read().union(OperationSet::write()),
            Timestamp::now_nanos(),
        )
    }

    #[test]
    fn test_capability_page_binding_creation() {
        let cap = create_test_capability();
        let binding = CapabilityPageBinding::new(
            cap.clone(),
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        );

        assert!(binding.is_ok());
        let binding = binding.unwrap();
        assert!(binding.is_active);
        assert_eq!(binding.page_table_entry.virtual_address, 0x10000);
        assert_eq!(binding.page_table_entry.physical_address, 0x1000);
    }

    #[test]
    fn test_capability_page_binding_unaligned_vaddr() {
        let cap = create_test_capability();
        let binding = CapabilityPageBinding::new(
            cap.clone(),
            0x10001, // Unaligned
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        );

        assert!(binding.is_err());
    }

    #[test]
    fn test_capability_page_binding_unaligned_paddr() {
        let cap = create_test_capability();
        let binding = CapabilityPageBinding::new(
            cap.clone(),
            0x10000,
            0x1001, // Unaligned
            crate::mmu_abstraction::PAGE_SIZE,
        );

        assert!(binding.is_err());
    }

    #[test]
    fn test_capability_page_binding_permissions() {
        let cap = create_test_capability();
        let binding = CapabilityPageBinding::new(
            cap.clone(),
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        // Should have both read and write
        assert!(binding.check_read_permission().is_ok());
        assert!(binding.check_write_permission().is_ok());
    }

    #[test]
    fn test_capability_page_binding_revoke() {
        let cap = create_test_capability();
        let mut binding = CapabilityPageBinding::new(
            cap.clone(),
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        assert!(binding.is_active);
        binding.revoke();
        assert!(!binding.is_active);

        // Should fail after revocation
        assert!(binding.check_read_permission().is_err());
    }

    #[test]
    fn test_binding_registry_register_and_lookup() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let cap = create_test_capability();
        let binding = CapabilityPageBinding::new(
            cap.clone(),
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        registry.register(binding).unwrap();
        assert!(registry.lookup(0x10000).is_some());
        assert!(registry.lookup(0x20000).is_none());
    }

    #[test]
    fn test_binding_registry_revoke() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let cap = create_test_capability();
        let binding = CapabilityPageBinding::new(
            cap.clone(),
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        registry.register(binding).unwrap();
        assert!(registry.has_active_binding(0x10000));

        registry.revoke(0x10000).unwrap();
        assert!(!registry.has_active_binding(0x10000));
    }

    #[test]
    fn test_binding_registry_count_active() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let cap = create_test_capability();

        for i in 0..5 {
            let binding = CapabilityPageBinding::new(
                cap.clone(),
                0x10000 + (i * 0x1000),
                0x1000 + (i * 0x1000),
                crate::mmu_abstraction::PAGE_SIZE,
            ).unwrap();
            registry.register(binding).unwrap();
        }

        assert_eq!(registry.count_active(), 5);
        assert_eq!(registry.count_total(), 5);

        registry.revoke(0x10000).unwrap();
        assert_eq!(registry.count_active(), 4);
        assert_eq!(registry.count_total(), 5);
    }

    #[test]
    fn test_binding_registry_revoke_capability() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let cap = create_test_capability();
        let cap_id = cap.id.clone();

        // Register three bindings with the same capability
        for i in 0..3 {
            let binding = CapabilityPageBinding::new(
                cap.clone(),
                0x10000 + (i * 0x1000),
                0x1000 + (i * 0x1000),
                crate::mmu_abstraction::PAGE_SIZE,
            ).unwrap();
            registry.register(binding).unwrap();
        }

        assert_eq!(registry.count_active(), 3);

        // Revoke all bindings for this capability
        registry.revoke_capability(&cap_id).unwrap();
        assert_eq!(registry.count_active(), 0);
        assert_eq!(registry.count_total(), 3);
    }

    #[test]
    fn test_binding_registry_bindings_for_agent() {
        let mut registry = CapabilityPageBindingRegistry::new();
        let agent1 = AgentID::new("agent-1");
        let agent2 = AgentID::new("agent-2");

        let cap1 = Capability::new(
            CapID::from_bytes([1u8; 32]),
            agent1.clone(),
            ResourceType::memory(),
            ResourceID::new("mem:0x1000"),
            OperationSet::all(),
            Timestamp::now_nanos(),
        );

        let cap2 = Capability::new(
            CapID::from_bytes([2u8; 32]),
            agent2.clone(),
            ResourceType::memory(),
            ResourceID::new("mem:0x2000"),
            OperationSet::all(),
            Timestamp::now_nanos(),
        );

        let binding1 = CapabilityPageBinding::new(
            cap1,
            0x10000,
            0x1000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        let binding2 = CapabilityPageBinding::new(
            cap2,
            0x20000,
            0x2000,
            crate::mmu_abstraction::PAGE_SIZE,
        ).unwrap();

        registry.register(binding1).unwrap();
        registry.register(binding2).unwrap();

        assert_eq!(registry.bindings_for_agent(&agent1).len(), 1);
        assert_eq!(registry.bindings_for_agent(&agent2).len(), 1);
    }
}

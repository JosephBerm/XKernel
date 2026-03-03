// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! MMU integration for memory protection in the Memory Manager.
//!
//! This module provides Memory Management Unit (MMU) integration for enforcing
//! memory protection domains, handling page faults, and managing hardware-level
//! memory protection features (NX bit, huge pages, TLB invalidation).
//!
//! See Engineering Plan § 4.1.3: Memory Protection & MMU Integration.

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};

/// MMU configuration parameters.
///
/// Specifies hardware capabilities and settings for memory protection.
/// See Engineering Plan § 4.1.3: MMU Configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MmuConfig {
    /// Page size in bytes (typically 4096 for 4KiB pages)
    pub page_size: u64,
    /// Whether the CPU supports huge pages (2MiB or 1GiB)
    pub huge_page_support: bool,
    /// Whether NX (no-execute) bit is supported
    pub nx_bit_support: bool,
}

impl MmuConfig {
    /// Creates a new MMU configuration with standard 4KiB pages.
    pub fn new(page_size: u64, huge_pages: bool, nx_bit: bool) -> Self {
        MmuConfig {
            page_size,
            huge_page_support: huge_pages,
            nx_bit_support: nx_bit,
        }
    }

    /// Creates a default configuration (4KiB pages, no huge pages, no NX).
    pub fn default_4k() -> Self {
        MmuConfig {
            page_size: 4096,
            huge_page_support: false,
            nx_bit_support: false,
        }
    }

    /// Creates a configuration with all features enabled.
    pub fn full_featured() -> Self {
        MmuConfig {
            page_size: 4096,
            huge_page_support: true,
            nx_bit_support: true,
        }
    }

    /// Validates a page size against this configuration.
    pub fn is_valid_page_size(&self, size: u64) -> bool {
        if size == self.page_size {
            return true;
        }
        if self.huge_page_support && (size == 2 * 1024 * 1024 || size == 1024 * 1024 * 1024) {
            return true;
        }
        false
    }
}

/// Protection domain enumeration - defines memory access rules.
///
/// Different domains enforce different isolation and access policies:
/// - KernelOnly: Only kernel can access
/// - ServiceProcess: Memory Manager service can access
/// - CtUserSpace: Individual CT can access
/// - SharedReadOnly: Multiple CTs can read
/// - SharedReadWrite: Multiple CTs can read and write
///
/// See Engineering Plan § 4.1.3: Protection Domains.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProtectionDomain {
    /// Kernel-only memory (never accessible by user code)
    KernelOnly,
    /// Service process (Memory Manager) only
    ServiceProcess,
    /// Single CT user space - only this CT can access
    CtUserSpace {
        ct_id: String,
    },
    /// Shared read-only across multiple CTs
    SharedReadOnly {
        crew_id: String,
    },
    /// Shared read-write across multiple CTs (requires CRDT)
    SharedReadWrite {
        crew_id: String,
    },
}

impl ProtectionDomain {
    /// Returns a human-readable name for this domain.
    pub fn name(&self) -> &str {
        match self {
            ProtectionDomain::KernelOnly => "kernel_only",
            ProtectionDomain::ServiceProcess => "service_process",
            ProtectionDomain::CtUserSpace { .. } => "ct_user_space",
            ProtectionDomain::SharedReadOnly { .. } => "shared_read_only",
            ProtectionDomain::SharedReadWrite { .. } => "shared_read_write",
        }
    }
}

/// Domain ID - strongly typed identifier for a protection domain.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DomainId(String);

impl DomainId {
    /// Creates a new domain ID.
    pub fn new(id: impl Into<String>) -> Self {
        DomainId(id.into())
    }

    /// Returns the domain ID as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Types of memory access - used in protection enforcement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AccessType {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute access
    Execute,
}

impl AccessType {
    /// Converts access type to bitmask (1=read, 2=write, 4=execute).
    pub fn to_bitmask(&self) -> u8 {
        match self {
            AccessType::Read => 1,
            AccessType::Write => 2,
            AccessType::Execute => 4,
        }
    }
}

/// Resolution for a page fault - what action to take.
///
/// Returned by the page fault handler to indicate how to resolve
/// the fault (map a page, demand page, CoW clone, deny, or OOM).
///
/// See Engineering Plan § 4.1.3: Page Fault Handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageFaultResolution {
    /// Map the physical page to the faulting address
    MapPage {
        physical_page: u64,
    },
    /// Demand-page: allocate and zero a new page
    DemandPage,
    /// Copy-on-Write: clone the parent page
    CowClone,
    /// Access denied (protection violation)
    AccessDenied,
    /// Out of memory
    SignalOom,
}

/// Page fault handler - resolves page faults from the hardware MMU.
///
/// When the CPU generates a page fault, the kernel calls this handler
/// to determine whether to map a page, allocate one, or signal an error.
///
/// See Engineering Plan § 4.1.3: Fault Handling.
pub struct PageFaultHandler {
    /// MMU configuration (page size, features)
    pub mmu_config: MmuConfig,
    /// Domain ID that faulted
    pub domain_id: DomainId,
}

impl PageFaultHandler {
    /// Creates a new page fault handler.
    pub fn new(mmu_config: MmuConfig, domain_id: DomainId) -> Self {
        PageFaultHandler {
            mmu_config,
            domain_id,
        }
    }

    /// Handles a page fault from the MMU.
    ///
    /// # Arguments
    ///
    /// * `fault_addr` - Virtual address that faulted
    /// * `access_type` - Type of access (read, write, execute)
    /// * `is_present` - Whether the page is marked present in the page table
    ///
    /// # Returns
    ///
    /// `Result<PageFaultResolution>` with the action to take
    ///
    /// See Engineering Plan § 4.1.3: Fault Resolution.
    pub fn handle_page_fault(
        &self,
        fault_addr: u64,
        access_type: AccessType,
        is_present: bool,
    ) -> Result<PageFaultResolution> {
        // If the page is not marked present, allocate it (demand paging)
        if !is_present {
            // Simple policy: always demand-page
            return Ok(PageFaultResolution::DemandPage);
        }

        // If present but access denied, check permissions
        // (This is a simplified model; real MMU would have permission bits)
        match access_type {
            AccessType::Read => {
                // Reads usually succeed if page is present
                Ok(PageFaultResolution::MapPage {
                    physical_page: 0, // Placeholder
                })
            }
            AccessType::Write => {
                // Check for CoW (Copy-on-Write) bit
                Ok(PageFaultResolution::CowClone)
            }
            AccessType::Execute => {
                // Check NX bit enforcement
                if self.mmu_config.nx_bit_support {
                    Ok(PageFaultResolution::AccessDenied)
                } else {
                    Ok(PageFaultResolution::MapPage {
                        physical_page: 0,
                    })
                }
            }
        }
    }
}

/// MMU state tracker - manages page tables and TLB.
///
/// Tracks page table entries and invalidates TLB entries when
/// mappings change.
///
/// See Engineering Plan § 4.1.3: TLB Management.
pub struct MmuStateTracker {
    /// Mapping of domain_id -> (virtual_addr, physical_page)
    page_table_entries: Vec<(DomainId, u64, u64)>,
    /// TLB entries that need flushing (domain_id, virtual_addr)
    tlb_invalidation_list: Vec<(DomainId, u64)>,
}

impl MmuStateTracker {
    /// Creates a new MMU state tracker.
    pub fn new() -> Self {
        MmuStateTracker {
            page_table_entries: Vec::new(),
            tlb_invalidation_list: Vec::new(),
        }
    }

    /// Adds a page table entry.
    pub fn add_pte(&mut self, domain_id: DomainId, virtual_addr: u64, physical_page: u64) {
        self.page_table_entries
            .push((domain_id, virtual_addr, physical_page));
    }

    /// Invalidates a TLB entry (marks for flush).
    pub fn invalidate_tlb_entry(&mut self, domain_id: DomainId, virtual_addr: u64) {
        self.tlb_invalidation_list
            .push((domain_id, virtual_addr));
    }

    /// Flushes all TLB invalidation entries (returns the list and clears).
    pub fn flush_tlb_list(&mut self) -> Vec<(DomainId, u64)> {
        let list = self.tlb_invalidation_list.clone();
        self.tlb_invalidation_list.clear();
        list
    }

    /// Returns the count of page table entries.
    pub fn pte_count(&self) -> usize {
        self.page_table_entries.len()
    }
}

/// Protection domain manager - creates and enforces domains.
///
/// Manages the lifecycle of protection domains and enforces
/// access control rules.
///
/// See Engineering Plan § 4.1.3: Domain Management.
pub struct ProtectionDomainManager {
    /// Map of domain_id -> protection domain
    domains: alloc::collections::BTreeMap<String, ProtectionDomain>,
    /// MMU configuration
    mmu_config: MmuConfig,
}

impl ProtectionDomainManager {
    /// Creates a new protection domain manager.
    pub fn new(mmu_config: MmuConfig) -> Self {
        ProtectionDomainManager {
            domains: alloc::collections::BTreeMap::new(),
            mmu_config,
        }
    }

    /// Creates a new protection domain for a CT.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - Cognitive Thread identifier
    /// * `tier` - Memory tier (for informational purposes)
    /// * `permissions` - Permission bitmask (1=read, 2=write, 4=execute)
    ///
    /// # Returns
    ///
    /// `Result<DomainId>` with the created domain's ID
    ///
    /// See Engineering Plan § 4.1.3: Domain Creation.
    pub fn create_protection_domain(
        &mut self,
        ct_id: impl Into<String>,
        _tier: &str,
        _permissions: u8,
    ) -> Result<DomainId> {
        let ct_id_str = ct_id.into();
        let domain_id = DomainId::new(format!("domain-ct-{}", ct_id_str));

        let domain = ProtectionDomain::CtUserSpace {
            ct_id: ct_id_str.clone(),
        };

        self.domains.insert(domain_id.as_str().to_string(), domain);

        Ok(domain_id)
    }

    /// Enforces protection for a domain access.
    ///
    /// # Arguments
    ///
    /// * `domain_id` - Domain being accessed
    /// * `access_type` - Type of access
    ///
    /// # Returns
    ///
    /// `Result<()>` if access is allowed
    ///
    /// See Engineering Plan § 4.1.3: Protection Enforcement.
    pub fn enforce_protection(&self, domain_id: &DomainId, access_type: AccessType) -> Result<()> {
        let domain = self
            .domains
            .get(domain_id.as_str())
            .ok_or(MemoryError::InvalidReference {
                reason: format!("domain {} not found", domain_id.as_str()),
            })?;

        // Simple enforcement: always allow for now (real implementation would check permissions)
        match domain {
            ProtectionDomain::KernelOnly => {
                Err(MemoryError::CapabilityDenied {
                    operation: format!("access_{:?}", access_type),
                    resource: "kernel_only_domain".to_string(),
                })
            }
            ProtectionDomain::ServiceProcess => Ok(()),
            ProtectionDomain::CtUserSpace { .. } => {
                // Allow CT access
                Ok(())
            }
            ProtectionDomain::SharedReadOnly { .. } => {
                // Only read allowed
                match access_type {
                    AccessType::Read => Ok(()),
                    _ => Err(MemoryError::CapabilityDenied {
                        operation: format!("access_{:?}", access_type),
                        resource: "shared_read_only".to_string(),
                    }),
                }
            }
            ProtectionDomain::SharedReadWrite { .. } => {
                // Read and write allowed
                Ok(())
            }
        }
    }

    /// Returns the count of registered domains.
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_mmu_config_creation() {
        let config = MmuConfig::new(4096, true, true);
        assert_eq!(config.page_size, 4096);
        assert!(config.huge_page_support);
        assert!(config.nx_bit_support);
    }

    #[test]
    fn test_mmu_config_default_4k() {
        let config = MmuConfig::default_4k();
        assert_eq!(config.page_size, 4096);
        assert!(!config.huge_page_support);
        assert!(!config.nx_bit_support);
    }

    #[test]
    fn test_mmu_config_full_featured() {
        let config = MmuConfig::full_featured();
        assert_eq!(config.page_size, 4096);
        assert!(config.huge_page_support);
        assert!(config.nx_bit_support);
    }

    #[test]
    fn test_mmu_config_is_valid_page_size() {
        let config = MmuConfig::new(4096, true, true);
        assert!(config.is_valid_page_size(4096));
        assert!(config.is_valid_page_size(2 * 1024 * 1024)); // 2MiB
        assert!(config.is_valid_page_size(1024 * 1024 * 1024)); // 1GiB
    }

    #[test]
    fn test_protection_domain_kernel_only() {
        let domain = ProtectionDomain::KernelOnly;
        assert_eq!(domain.name(), "kernel_only");
    }

    #[test]
    fn test_protection_domain_service_process() {
        let domain = ProtectionDomain::ServiceProcess;
        assert_eq!(domain.name(), "service_process");
    }

    #[test]
    fn test_protection_domain_ct_user_space() {
        let domain = ProtectionDomain::CtUserSpace {
            ct_id: "ct-001".to_string(),
        };
        assert_eq!(domain.name(), "ct_user_space");
    }

    #[test]
    fn test_protection_domain_shared_read_only() {
        let domain = ProtectionDomain::SharedReadOnly {
            crew_id: "crew-001".to_string(),
        };
        assert_eq!(domain.name(), "shared_read_only");
    }

    #[test]
    fn test_protection_domain_shared_read_write() {
        let domain = ProtectionDomain::SharedReadWrite {
            crew_id: "crew-001".to_string(),
        };
        assert_eq!(domain.name(), "shared_read_write");
    }

    #[test]
    fn test_domain_id_creation() {
        let id = DomainId::new("domain-123");
        assert_eq!(id.as_str(), "domain-123");
    }

    #[test]
    fn test_access_type_to_bitmask() {
        assert_eq!(AccessType::Read.to_bitmask(), 1);
        assert_eq!(AccessType::Write.to_bitmask(), 2);
        assert_eq!(AccessType::Execute.to_bitmask(), 4);
    }

    #[test]
    fn test_page_fault_handler_creation() {
        let config = MmuConfig::default_4k();
        let domain_id = DomainId::new("domain-001");
        let handler = PageFaultHandler::new(config, domain_id.clone());
        assert_eq!(handler.domain_id, domain_id);
    }

    #[test]
    fn test_page_fault_handler_not_present() {
        let config = MmuConfig::default_4k();
        let domain_id = DomainId::new("domain-001");
        let handler = PageFaultHandler::new(config, domain_id);

        let result = handler.handle_page_fault(0x1000, AccessType::Read, false);
        assert!(result.is_ok());
        match result.unwrap() {
            PageFaultResolution::DemandPage => {
                // Expected
            }
            _ => panic!("Expected DemandPage"),
        }
    }

    #[test]
    fn test_page_fault_handler_present_read() {
        let config = MmuConfig::default_4k();
        let domain_id = DomainId::new("domain-001");
        let handler = PageFaultHandler::new(config, domain_id);

        let result = handler.handle_page_fault(0x1000, AccessType::Read, true);
        assert!(result.is_ok());
        match result.unwrap() {
            PageFaultResolution::MapPage { .. } => {
                // Expected
            }
            _ => panic!("Expected MapPage"),
        }
    }

    #[test]
    fn test_mmu_state_tracker_creation() {
        let tracker = MmuStateTracker::new();
        assert_eq!(tracker.pte_count(), 0);
    }

    #[test]
    fn test_mmu_state_tracker_add_pte() {
        let mut tracker = MmuStateTracker::new();
        let domain_id = DomainId::new("domain-001");
        tracker.add_pte(domain_id, 0x1000, 100);
        assert_eq!(tracker.pte_count(), 1);
    }

    #[test]
    fn test_mmu_state_tracker_invalidate_tlb() {
        let mut tracker = MmuStateTracker::new();
        let domain_id = DomainId::new("domain-001");
        tracker.invalidate_tlb_entry(domain_id.clone(), 0x1000);
        assert_eq!(tracker.tlb_invalidation_list.len(), 1);

        let list = tracker.flush_tlb_list();
        assert_eq!(list.len(), 1);
        assert_eq!(tracker.tlb_invalidation_list.len(), 0);
    }

    #[test]
    fn test_protection_domain_manager_creation() {
        let config = MmuConfig::default_4k();
        let manager = ProtectionDomainManager::new(config);
        assert_eq!(manager.domain_count(), 0);
    }

    #[test]
    fn test_protection_domain_manager_create_ct_domain() {
        let config = MmuConfig::default_4k();
        let mut manager = ProtectionDomainManager::new(config);

        let result = manager.create_protection_domain("ct-001", "L1", 0b011);
        assert!(result.is_ok());
        assert_eq!(manager.domain_count(), 1);
    }

    #[test]
    fn test_protection_domain_manager_enforce_service_process() {
        let config = MmuConfig::default_4k();
        let mut manager = ProtectionDomainManager::new(config);

        // Manually add a ServiceProcess domain for testing
        let domain_id = DomainId::new("service-domain");
        manager
            .domains
            .insert(domain_id.as_str().to_string(), ProtectionDomain::ServiceProcess);

        let result = manager.enforce_protection(&domain_id, AccessType::Read);
        assert!(result.is_ok());
    }

    #[test]
    fn test_protection_domain_manager_enforce_kernel_only_denied() {
        let config = MmuConfig::default_4k();
        let mut manager = ProtectionDomainManager::new(config);

        let domain_id = DomainId::new("kernel-domain");
        manager
            .domains
            .insert(domain_id.as_str().to_string(), ProtectionDomain::KernelOnly);

        let result = manager.enforce_protection(&domain_id, AccessType::Read);
        assert!(result.is_err());
    }

    #[test]
    fn test_protection_domain_manager_enforce_shared_read_only() {
        let config = MmuConfig::default_4k();
        let mut manager = ProtectionDomainManager::new(config);

        let domain_id = DomainId::new("shared-domain");
        manager.domains.insert(
            domain_id.as_str().to_string(),
            ProtectionDomain::SharedReadOnly {
                crew_id: "crew-001".to_string(),
            },
        );

        // Read should succeed
        assert!(manager.enforce_protection(&domain_id, AccessType::Read).is_ok());

        // Write should fail
        assert!(manager.enforce_protection(&domain_id, AccessType::Write).is_err());
    }
}

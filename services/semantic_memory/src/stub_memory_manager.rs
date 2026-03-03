// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Stub Memory Manager process skeleton with IPC handler loop.
//!
//! Implements the Memory Manager service process for Phase 0, including:
//! - Initialization of L1 memory based on model context window
//! - IPC request handler loop
//! - Process lifecycle management (Ready -> Serving -> Draining -> Terminated)
//! - Integration with L1 allocator, page pool, and context sizing
//!
//! See Engineering Plan § 4.1.0: Memory Manager Service.

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::{MemoryError, Result};
use crate::ipc_interface::{MemoryRequest, MemoryResponse, MemoryTierSpec};
use crate::process_lifecycle::{MemoryManagerProcess, MemoryManagerState};
use crate::ids::MemoryRegionID;
use crate::context_sizing::{L1SizingCalculator, ModelContextWindow};
use crate::l1_allocator::L1Allocator;
use crate::heap_allocator::HeapAllocator;

/// Stub Memory Manager configuration for Phase 0.
///
/// Contains model, hardware, and sizing parameters.
///
/// See Engineering Plan § 4.1.0: Configuration.
#[derive(Clone, Debug)]
pub struct MemoryManagerConfig {
    /// Model context window specification
    pub model_context: ModelContextWindow,
    /// Maximum L1 HBM available (e.g., 8GB)
    pub max_l1_hbm_bytes: u64,
    /// L1 base physical address
    pub l1_base_address: u64,
    /// Heap base address (for internal MM data structures)
    pub heap_base_address: u64,
    /// Heap size (internal allocations)
    pub heap_size: u64,
}

impl MemoryManagerConfig {
    /// Creates a default configuration (Claude-3, 8GB HBM).
    pub fn default_claude3_8gb() -> Self {
        MemoryManagerConfig {
            model_context: ModelContextWindow::claude_128k(),
            max_l1_hbm_bytes: 8 * 1024 * 1024 * 1024,
            l1_base_address: 0x0000_0100_0000_0000, // VM address space
            heap_base_address: 0x0000_0200_0000_0000,
            heap_size: 256 * 1024 * 1024, // 256MB for MM internals
        }
    }

    /// Creates a configuration for smaller models.
    pub fn compact_4gb() -> Self {
        MemoryManagerConfig {
            model_context: ModelContextWindow::context_32k(),
            max_l1_hbm_bytes: 4 * 1024 * 1024 * 1024,
            l1_base_address: 0x0000_0100_0000_0000,
            heap_base_address: 0x0000_0200_0000_0000,
            heap_size: 128 * 1024 * 1024,
        }
    }
}

/// Stub Memory Manager service for Phase 0.
///
/// Manages L1 memory allocation and IPC request handling.
///
/// See Engineering Plan § 4.1.0: Memory Manager Implementation.
pub struct StubMemoryManager {
    /// Process lifecycle state machine
    process: MemoryManagerProcess,
    /// Configuration
    config: MemoryManagerConfig,
    /// L1 allocator
    l1_allocator: Option<L1Allocator>,
    /// Internal heap for MM data structures
    heap: HeapAllocator,
    /// Request count (for metrics)
    request_count: u64,
    /// Error count (for health monitoring)
    error_count: u64,
}

impl StubMemoryManager {
    /// Creates a new Memory Manager service.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration parameters
    ///
    /// # Returns
    ///
    /// `Result<Self>` on success
    pub fn new(config: MemoryManagerConfig) -> Result<Self> {
        let process = MemoryManagerProcess::new();

        let heap = HeapAllocator::new(config.heap_base_address, config.heap_size);

        Ok(StubMemoryManager {
            process,
            config,
            l1_allocator: None,
            heap,
            request_count: 0,
            error_count: 0,
        })
    }

    /// Initializes the Memory Manager service.
    ///
    /// This should be called once after process creation.
    /// Sets up L1 memory allocation based on model context window.
    ///
    /// # Returns
    ///
    /// `Result<()>` on success
    ///
    /// See Engineering Plan § 4.1.0: Initialization.
    pub fn initialize(&mut self) -> Result<()> {
        // Initialize process lifecycle
        self.process.initialize()?;

        // Calculate L1 size based on context window
        let l1_calc = L1SizingCalculator::new(
            self.config.model_context.clone(),
            self.config.max_l1_hbm_bytes,
            0.10, // 10% overhead
        );

        let l1_size_bytes = l1_calc.calculate_l1_size()?;

        // Convert size to pages
        let l1_page_count = (l1_size_bytes + 4095) / 4096;

        // Create L1 allocator
        let l1_allocator = L1Allocator::new(
            MemoryRegionID::l1_gpu_local(),
            l1_page_count,
            self.config.l1_base_address,
        )?;

        self.l1_allocator = Some(l1_allocator);

        Ok(())
    }

    /// Handles a single IPC request from a Cognitive Thread.
    ///
    /// # Arguments
    ///
    /// * `request` - Memory request from CT
    ///
    /// # Returns
    ///
    /// `Result<MemoryResponse>` with the response
    ///
    /// See Engineering Plan § 4.1.1: Request Processing.
    pub fn handle_request(&mut self, request: MemoryRequest) -> Result<MemoryResponse> {
        self.request_count = self.request_count.saturating_add(1);

        // Check if we can accept requests
        if !self.process.state().accepts_requests() {
            self.error_count = self.error_count.saturating_add(1);
            return Err(MemoryError::Other(format!(
                "cannot serve request in {} state",
                self.process.state().name()
            )));
        }

        // Dispatch based on request type
        let response = match request {
            MemoryRequest::Allocate {
                tier,
                size,
                capability,
            } => self.handle_allocate(tier, size, &capability)?,

            MemoryRequest::Read {
                region_id,
                offset,
                len,
                capability,
            } => self.handle_read(&region_id, offset, len, &capability)?,

            MemoryRequest::Write {
                region_id,
                offset,
                data,
                capability,
            } => self.handle_write(&region_id, offset, &data, &capability)?,

            MemoryRequest::Mount {
                mount_point,
                capability,
            } => self.handle_mount(&mount_point, &capability)?,

            MemoryRequest::Query {
                region_id,
                capability,
            } => self.handle_query(&region_id, &capability)?,

            MemoryRequest::Evict {
                region_id,
                target_bytes,
                capability,
            } => self.handle_evict(&region_id, target_bytes, &capability)?,
        };

        Ok(response)
    }

    /// Allocates memory in the specified tier.
    fn handle_allocate(
        &mut self,
        tier: MemoryTierSpec,
        size: u64,
        _capability: &str,
    ) -> Result<MemoryResponse> {
        match tier {
            MemoryTierSpec::L1 => {
                if let Some(allocator) = &mut self.l1_allocator {
                    // Use a dummy CT ID (0 in Phase 0)
                    let (alloc_id, vaddr, paddr) = allocator.allocate(size, 0)?;

                    Ok(MemoryResponse::Allocated {
                        region_id: format!("l1-alloc-{}", alloc_id),
                        mapped_addr: vaddr,
                    })
                } else {
                    self.error_count = self.error_count.saturating_add(1);
                    Err(MemoryError::Other("L1 allocator not initialized".to_string()))
                }
            }
            MemoryTierSpec::L2 => {
                // Phase 0: L2 not implemented
                self.error_count = self.error_count.saturating_add(1);
                Err(MemoryError::InvalidTier {
                    operation: "allocate".to_string(),
                    tier: "L2".to_string(),
                })
            }
            MemoryTierSpec::L3 => {
                // Phase 0: L3 not implemented
                self.error_count = self.error_count.saturating_add(1);
                Err(MemoryError::InvalidTier {
                    operation: "allocate".to_string(),
                    tier: "L3".to_string(),
                })
            }
        }
    }

    /// Reads data from a memory region.
    fn handle_read(
        &mut self,
        _region_id: &str,
        _offset: u64,
        _len: u64,
        _capability: &str,
    ) -> Result<MemoryResponse> {
        // Phase 0: Read not implemented
        self.error_count = self.error_count.saturating_add(1);
        Err(MemoryError::Other("read not implemented in Phase 0".to_string()))
    }

    /// Writes data to a memory region.
    fn handle_write(
        &mut self,
        _region_id: &str,
        _offset: u64,
        _data: &[u8],
        _capability: &str,
    ) -> Result<MemoryResponse> {
        // Phase 0: Write not implemented
        self.error_count = self.error_count.saturating_add(1);
        Err(MemoryError::Other("write not implemented in Phase 0".to_string()))
    }

    /// Mounts a knowledge source or storage.
    fn handle_mount(
        &mut self,
        _mount_point: &str,
        _capability: &str,
    ) -> Result<MemoryResponse> {
        // Phase 0: Mount not implemented
        self.error_count = self.error_count.saturating_add(1);
        Err(MemoryError::Other("mount not implemented in Phase 0".to_string()))
    }

    /// Queries region statistics.
    fn handle_query(
        &mut self,
        region_id: &str,
        _capability: &str,
    ) -> Result<MemoryResponse> {
        if region_id == "l1-gpu-local" {
            if let Some(allocator) = &self.l1_allocator {
                let total = allocator.total_capacity_bytes();
                let used = allocator.total_allocated_bytes();
                let free = allocator.total_free_bytes();

                Ok(MemoryResponse::QueryResult {
                    stats: crate::ipc_interface::RegionStats::new(total, used, free, 100),
                })
            } else {
                self.error_count = self.error_count.saturating_add(1);
                Err(MemoryError::Other("L1 allocator not initialized".to_string()))
            }
        } else {
            // Phase 0: Only L1 query supported
            self.error_count = self.error_count.saturating_add(1);
            Err(MemoryError::Other(format!(
                "query not supported for region {}",
                region_id
            )))
        }
    }

    /// Evicts data from a region.
    fn handle_evict(
        &mut self,
        _region_id: &str,
        _target_bytes: u64,
        _capability: &str,
    ) -> Result<MemoryResponse> {
        // Phase 0: Eviction not implemented
        self.error_count = self.error_count.saturating_add(1);
        Err(MemoryError::Other("eviction not implemented in Phase 0".to_string()))
    }

    /// Initiates graceful shutdown.
    pub fn shutdown(&mut self) -> Result<()> {
        self.process.drain()?;
        self.process.shutdown()?;
        Ok(())
    }

    /// Returns health status.
    pub fn health_status(&self) -> crate::process_lifecycle::HealthStatus {
        self.process.health_status()
    }

    /// Returns the L1 allocator (if initialized).
    pub fn l1_allocator(&self) -> Option<&L1Allocator> {
        self.l1_allocator.as_ref()
    }

    /// Returns the L1 allocator mutably (if initialized).
    pub fn l1_allocator_mut(&mut self) -> Option<&mut L1Allocator> {
        self.l1_allocator.as_mut()
    }

    /// Returns request count.
    pub fn request_count(&self) -> u64 {
        self.request_count
    }

    /// Returns error count.
    pub fn error_count(&self) -> u64 {
        self.error_count
    }

    /// Returns internal heap allocator.
    pub fn heap(&self) -> &HeapAllocator {
        &self.heap
    }

    /// Returns internal heap allocator mutably.
    pub fn heap_mut(&mut self) -> &mut HeapAllocator {
        &mut self.heap
    }

    /// Returns the configuration.
    pub fn config(&self) -> &MemoryManagerConfig {
        &self.config
    }

    /// Returns the process state.
    pub fn state(&self) -> &MemoryManagerState {
        self.process.state()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::string::ToString;

    #[test]
    fn test_memory_manager_config_default() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        assert_eq!(config.max_l1_hbm_bytes, 8 * 1024 * 1024 * 1024);
        assert_eq!(config.model_context.max_tokens, 128 * 1024);
    }

    #[test]
    fn test_memory_manager_config_compact() {
        let config = MemoryManagerConfig::compact_4gb();
        assert_eq!(config.max_l1_hbm_bytes, 4 * 1024 * 1024 * 1024);
        assert_eq!(config.model_context.max_tokens, 32 * 1024);
    }

    #[test]
    fn test_stub_memory_manager_creation() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mm = StubMemoryManager::new(config).unwrap();

        assert_eq!(mm.request_count(), 0);
        assert_eq!(mm.error_count(), 0);
        assert!(mm.l1_allocator().is_none());
    }

    #[test]
    fn test_stub_memory_manager_initialize() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();

        let result = mm.initialize();
        assert!(result.is_ok());
        assert!(mm.l1_allocator().is_some());
    }

    #[test]
    fn test_stub_memory_manager_allocate_l1() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        let request = MemoryRequest::Allocate {
            tier: MemoryTierSpec::L1,
            size: 4096,
            capability: "cap-001".to_string(),
        };

        let response = mm.handle_request(request);
        assert!(response.is_ok());
        assert_eq!(mm.request_count(), 1);
    }

    #[test]
    fn test_stub_memory_manager_allocate_l2_fails() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        let request = MemoryRequest::Allocate {
            tier: MemoryTierSpec::L2,
            size: 4096,
            capability: "cap-001".to_string(),
        };

        let response = mm.handle_request(request);
        assert!(response.is_err());
        assert_eq!(mm.error_count(), 1);
    }

    #[test]
    fn test_stub_memory_manager_query_l1() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        let request = MemoryRequest::Query {
            region_id: "l1-gpu-local".to_string(),
            capability: "cap-001".to_string(),
        };

        let response = mm.handle_request(request);
        assert!(response.is_ok());
    }

    #[test]
    fn test_stub_memory_manager_not_initialized() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();

        let request = MemoryRequest::Allocate {
            tier: MemoryTierSpec::L1,
            size: 4096,
            capability: "cap-001".to_string(),
        };

        // Should fail because not initialized
        let response = mm.handle_request(request);
        assert!(response.is_err());
    }

    #[test]
    fn test_stub_memory_manager_multiple_allocations() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        for i in 0..10 {
            let request = MemoryRequest::Allocate {
                tier: MemoryTierSpec::L1,
                size: (i + 1) * 4096,
                capability: "cap-001".to_string(),
            };

            let response = mm.handle_request(request);
            assert!(response.is_ok());
        }

        assert_eq!(mm.request_count(), 10);
        assert_eq!(mm.error_count(), 0);
    }

    #[test]
    fn test_stub_memory_manager_health_status() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        let status = mm.health_status();
        assert_eq!(status.state, MemoryManagerState::Ready);
        assert!(status.is_healthy());
    }

    #[test]
    fn test_stub_memory_manager_shutdown() {
        let config = MemoryManagerConfig::default_claude3_8gb();
        let mut mm = StubMemoryManager::new(config).unwrap();
        mm.initialize().unwrap();

        let result = mm.shutdown();
        assert!(result.is_ok());
        assert_eq!(mm.state(), &MemoryManagerState::Terminated);
    }
}

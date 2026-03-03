// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! VRAM Manager — allocation and lifecycle for single-model scenarios.
//!
//! Implements VRAM management for Phase 0 (Week 4) where a single model
//! resides in VRAM at a time. Tracks allocations via cuMemAlloc (CUDA) or
//! hipMalloc (ROCm), with bound model lifecycle management.
//!
//! Reference: Engineering Plan § VRAM Management, Single-Model Phase
//!
//! Single-model VRAM lifecycle:
//! 1. initialize() reserves a VRAM partition (e.g., 16 GB)
//! 2. allocate() assigns space for a model's weights
//! 3. free() releases space when model is unloaded
//! 4. shutdown() releases the entire partition

use crate::error::GpuError;
use alloc::vec::Vec;
use core::fmt;

/// VRAM allocation record — tracks a single VRAM allocation.
///
/// Represents a contiguous block of VRAM allocated for a model's weights
/// and intermediate buffers. Used for lifetime management and coherency checks.
///
/// Reference: Engineering Plan § VRAM Allocation Tracking
#[derive(Clone, Debug)]
pub struct VramAllocation {
    /// GPU device pointer (opaque from driver).
    /// In CUDA, this is a CUdeviceptr; in ROCm, a hipDeviceptr_t.
    pub device_ptr: u64,

    /// Allocation size in bytes.
    pub size_bytes: u64,

    /// Model ID this allocation belongs to.
    pub model_id: [u8; 32],

    /// Allocation type (model weights, inference buffers, etc.).
    pub allocation_type: VramAllocationType,

    /// Is this allocation currently in use?
    pub is_active: bool,
}

impl VramAllocation {
    /// Create a new VRAM allocation.
    pub fn new(
        device_ptr: u64,
        size_bytes: u64,
        model_id: [u8; 32],
        allocation_type: VramAllocationType,
    ) -> Self {
        VramAllocation {
            device_ptr,
            size_bytes,
            model_id,
            allocation_type,
            is_active: true,
        }
    }
}

impl fmt::Display for VramAllocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VramAllocation(ptr=0x{:x}, size={}B, type={:?}, active={})",
            self.device_ptr, self.size_bytes, self.allocation_type, self.is_active
        )
    }
}

/// Types of VRAM allocations tracked by the VRAM Manager.
///
/// Reference: Engineering Plan § VRAM Allocation Types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VramAllocationType {
    /// Model weight tensors (loaded once, read-only).
    ModelWeights,

    /// Inference buffers (activations, KV-cache, attention, MLPs).
    InferenceBuffers,

    /// Scratch/temporary buffers (freed after kernel execution).
    ScratchBuffer,
}

impl fmt::Display for VramAllocationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VramAllocationType::ModelWeights => write!(f, "ModelWeights"),
            VramAllocationType::InferenceBuffers => write!(f, "InferenceBuffers"),
            VramAllocationType::ScratchBuffer => write!(f, "ScratchBuffer"),
        }
    }
}

/// VRAM Manager — single-model VRAM lifecycle management.
///
/// Manages a fixed partition of GPU VRAM reserved for single-model inference.
/// For Phase 0, one model can be resident at a time.
///
/// Reference: Engineering Plan § VRAM Manager, Phase 0
#[derive(Clone, Debug)]
pub struct VramManager {
    /// Allocated VRAM partitions per device (indexed by device ordinal).
    /// Key: device ordinal, Value: (total_bytes, used_bytes, allocations)
    device_partitions: Vec<(u64, u64, Vec<VramAllocation>)>,

    /// GPU device ordinal this manager is bound to (set during initialize).
    device_ordinal: Option<u32>,

    /// Total VRAM partition size reserved for models.
    partition_size_bytes: u64,

    /// Free VRAM remaining in the partition.
    free_vram_bytes: u64,

    /// Is the manager initialized?
    initialized: bool,
}

impl VramManager {
    /// Create a new VRAM Manager.
    ///
    /// The manager starts uninitialized. Call `initialize()` to set up
    /// the VRAM partition.
    pub fn new() -> Self {
        VramManager {
            device_partitions: Vec::new(),
            device_ordinal: None,
            partition_size_bytes: 0,
            free_vram_bytes: 0,
            initialized: false,
        }
    }

    /// Initialize the VRAM Manager with a reserved partition.
    ///
    /// # Arguments
    ///
    /// * `device_ordinal` - GPU device ordinal (0-based index)
    /// * `partition_size_bytes` - VRAM size to reserve for models
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(GpuError)` on failure.
    ///
    /// Reference: Engineering Plan § VRAM Partition Initialization
    pub fn initialize(
        &mut self,
        device_ordinal: u32,
        partition_size_bytes: u64,
    ) -> Result<(), GpuError> {
        if self.initialized {
            return Err(GpuError::DriverError); // Already initialized
        }

        // Phase A: In a real implementation, this would call
        // cuMemAlloc (CUDA) or hipMalloc (ROCm) to allocate the partition.
        // For now, we simulate the allocation.

        self.device_ordinal = Some(device_ordinal);
        self.partition_size_bytes = partition_size_bytes;
        self.free_vram_bytes = partition_size_bytes;

        // Ensure we have space in device_partitions for this device
        while self.device_partitions.len() <= device_ordinal as usize {
            self.device_partitions
                .push((partition_size_bytes, 0, Vec::new()));
        }

        self.device_partitions[device_ordinal as usize] =
            (partition_size_bytes, 0, Vec::new());

        self.initialized = true;
        Ok(())
    }

    /// Allocate VRAM for a model.
    ///
    /// # Arguments
    ///
    /// * `model_id` - 32-byte model identifier
    /// * `size_bytes` - Bytes to allocate
    /// * `allocation_type` - Type of allocation
    ///
    /// # Returns
    ///
    /// A VramAllocation on success, or GpuError if insufficient free VRAM.
    ///
    /// Reference: Engineering Plan § Model Loading Path
    pub fn allocate(
        &mut self,
        model_id: [u8; 32],
        size_bytes: u64,
        allocation_type: VramAllocationType,
    ) -> Result<VramAllocation, GpuError> {
        if !self.initialized {
            return Err(GpuError::DriverError);
        }

        if size_bytes > self.free_vram_bytes {
            return Err(GpuError::VramExhausted);
        }

        let device_ordinal = self.device_ordinal.ok_or(GpuError::DriverError)?;

        // Phase A: Call cuMemAlloc (CUDA) or hipMalloc (ROCm)
        // For simulation, generate a mock device pointer
        let mock_device_ptr = 0x100000000u64 + (self.partition_size_bytes - self.free_vram_bytes);

        let allocation = VramAllocation::new(mock_device_ptr, size_bytes, model_id, allocation_type);

        // Track the allocation
        self.free_vram_bytes -= size_bytes;

        let partition = &mut self.device_partitions[device_ordinal as usize];
        partition.1 += size_bytes; // Update used_bytes
        partition.2.push(allocation.clone());

        Ok(allocation)
    }

    /// Free VRAM allocated to a model.
    ///
    /// # Arguments
    ///
    /// * `model_id` - Model identifier to free
    ///
    /// # Returns
    ///
    /// Total bytes freed, or GpuError if model not found.
    ///
    /// Reference: Engineering Plan § Model Unloading Path
    pub fn free(&mut self, model_id: &[u8; 32]) -> Result<u64, GpuError> {
        if !self.initialized {
            return Err(GpuError::DriverError);
        }

        let device_ordinal = self.device_ordinal.ok_or(GpuError::DriverError)?;
        let partition = &mut self.device_partitions[device_ordinal as usize];

        // Find all allocations for this model
        let mut total_freed = 0u64;
        partition.2.retain(|alloc| {
            if alloc.model_id == *model_id {
                total_freed += alloc.size_bytes;
                // Phase A: Call cuMemFree (CUDA) or hipFree (ROCm)
                false // Remove from vector
            } else {
                true
            }
        });

        // Update partition tracking
        partition.1 = partition.1.saturating_sub(total_freed);
        self.free_vram_bytes += total_freed;

        if total_freed == 0 {
            return Err(GpuError::AllocationFailed);
        }

        Ok(total_freed)
    }

    /// Verify GPU memory coherency after model load/unload.
    ///
    /// Phase A: Placeholder for future coherency checks.
    /// In real implementation, this would verify memory consistency
    /// after GPU-side operations.
    ///
    /// Reference: Engineering Plan § Memory Coherency
    pub fn verify_coherency(&self) -> Result<(), GpuError> {
        if !self.initialized {
            return Err(GpuError::DriverError);
        }

        // Phase A: Simple check that free + used = total
        let device_ordinal = self.device_ordinal.ok_or(GpuError::DriverError)?;
        let partition = &self.device_partitions[device_ordinal as usize];

        let used_sum: u64 = partition.2.iter().map(|a| a.size_bytes).sum();
        if used_sum != partition.1 {
            return Err(GpuError::DriverError);
        }

        Ok(())
    }

    /// Get free VRAM available in the partition.
    pub fn free_vram_bytes(&self) -> u64 {
        self.free_vram_bytes
    }

    /// Get used VRAM in the partition.
    pub fn used_vram_bytes(&self) -> u64 {
        self.partition_size_bytes - self.free_vram_bytes
    }

    /// Get total VRAM partition size.
    pub fn partition_size_bytes(&self) -> u64 {
        self.partition_size_bytes
    }

    /// Get the number of active allocations.
    pub fn allocation_count(&self) -> usize {
        if let Some(device_ordinal) = self.device_ordinal {
            if (device_ordinal as usize) < self.device_partitions.len() {
                return self.device_partitions[device_ordinal as usize]
                    .2
                    .len();
            }
        }
        0
    }

    /// Shutdown the VRAM Manager and release the partition.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// Reference: Engineering Plan § GPU Manager Shutdown
    pub fn shutdown(&mut self) -> Result<(), GpuError> {
        // Phase A: Call cuMemFree (CUDA) or hipFree (ROCm) on the partition
        self.device_partitions.clear();
        self.initialized = false;
        Ok(())
    }

    /// Check if manager is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl Default for VramManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vram_manager_creation() {
        let manager = VramManager::new();
        assert!(!manager.is_initialized());
    }

    #[test]
    fn test_vram_manager_initialize() {
        let mut manager = VramManager::new();
        let result = manager.initialize(0, 16 * 1024 * 1024 * 1024);
        assert!(result.is_ok());
        assert!(manager.is_initialized());
    }

    #[test]
    fn test_vram_manager_allocate() {
        let mut manager = VramManager::new();
        let _ = manager.initialize(0, 16 * 1024 * 1024 * 1024);

        let model_id = [1u8; 32];
        let alloc = manager.allocate(model_id, 1024 * 1024, VramAllocationType::ModelWeights);
        assert!(alloc.is_ok());
        let alloc_obj = alloc.unwrap();
        assert_eq!(alloc_obj.model_id, model_id);
        assert_eq!(alloc_obj.size_bytes, 1024 * 1024);
    }

    #[test]
    fn test_vram_manager_free() {
        let mut manager = VramManager::new();
        let _ = manager.initialize(0, 16 * 1024 * 1024 * 1024);

        let model_id = [1u8; 32];
        let _ = manager.allocate(model_id, 1024 * 1024, VramAllocationType::ModelWeights);

        let freed = manager.free(&model_id);
        assert!(freed.is_ok());
        assert_eq!(freed.unwrap(), 1024 * 1024);
    }

    #[test]
    fn test_vram_manager_exhausted() {
        let mut manager = VramManager::new();
        let partition_size = 1024 * 1024; // 1 MB
        let _ = manager.initialize(0, partition_size);

        let model_id = [1u8; 32];
        let alloc = manager.allocate(model_id, partition_size, VramAllocationType::ModelWeights);
        assert!(alloc.is_ok());

        // Try to allocate more than available
        let model_id2 = [2u8; 32];
        let alloc2 = manager.allocate(model_id2, 1024, VramAllocationType::ModelWeights);
        assert!(alloc2.is_err());
    }

    #[test]
    fn test_vram_manager_free_vram_tracking() {
        let mut manager = VramManager::new();
        let partition_size = 10 * 1024 * 1024; // 10 MB
        let _ = manager.initialize(0, partition_size);

        let initial_free = manager.free_vram_bytes();
        assert_eq!(initial_free, partition_size);

        let model_id = [1u8; 32];
        let _ = manager.allocate(model_id, 1024 * 1024, VramAllocationType::ModelWeights);

        let after_alloc_free = manager.free_vram_bytes();
        assert_eq!(after_alloc_free, partition_size - 1024 * 1024);

        let _ = manager.free(&model_id);

        let after_free_free = manager.free_vram_bytes();
        assert_eq!(after_free_free, partition_size);
    }

    #[test]
    fn test_vram_manager_used_vram_tracking() {
        let mut manager = VramManager::new();
        let _ = manager.initialize(0, 10 * 1024 * 1024);

        assert_eq!(manager.used_vram_bytes(), 0);

        let model_id = [1u8; 32];
        let _ = manager.allocate(model_id, 1024 * 1024, VramAllocationType::ModelWeights);
        assert_eq!(manager.used_vram_bytes(), 1024 * 1024);

        let _ = manager.free(&model_id);
        assert_eq!(manager.used_vram_bytes(), 0);
    }

    #[test]
    fn test_vram_allocation_creation() {
        let model_id = [1u8; 32];
        let alloc = VramAllocation::new(
            0x100000000,
            1024 * 1024,
            model_id,
            VramAllocationType::ModelWeights,
        );
        assert_eq!(alloc.model_id, model_id);
        assert_eq!(alloc.device_ptr, 0x100000000);
        assert!(alloc.is_active);
    }

    #[test]
    fn test_vram_manager_verify_coherency() {
        let mut manager = VramManager::new();
        let _ = manager.initialize(0, 10 * 1024 * 1024);

        let result = manager.verify_coherency();
        assert!(result.is_ok());

        let model_id = [1u8; 32];
        let _ = manager.allocate(model_id, 1024 * 1024, VramAllocationType::ModelWeights);
        let result = manager.verify_coherency();
        assert!(result.is_ok());
    }
}

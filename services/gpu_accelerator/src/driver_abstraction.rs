// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU driver abstraction layer.
//!
//! Defines the DriverInterface trait (CUDA Driver API / ROCm HIP interface)
//! and provides stub implementations (CudaDriverAdapter, RocmHipAdapter).
//!
//! This layer enables Phase A (v1.0) flexibility to use existing, proven
//! driver APIs. Phase B (v2.0, post-GA) may add native direct-MMIO drivers.
//!
//! Reference: Engineering Plan § Driver Abstraction, Phase A Strategy

use crate::checkpoint::GpuCheckpoint;
use crate::error::GpuError;
use crate::ids::{GpuDeviceID, KernelLaunchID};
use crate::kernel_launch::KernelLaunch;
use alloc::vec::Vec;
use core::fmt;

/// Driver API abstraction (from device.rs, re-exported).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DriverApi {
    /// CUDA Driver API
    CudaDriverApi,
    /// ROCm HIP
    RocmHip,
}

/// Low-level driver interface for GPU communication.
///
/// Abstracts the underlying CUDA Driver / ROCm HIP implementation.
/// All GPU operations (memory management, kernel launch, sync) go through
/// this trait, enabling driver portability.
///
/// Reference: Engineering Plan § Driver Abstraction
pub trait DriverInterface: core::fmt::Debug {
    /// Initialize driver and enumerate devices.
    ///
    /// Called at system startup to discover GPU hardware.
    ///
    /// # Returns
    ///
    /// Vector of device IDs if successful, error otherwise.
    fn initialize(&self) -> Result<Vec<GpuDeviceID>, GpuError>;

    /// Allocate VRAM on a device.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Target GPU device
    /// * `size_bytes` - Allocation size in bytes
    ///
    /// # Returns
    ///
    /// GPU virtual address if successful (as u64 offset), error otherwise.
    fn allocate_memory(&self, device_id: GpuDeviceID, size_bytes: u64) -> Result<u64, GpuError>;

    /// Free VRAM on a device.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Target GPU device
    /// * `gpu_ptr` - GPU virtual address (from allocate_memory)
    fn free_memory(&self, device_id: GpuDeviceID, gpu_ptr: u64) -> Result<(), GpuError>;

    /// Launch a kernel on a device.
    ///
    /// Enqueues the kernel for execution on the specified device and stream.
    /// Returns immediately (asynchronous launch).
    ///
    /// # Arguments
    ///
    /// * `device_id` - Target GPU device
    /// * `kernel_launch` - Kernel configuration and parameters
    fn launch_kernel(&self, device_id: GpuDeviceID, kernel_launch: &KernelLaunch) -> Result<KernelLaunchID, GpuError>;

    /// Synchronize with GPU execution.
    ///
    /// Blocks until all previously launched kernels complete.
    /// Used for latency-critical operations.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Target GPU device
    /// * `stream` - Stream number (0 = default / all streams)
    fn synchronize(&self, device_id: GpuDeviceID, stream: u32) -> Result<(), GpuError>;

    /// Create a checkpoint of GPU state.
    ///
    /// Captures GPU state (VRAM, registers, kernel state) for the crew.
    /// May be expensive (synchronous operation).
    ///
    /// # Arguments
    ///
    /// * `device_id` - Source GPU device
    /// * `crew_id` - Crew to checkpoint
    fn checkpoint(&self, device_id: GpuDeviceID, crew_id: [u8; 16]) -> Result<GpuCheckpoint, GpuError>;

    /// Restore GPU state from a checkpoint.
    ///
    /// Restores previously saved GPU state to a (possibly different) device.
    /// Validates compatibility and atomicity.
    ///
    /// # Arguments
    ///
    /// * `device_id` - Target GPU device
    /// * `checkpoint` - Checkpoint to restore
    fn restore(&self, device_id: GpuDeviceID, checkpoint: &GpuCheckpoint) -> Result<(), GpuError>;
}

/// CUDA Driver API implementation stub.
///
/// Represents a concrete driver using NVIDIA's CUDA Driver API (libcuda).
/// In a real implementation, this would contain FFI bindings and driver state.
///
/// Reference: Engineering Plan § CUDA Driver Integration
#[derive(Debug, Clone, Copy)]
pub struct CudaDriverAdapter;

impl CudaDriverAdapter {
    /// Create a new CUDA driver adapter.
    pub fn new() -> Self {
        CudaDriverAdapter
    }
}

impl Default for CudaDriverAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl DriverInterface for CudaDriverAdapter {
    fn initialize(&self) -> Result<Vec<GpuDeviceID>, GpuError> {
        // STUB: In a real implementation, would call cuInit() and cuDeviceGetCount()
        // For now, returns empty device list
        Ok(Vec::new())
    }

    fn allocate_memory(&self, _device_id: GpuDeviceID, _size_bytes: u64) -> Result<u64, GpuError> {
        // STUB: Would call cuMemAlloc()
        Err(GpuError::AllocationFailed)
    }

    fn free_memory(&self, _device_id: GpuDeviceID, _gpu_ptr: u64) -> Result<(), GpuError> {
        // STUB: Would call cuMemFree()
        Ok(())
    }

    fn launch_kernel(
        &self,
        _device_id: GpuDeviceID,
        _kernel_launch: &KernelLaunch,
    ) -> Result<KernelLaunchID, GpuError> {
        // STUB: Would call cuLaunchKernel()
        Err(GpuError::KernelLaunchFailed)
    }

    fn synchronize(&self, _device_id: GpuDeviceID, _stream: u32) -> Result<(), GpuError> {
        // STUB: Would call cuStreamSynchronize() or cuCtxSynchronize()
        Ok(())
    }

    fn checkpoint(&self, _device_id: GpuDeviceID, _crew_id: [u8; 16]) -> Result<GpuCheckpoint, GpuError> {
        // STUB: Would capture GPU state via CUDA managed memory or PCIe DMAs
        Err(GpuError::CheckpointFailed)
    }

    fn restore(&self, _device_id: GpuDeviceID, _checkpoint: &GpuCheckpoint) -> Result<(), GpuError> {
        // STUB: Would restore GPU state
        Err(GpuError::RestoreFailed)
    }
}

/// ROCm HIP driver implementation stub.
///
/// Represents a concrete driver using AMD's ROCm HIP API.
/// In a real implementation, this would contain FFI bindings to HIP runtime.
///
/// Reference: Engineering Plan § ROCm HIP Integration
#[derive(Debug, Clone, Copy)]
pub struct RocmHipAdapter;

impl RocmHipAdapter {
    /// Create a new ROCm HIP adapter.
    pub fn new() -> Self {
        RocmHipAdapter
    }
}

impl Default for RocmHipAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl DriverInterface for RocmHipAdapter {
    fn initialize(&self) -> Result<Vec<GpuDeviceID>, GpuError> {
        // STUB: In a real implementation, would call hipInit() and hipGetDeviceCount()
        Ok(Vec::new())
    }

    fn allocate_memory(&self, _device_id: GpuDeviceID, _size_bytes: u64) -> Result<u64, GpuError> {
        // STUB: Would call hipMalloc()
        Err(GpuError::AllocationFailed)
    }

    fn free_memory(&self, _device_id: GpuDeviceID, _gpu_ptr: u64) -> Result<(), GpuError> {
        // STUB: Would call hipFree()
        Ok(())
    }

    fn launch_kernel(
        &self,
        _device_id: GpuDeviceID,
        _kernel_launch: &KernelLaunch,
    ) -> Result<KernelLaunchID, GpuError> {
        // STUB: Would call hipModuleLaunchKernel() or hipLaunchKernel()
        Err(GpuError::KernelLaunchFailed)
    }

    fn synchronize(&self, _device_id: GpuDeviceID, _stream: u32) -> Result<(), GpuError> {
        // STUB: Would call hipStreamSynchronize() or hipDeviceSynchronize()
        Ok(())
    }

    fn checkpoint(&self, _device_id: GpuDeviceID, _crew_id: [u8; 16]) -> Result<GpuCheckpoint, GpuError> {
        // STUB: Would capture GPU state via HIP managed memory
        Err(GpuError::CheckpointFailed)
    }

    fn restore(&self, _device_id: GpuDeviceID, _checkpoint: &GpuCheckpoint) -> Result<(), GpuError> {
        // STUB: Would restore GPU state
        Err(GpuError::RestoreFailed)
    }
}

impl fmt::Display for DriverApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverApi::CudaDriverApi => write!(f, "CUDA Driver API"),
            DriverApi::RocmHip => write!(f, "ROCm HIP"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_cuda_driver_adapter_creation() {
        let _adapter = CudaDriverAdapter::new();
    }

    #[test]
    fn test_rocm_hip_adapter_creation() {
        let _adapter = RocmHipAdapter::new();
    }

    #[test]
    fn test_cuda_driver_adapter_initialize() {
        let adapter = CudaDriverAdapter::new();
        let result = adapter.initialize();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0); // Stub returns empty
    }

    #[test]
    fn test_rocm_hip_adapter_initialize() {
        let adapter = RocmHipAdapter::new();
        let result = adapter.initialize();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0); // Stub returns empty
    }

    #[test]
    fn test_cuda_driver_adapter_allocate_memory() {
        let adapter = CudaDriverAdapter::new();
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);

        let result = adapter.allocate_memory(device_id, 1000);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), GpuError::AllocationFailed);
    }

    #[test]
    fn test_cuda_driver_adapter_synchronize() {
        let adapter = CudaDriverAdapter::new();
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);

        let result = adapter.synchronize(device_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rocm_hip_adapter_synchronize() {
        let adapter = RocmHipAdapter::new();
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);

        let result = adapter.synchronize(device_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_driver_api_display() {
        assert_eq!(format!("{}", DriverApi::CudaDriverApi), "CUDA Driver API");
        assert_eq!(format!("{}", DriverApi::RocmHip), "ROCm HIP");
    }

    #[test]
    fn test_cuda_free_memory() {
        let adapter = CudaDriverAdapter::new();
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);

        let result = adapter.free_memory(device_id, 12345);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rocm_free_memory() {
        let adapter = RocmHipAdapter::new();
        let device_id = GpuDeviceID::from_bytes([0u8; 16]);

        let result = adapter.free_memory(device_id, 12345);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cuda_adapter_default() {
        let _adapter = CudaDriverAdapter::default();
    }

    #[test]
    fn test_rocm_adapter_default() {
        let _adapter = RocmHipAdapter::default();
    }
}

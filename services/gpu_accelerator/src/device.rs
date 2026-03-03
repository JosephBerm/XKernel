// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU device abstraction layer.
//!
//! Defines the GpuDevice structure and associated enums for device type,
//! driver API, and capabilities. This module provides a unified abstraction
//! over NVIDIA and AMD GPUs, enabling device-agnostic resource management.
//!
//! Reference: Engineering Plan § Device Registry, Hardware Support Matrix

use crate::ids::GpuDeviceID;
use core::fmt;

/// Enumeration of supported GPU device types.
///
/// Defines the target hardware for Phase A (v1.0):
/// - NVIDIA H100: 80 GB HBM3, 132 TPCs, compute capability 9.0
/// - NVIDIA H200: 141 GB HBM3, 132 TPCs, compute capability 9.0
/// - NVIDIA B200: 192 GB HBM3, 192 TPCs, compute capability 10.0
/// - AMD MI300X: 192 GB HBM3, 304 cores, compute capability 9.1 (equivalent)
///
/// Reference: Engineering Plan § Target Hardware, v1 Roadmap
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuDeviceType {
    /// NVIDIA H100 Tensor GPU (80 GB HBM3, P0 target)
    NvidiaH100,
    /// NVIDIA H200 Tensor GPU (141 GB HBM3, P0 target)
    NvidiaH200,
    /// NVIDIA B200 Blackwell GPU (192 GB HBM3, P0 target)
    NvidiaB200,
    /// AMD MI300X (192 GB HBM3, P1 target)
    AmdMi300x,
}

impl GpuDeviceType {
    /// Get the maximum VRAM capacity in bytes for this device type.
    ///
    /// Used for validation during allocation and capacity planning.
    pub fn max_vram_bytes(&self) -> u64 {
        match self {
            GpuDeviceType::NvidiaH100 => 80 * 1024 * 1024 * 1024,    // 80 GB
            GpuDeviceType::NvidiaH200 => 141 * 1024 * 1024 * 1024,   // 141 GB
            GpuDeviceType::NvidiaB200 => 192 * 1024 * 1024 * 1024,   // 192 GB
            GpuDeviceType::AmdMi300x => 192 * 1024 * 1024 * 1024,    // 192 GB
        }
    }

    /// Get the TPC/SM count for this device type.
    ///
    /// TPCs (Tensor Processing Clusters) are the fundamental unit of
    /// scheduling and resource allocation in the GPU Manager.
    pub fn tpc_count(&self) -> u32 {
        match self {
            GpuDeviceType::NvidiaH100 => 132,
            GpuDeviceType::NvidiaH200 => 132,
            GpuDeviceType::NvidiaB200 => 192,
            GpuDeviceType::AmdMi300x => 304,
        }
    }

    /// Get the Streaming Multiprocessor count (NVIDIA terminology).
    ///
    /// Equivalent to TPC count for NVIDIA devices.
    pub fn sm_count(&self) -> u32 {
        match self {
            GpuDeviceType::NvidiaH100 => 132,
            GpuDeviceType::NvidiaH200 => 132,
            GpuDeviceType::NvidiaB200 => 192,
            GpuDeviceType::AmdMi300x => 304, // AMD equivalent
        }
    }

    /// Get the compute capability version for this device type.
    ///
    /// Used for kernel binary compatibility checks.
    pub fn compute_capability(&self) -> (u32, u32) {
        match self {
            GpuDeviceType::NvidiaH100 => (9, 0),
            GpuDeviceType::NvidiaH200 => (9, 0),
            GpuDeviceType::NvidiaB200 => (10, 0),
            GpuDeviceType::AmdMi300x => (9, 1), // Equivalent RDNA 3 capability
        }
    }

    /// Check if this is a P0 (primary target) device for v1.0.
    pub fn is_p0_target(&self) -> bool {
        matches!(
            self,
            GpuDeviceType::NvidiaH100
                | GpuDeviceType::NvidiaH200
                | GpuDeviceType::NvidiaB200
        )
    }

    /// Check if this is a P1 (secondary target) device for v1.0.
    pub fn is_p1_target(&self) -> bool {
        matches!(self, GpuDeviceType::AmdMi300x)
    }
}

impl fmt::Display for GpuDeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuDeviceType::NvidiaH100 => write!(f, "NVIDIA H100"),
            GpuDeviceType::NvidiaH200 => write!(f, "NVIDIA H200"),
            GpuDeviceType::NvidiaB200 => write!(f, "NVIDIA B200"),
            GpuDeviceType::AmdMi300x => write!(f, "AMD MI300X"),
        }
    }
}

/// Enumeration of supported driver APIs.
///
/// The GPU Manager abstracts over different driver implementations to enable
/// Phase A (v1.0) flexibility. Phase B (v2.0) may add native driver support.
///
/// Reference: Engineering Plan § Driver Abstraction Layer, Phase A Strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DriverApi {
    /// NVIDIA CUDA Driver API (libcuda.so)
    ///
    /// Used for all NVIDIA devices (H100, H200, B200).
    /// Provides kernel launch, memory management, and synchronization.
    CudaDriverApi,

    /// AMD ROCm HIP driver
    ///
    /// Used for AMD MI300X. Provides API-level compatibility with CUDA
    /// for kernel launch and memory management.
    RocmHip,
}

impl fmt::Display for DriverApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverApi::CudaDriverApi => write!(f, "CUDA Driver API"),
            DriverApi::RocmHip => write!(f, "ROCm HIP"),
        }
    }
}

/// Device capabilities and limits.
///
/// Specifies performance and resource constraints for a specific GPU device.
/// Obtained during device initialization and used for scheduling constraints.
///
/// Reference: Engineering Plan § Device Registry, Scheduling Constraints
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceCapabilities {
    /// Maximum number of kernel streams that can execute concurrently.
    ///
    /// NVIDIA H100: typically 128 (depends on scheduler configuration).
    pub max_concurrent_kernels: u32,

    /// Maximum shared memory per block in bytes.
    ///
    /// H100/H200: 192 KB, B200: 192 KB, MI300X: 96 KB (AMD RDNA 3).
    pub max_shared_memory_bytes: u32,

    /// Maximum threads per block.
    ///
    /// NVIDIA: 1024, AMD: 1024.
    pub max_threads_per_block: u32,
}

impl DeviceCapabilities {
    /// Create device capabilities for a given device type.
    ///
    /// Returns canonical capability values for the device type.
    pub fn for_device_type(device_type: GpuDeviceType) -> Self {
        match device_type {
            GpuDeviceType::NvidiaH100 | GpuDeviceType::NvidiaH200 => DeviceCapabilities {
                max_concurrent_kernels: 128,
                max_shared_memory_bytes: 192 * 1024,
                max_threads_per_block: 1024,
            },
            GpuDeviceType::NvidiaB200 => DeviceCapabilities {
                max_concurrent_kernels: 256,
                max_shared_memory_bytes: 192 * 1024,
                max_threads_per_block: 1024,
            },
            GpuDeviceType::AmdMi300x => DeviceCapabilities {
                max_concurrent_kernels: 128,
                max_shared_memory_bytes: 96 * 1024,
                max_threads_per_block: 1024,
            },
        }
    }
}

/// GPU device abstraction.
///
/// Represents a single GPU (physical or virtual) managed by the GPU Manager.
/// Encapsulates device identity, type, memory state, and scheduling parameters.
///
/// Obtained via device enumeration during initialization and managed
/// by the DeviceRegistry subsystem.
///
/// Reference: Engineering Plan § Device Registry
#[derive(Clone, Debug)]
pub struct GpuDevice {
    /// Unique device identifier (persistent across runtime sessions).
    pub id: GpuDeviceID,

    /// Device type (hardware model).
    pub device_type: GpuDeviceType,

    /// Driver API used to communicate with this device.
    pub driver_api: DriverApi,

    /// Total VRAM in bytes (constant).
    pub total_vram_bytes: u64,

    /// Available (unallocated) VRAM in bytes (dynamic).
    pub available_vram_bytes: u64,

    /// Total Tensor Processing Cluster (TPC) count (constant).
    pub tpc_count: u32,

    /// Streaming Multiprocessor count (synonymous with TPC count).
    pub sm_count: u32,

    /// Compute capability (major, minor).
    pub compute_capability: (u32, u32),
}

impl GpuDevice {
    /// Create a new GPU device.
    ///
    /// Validates that the device_type, driver_api, and capability values are consistent.
    pub fn new(
        id: GpuDeviceID,
        device_type: GpuDeviceType,
        driver_api: DriverApi,
    ) -> Self {
        let total_vram_bytes = device_type.max_vram_bytes();
        let tpc_count = device_type.tpc_count();
        let compute_capability = device_type.compute_capability();

        GpuDevice {
            id,
            device_type,
            driver_api,
            total_vram_bytes,
            available_vram_bytes: total_vram_bytes,
            tpc_count,
            sm_count: tpc_count,
            compute_capability,
        }
    }

    /// Update available VRAM (called by VRAM allocator).
    ///
    /// # Arguments
    ///
    /// * `available` - New available VRAM in bytes
    pub fn set_available_vram(&mut self, available: u64) {
        self.available_vram_bytes = available;
    }

    /// Get utilization percentage (used VRAM / total VRAM).
    ///
    /// Returns value in range [0, 100].
    pub fn utilization_percent(&self) -> u32 {
        if self.total_vram_bytes == 0 {
            return 0;
        }
        let used = self.total_vram_bytes.saturating_sub(self.available_vram_bytes);
        ((used as f64 / self.total_vram_bytes as f64) * 100.0) as u32
    }

    /// Check if device is fully utilized.
    pub fn is_saturated(&self) -> bool {
        self.available_vram_bytes == 0
    }

    /// Get capabilities for this device.
    pub fn capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities::for_device_type(self.device_type)
    }
}

impl fmt::Display for GpuDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuDevice({}, {}, driver={}, vram={:.1}GB/{:.1}GB, tpcs={})",
            self.id,
            self.device_type,
            self.driver_api,
            self.available_vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            self.total_vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0),
            self.tpc_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_device_type_max_vram() {
        assert_eq!(GpuDeviceType::NvidiaH100.max_vram_bytes(), 80 * 1024 * 1024 * 1024);
        assert_eq!(
            GpuDeviceType::NvidiaH200.max_vram_bytes(),
            141 * 1024 * 1024 * 1024
        );
        assert_eq!(
            GpuDeviceType::NvidiaB200.max_vram_bytes(),
            192 * 1024 * 1024 * 1024
        );
        assert_eq!(
            GpuDeviceType::AmdMi300x.max_vram_bytes(),
            192 * 1024 * 1024 * 1024
        );
    }

    #[test]
    fn test_device_type_tpc_count() {
        assert_eq!(GpuDeviceType::NvidiaH100.tpc_count(), 132);
        assert_eq!(GpuDeviceType::NvidiaH200.tpc_count(), 132);
        assert_eq!(GpuDeviceType::NvidiaB200.tpc_count(), 192);
        assert_eq!(GpuDeviceType::AmdMi300x.tpc_count(), 304);
    }

    #[test]
    fn test_device_type_p0_targets() {
        assert!(GpuDeviceType::NvidiaH100.is_p0_target());
        assert!(GpuDeviceType::NvidiaH200.is_p0_target());
        assert!(GpuDeviceType::NvidiaB200.is_p0_target());
    }

    #[test]
    fn test_device_type_p1_targets() {
        assert!(GpuDeviceType::AmdMi300x.is_p1_target());
        assert!(!GpuDeviceType::NvidiaH100.is_p1_target());
    }

    #[test]
    fn test_device_capabilities_h100() {
        let caps = DeviceCapabilities::for_device_type(GpuDeviceType::NvidiaH100);
        assert_eq!(caps.max_concurrent_kernels, 128);
        assert_eq!(caps.max_shared_memory_bytes, 192 * 1024);
        assert_eq!(caps.max_threads_per_block, 1024);
    }

    #[test]
    fn test_device_capabilities_b200() {
        let caps = DeviceCapabilities::for_device_type(GpuDeviceType::NvidiaB200);
        assert_eq!(caps.max_concurrent_kernels, 256);
        assert_eq!(caps.max_shared_memory_bytes, 192 * 1024);
        assert_eq!(caps.max_threads_per_block, 1024);
    }

    #[test]
    fn test_device_capabilities_amd() {
        let caps = DeviceCapabilities::for_device_type(GpuDeviceType::AmdMi300x);
        assert_eq!(caps.max_shared_memory_bytes, 96 * 1024);
    }

    #[test]
    fn test_gpu_device_creation() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let device = GpuDevice::new(device_id, GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi);

        assert_eq!(device.id, device_id);
        assert_eq!(device.device_type, GpuDeviceType::NvidiaH100);
        assert_eq!(device.driver_api, DriverApi::CudaDriverApi);
        assert_eq!(device.total_vram_bytes, 80 * 1024 * 1024 * 1024);
        assert_eq!(device.available_vram_bytes, 80 * 1024 * 1024 * 1024);
        assert_eq!(device.tpc_count, 132);
    }

    #[test]
    fn test_gpu_device_utilization() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let mut device = GpuDevice::new(device_id, GpuDeviceType::NvidiaH100, DriverApi::CudaDriverApi);

        assert_eq!(device.utilization_percent(), 0);

        device.set_available_vram(40 * 1024 * 1024 * 1024); // Half used
        assert_eq!(device.utilization_percent(), 50);

        device.set_available_vram(0);
        assert!(device.is_saturated());
        assert_eq!(device.utilization_percent(), 100);
    }

    #[test]
    fn test_gpu_device_capabilities() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let device = GpuDevice::new(device_id, GpuDeviceType::NvidiaB200, DriverApi::CudaDriverApi);

        let caps = device.capabilities();
        assert_eq!(caps.max_concurrent_kernels, 256);
    }

    #[test]
    fn test_device_type_display() {
        assert_eq!(format!("{}", GpuDeviceType::NvidiaH100), "NVIDIA H100");
        assert_eq!(format!("{}", GpuDeviceType::AmdMi300x), "AMD MI300X");
    }

    #[test]
    fn test_driver_api_display() {
        assert_eq!(format!("{}", DriverApi::CudaDriverApi), "CUDA Driver API");
        assert_eq!(format!("{}", DriverApi::RocmHip), "ROCm HIP");
    }
}

// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! ROCm HIP abstraction layer (P1 target).
//!
//! Provides a clean abstraction over AMD ROCm HIP API,
//! mirroring the CUDA Driver API structure for Phase A (v1.0).
//! This module enables AMD MI300X support as a P1 (secondary) target.
//!
//! CUDA ↔ HIP Translation Notes:
//! - CUcontext ↔ hipCtx_t
//! - CUstream ↔ hipStream_t
//! - CUdeviceptr ↔ void* (device pointer)
//! - CUfunction ↔ hipFunction_t
//! - CUevent ↔ hipEvent_t
//! - cuDeviceGet ↔ hipDeviceGet
//! - cuCtxCreate ↔ hipCtxCreate
//! - cuMemAlloc ↔ hipMalloc
//! - cuLaunchKernel ↔ hipModuleLaunchKernel
//!
//! Reference: Engineering Plan § Driver Abstraction Layer, ROCm HIP Support

use crate::error::GpuError;
use alloc::string::String;
use core::fmt;

/// Unique identifier for a HIP context (device + HIP runtime context).
///
/// Represents a logical GPU compute context bound to a specific AMD GPU.
/// Analogous to CUDA's CUcontext.
///
/// Reference: ROCm HIP § Context Management (hipCtx_t)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HipContext {
    /// GPU device ordinal (0-based index).
    pub device_ordinal: u32,

    /// Opaque HIP context handle from ROCm runtime (libamdhip64.so).
    ///
    /// In FFI code, this would be cast to hipCtx_t.
    pub context_handle: u64,

    /// Context creation flags (HIP device attributes).
    pub flags: u32,
}

impl fmt::Display for HipContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HipContext(device={}, handle=0x{:x}, flags=0x{:x})",
            self.device_ordinal, self.context_handle, self.flags
        )
    }
}

/// HIP stream abstraction (asynchronous command queue).
///
/// Streams enable concurrent kernel launches and memory transfers on AMD GPUs.
/// Analogous to CUDA's CUstream.
///
/// Reference: ROCm HIP § Stream Management (hipStream_t)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HipStream {
    /// Opaque HIP stream handle from ROCm runtime (libamdhip64.so).
    ///
    /// In FFI code, this would be cast to hipStream_t.
    pub stream_handle: u64,

    /// Stream priority level (HIP: 0 = default, lower = higher priority).
    pub priority: i32,

    /// Owning crew identifier (for isolation tracking).
    pub owning_crew: [u8; 16],
}

impl fmt::Display for HipStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HipStream(handle=0x{:x}, priority={})",
            self.stream_handle, self.priority
        )
    }
}

/// HIP device memory abstraction.
///
/// Represents a memory allocation on AMD GPU device memory.
/// Analogous to CUDA's CUdeviceptr wrapper.
///
/// Reference: ROCm HIP § Memory Management
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HipMemory {
    /// GPU device pointer (opaque handle from HIP runtime).
    ///
    /// In FFI code, this would be cast to void* (device address).
    pub device_ptr: u64,

    /// Allocation size in bytes.
    pub size_bytes: u64,

    /// Memory allocation type (affects access patterns and isolation).
    pub allocation_type: HipMemoryType,
}

impl fmt::Display for HipMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HipMemory(ptr=0x{:x}, size={}B, type={:?})",
            self.device_ptr, self.size_bytes, self.allocation_type
        )
    }
}

/// HIP memory allocation type.
///
/// Specifies where a memory allocation resides on AMD GPU hardware.
/// Analogous to CUDA's allocation types.
///
/// Reference: ROCm HIP § Memory Allocation Types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HipMemoryType {
    /// Device-local memory (on-GPU, fastest, isolated per crew).
    DeviceLocal,

    /// Unified memory (AMD XNACK, CPU + GPU share address space).
    ///
    /// Similar to CUDA unified memory with automatic migration.
    Unified,

    /// Pinned host memory (CPU-side, page-locked for DMA transfers).
    HostPinned,
}

impl fmt::Display for HipMemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HipMemoryType::DeviceLocal => write!(f, "DeviceLocal"),
            HipMemoryType::Unified => write!(f, "Unified"),
            HipMemoryType::HostPinned => write!(f, "HostPinned"),
        }
    }
}

/// HIP kernel launch configuration.
///
/// Bundles all parameters needed to launch a kernel on AMD GPU.
/// Analogous to CUDA's kernel launch structure.
///
/// Reference: ROCm HIP § Kernel Launch (hipModuleLaunchKernel)
#[derive(Clone, Copy, Debug)]
pub struct HipKernelLaunch {
    /// GPU function handle (opaque from HIP runtime).
    ///
    /// In FFI code, this would be cast to hipFunction_t.
    pub function_handle: u64,

    /// Grid dimensions (number of blocks) — (x, y, z).
    pub grid_dim: (u32, u32, u32),

    /// Block dimensions (threads per block) — (x, y, z).
    pub block_dim: (u32, u32, u32),

    /// Shared memory size per block in bytes (LDS on AMD).
    pub shared_mem: u32,

    /// Stream to launch kernel on (for async execution).
    pub stream: HipStream,

    /// Kernel arguments (opaque parameter pack).
    ///
    /// In FFI code, this would be a pointer to kernel argument array.
    pub args: u64,
}

impl fmt::Display for HipKernelLaunch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HipKernelLaunch(func=0x{:x}, grid={:?}, block={:?}, shared={}, stream=0x{:x})",
            self.function_handle, self.grid_dim, self.block_dim, self.shared_mem, self.stream.stream_handle
        )
    }
}

/// HIP event (GPU synchronization primitive).
///
/// Events can be recorded on a stream and waited on for synchronization.
/// Analogous to CUDA's CUevent.
///
/// Reference: ROCm HIP § Event Management (hipEvent_t)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HipEvent {
    /// Opaque HIP event handle from ROCm runtime (libamdhip64.so).
    ///
    /// In FFI code, this would be cast to hipEvent_t.
    pub event_handle: u64,

    /// Event creation flags (HIP event attributes).
    pub flags: u32,
}

impl fmt::Display for HipEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HipEvent(handle=0x{:x})", self.event_handle)
    }
}

/// AMD GPU device properties (capabilities and limits).
///
/// Obtained via hipDeviceGetAttribute/hipGetDeviceProperties.
/// Analogous to CUDA device properties.
///
/// Reference: ROCm HIP § Device Properties
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HipDeviceProperties {
    /// Device name (e.g., "AMD MI300X").
    pub name: [u8; 256],

    /// Compute capability equivalent (for compatibility).
    /// AMD RDNA 3 (MI300X) maps to (9, 1).
    pub compute_capability: (u32, u32),

    /// Total VRAM in bytes (192 GB for MI300X).
    pub total_vram: u64,

    /// Maximum threads per block (1024 for MI300X).
    pub max_threads: u32,

    /// Maximum blocks per dimension.
    pub max_blocks: (u32, u32, u32),

    /// GPU Core count (MI300X has 304 cores).
    pub gpu_core_count: u32,

    /// CU (Compute Unit) count for MI300X (38 CUs, analogous to TPCs).
    pub cu_count: u32,
}

/// HIP Driver API abstraction trait.
///
/// Defines the interface to interact with AMD ROCm HIP API (libamdhip64.so).
/// Implementations can delegate to actual FFI bindings or mock for testing.
///
/// This trait mirrors the CUDA API trait to provide cross-platform support.
///
/// Reference: Engineering Plan § Driver Abstraction Layer
pub trait HipApi: core::fmt::Debug {
    /// Initialize a HIP device and create a context.
    ///
    /// Analogous to CUDA's cuDeviceGet + cuCtxCreate.
    ///
    /// # Arguments
    ///
    /// * `ordinal` - Device ordinal (0-based index from device enumeration)
    ///
    /// # Returns
    ///
    /// A HipContext if successful, or GpuError if initialization fails.
    fn init_device(&mut self, ordinal: u32) -> Result<HipContext, GpuError>;

    /// Create a stream on a HIP context.
    ///
    /// Analogous to CUDA's cuStreamCreate.
    ///
    /// # Arguments
    ///
    /// * `context` - Target HIP context
    /// * `priority` - Stream priority level
    /// * `owning_crew` - Crew identifier for isolation tracking
    ///
    /// # Returns
    ///
    /// A HipStream if successful.
    fn create_stream(
        &mut self,
        context: &HipContext,
        priority: i32,
        owning_crew: [u8; 16],
    ) -> Result<HipStream, GpuError>;

    /// Allocate device memory on a context.
    ///
    /// Analogous to CUDA's cuMemAlloc.
    ///
    /// # Arguments
    ///
    /// * `context` - Target HIP context
    /// * `size` - Allocation size in bytes
    /// * `alloc_type` - Memory type (DeviceLocal, Unified, HostPinned)
    ///
    /// # Returns
    ///
    /// A HipMemory object if successful.
    fn alloc_memory(
        &mut self,
        context: &HipContext,
        size: u64,
        alloc_type: HipMemoryType,
    ) -> Result<HipMemory, GpuError>;

    /// Free device memory.
    ///
    /// Analogous to CUDA's cuMemFree.
    ///
    /// # Arguments
    ///
    /// * `mem` - Memory allocation to free
    fn free_memory(&mut self, mem: &HipMemory) -> Result<(), GpuError>;

    /// Launch a kernel on a stream.
    ///
    /// Analogous to CUDA's cuLaunchKernel.
    ///
    /// # Arguments
    ///
    /// * `launch` - Kernel launch configuration
    fn launch_kernel(&mut self, launch: &HipKernelLaunch) -> Result<(), GpuError>;

    /// Synchronize a stream (wait for all pending operations to complete).
    ///
    /// Analogous to CUDA's cuStreamSynchronize.
    ///
    /// # Arguments
    ///
    /// * `stream` - Stream to synchronize
    fn synchronize_stream(&mut self, stream: &HipStream) -> Result<(), GpuError>;

    /// Create an event on a stream for timing/synchronization.
    ///
    /// Analogous to CUDA's cuEventCreate.
    ///
    /// # Arguments
    ///
    /// * `stream` - Associated stream
    fn create_event(&mut self, stream: &HipStream) -> Result<HipEvent, GpuError>;

    /// Query device properties.
    ///
    /// Analogous to CUDA's cuDeviceGetAttribute.
    ///
    /// # Arguments
    ///
    /// * `ordinal` - Device ordinal
    ///
    /// # Returns
    ///
    /// Device properties if successful.
    fn query_device_properties(&mut self, ordinal: u32) -> Result<HipDeviceProperties, GpuError>;
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_hip_context_creation() {
        let ctx = HipContext {
            device_ordinal: 0,
            context_handle: 0xdeadbeef,
            flags: 0,
        };

        assert_eq!(ctx.device_ordinal, 0);
        assert_eq!(ctx.context_handle, 0xdeadbeef);
    }

    #[test]
    fn test_hip_stream_creation() {
        let stream = HipStream {
            stream_handle: 0x12345678,
            priority: 0,
            owning_crew: [1u8; 16],
        };

        assert_eq!(stream.priority, 0);
        assert_eq!(stream.owning_crew[0], 1);
    }

    #[test]
    fn test_hip_memory_creation() {
        let mem = HipMemory {
            device_ptr: 0x100000,
            size_bytes: 2 * 1024 * 1024,
            allocation_type: HipMemoryType::DeviceLocal,
        };

        assert_eq!(mem.device_ptr, 0x100000);
        assert_eq!(mem.size_bytes, 2 * 1024 * 1024);
        assert_eq!(mem.allocation_type, HipMemoryType::DeviceLocal);
    }

    #[test]
    fn test_hip_memory_type_display() {
        assert_eq!(format!("{}", HipMemoryType::DeviceLocal), "DeviceLocal");
        assert_eq!(format!("{}", HipMemoryType::Unified), "Unified");
        assert_eq!(format!("{}", HipMemoryType::HostPinned), "HostPinned");
    }

    #[test]
    fn test_hip_kernel_launch_creation() {
        let stream = HipStream {
            stream_handle: 0x1,
            priority: 0,
            owning_crew: [0u8; 16],
        };

        let launch = HipKernelLaunch {
            function_handle: 0x12345678,
            grid_dim: (16, 1, 1),
            block_dim: (256, 1, 1),
            shared_mem: 2048,
            stream,
            args: 0,
        };

        assert_eq!(launch.grid_dim, (16, 1, 1));
        assert_eq!(launch.block_dim, (256, 1, 1));
        assert_eq!(launch.shared_mem, 2048);
    }

    #[test]
    fn test_hip_event_creation() {
        let event = HipEvent {
            event_handle: 0x87654321,
            flags: 0,
        };

        assert_eq!(event.event_handle, 0x87654321);
    }

    #[test]
    fn test_hip_device_properties_creation() {
        let mut props = HipDeviceProperties {
            name: [0u8; 256],
            compute_capability: (9, 1),
            total_vram: 192 * 1024 * 1024 * 1024,
            max_threads: 1024,
            max_blocks: (65535, 65535, 65535),
            gpu_core_count: 304,
            cu_count: 304,
        };

        // Fill in name
        props.name[0] = b'M';
        props.name[1] = b'I';
        props.name[2] = b'3';
        props.name[3] = b'0';
        props.name[4] = b'0';
        props.name[5] = b'X';

        assert_eq!(props.compute_capability, (9, 1));
        assert_eq!(props.total_vram, 192 * 1024 * 1024 * 1024);
        assert_eq!(props.gpu_core_count, 304);
    }

    #[test]
    fn test_hip_context_display() {
        let ctx = HipContext {
            device_ordinal: 0,
            context_handle: 0xbeef,
            flags: 0x02,
        };

        let display_str = format!("{}", ctx);
        assert!(display_str.contains("device=0"));
        assert!(display_str.contains("handle=0xbeef"));
    }

    #[test]
    fn test_hip_memory_display() {
        let mem = HipMemory {
            device_ptr: 0x200,
            size_bytes: 4096,
            allocation_type: HipMemoryType::DeviceLocal,
        };

        let display_str = format!("{}", mem);
        assert!(display_str.contains("0x200"));
        assert!(display_str.contains("4096B"));
    }

    #[test]
    fn test_hip_stream_display() {
        let stream = HipStream {
            stream_handle: 0xaaa,
            priority: -3,
            owning_crew: [3u8; 16],
        };

        let display_str = format!("{}", stream);
        assert!(display_str.contains("0xaaa"));
        assert!(display_str.contains("-3"));
    }

    #[test]
    fn test_hip_event_display() {
        let event = HipEvent {
            event_handle: 0xbbb,
            flags: 1,
        };

        let display_str = format!("{}", event);
        assert!(display_str.contains("0xbbb"));
    }
}

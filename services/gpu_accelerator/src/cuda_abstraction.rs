// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! CUDA Driver API abstraction layer.
//!
//! Provides a clean abstraction over the NVIDIA CUDA Driver API (libcuda.so),
//! enabling safe kernel launches, memory management, and device interaction.
//! This module implements Phase A (v1.0) strategy per Addendum v2.5.1.
//!
//! Reference: Engineering Plan § Driver Abstraction Layer, Phase A Strategy

use crate::error::GpuError;
use alloc::string::String;
use core::fmt;

/// Unique identifier for a CUDA context (device + driver context handle).
///
/// Represents a logical GPU compute context bound to a specific device.
/// All memory allocations and kernel launches must occur within a context.
///
/// Reference: CUDA Driver API § Context Management
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CudaContext {
    /// GPU device ordinal (0-based index).
    pub device_ordinal: u32,

    /// Opaque CUDA context handle from driver (libcuda.so).
    ///
    /// In FFI code, this would be cast to CUcontext.
    pub context_handle: u64,

    /// Context creation flags (e.g., CU_CTX_SCHED_AUTO, CU_CTX_MAP_HOST).
    pub flags: u32,
}

impl fmt::Display for CudaContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CudaContext(device={}, handle=0x{:x}, flags=0x{:x})",
            self.device_ordinal, self.context_handle, self.flags
        )
    }
}

/// CUDA stream abstraction (asynchronous command queue).
///
/// Streams enable concurrent kernel launches and memory transfers.
/// Each stream maintains its own command queue and can be synchronized independently.
///
/// Reference: CUDA Driver API § Stream Management
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CudaStream {
    /// Opaque CUDA stream handle from driver (libcuda.so).
    ///
    /// In FFI code, this would be cast to CUstream.
    pub stream_handle: u64,

    /// Stream priority level (lower value = higher priority).
    /// Typical range: -10 to 0 (0 = default).
    pub priority: i32,

    /// Owning crew identifier (for isolation tracking).
    pub owning_crew: [u8; 16],
}

impl fmt::Display for CudaStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CudaStream(handle=0x{:x}, priority={})",
            self.stream_handle, self.priority
        )
    }
}

/// CUDA device memory abstraction.
///
/// Represents a memory allocation on GPU device memory.
/// Tracks allocation type (device-local, unified, or pinned host) for optimization.
///
/// Reference: CUDA Driver API § Memory Management
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CudaMemory {
    /// GPU device pointer (opaque handle from driver).
    ///
    /// In FFI code, this would be cast to CUdeviceptr.
    pub device_ptr: u64,

    /// Allocation size in bytes.
    pub size_bytes: u64,

    /// Memory allocation type (affects access patterns and isolation).
    pub allocation_type: CudaMemoryType,
}

impl fmt::Display for CudaMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CudaMemory(ptr=0x{:x}, size={}B, type={:?})",
            self.device_ptr, self.size_bytes, self.allocation_type
        )
    }
}

/// CUDA memory allocation type.
///
/// Specifies where a memory allocation resides and how it can be accessed.
///
/// Reference: CUDA Driver API § Memory Types
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CudaMemoryType {
    /// Device-local memory (on-GPU, fastest, isolated per crew).
    DeviceLocal,

    /// Unified memory (CPU + GPU share address space, automatic migration).
    ///
    /// Useful for unified memory semantics but has performance implications.
    Unified,

    /// Pinned host memory (CPU-side, page-locked for DMA transfers).
    ///
    /// Used for host<->device transfers to avoid paging during copy.
    HostPinned,
}

impl fmt::Display for CudaMemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CudaMemoryType::DeviceLocal => write!(f, "DeviceLocal"),
            CudaMemoryType::Unified => write!(f, "Unified"),
            CudaMemoryType::HostPinned => write!(f, "HostPinned"),
        }
    }
}

/// CUDA kernel launch configuration.
///
/// Bundles all parameters needed to launch a kernel on the GPU.
/// This includes grid dimensions, block dimensions, shared memory, and stream.
///
/// Reference: CUDA Driver API § Kernel Launch
#[derive(Clone, Copy, Debug)]
pub struct CudaKernelLaunch {
    /// GPU function handle (opaque from driver).
    ///
    /// In FFI code, this would be cast to CUfunction.
    pub function_handle: u64,

    /// Grid dimensions (number of blocks) — (x, y, z).
    pub grid_dim: (u32, u32, u32),

    /// Block dimensions (threads per block) — (x, y, z).
    pub block_dim: (u32, u32, u32),

    /// Shared memory size per block in bytes.
    pub shared_mem: u32,

    /// Stream to launch kernel on (for async execution).
    pub stream: CudaStream,

    /// Kernel arguments (opaque parameter pack).
    ///
    /// In FFI code, this would be a pointer to kernel argument array.
    pub args: u64,
}

impl fmt::Display for CudaKernelLaunch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CudaKernelLaunch(func=0x{:x}, grid={:?}, block={:?}, shared={}, stream=0x{:x})",
            self.function_handle, self.grid_dim, self.block_dim, self.shared_mem, self.stream.stream_handle
        )
    }
}

/// CUDA event (GPU synchronization primitive).
///
/// Events can be recorded on a stream and waited on for synchronization.
/// Used to measure kernel execution time and coordinate between streams.
///
/// Reference: CUDA Driver API § Event Management
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CudaEvent {
    /// Opaque CUDA event handle from driver (libcuda.so).
    ///
    /// In FFI code, this would be cast to CUevent.
    pub event_handle: u64,

    /// Event creation flags (e.g., CU_EVENT_BLOCKING_SYNC).
    pub flags: u32,
}

impl fmt::Display for CudaEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CudaEvent(handle=0x{:x})", self.event_handle)
    }
}

/// GPU device properties (capabilities and limits).
///
/// Obtained via cuDeviceGetAttribute/cuDeviceGetProperties.
/// Used for scheduling decisions and kernel configuration validation.
///
/// Reference: CUDA Driver API § Device Properties
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceProperties {
    /// Device name (e.g., "NVIDIA H100 PCIe").
    pub name: [u8; 256],

    /// Compute capability (major, minor).
    pub compute_capability: (u32, u32),

    /// Total VRAM in bytes.
    pub total_vram: u64,

    /// Maximum threads per block.
    pub max_threads: u32,

    /// Maximum blocks per dimension.
    pub max_blocks: (u32, u32, u32),

    /// Streaming Multiprocessor (SM) count.
    pub sm_count: u32,

    /// TPC (Tensor Processing Cluster) count (same as SM count for NVIDIA).
    pub tpc_count: u32,
}

/// CUDA Driver API abstraction trait.
///
/// Defines the interface to interact with the NVIDIA CUDA Driver API (libcuda.so).
/// Implementations can delegate to actual FFI bindings or mock for testing.
///
/// Reference: Engineering Plan § Driver Abstraction Layer
pub trait CudaApi: core::fmt::Debug {
    /// Initialize a CUDA device and create a context.
    ///
    /// # Arguments
    ///
    /// * `ordinal` - Device ordinal (0-based index from device enumeration)
    ///
    /// # Returns
    ///
    /// A CudaContext if successful, or GpuError if initialization fails.
    ///
    /// # Errors
    ///
    /// - `GpuError::DeviceNotFound`: Device ordinal invalid or unavailable
    /// - `GpuError::DriverError`: Driver-level initialization failure
    fn init_device(&mut self, ordinal: u32) -> Result<CudaContext, GpuError>;

    /// Create a stream on a CUDA context.
    ///
    /// # Arguments
    ///
    /// * `context` - Target CUDA context
    /// * `priority` - Stream priority level (typically -10 to 0)
    /// * `owning_crew` - Crew identifier for isolation tracking
    ///
    /// # Returns
    ///
    /// A CudaStream if successful.
    fn create_stream(
        &mut self,
        context: &CudaContext,
        priority: i32,
        owning_crew: [u8; 16],
    ) -> Result<CudaStream, GpuError>;

    /// Allocate device memory on a context.
    ///
    /// # Arguments
    ///
    /// * `context` - Target CUDA context
    /// * `size` - Allocation size in bytes
    /// * `alloc_type` - Memory type (DeviceLocal, Unified, HostPinned)
    ///
    /// # Returns
    ///
    /// A CudaMemory object if successful.
    fn alloc_memory(
        &mut self,
        context: &CudaContext,
        size: u64,
        alloc_type: CudaMemoryType,
    ) -> Result<CudaMemory, GpuError>;

    /// Free device memory.
    ///
    /// # Arguments
    ///
    /// * `mem` - Memory allocation to free
    ///
    /// # Errors
    ///
    /// - `GpuError::DriverError`: Memory deallocation failed
    fn free_memory(&mut self, mem: &CudaMemory) -> Result<(), GpuError>;

    /// Launch a kernel on a stream.
    ///
    /// # Arguments
    ///
    /// * `launch` - Kernel launch configuration
    ///
    /// # Errors
    ///
    /// - `GpuError::KernelLaunchFailed`: Grid/block dims invalid or resource exhaustion
    /// - `GpuError::DriverError`: Driver error during launch
    fn launch_kernel(&mut self, launch: &CudaKernelLaunch) -> Result<(), GpuError>;

    /// Synchronize a stream (wait for all pending operations to complete).
    ///
    /// # Arguments
    ///
    /// * `stream` - Stream to synchronize
    ///
    /// # Errors
    ///
    /// - `GpuError::DriverError`: Synchronization failed or kernel error
    fn synchronize_stream(&mut self, stream: &CudaStream) -> Result<(), GpuError>;

    /// Create an event on a stream for timing/synchronization.
    ///
    /// # Arguments
    ///
    /// * `stream` - Associated stream
    ///
    /// # Returns
    ///
    /// A CudaEvent if successful.
    fn create_event(&mut self, stream: &CudaStream) -> Result<CudaEvent, GpuError>;

    /// Query device properties.
    ///
    /// # Arguments
    ///
    /// * `ordinal` - Device ordinal
    ///
    /// # Returns
    ///
    /// Device properties if successful.
    fn query_device_properties(&mut self, ordinal: u32) -> Result<DeviceProperties, GpuError>;
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_cuda_context_creation() {
        let ctx = CudaContext {
            device_ordinal: 0,
            context_handle: 0xdeadbeef,
            flags: 0,
        };

        assert_eq!(ctx.device_ordinal, 0);
        assert_eq!(ctx.context_handle, 0xdeadbeef);
    }

    #[test]
    fn test_cuda_stream_creation() {
        let stream = CudaStream {
            stream_handle: 0x12345678,
            priority: 0,
            owning_crew: [1u8; 16],
        };

        assert_eq!(stream.priority, 0);
        assert_eq!(stream.owning_crew[0], 1);
    }

    #[test]
    fn test_cuda_memory_creation() {
        let mem = CudaMemory {
            device_ptr: 0x100000,
            size_bytes: 1024 * 1024,
            allocation_type: CudaMemoryType::DeviceLocal,
        };

        assert_eq!(mem.device_ptr, 0x100000);
        assert_eq!(mem.size_bytes, 1024 * 1024);
        assert_eq!(mem.allocation_type, CudaMemoryType::DeviceLocal);
    }

    #[test]
    fn test_cuda_memory_type_display() {
        assert_eq!(format!("{}", CudaMemoryType::DeviceLocal), "DeviceLocal");
        assert_eq!(format!("{}", CudaMemoryType::Unified), "Unified");
        assert_eq!(format!("{}", CudaMemoryType::HostPinned), "HostPinned");
    }

    #[test]
    fn test_cuda_kernel_launch_creation() {
        let stream = CudaStream {
            stream_handle: 0x1,
            priority: 0,
            owning_crew: [0u8; 16],
        };

        let launch = CudaKernelLaunch {
            function_handle: 0x12345678,
            grid_dim: (8, 1, 1),
            block_dim: (256, 1, 1),
            shared_mem: 4096,
            stream,
            args: 0,
        };

        assert_eq!(launch.grid_dim, (8, 1, 1));
        assert_eq!(launch.block_dim, (256, 1, 1));
        assert_eq!(launch.shared_mem, 4096);
    }

    #[test]
    fn test_cuda_event_creation() {
        let event = CudaEvent {
            event_handle: 0x87654321,
            flags: 0,
        };

        assert_eq!(event.event_handle, 0x87654321);
    }

    #[test]
    fn test_device_properties_creation() {
        let mut props = DeviceProperties {
            name: [0u8; 256],
            compute_capability: (9, 0),
            total_vram: 80 * 1024 * 1024 * 1024,
            max_threads: 1024,
            max_blocks: (65535, 65535, 65535),
            sm_count: 132,
            tpc_count: 132,
        };

        // Fill in name
        props.name[0] = b'H';
        props.name[1] = b'1';
        props.name[2] = b'0';
        props.name[3] = b'0';

        assert_eq!(props.compute_capability, (9, 0));
        assert_eq!(props.total_vram, 80 * 1024 * 1024 * 1024);
        assert_eq!(props.sm_count, 132);
        assert_eq!(props.tpc_count, 132);
    }

    #[test]
    fn test_cuda_context_display() {
        let ctx = CudaContext {
            device_ordinal: 1,
            context_handle: 0xdead,
            flags: 0x01,
        };

        let display_str = format!("{}", ctx);
        assert!(display_str.contains("device=1"));
        assert!(display_str.contains("handle=0xdead"));
    }

    #[test]
    fn test_cuda_memory_display() {
        let mem = CudaMemory {
            device_ptr: 0x100,
            size_bytes: 2048,
            allocation_type: CudaMemoryType::DeviceLocal,
        };

        let display_str = format!("{}", mem);
        assert!(display_str.contains("0x100"));
        assert!(display_str.contains("2048B"));
    }

    #[test]
    fn test_cuda_stream_display() {
        let stream = CudaStream {
            stream_handle: 0x999,
            priority: -5,
            owning_crew: [2u8; 16],
        };

        let display_str = format!("{}", stream);
        assert!(display_str.contains("0x999"));
        assert!(display_str.contains("-5"));
    }
}

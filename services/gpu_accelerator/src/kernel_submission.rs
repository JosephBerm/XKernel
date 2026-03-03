// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU kernel submission via CUDA/HIP APIs.
//!
//! Implements the kernel submission path: inference framework → syscall → GPU Manager →
//! cuLaunchKernel/hipModuleLaunchKernel. Handles context/model binding and GPU memory
//! argument preparation.
//!
//! ## Submission Flow
//!
//! ```
//! Framework (vLLM/TensorRT-LLM)
//!     ↓ (kernel submission request)
//! GPU Manager Kernel Submission
//!     ├─ Bind CUDA/HIP context to crew
//!     ├─ Resolve kernel function handle
//!     ├─ Prepare grid/block dims
//!     ├─ Package VRAM arguments
//!     ↓
//! cuLaunchKernel / hipModuleLaunchKernel
//!     ↓
//! GPU Hardware Execution
//! ```
//!
//! Reference: Engineering Plan § Kernel Submission Path, Week 5 Addendum v2.5.1

use crate::command_queue::{CommandQueueEntry, SubmissionId};
use crate::cuda_abstraction::CudaContext;
use crate::rocm_abstraction::HipContext;
use crate::error::GpuError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Kernel submission configuration from inference framework.
///
/// Encapsulates all information needed to launch a GPU kernel:
/// function identity, grid/block dimensions, shared memory, and VRAM arguments.
///
/// Reference: Engineering Plan § Kernel Submission Configuration
#[derive(Clone, Debug)]
pub struct KernelSubmissionConfig {
    /// GPU kernel function identifier (e.g., model_layer_attention_kernel).
    ///
    /// Resolved to CUDA CUfunction or HIP hipFunction_t by GPU Manager.
    pub kernel_name: [u8; 64],

    /// Grid dimensions (number of thread blocks).
    pub grid_dims: (u32, u32, u32),

    /// Block dimensions (threads per thread block).
    pub block_dims: (u32, u32, u32),

    /// Shared memory per block in bytes.
    pub shared_memory_bytes: u32,

    /// Kernel argument buffer (opaque, passed to GPU).
    ///
    /// Contains VRAM pointers, scalars, and other kernel parameters.
    /// Packed according to ABI (typically device pointers follow scalar args).
    pub args_buffer: u64,

    /// Size of arguments buffer in bytes.
    pub args_size: u32,

    /// Crew identifier submitting this kernel.
    pub crew_id: [u8; 16],

    /// Model version/identifier for binding.
    pub model_id: [u8; 32],

    /// Priority level (higher = execute sooner).
    pub priority: u32,

    /// Deadline for execution in nanoseconds (0 = no deadline).
    pub deadline_ns: u64,

    /// GPU device ordinal to execute on.
    pub device_ordinal: u32,
}

impl KernelSubmissionConfig {
    /// Create a new kernel submission configuration.
    pub fn new(
        kernel_name: [u8; 64],
        grid_dims: (u32, u32, u32),
        block_dims: (u32, u32, u32),
        shared_memory_bytes: u32,
        crew_id: [u8; 16],
        model_id: [u8; 32],
        priority: u32,
        device_ordinal: u32,
    ) -> Self {
        KernelSubmissionConfig {
            kernel_name,
            grid_dims,
            block_dims,
            shared_memory_bytes,
            args_buffer: 0,
            args_size: 0,
            crew_id,
            model_id,
            priority,
            deadline_ns: 0,
            device_ordinal,
        }
    }

    /// Set kernel arguments buffer.
    pub fn with_args(mut self, args_buffer: u64, args_size: u32) -> Self {
        self.args_buffer = args_buffer;
        self.args_size = args_size;
        self
    }

    /// Set execution deadline.
    pub fn with_deadline(mut self, deadline_ns: u64) -> Self {
        self.deadline_ns = deadline_ns;
        self
    }

    /// Validate kernel configuration.
    ///
    /// Checks grid/block dims are non-zero and within reasonable bounds.
    pub fn validate(&self) -> Result<(), GpuError> {
        // Check grid dimensions
        if self.grid_dims.0 == 0 || self.block_dims.0 == 0 {
            return Err(GpuError::KernelLaunchFailed);
        }

        // Check block dimensions don't exceed typical limits
        // (H100: max 1024 threads/block, max 32 blocks in each dimension)
        let total_threads = (self.block_dims.0 as u64)
            * (self.block_dims.1 as u64)
            * (self.block_dims.2 as u64);
        if total_threads > 1024 {
            return Err(GpuError::KernelLaunchFailed);
        }

        if self.grid_dims.0 > 65535 || self.grid_dims.1 > 65535 || self.grid_dims.2 > 65535 {
            return Err(GpuError::KernelLaunchFailed);
        }

        Ok(())
    }
}

impl fmt::Display for KernelSubmissionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KernelSubmission(kernel={:?}, grid={:?}, block={:?}, smem={})",
            &self.kernel_name[..8], self.grid_dims, self.block_dims, self.shared_memory_bytes
        )
    }
}

/// Result of a kernel submission to GPU.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmissionResult {
    /// Submission ID for tracking.
    pub submission_id: SubmissionId,

    /// Submission timestamp in nanoseconds.
    pub submitted_at_ns: u64,

    /// GPU stream on which kernel was submitted (opaque handle).
    pub stream_handle: u64,

    /// Estimated latency to GPU in nanoseconds.
    pub latency_ns: u64,
}

impl fmt::Display for SubmissionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SubmissionResult({}, latency={}ns)",
            self.submission_id, self.latency_ns
        )
    }
}

/// Kernel function registry for resolving kernel names to GPU function handles.
///
/// Maps kernel identifiers (from inference framework) to GPU function handles
/// (CUDA CUfunction / HIP hipFunction_t). Enables dynamic kernel resolution
/// and per-model kernel registration.
///
/// Reference: Engineering Plan § Kernel Function Resolution
#[derive(Debug)]
pub struct KernelFunctionRegistry {
    /// Mapping of (model_id, kernel_name) → function handle.
    functions: BTreeMap<(ModelKey, [u8; 64]), u64>,

    /// Next function ID counter (for local tracking).
    next_function_id: u64,
}

/// Key for kernel function lookup: (model_id, device_ordinal).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ModelKey {
    model_id: [u8; 32],
    device_ordinal: u32,
}

impl KernelFunctionRegistry {
    /// Create a new kernel function registry.
    pub fn new() -> Self {
        KernelFunctionRegistry {
            functions: BTreeMap::new(),
            next_function_id: 1,
        }
    }

    /// Register a kernel function for a model.
    ///
    /// # Arguments
    ///
    /// * `model_id` - Model identifier
    /// * `kernel_name` - Kernel name/identifier
    /// * `function_handle` - GPU function handle (from CUDA/HIP)
    /// * `device_ordinal` - GPU device ordinal
    pub fn register_function(
        &mut self,
        model_id: [u8; 32],
        kernel_name: [u8; 64],
        function_handle: u64,
        device_ordinal: u32,
    ) -> Result<(), GpuError> {
        let key = ModelKey {
            model_id,
            device_ordinal,
        };
        self.functions.insert((key, kernel_name), function_handle);
        Ok(())
    }

    /// Resolve a kernel name to a function handle.
    ///
    /// Returns the registered function handle or an error if not found.
    pub fn resolve_function(
        &self,
        model_id: [u8; 32],
        kernel_name: [u8; 64],
        device_ordinal: u32,
    ) -> Result<u64, GpuError> {
        let key = ModelKey {
            model_id,
            device_ordinal,
        };

        self.functions
            .get(&(key, kernel_name))
            .copied()
            .ok_or(GpuError::KernelLaunchFailed)
    }

    /// Unregister a kernel function.
    pub fn unregister_function(
        &mut self,
        model_id: [u8; 32],
        kernel_name: [u8; 64],
        device_ordinal: u32,
    ) -> Result<(), GpuError> {
        let key = ModelKey {
            model_id,
            device_ordinal,
        };

        self.functions
            .remove(&(key, kernel_name))
            .map(|_| ())
            .ok_or(GpuError::KernelLaunchFailed)
    }

    /// Get the number of registered functions.
    pub fn function_count(&self) -> usize {
        self.functions.len()
    }

    /// Clear all registered functions.
    pub fn clear(&mut self) {
        self.functions.clear();
    }
}

impl Default for KernelFunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Kernel submission manager.
///
/// Orchestrates the kernel submission flow:
/// 1. Validate configuration
/// 2. Resolve kernel function handle
/// 3. Bind CUDA/HIP context to crew
/// 4. Prepare GPU arguments
/// 5. Enqueue to command queue
/// 6. Return submission result with tracking ID
///
/// Reference: Engineering Plan § Kernel Submission Manager
#[derive(Debug)]
pub struct KernelSubmissionManager {
    /// Function registry for kernel resolution.
    function_registry: KernelFunctionRegistry,

    /// Statistics on submissions.
    pub stats: SubmissionStats,
}

/// Statistics for kernel submissions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubmissionStats {
    /// Total kernels submitted to GPU.
    pub total_submitted: u64,

    /// Total kernels successfully launched.
    pub total_launched: u64,

    /// Total kernels failed to launch.
    pub total_failed: u64,

    /// Total validation errors.
    pub total_validation_errors: u64,

    /// Average submission latency in nanoseconds.
    pub avg_latency_ns: u64,
}

impl KernelSubmissionManager {
    /// Create a new kernel submission manager.
    pub fn new() -> Self {
        KernelSubmissionManager {
            function_registry: KernelFunctionRegistry::new(),
            stats: SubmissionStats {
                total_submitted: 0,
                total_launched: 0,
                total_failed: 0,
                total_validation_errors: 0,
                avg_latency_ns: 0,
            },
        }
    }

    /// Register a kernel function.
    ///
    /// Called during model loading to associate kernel names with GPU function handles.
    pub fn register_kernel(
        &mut self,
        model_id: [u8; 32],
        kernel_name: [u8; 64],
        function_handle: u64,
        device_ordinal: u32,
    ) -> Result<(), GpuError> {
        self.function_registry
            .register_function(model_id, kernel_name, function_handle, device_ordinal)
    }

    /// Validate kernel submission configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Kernel submission configuration to validate
    ///
    /// # Returns
    ///
    /// Ok if valid, GpuError if configuration is invalid.
    pub fn validate_submission(&mut self, config: &KernelSubmissionConfig) -> Result<(), GpuError> {
        // Validate grid/block dimensions
        config.validate()?;

        // Verify kernel exists in registry
        self._resolve_function_handle(config)?;

        Ok(())
    }

    /// Prepare kernel submission for GPU execution.
    ///
    /// Resolves kernel function, validates configuration, and returns
    /// a CommandQueueEntry ready for submission to GPU via CUDA/HIP.
    ///
    /// # Arguments
    ///
    /// * `config` - Kernel submission configuration
    /// * `context_handle` - CUDA/HIP context handle (opaque)
    /// * `submission_timestamp_ns` - Timestamp of submission
    ///
    /// # Returns
    ///
    /// CommandQueueEntry if successful, GpuError if configuration invalid or kernel not found.
    pub fn prepare_submission(
        &mut self,
        config: &KernelSubmissionConfig,
        context_handle: u64,
        submission_timestamp_ns: u64,
    ) -> Result<CommandQueueEntry, GpuError> {
        // Validate configuration
        self.validate_submission(config)?;

        // Resolve kernel function handle
        let function_handle = self._resolve_function_handle(config)?;

        // Create command queue entry
        let entry = CommandQueueEntry::new(
            SubmissionId::new(0), // Will be assigned by queue
            function_handle,
            config.grid_dims,
            config.block_dims,
            config.shared_memory_bytes,
            context_handle,
            config.args_buffer,
            config.crew_id,
            config.priority,
            submission_timestamp_ns,
        )
        .with_deadline(config.deadline_ns);

        self.stats.total_submitted += 1;

        Ok(entry)
    }

    /// Resolve kernel function handle from registry.
    fn _resolve_function_handle(&self, config: &KernelSubmissionConfig) -> Result<u64, GpuError> {
        self.function_registry.resolve_function(
            config.model_id,
            config.kernel_name,
            config.device_ordinal,
        )
    }

    /// Get submission statistics.
    pub fn stats(&self) -> SubmissionStats {
        self.stats
    }

    /// Record a successful submission launch.
    pub fn record_launch(&mut self, latency_ns: u64) {
        self.stats.total_launched += 1;

        // Update average latency
        let new_avg = (self.stats.avg_latency_ns * (self.stats.total_launched - 1) + latency_ns)
            / self.stats.total_launched;
        self.stats.avg_latency_ns = new_avg;
    }

    /// Record a failed submission.
    pub fn record_failure(&mut self) {
        self.stats.total_failed += 1;
    }

    /// Record a validation error.
    pub fn record_validation_error(&mut self) {
        self.stats.total_validation_errors += 1;
    }
}

impl Default for KernelSubmissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_kernel_submission_config_creation() {
        let mut config = KernelSubmissionConfig::new(
            [1u8; 64],
            (16, 1, 1),
            (256, 1, 1),
            1024,
            [2u8; 16],
            [3u8; 32],
            5,
            0,
        );

        assert_eq!(config.grid_dims, (16, 1, 1));
        assert_eq!(config.block_dims, (256, 1, 1));
        assert_eq!(config.shared_memory_bytes, 1024);

        config = config.with_args(0x1000, 256);
        assert_eq!(config.args_buffer, 0x1000);
        assert_eq!(config.args_size, 256);
    }

    #[test]
    fn test_kernel_submission_validation() {
        let config = KernelSubmissionConfig::new(
            [1u8; 64],
            (16, 1, 1),
            (256, 1, 1),
            0,
            [2u8; 16],
            [3u8; 32],
            5,
            0,
        );

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_kernel_submission_invalid_grid() {
        let config = KernelSubmissionConfig::new(
            [1u8; 64],
            (0, 1, 1), // Invalid: grid_x = 0
            (256, 1, 1),
            0,
            [2u8; 16],
            [3u8; 32],
            5,
            0,
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_kernel_submission_invalid_block() {
        let config = KernelSubmissionConfig::new(
            [1u8; 64],
            (16, 1, 1),
            (0, 1, 1), // Invalid: block_x = 0
            0,
            [2u8; 16],
            [3u8; 32],
            5,
            0,
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_kernel_submission_exceeds_thread_limit() {
        let config = KernelSubmissionConfig::new(
            [1u8; 64],
            (16, 1, 1),
            (512, 512, 1), // 512*512 = 262144 threads > 1024 limit
            0,
            [2u8; 16],
            [3u8; 32],
            5,
            0,
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_kernel_function_registry() {
        let mut registry = KernelFunctionRegistry::new();

        let model_id = [1u8; 32];
        let kernel_name = [2u8; 64];
        let function_handle = 0x1234;
        let device_ordinal = 0;

        assert!(registry
            .register_function(model_id, kernel_name, function_handle, device_ordinal)
            .is_ok());

        let resolved = registry.resolve_function(model_id, kernel_name, device_ordinal);
        assert!(resolved.is_ok());
        assert_eq!(resolved.unwrap(), function_handle);
    }

    #[test]
    fn test_kernel_function_registry_not_found() {
        let registry = KernelFunctionRegistry::new();

        let result = registry.resolve_function([1u8; 32], [2u8; 64], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_kernel_submission_manager_registration() {
        let mut manager = KernelSubmissionManager::new();

        let model_id = [1u8; 32];
        let kernel_name = [2u8; 64];
        let function_handle = 0x1234;

        assert!(manager
            .register_kernel(model_id, kernel_name, function_handle, 0)
            .is_ok());
    }

    #[test]
    fn test_kernel_submission_manager_stats() {
        let mut manager = KernelSubmissionManager::new();

        assert_eq!(manager.stats().total_submitted, 0);
        assert_eq!(manager.stats().total_launched, 0);

        manager.record_launch(100);
        assert_eq!(manager.stats().total_launched, 1);
        assert_eq!(manager.stats().avg_latency_ns, 100);
    }

    #[test]
    fn test_submission_result_display() {
        let result = SubmissionResult {
            submission_id: SubmissionId::new(1),
            submitted_at_ns: 1000,
            stream_handle: 0x5678,
            latency_ns: 50,
        };

        let display = format!("{}", result);
        assert!(display.contains("SubmissionResult"));
        assert!(display.contains("latency=50ns"));
    }
}

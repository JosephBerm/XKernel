// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Inference framework integration (vLLM & TensorRT-LLM).
//!
//! Provides minimal-change wrappers for popular LLM inference frameworks
//! to submit GPU kernels through the Cognitive Substrate GPU Manager.
//! Enables vLLM and TensorRT-LLM to use CUDA streams managed by GPU Manager.
//!
//! ## Framework Integration Points
//!
//! **vLLM:**
//! - `vLLMStreamAdapter`: Wraps vLLM's StreamWrapper with GPU Manager stream handling
//! - Intercepts kernel submissions (attention, MLP, etc.)
//! - Routes through GPU Manager's command queue
//!
//! **TensorRT-LLM:**
//! - `TensorRTStreamAdapter`: Wraps TensorRT-LLM's CudaStream with GPU Manager binding
//! - Provides stream creation/destruction callbacks
//! - Manages context binding per crew
//!
//! Reference: Engineering Plan § Framework Integration, Week 5 Addendum v2.5.1

use crate::command_queue::{CommandQueue, CommandQueueEntry, SubmissionId};
use crate::kernel_submission::{KernelSubmissionConfig, KernelSubmissionManager};
use crate::error::GpuError;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Framework type identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameworkType {
    /// NVIDIA vLLM (vllm.ai)
    VLlm,

    /// NVIDIA TensorRT-LLM
    TensorRtLlm,

    /// Custom / third-party framework
    Custom,
}

impl fmt::Display for FrameworkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameworkType::VLlm => write!(f, "vLLM"),
            FrameworkType::TensorRtLlm => write!(f, "TensorRT-LLM"),
            FrameworkType::Custom => write!(f, "Custom"),
        }
    }
}

/// Framework stream adapter for vLLM.
///
/// Wraps vLLM's StreamWrapper to intercept kernel submissions and route
/// through GPU Manager's command queue. Minimal changes to vLLM codebase.
///
/// **Integration Points:**
/// - vLLM creates streams via `torch.cuda.Stream()`
/// - Wrapper intercepts CUDA kernel launches
/// - Routes to GPU Manager command queue
/// - vLLM waits for completion as normal
///
/// Reference: Engineering Plan § vLLM Integration
#[derive(Debug)]
pub struct VLlmStreamAdapter {
    /// Underlying CUDA stream handle (from vLLM context).
    pub stream_handle: u64,

    /// Crew identifier (vLLM process or thread ID).
    pub crew_id: [u8; 16],

    /// GPU device ordinal.
    pub device_ordinal: u32,

    /// Kernel submission manager reference.
    kernel_manager: KernelSubmissionManager,

    /// Statistics.
    pub stats: IntegrationStats,
}

impl VLlmStreamAdapter {
    /// Create a vLLM stream adapter.
    ///
    /// # Arguments
    ///
    /// * `stream_handle` - CUDA stream handle from vLLM
    /// * `crew_id` - Crew identifier
    /// * `device_ordinal` - GPU device
    pub fn new(stream_handle: u64, crew_id: [u8; 16], device_ordinal: u32) -> Self {
        VLlmStreamAdapter {
            stream_handle,
            crew_id,
            device_ordinal,
            kernel_manager: KernelSubmissionManager::new(),
            stats: IntegrationStats {
                total_kernel_submissions: 0,
                total_kernel_launches: 0,
                total_kernel_errors: 0,
            },
        }
    }

    /// Register a kernel from vLLM model.
    ///
    /// Called when vLLM loads a model to register all kernel functions.
    pub fn register_kernel(
        &mut self,
        model_id: [u8; 32],
        kernel_name: [u8; 64],
        function_handle: u64,
    ) -> Result<(), GpuError> {
        self.kernel_manager
            .register_kernel(model_id, kernel_name, function_handle, self.device_ordinal)
    }

    /// Submit a kernel from vLLM execution.
    ///
    /// Called for each kernel launch in the vLLM inference pipeline
    /// (attention, mlp, etc.).
    ///
    /// # Arguments
    ///
    /// * `kernel_name` - Name of the kernel to launch
    /// * `grid` - Grid dimensions
    /// * `block` - Block dimensions
    /// * `shared_mem` - Shared memory bytes
    /// * `model_id` - Model identifier
    /// * `priority` - Execution priority
    pub fn submit_kernel(
        &mut self,
        kernel_name: [u8; 64],
        grid: (u32, u32, u32),
        block: (u32, u32, u32),
        shared_mem: u32,
        model_id: [u8; 32],
        priority: u32,
    ) -> Result<SubmissionId, GpuError> {
        let mut config = KernelSubmissionConfig::new(
            kernel_name,
            grid,
            block,
            shared_mem,
            self.crew_id,
            model_id,
            priority,
            self.device_ordinal,
        );

        // Validate before submission
        config.validate()?;

        // Prepare submission
        let entry = self
            .kernel_manager
            .prepare_submission(&config, self.stream_handle, 0)?;

        self.stats.total_kernel_submissions += 1;

        // Return submission ID for tracking
        Ok(entry.submission_id)
    }

    /// Get vLLM adapter statistics.
    pub fn stats(&self) -> IntegrationStats {
        self.stats
    }
}

impl fmt::Display for VLlmStreamAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VLlmStreamAdapter(stream=0x{:x}, device={}, submissions={})",
            self.stream_handle, self.device_ordinal, self.stats.total_kernel_submissions
        )
    }
}

/// Framework stream adapter for TensorRT-LLM.
///
/// Wraps TensorRT-LLM's CudaStream and provides integration with GPU Manager.
/// TensorRT-LLM manages graphs and kernels; adapter intercepts stream operations.
///
/// **Integration Points:**
/// - TensorRT-LLM creates streams via `CudaStream` wrapper
/// - Adapter binds stream to GPU Manager
/// - Kernels execute through GPU Manager queue
/// - Context management integrated per crew
///
/// Reference: Engineering Plan § TensorRT-LLM Integration
#[derive(Debug)]
pub struct TensorRtStreamAdapter {
    /// Underlying CUDA stream handle.
    pub stream_handle: u64,

    /// CUDA context handle (TensorRT-LLM context).
    pub context_handle: u64,

    /// Crew identifier.
    pub crew_id: [u8; 16],

    /// GPU device ordinal.
    pub device_ordinal: u32,

    /// Kernel submission manager.
    kernel_manager: KernelSubmissionManager,

    /// Statistics.
    pub stats: IntegrationStats,
}

impl TensorRtStreamAdapter {
    /// Create a TensorRT-LLM stream adapter.
    ///
    /// # Arguments
    ///
    /// * `stream_handle` - CUDA stream handle
    /// * `context_handle` - CUDA context handle
    /// * `crew_id` - Crew identifier
    /// * `device_ordinal` - GPU device
    pub fn new(
        stream_handle: u64,
        context_handle: u64,
        crew_id: [u8; 16],
        device_ordinal: u32,
    ) -> Self {
        TensorRtStreamAdapter {
            stream_handle,
            context_handle,
            crew_id,
            device_ordinal,
            kernel_manager: KernelSubmissionManager::new(),
            stats: IntegrationStats {
                total_kernel_submissions: 0,
                total_kernel_launches: 0,
                total_kernel_errors: 0,
            },
        }
    }

    /// Register a TensorRT-LLM kernel function.
    pub fn register_kernel(
        &mut self,
        model_id: [u8; 32],
        kernel_name: [u8; 64],
        function_handle: u64,
    ) -> Result<(), GpuError> {
        self.kernel_manager
            .register_kernel(model_id, kernel_name, function_handle, self.device_ordinal)
    }

    /// Submit a kernel from TensorRT-LLM execution.
    ///
    /// Similar to vLLM but also tracks context binding.
    pub fn submit_kernel(
        &mut self,
        kernel_name: [u8; 64],
        grid: (u32, u32, u32),
        block: (u32, u32, u32),
        shared_mem: u32,
        model_id: [u8; 32],
        priority: u32,
    ) -> Result<SubmissionId, GpuError> {
        let mut config = KernelSubmissionConfig::new(
            kernel_name,
            grid,
            block,
            shared_mem,
            self.crew_id,
            model_id,
            priority,
            self.device_ordinal,
        );

        config.validate()?;

        let entry = self
            .kernel_manager
            .prepare_submission(&config, self.context_handle, 0)?;

        self.stats.total_kernel_submissions += 1;

        Ok(entry.submission_id)
    }

    /// Get TensorRT-LLM adapter statistics.
    pub fn stats(&self) -> IntegrationStats {
        self.stats
    }
}

impl fmt::Display for TensorRtStreamAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TensorRtStreamAdapter(stream=0x{:x}, context=0x{:x}, device={})",
            self.stream_handle, self.context_handle, self.device_ordinal
        )
    }
}

/// Integration statistics for framework adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegrationStats {
    /// Total kernel submissions from framework.
    pub total_kernel_submissions: u64,

    /// Total kernels successfully launched on GPU.
    pub total_kernel_launches: u64,

    /// Total kernel submission errors.
    pub total_kernel_errors: u64,
}

/// Framework integration coordinator.
///
/// Manages multiple framework adapters (vLLM, TensorRT-LLM, etc.)
/// and coordinates kernel submissions across different frameworks.
///
/// Reference: Engineering Plan § Framework Coordination
#[derive(Debug)]
pub struct FrameworkIntegrationCoordinator {
    /// Registered framework adapters (framework_id → adapter info).
    adapters: BTreeMap<FrameworkType, IntegrationAdapterInfo>,

    /// Statistics.
    pub stats: CoordinatorStats,
}

/// Information about a registered adapter.
#[derive(Clone, Copy, Debug)]
struct IntegrationAdapterInfo {
    framework_type: FrameworkType,
    device_ordinal: u32,
    active_crews: u32,
}

/// Coordinator statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoordinatorStats {
    /// Total registered adapters.
    pub total_adapters: u64,

    /// Total kernel submissions across all adapters.
    pub total_submissions: u64,

    /// Total submission errors.
    pub total_errors: u64,
}

impl FrameworkIntegrationCoordinator {
    /// Create a new framework integration coordinator.
    pub fn new() -> Self {
        FrameworkIntegrationCoordinator {
            adapters: BTreeMap::new(),
            stats: CoordinatorStats {
                total_adapters: 0,
                total_submissions: 0,
                total_errors: 0,
            },
        }
    }

    /// Register a framework adapter.
    ///
    /// # Arguments
    ///
    /// * `framework_type` - Type of framework (vLLM, TensorRT-LLM, etc.)
    /// * `device_ordinal` - GPU device for this adapter
    pub fn register_adapter(
        &mut self,
        framework_type: FrameworkType,
        device_ordinal: u32,
    ) -> Result<(), GpuError> {
        let info = IntegrationAdapterInfo {
            framework_type,
            device_ordinal,
            active_crews: 0,
        };

        self.adapters.insert(framework_type, info);
        self.stats.total_adapters += 1;

        Ok(())
    }

    /// Unregister a framework adapter.
    pub fn unregister_adapter(&mut self, framework_type: FrameworkType) -> Result<(), GpuError> {
        self.adapters
            .remove(&framework_type)
            .ok_or(GpuError::DriverError)?;

        Ok(())
    }

    /// Get adapter information.
    pub fn get_adapter_info(&self, framework_type: FrameworkType) -> Option<IntegrationAdapterInfo> {
        self.adapters.get(&framework_type).copied()
    }

    /// Record a kernel submission from any framework.
    pub fn record_submission(&mut self) {
        self.stats.total_submissions += 1;
    }

    /// Record a submission error.
    pub fn record_error(&mut self) {
        self.stats.total_errors += 1;
    }

    /// Get coordinator statistics.
    pub fn stats(&self) -> CoordinatorStats {
        self.stats
    }

    /// Get the number of registered adapters.
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }
}

impl Default for FrameworkIntegrationCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_vllm_adapter_creation() {
        let adapter = VLlmStreamAdapter::new(0x1234, [1u8; 16], 0);

        assert_eq!(adapter.stream_handle, 0x1234);
        assert_eq!(adapter.device_ordinal, 0);
        assert_eq!(adapter.stats.total_kernel_submissions, 0);
    }

    #[test]
    fn test_vllm_register_kernel() {
        let mut adapter = VLlmStreamAdapter::new(0x1234, [1u8; 16], 0);

        let result = adapter.register_kernel([1u8; 32], [2u8; 64], 0x5678);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vllm_submit_kernel() {
        let mut adapter = VLlmStreamAdapter::new(0x1234, [1u8; 16], 0);

        adapter.register_kernel([1u8; 32], [2u8; 64], 0x5678).unwrap();

        let result = adapter.submit_kernel([2u8; 64], (16, 1, 1), (256, 1, 1), 0, [1u8; 32], 5);
        assert!(result.is_ok());
        assert_eq!(adapter.stats.total_kernel_submissions, 1);
    }

    #[test]
    fn test_tensorrt_adapter_creation() {
        let adapter = TensorRtStreamAdapter::new(0x1234, 0x5678, [1u8; 16], 0);

        assert_eq!(adapter.stream_handle, 0x1234);
        assert_eq!(adapter.context_handle, 0x5678);
        assert_eq!(adapter.device_ordinal, 0);
    }

    #[test]
    fn test_tensorrt_register_kernel() {
        let mut adapter = TensorRtStreamAdapter::new(0x1234, 0x5678, [1u8; 16], 0);

        let result = adapter.register_kernel([1u8; 32], [2u8; 64], 0x9abc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tensorrt_submit_kernel() {
        let mut adapter = TensorRtStreamAdapter::new(0x1234, 0x5678, [1u8; 16], 0);

        adapter.register_kernel([1u8; 32], [2u8; 64], 0x9abc).unwrap();

        let result = adapter.submit_kernel([2u8; 64], (16, 1, 1), (256, 1, 1), 0, [1u8; 32], 5);
        assert!(result.is_ok());
        assert_eq!(adapter.stats.total_kernel_submissions, 1);
    }

    #[test]
    fn test_framework_type_display() {
        assert_eq!(format!("{}", FrameworkType::VLlm), "vLLM");
        assert_eq!(format!("{}", FrameworkType::TensorRtLlm), "TensorRT-LLM");
        assert_eq!(format!("{}", FrameworkType::Custom), "Custom");
    }

    #[test]
    fn test_coordinator_creation() {
        let coordinator = FrameworkIntegrationCoordinator::new();

        assert_eq!(coordinator.adapter_count(), 0);
        assert_eq!(coordinator.stats.total_adapters, 0);
    }

    #[test]
    fn test_coordinator_register_adapter() {
        let mut coordinator = FrameworkIntegrationCoordinator::new();

        let result = coordinator.register_adapter(FrameworkType::VLlm, 0);
        assert!(result.is_ok());
        assert_eq!(coordinator.adapter_count(), 1);
        assert_eq!(coordinator.stats.total_adapters, 1);
    }

    #[test]
    fn test_coordinator_get_adapter_info() {
        let mut coordinator = FrameworkIntegrationCoordinator::new();

        coordinator.register_adapter(FrameworkType::VLlm, 0).unwrap();

        let info = coordinator.get_adapter_info(FrameworkType::VLlm);
        assert!(info.is_some());
        assert_eq!(info.unwrap().framework_type, FrameworkType::VLlm);
    }

    #[test]
    fn test_coordinator_record_submission() {
        let mut coordinator = FrameworkIntegrationCoordinator::new();

        assert_eq!(coordinator.stats.total_submissions, 0);
        coordinator.record_submission();
        assert_eq!(coordinator.stats.total_submissions, 1);
    }

    #[test]
    fn test_coordinator_record_error() {
        let mut coordinator = FrameworkIntegrationCoordinator::new();

        assert_eq!(coordinator.stats.total_errors, 0);
        coordinator.record_error();
        assert_eq!(coordinator.stats.total_errors, 1);
    }

    #[test]
    fn test_vllm_adapter_display() {
        let adapter = VLlmStreamAdapter::new(0x1234, [1u8; 16], 0);
        let display = format!("{}", adapter);

        assert!(display.contains("VLlmStreamAdapter"));
        assert!(display.contains("0x1234"));
    }

    #[test]
    fn test_tensorrt_adapter_display() {
        let adapter = TensorRtStreamAdapter::new(0x1234, 0x5678, [1u8; 16], 0);
        let display = format!("{}", adapter);

        assert!(display.contains("TensorRtStreamAdapter"));
        assert!(display.contains("0x1234"));
    }
}

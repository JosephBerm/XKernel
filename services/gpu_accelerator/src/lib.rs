// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU Manager Service — Week 06 Phase 0 Completion
//!
//! This crate implements the GPU/Accelerator Manager for the Cognitive Substrate OS,
//! enabling safe, efficient management of GPU resources (NVIDIA/AMD) for distributed
//! LLM inference and training workloads.
//!
//! ## Two-Phase GPU Strategy
//!
//! Per Engineering Plan (Addendum v2.5.1):
//! - **Phase A (v1.0)**: L1 kernel service using CUDA Driver API / ROCm HIP
//!   (leverages existing, battle-tested driver stacks)
//! - **Phase B (v2.0, post-GA)**: Native GPU driver with direct MMIO
//!   (long-term ambition; lower priority for MVP)
//!
//! Reference architectures: LithOS (SOSP 2025, TPC-level scheduling),
//! PhoenixOS (SOSP 2025, checkpoint/restore primitives).
//!
//! ## Architecture
//!
//! ```
//! Cognitive Scheduler
//!     ↓ (SchedulerDirective)
//! GPU Manager (this crate)
//!     ├─ Device Management (device.rs, device_discovery.rs)
//!     ├─ TPC Scheduling (scheduler.rs)
//!     ├─ VRAM Isolation (vram.rs, memory_interface.rs)
//!     ├─ Kernel Atomization (kernel_launch.rs)
//!     ├─ Command Submission (command_submission.rs)
//!     ├─ Checkpoint/Restore (checkpoint.rs)
//!     ├─ Driver Abstraction (cuda_abstraction.rs, rocm_abstraction.rs, driver_abstraction.rs)
//!     ├─ Event Handling (event_handling.rs)
//!     ├─ GPU Manager Service (gpu_manager.rs) [Week 04]
//!     ├─ Model Registry (model_registry.rs) [Week 04]
//!     ├─ VRAM Manager (vram_manager.rs) [Week 04]
//!     ├─ Model Loading (model_loading.rs) [Week 04]
//!     ├─ Model Unloading (model_unloading.rs) [Week 04]
//!     ├─ Command Queue (command_queue.rs) [Week 05]
//!     ├─ Kernel Submission (kernel_submission.rs) [Week 05]
//!     ├─ Async Execution (async_execution.rs) [Week 05]
//!     ├─ Framework Integration (inference_integration.rs) [Week 05]
//!     ├─ Completion Notification (completion_notification.rs) [Week 05]
//!     ├─ GPU Error Handling (gpu_error_handling.rs) [Week 05]
//!     ├─ Telemetry (telemetry_hooks.rs)
//!     ├─ GPU Integration Tests (gpu_integration_tests.rs) [Week 06]
//!     ├─ GPU Performance Profiling (gpu_performance_profiling.rs) [Week 06]
//!     ├─ Scheduler Feedback (scheduler_feedback.rs) [Week 06]
//!     ├─ GPU Error Recovery (gpu_error_recovery.rs) [Week 06]
//!     └─ Phase 0 Completion Report (phase0_completion_report.rs) [Week 06]
//!     ↓ (DriverInterface)
//! GPU Hardware (CUDA Driver / ROCm HIP)
//!     ↓
//! NVIDIA H100/H200/B200 or AMD MI300X
//! ```
//!
//! ## Module Organization
//!
//! **Week 01 (Core)**
//! - [`ids`]: Strongly-typed resource identifiers
//! - [`device`]: GPU device abstraction and capabilities
//! - [`scheduler`]: TPC-level scheduling directives
//! - [`vram`]: VRAM region management and isolation
//! - [`kernel_launch`]: Kernel atomization and launch queue
//! - [`checkpoint`]: Checkpoint/restore state management
//! - [`driver_abstraction`]: CUDA/ROCm driver interface adapter
//! - [`error`]: GPU Manager error types
//!
//! **Week 02 (Control Flow)**
//! - [`state_machine`]: GPU Manager state machine and lifecycle
//! - [`scheduler_interface`]: Cognitive Scheduler ↔ GPU Manager protocol
//! - [`data_flow`]: End-to-end inference request tracking
//! - [`telemetry_hooks`]: Performance monitoring and alerts
//!
//! **Week 03 (Driver & Command Submission)**
//! - [`cuda_abstraction`]: CUDA Driver API abstraction (CudaContext, CudaStream, etc.)
//! - [`rocm_abstraction`]: ROCm HIP abstraction (HipContext, HipStream, etc.)
//! - [`command_submission`]: GPU command queue with priority scheduling
//! - [`event_handling`]: GPU event dispatch and completion notification
//! - [`memory_interface`]: GPU memory allocation and transfer operations
//! - [`device_discovery`]: Device enumeration and health monitoring
//!
//! **Week 04 (Model Management & Registry)**
//! - [`gpu_manager`]: L1 kernel service initialization and coordination
//! - [`model_registry`]: Central tracking of loaded models (model_id, vram_footprint, bound_ct_list)
//! - [`vram_manager`]: Single-model VRAM allocation and lifecycle
//! - [`model_loading`]: Model load pipeline (file → CUDA/ROCm allocation → registry → ready)
//! - [`model_unloading`]: Model unload pipeline (registry removal → cuMemFree/hipMemFree → coherency)
//!
//! **Week 05 (Command Submission & Async Execution)**
//! - [`command_queue`]: CUDA/HIP stream-based command queue for kernel submissions
//! - [`kernel_submission`]: Kernel submission path (framework → GPU Manager → cuLaunchKernel)
//! - [`async_execution`]: Async execution model with event-based completion (cuEventRecord/hipEventRecord)
//! - [`inference_integration`]: vLLM & TensorRT-LLM framework integration adapters
//! - [`completion_notification`]: GPU event-based completion notification & CT resumption
//! - [`gpu_error_handling`]: Error detection, reporting, and recovery (malformed commands, timeouts, faults)
//!
//! **Week 06 (Phase 0 Completion & Validation)**
//! - [`gpu_integration_tests`]: End-to-end integration test suite
//! - [`gpu_performance_profiling`]: Performance baseline measurement and validation
//! - [`scheduler_feedback`]: GPU → Scheduler feedback loop (utilization reports, thermal state)
//! - [`gpu_error_recovery`]: Robust error recovery, memory leak detection, fault isolation
//! - [`phase0_completion_report`]: Phase 0 architecture validation and Phase 1 readiness

#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations)]

extern crate alloc;

// Core modules (Week 01)
pub mod checkpoint;
pub mod device;
pub mod driver_abstraction;
pub mod error;
pub mod ids;
pub mod kernel_launch;
pub mod scheduler;
pub mod vram;

// Week 02 modules
pub mod data_flow;
pub mod scheduler_interface;
pub mod state_machine;
pub mod telemetry_hooks;

// Week 03 modules (Driver abstraction & Command submission)
pub mod command_submission;
pub mod cuda_abstraction;
pub mod device_discovery;
pub mod event_handling;
pub mod memory_interface;
pub mod rocm_abstraction;

// Week 04 modules (Model Management & Registry)
pub mod gpu_manager;
pub mod model_loading;
pub mod model_registry;
pub mod model_unloading;
pub mod vram_manager;

// Week 05 modules (Command Submission & Async Execution)
pub mod async_execution;
pub mod command_queue;
pub mod completion_notification;
pub mod gpu_error_handling;
pub mod inference_integration;
pub mod kernel_submission;

// Week 06 modules (Phase 0 Completion & Validation)
pub mod gpu_error_recovery;
pub mod gpu_integration_tests;
pub mod gpu_performance_profiling;
pub mod phase0_completion_report;
pub mod scheduler_feedback;

// L1 scaffold modules (scheduling, profiling, vram management)
pub mod profiling;
pub mod scheduling;
pub mod vram_management;

// Re-exports for convenience
pub use async_execution::{AsyncExecutionManager, AsyncExecutionStats, EventCompletion, EventStatus, GpuEventHandle};
pub use checkpoint::{CheckpointStrategy, GpuCheckpoint, RestoreConfig};
pub use command_queue::{CommandQueue, CommandQueueEntry, CommandQueueStats, SubmissionId, SubmissionStatus};
pub use command_submission::{CommandHandle, CommandStatus, GpuCommand};
pub use completion_notification::{CompletionNotification, CompletionNotificationManager, CompletionStatus, CtResumptionRequest};
pub use cuda_abstraction::{CudaApi, CudaContext, CudaEvent, CudaKernelLaunch, CudaMemory, CudaMemoryType, CudaStream, DeviceProperties as CudaDeviceProperties};
pub use device::{GpuDevice, GpuDeviceType};
pub use error::GpuError;
pub use gpu_error_handling::{GpuErrorHandler, GpuErrorReport, GpuFaultCode, RecoveryAction, RecoveryStrategy};
pub use gpu_error_recovery::{DeviceRecoveryManager, DeviceRecoveryState, ErrorRecoveryCoordinator, MemoryLeakReport};
pub use gpu_manager::{GpuManager, GpuManagerConfig, GpuManagerState};
pub use gpu_performance_profiling::{
    ExecutionOverheadProfile, GpuUtilizationMetrics, ModelLoadPerformance, PerformanceProfilingReport,
    SubmissionLatencyProfile, ThroughputMetrics,
};
pub use ids::{GpuDeviceID, KernelLaunchID, TpcID, VramRegionID};
pub use inference_integration::{FrameworkIntegrationCoordinator, FrameworkType, IntegrationStats, TensorRtStreamAdapter, VLlmStreamAdapter};
pub use kernel_launch::{AtomizationConfig, KernelDimensions, KernelLaunch, LaunchQueue};
pub use kernel_submission::{KernelFunctionRegistry, KernelSubmissionConfig, KernelSubmissionManager, SubmissionResult};
pub use model_loading::{ModelLoadRequest, ModelLoadStatus, ModelLoader};
pub use model_registry::{ModelEntry, ModelLoadState, ModelRegistry};
pub use model_unloading::{ModelUnloadRequest, ModelUnloadStatus, ModelUnloader};
pub use phase0_completion_report::{ArchitectureChecklistItem, Phase0CompletionReport, Phase0Summary, PerformanceBaselines};
pub use rocm_abstraction::{HipContext, HipEvent, HipMemory, HipStream};
pub use scheduler_feedback::{FeedbackGenerator, GpuUtilizationReport, SchedulerAction, SchedulerFeedbackMessage, ThermalState};
pub use vram::{VramIsolationMode, VramRegion};
pub use vram_manager::{VramAllocation, VramAllocationType, VramManager};

// Version info
/// GPU Manager version (Week 06 — Phase 0 Completion)
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GPU Manager phase strategy (Phase A: CUDA Driver API / ROCm HIP)
pub const PHASE: &str = "A (v1.0) - CUDA Driver API / ROCm HIP";

/// Phase 0 Status (Completion + Validation)
pub const PHASE_0_STATUS: &str = "Complete - Ready for Phase 1 Planning";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_metadata() {
        assert!(!CRATE_VERSION.is_empty());
        assert!(PHASE.contains("Phase A"));
        assert!(PHASE_0_STATUS.contains("Complete"));
    }

    #[test]
    fn test_gpu_manager_integration() {
        let config = GpuManagerConfig::default();
        let mut manager = GpuManager::new(config);
        assert!(manager.initialize().is_ok());
        assert!(manager.is_ready());
    }

    #[test]
    fn test_model_registry_integration() {
        let registry = ModelRegistry::new();
        assert_eq!(registry.model_count(), 0);
    }

    #[test]
    fn test_vram_manager_integration() {
        let mut vram_mgr = VramManager::new();
        assert!(vram_mgr
            .initialize(0, 16 * 1024 * 1024 * 1024)
            .is_ok());
        assert!(vram_mgr.is_initialized());
    }

    #[test]
    fn test_model_loader_integration() {
        let loader = ModelLoader::new();
        let mut config = GpuManagerConfig::default();
        config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024;
        let mut gpu_manager = GpuManager::new(config);
        let _ = gpu_manager.initialize();

        let request = ModelLoadRequest::new([1u8; 32], [0u8; 256], 1024 * 1024 * 1024);
        let result = loader.load_model(&mut gpu_manager, request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_model_unloader_integration() {
        let loader = ModelLoader::new();
        let unloader = ModelUnloader::new();
        let mut config = GpuManagerConfig::default();
        config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024;
        let mut gpu_manager = GpuManager::new(config);
        let _ = gpu_manager.initialize();

        let model_id = [1u8; 32];
        let load_request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024);
        let _ = loader.load_model(&mut gpu_manager, load_request);

        let unload_request = ModelUnloadRequest::new(model_id);
        let result = unloader.unload_model(&mut gpu_manager, unload_request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_command_queue_integration() {
        let mut queue = CommandQueue::new([1u8; 16], 0, 32);
        assert_eq!(queue.depth(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_async_execution_integration() {
        let mut async_exec = AsyncExecutionManager::new();
        assert_eq!(async_exec.pending_event_count(), 0);
    }

    #[test]
    fn test_kernel_submission_manager_integration() {
        let manager = KernelSubmissionManager::new();
        assert_eq!(manager.stats().total_submitted, 0);
    }

    #[test]
    fn test_completion_notification_integration() {
        let manager = CompletionNotificationManager::new();
        assert_eq!(manager.pending_count(), 0);
    }

    #[test]
    fn test_gpu_error_handler_integration() {
        let handler = GpuErrorHandler::new();
        assert_eq!(handler.stats().total_errors, 0);
    }

    #[test]
    fn test_framework_integration_coordinator() {
        let coordinator = FrameworkIntegrationCoordinator::new();
        assert_eq!(coordinator.adapter_count(), 0);
    }

    #[test]
    fn test_phase0_completion_report() {
        let report = Phase0CompletionReport::new();
        assert!(!report.ready_for_ga);
        assert!(!report.ready_for_phase1);
    }

    #[test]
    fn test_error_recovery_coordinator() {
        let coordinator = ErrorRecoveryCoordinator::new();
        assert_eq!(coordinator.healthy_device_count(), 0);
    }

    #[test]
    fn test_scheduler_feedback_generator() {
        let gen = FeedbackGenerator::new(100_000_000);
        assert!(gen.should_report(200_000_000));
    }

    #[test]
    fn test_gpu_integration_tests_pass() {
        assert!(gpu_integration_tests::run_all_integration_tests().is_ok());
    }
}

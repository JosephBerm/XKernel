// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU Manager — L1 kernel service for GPU resource management.
//!
//! Implements the Week 4 GPU Manager skeleton, establishing service-level initialization,
//! device discovery, and integration with CUDA Driver API / ROCm HIP.
//!
//! This module coordinates device enumeration, context management, and serves as the
//! entry point for all GPU resource requests from the Cognitive Scheduler.
//!
//! Reference: Engineering Plan § GPU Manager Architecture, Week 4 Deliverables

use crate::cuda_abstraction::{CudaApi, CudaContext};
use crate::device::{DriverApi, GpuDevice, GpuDeviceType};
use crate::error::GpuError;
use crate::ids::GpuDeviceID;
use crate::model_registry::ModelRegistry;
use crate::vram_manager::VramManager;
use alloc::vec::Vec;
use core::fmt;

/// GPU Manager service state lifecycle.
///
/// Tracks initialization progress and operational readiness of the GPU Manager.
///
/// Reference: Engineering Plan § Service Lifecycle
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuManagerState {
    /// Service not yet initialized; no resources allocated.
    Uninitialized,

    /// Device discovery in progress.
    Discovering,

    /// Contexts being initialized via CUDA Driver API / ROCm HIP.
    InitializingContexts,

    /// Service fully operational and ready to accept requests.
    Ready,

    /// Service encountered recoverable error; attempting recovery.
    Recovering,

    /// Service encountered fatal error; no new operations allowed.
    Faulted,

    /// Service is shutting down; cleanup in progress.
    Shutdown,
}

impl fmt::Display for GpuManagerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuManagerState::Uninitialized => write!(f, "Uninitialized"),
            GpuManagerState::Discovering => write!(f, "Discovering"),
            GpuManagerState::InitializingContexts => write!(f, "InitializingContexts"),
            GpuManagerState::Ready => write!(f, "Ready"),
            GpuManagerState::Recovering => write!(f, "Recovering"),
            GpuManagerState::Faulted => write!(f, "Faulted"),
            GpuManagerState::Shutdown => write!(f, "Shutdown"),
        }
    }
}

/// GPU Manager configuration parameters.
///
/// Defines the initialization parameters for the GPU Manager,
/// including device selection, VRAM partitioning, and isolation mode.
///
/// Reference: Engineering Plan § GPU Manager Configuration
#[derive(Clone, Debug)]
pub struct GpuManagerConfig {
    /// Preferred GPU device type (NVIDIA H100/H200/B200 or AMD MI300X).
    pub preferred_device_type: Option<GpuDeviceType>,

    /// Maximum VRAM to reserve for single-model scenario (Phase 0).
    /// Typical: 16 GB to 32 GB for model weights + intermediate tensors.
    pub single_model_vram_partition_bytes: u64,

    /// Enable GPU Manager telemetry and event logging.
    pub enable_telemetry: bool,

    /// Panic on allocation errors (strict mode) or recover gracefully.
    pub strict_mode: bool,
}

impl GpuManagerConfig {
    /// Create a default configuration.
    ///
    /// Reserves 16 GB for single-model, enables telemetry, recoverable mode.
    pub fn default() -> Self {
        GpuManagerConfig {
            preferred_device_type: None,
            single_model_vram_partition_bytes: 16 * 1024 * 1024 * 1024, // 16 GB
            enable_telemetry: true,
            strict_mode: false,
        }
    }
}

/// GPU Manager — L1 kernel service.
///
/// Central coordination point for all GPU resource management. Maintains:
/// - Device inventory and CUDA/ROCm contexts
/// - Model registry and lifecycle
/// - VRAM allocation and isolation
///
/// Reference: Engineering Plan § GPU Manager, Section 4.2.1
pub struct GpuManager {
    /// Current service state.
    state: GpuManagerState,

    /// Configuration parameters.
    config: GpuManagerConfig,

    /// Enumerated GPU devices and their CUDA/ROCm contexts.
    devices: Vec<GpuDevice>,

    /// Primary GPU context (for Phase 0 single-model scenario).
    primary_context: Option<CudaContext>,

    /// Model registry (tracks loaded models and VRAM footprints).
    model_registry: ModelRegistry,

    /// VRAM manager (allocates and tracks model VRAM).
    vram_manager: VramManager,

    /// Initialization error (if state == Faulted).
    last_error: Option<GpuError>,
}

impl GpuManager {
    /// Create a new GPU Manager instance.
    ///
    /// # Arguments
    ///
    /// * `config` - GPU Manager configuration
    ///
    /// # Returns
    ///
    /// A new GpuManager in Uninitialized state. Call `initialize()` to proceed.
    ///
    /// Reference: Engineering Plan § GPU Manager Initialization
    pub fn new(config: GpuManagerConfig) -> Self {
        GpuManager {
            state: GpuManagerState::Uninitialized,
            config,
            devices: Vec::new(),
            primary_context: None,
            model_registry: ModelRegistry::new(),
            vram_manager: VramManager::new(),
            last_error: None,
        }
    }

    /// Get the current GPU Manager state.
    pub fn state(&self) -> GpuManagerState {
        self.state
    }

    /// Check if GPU Manager is operational (Ready state).
    pub fn is_ready(&self) -> bool {
        self.state == GpuManagerState::Ready
    }

    /// Initialize the GPU Manager.
    ///
    /// This performs:
    /// 1. Device discovery via CUDA Driver API / ROCm HIP
    /// 2. Context creation for primary GPU
    /// 3. VRAM partition initialization
    /// 4. Model registry setup
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err(GpuError)` on failure.
    ///
    /// Reference: Engineering Plan § GPU Manager Initialization Pipeline
    pub fn initialize(&mut self) -> Result<(), GpuError> {
        if self.state != GpuManagerState::Uninitialized {
            return Err(GpuError::DriverError); // Cannot reinitialize
        }

        self.state = GpuManagerState::Discovering;

        // Step 1: Device discovery
        self.discover_devices()?;

        if self.devices.is_empty() {
            self.state = GpuManagerState::Faulted;
            self.last_error = Some(GpuError::DeviceNotFound);
            return Err(GpuError::DeviceNotFound);
        }

        // Step 2: Initialize contexts
        self.state = GpuManagerState::InitializingContexts;
        self.initialize_contexts()?;

        // Step 3: Initialize VRAM manager with partition
        self.vram_manager.initialize(
            self.primary_context.ok_or(GpuError::DriverError)?.device_ordinal,
            self.config.single_model_vram_partition_bytes,
        )?;

        // Step 4: Mark as ready
        self.state = GpuManagerState::Ready;
        Ok(())
    }

    /// Discover available GPU devices.
    ///
    /// Enumerates GPUs via CUDA Driver API / ROCm HIP and populates
    /// the device inventory.
    fn discover_devices(&mut self) -> Result<(), GpuError> {
        // Phase A: Use CUDA Driver API / ROCm HIP for device discovery
        // (In a real implementation, this would call cuDeviceGetCount / hipGetDeviceCount)

        // For now, create a mock device for testing
        // Real implementation will enumerate actual GPUs
        let mock_device = GpuDevice::new(
            GpuDeviceID::from_bytes([0u8; 16]),
            GpuDeviceType::NvidiaH100,
            DriverApi::CudaDriverApi,
        );

        self.devices.push(mock_device);
        Ok(())
    }

    /// Initialize CUDA Driver API / ROCm HIP contexts for discovered devices.
    fn initialize_contexts(&mut self) -> Result<(), GpuError> {
        if self.devices.is_empty() {
            return Err(GpuError::DeviceNotFound);
        }

        // Phase A: Use CUDA Driver API / ROCm HIP context creation
        // (In a real implementation, this would call cuCtxCreate / hipCtxCreate)

        // For now, create a mock context for the primary device
        let ctx = CudaContext {
            device_ordinal: 0,
            context_handle: 0x1000, // Mock handle
            flags: 0,
        };

        self.primary_context = Some(ctx);
        Ok(())
    }

    /// Get reference to the model registry.
    pub fn model_registry(&self) -> &ModelRegistry {
        &self.model_registry
    }

    /// Get mutable reference to the model registry.
    pub fn model_registry_mut(&mut self) -> &mut ModelRegistry {
        &mut self.model_registry
    }

    /// Get reference to the VRAM manager.
    pub fn vram_manager(&self) -> &VramManager {
        &self.vram_manager
    }

    /// Get mutable reference to the VRAM manager.
    pub fn vram_manager_mut(&mut self) -> &mut VramManager {
        &mut self.vram_manager
    }

    /// Get the primary GPU context.
    pub fn primary_context(&self) -> Option<CudaContext> {
        self.primary_context
    }

    /// Get the enumerated devices.
    pub fn devices(&self) -> &[GpuDevice] {
        &self.devices
    }

    /// Shutdown the GPU Manager and release all resources.
    ///
    /// This must be called before dropping the GPU Manager to ensure
    /// proper cleanup of CUDA/ROCm contexts and VRAM allocations.
    ///
    /// Reference: Engineering Plan § GPU Manager Shutdown
    pub fn shutdown(&mut self) -> Result<(), GpuError> {
        self.state = GpuManagerState::Shutdown;

        // Unload all models
        self.model_registry.clear();

        // Release VRAM manager resources
        self.vram_manager.shutdown()?;

        // Release primary context (in real implementation, cuCtxDestroy / hipCtxDestroy)
        self.primary_context = None;

        Ok(())
    }
}

impl fmt::Debug for GpuManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpuManager")
            .field("state", &self.state)
            .field("devices_count", &self.devices.len())
            .field("primary_context", &self.primary_context)
            .field("has_model_registry", &!self.model_registry.is_empty())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_gpu_manager_creation() {
        let config = GpuManagerConfig::default();
        let manager = GpuManager::new(config);
        assert_eq!(manager.state(), GpuManagerState::Uninitialized);
    }

    #[test]
    fn test_gpu_manager_initialization() {
        let config = GpuManagerConfig::default();
        let mut manager = GpuManager::new(config);
        let result = manager.initialize();
        assert!(result.is_ok());
        assert_eq!(manager.state(), GpuManagerState::Ready);
    }

    #[test]
    fn test_gpu_manager_is_ready() {
        let config = GpuManagerConfig::default();
        let mut manager = GpuManager::new(config);
        assert!(!manager.is_ready());
        let _ = manager.initialize();
        assert!(manager.is_ready());
    }

    #[test]
    fn test_gpu_manager_device_discovery() {
        let config = GpuManagerConfig::default();
        let mut manager = GpuManager::new(config);
        let result = manager.initialize();
        assert!(result.is_ok());
        assert!(!manager.devices().is_empty());
    }

    #[test]
    fn test_gpu_manager_context_creation() {
        let config = GpuManagerConfig::default();
        let mut manager = GpuManager::new(config);
        let _ = manager.initialize();
        assert!(manager.primary_context().is_some());
    }

    #[test]
    fn test_gpu_manager_shutdown() {
        let config = GpuManagerConfig::default();
        let mut manager = GpuManager::new(config);
        let _ = manager.initialize();
        let result = manager.shutdown();
        assert!(result.is_ok());
        assert_eq!(manager.state(), GpuManagerState::Shutdown);
    }

    #[test]
    fn test_gpu_manager_state_display() {
        assert_eq!(format!("{}", GpuManagerState::Uninitialized), "Uninitialized");
        assert_eq!(format!("{}", GpuManagerState::Ready), "Ready");
        assert_eq!(format!("{}", GpuManagerState::Faulted), "Faulted");
    }
}

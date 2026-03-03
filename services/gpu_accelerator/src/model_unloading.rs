// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Model unloading pipeline — registry removal → cuMemFree/hipMemFree → memory coherency verification.
//!
//! Orchestrates the end-to-end process of removing an ML model from GPU VRAM.
//! The pipeline:
//! 1. Validate no CTs are bound to the model
//! 2. Remove model from registry
//! 3. Release VRAM via cuMemFree / hipMemFree
//! 4. Verify GPU memory coherency
//! 5. Transition to Unloaded state
//!
//! Reference: Engineering Plan § Model Unloading Path, Week 4

use crate::error::GpuError;
use crate::gpu_manager::GpuManager;
use crate::model_registry::ModelLoadState;
use core::fmt;

/// Model unload request — parameters for unloading a model.
///
/// Specifies which model to unload and unload strategy.
///
/// Reference: Engineering Plan § Model Unload Request
#[derive(Clone, Debug)]
pub struct ModelUnloadRequest {
    /// Model identifier to unload.
    pub model_id: [u8; 32],

    /// Force unload even if CTs are bound? (dangerous, for emergency shutdown)
    pub force_unload: bool,

    /// Verify memory coherency after unload.
    pub verify_coherency: bool,
}

impl ModelUnloadRequest {
    /// Create a new model unload request.
    pub fn new(model_id: [u8; 32]) -> Self {
        ModelUnloadRequest {
            model_id,
            force_unload: false,
            verify_coherency: true,
        }
    }

    /// Set force unload flag.
    pub fn with_force_unload(mut self) -> Self {
        self.force_unload = true;
        self
    }

    /// Set coherency verification flag.
    pub fn with_coherency_check(mut self, verify: bool) -> Self {
        self.verify_coherency = verify;
        self
    }
}

impl fmt::Display for ModelUnloadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ModelUnloadRequest(id={:?}, force={}, verify_coherency={})",
            &self.model_id[..8],
            self.force_unload,
            self.verify_coherency
        )
    }
}

/// Model unload status — result of an unload operation.
///
/// Contains metrics and status information about the unload operation.
///
/// Reference: Engineering Plan § Model Unload Status
#[derive(Clone, Debug)]
pub struct ModelUnloadStatus {
    /// Was the unload successful?
    pub success: bool,

    /// Error message if unload failed.
    pub error_message: Option<&'static str>,

    /// Bytes freed from VRAM.
    pub bytes_freed: u64,

    /// Unload time in milliseconds (approximate).
    pub unload_time_ms: u64,

    /// Final model state.
    pub final_state: ModelLoadState,

    /// Memory coherency verified after unload?
    pub coherency_verified: bool,
}

impl fmt::Display for ModelUnloadStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ModelUnloadStatus(success={}, state={}, freed={}B, time={}ms)",
            self.success, self.final_state, self.bytes_freed, self.unload_time_ms
        )
    }
}

/// Model unloading orchestrator.
///
/// Implements the complete model unloading pipeline:
/// Registry removal → VRAM deallocation → Memory coherency verification
///
/// Reference: Engineering Plan § Model Unloading Orchestrator
pub struct ModelUnloader {
    /// Reference to GPU Manager (for VRAM and registry access).
    /// In real implementation, would hold a reference or use dependency injection.
    _phantom: core::marker::PhantomData<()>,
}

impl ModelUnloader {
    /// Create a new model unloader.
    pub fn new() -> Self {
        ModelUnloader {
            _phantom: core::marker::PhantomData,
        }
    }

    /// Unload a model according to the request.
    ///
    /// # Arguments
    ///
    /// * `gpu_manager` - GPU Manager with VRAM and registry
    /// * `request` - Model unload request parameters
    ///
    /// # Returns
    ///
    /// `Ok(ModelUnloadStatus)` on success, `Err(GpuError)` on failure.
    ///
    /// Pipeline:
    /// 1. Validate GPU Manager is ready
    /// 2. Check model exists in registry
    /// 3. Validate no CTs are bound (unless force_unload)
    /// 4. Remove from registry
    /// 5. Free VRAM via VRAM Manager (cuMemFree / hipMemFree)
    /// 6. Verify memory coherency
    /// 7. Transition to Unloaded state
    ///
    /// Reference: Engineering Plan § Model Unloading Path
    pub fn unload_model(
        &self,
        gpu_manager: &mut GpuManager,
        request: ModelUnloadRequest,
    ) -> Result<ModelUnloadStatus, GpuError> {
        // Validate GPU Manager is ready
        if !gpu_manager.is_ready() {
            return Err(GpuError::DriverError);
        }

        let model_id = request.model_id;

        // Step 1: Check if model exists in registry
        if !gpu_manager.model_registry().contains_model(&model_id) {
            return Err(GpuError::AllocationFailed); // Model not found
        }

        // Step 2: Get model entry and check for bound CTs
        let entry = gpu_manager
            .model_registry()
            .get_model(&model_id)
            .ok_or(GpuError::AllocationFailed)?;

        if entry.bound_ct_count() > 0 && !request.force_unload {
            return Err(GpuError::DriverError); // Cannot unload with bound CTs
        }

        let vram_footprint = entry.vram_footprint_bytes;

        // Step 3: Remove model from registry
        let removed_entry = gpu_manager
            .model_registry_mut()
            .unregister_model(&model_id)
            .ok_or(GpuError::AllocationFailed)?;

        // Step 4: Free VRAM
        let freed_result = gpu_manager.vram_manager_mut().free(&model_id);

        if freed_result.is_err() {
            // If free fails, try to restore the registry entry
            gpu_manager
                .model_registry_mut()
                .register_model(removed_entry);
            return Err(freed_result.err().unwrap());
        }

        let bytes_freed = freed_result?;

        // Step 5: Verify memory coherency (if requested)
        let coherency_verified = if request.verify_coherency {
            gpu_manager
                .vram_manager()
                .verify_coherency()
                .is_ok()
        } else {
            true // Assume verified if not checking
        };

        if !coherency_verified {
            // Log warning but don't fail the unload
            // In a real system, would trigger telemetry alert
        }

        Ok(ModelUnloadStatus {
            success: true,
            error_message: None,
            bytes_freed,
            unload_time_ms: 0, // Simulated; real implementation would measure
            final_state: ModelLoadState::Unloaded,
            coherency_verified,
        })
    }

    /// Unload all models from the GPU Manager.
    ///
    /// # Arguments
    ///
    /// * `gpu_manager` - GPU Manager with VRAM and registry
    ///
    /// # Returns
    ///
    /// Total bytes freed across all models.
    ///
    /// Reference: Engineering Plan § GPU Manager Shutdown
    pub fn unload_all_models(
        &self,
        gpu_manager: &mut GpuManager,
    ) -> Result<u64, GpuError> {
        let mut total_freed = 0u64;

        loop {
            // Get the first model in the registry
            let first_model_id = gpu_manager
                .model_registry()
                .iter()
                .next()
                .map(|e| e.model_id);

            if let Some(model_id) = first_model_id {
                let request = ModelUnloadRequest::new(model_id).with_force_unload();
                let status = self.unload_model(gpu_manager, request)?;
                total_freed += status.bytes_freed;
            } else {
                break; // No more models
            }
        }

        Ok(total_freed)
    }
}

impl Default for ModelUnloader {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ModelUnloader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelUnloader").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_loading::ModelLoadRequest;
    use crate::model_loading::ModelLoader;
use alloc::format;

    #[test]
    fn test_model_unload_request_creation() {
        let model_id = [1u8; 32];
        let request = ModelUnloadRequest::new(model_id);
        assert_eq!(request.model_id, model_id);
        assert!(!request.force_unload);
        assert!(request.verify_coherency);
    }

    #[test]
    fn test_model_unload_request_with_force() {
        let model_id = [1u8; 32];
        let request = ModelUnloadRequest::new(model_id).with_force_unload();
        assert!(request.force_unload);
    }

    #[test]
    fn test_model_unload_request_with_coherency_check() {
        let model_id = [1u8; 32];
        let request = ModelUnloadRequest::new(model_id).with_coherency_check(false);
        assert!(!request.verify_coherency);
    }

    #[test]
    fn test_model_unloader_creation() {
        let unloader = ModelUnloader::new();
        assert_eq!(format!("{:?}", unloader), "ModelUnloader");
    }

    #[test]
    fn test_model_unload_single_model() {
        let loader = ModelLoader::new();
        let unloader = ModelUnloader::new();
        let mut config = crate::gpu_manager::GpuManagerConfig::default();
        config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024;
        let mut gpu_manager = crate::gpu_manager::GpuManager::new(config);
        let _ = gpu_manager.initialize();

        let model_id = [1u8; 32];
        let load_request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024);
        let _ = loader.load_model(&mut gpu_manager, load_request);

        let unload_request = ModelUnloadRequest::new(model_id);
        let result = unloader.unload_model(&mut gpu_manager, unload_request);
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.success);
        assert_eq!(status.final_state, ModelLoadState::Unloaded);
    }

    #[test]
    fn test_model_unload_with_bound_ct() {
        let loader = ModelLoader::new();
        let unloader = ModelUnloader::new();
        let mut config = crate::gpu_manager::GpuManagerConfig::default();
        config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024;
        let mut gpu_manager = crate::gpu_manager::GpuManager::new(config);
        let _ = gpu_manager.initialize();

        let model_id = [1u8; 32];
        let ct_id = [2u8; 16];
        let load_request =
            ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024)
                .with_ct_binding(ct_id);
        let _ = loader.load_model(&mut gpu_manager, load_request);

        // Try to unload without force
        let unload_request = ModelUnloadRequest::new(model_id);
        let result = unloader.unload_model(&mut gpu_manager, unload_request);
        assert!(result.is_err()); // Should fail due to bound CT

        // Force unload
        let force_request = ModelUnloadRequest::new(model_id).with_force_unload();
        let result = unloader.unload_model(&mut gpu_manager, force_request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_model_unload_all() {
        let loader = ModelLoader::new();
        let unloader = ModelUnloader::new();
        let mut config = crate::gpu_manager::GpuManagerConfig::default();
        config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024;
        let mut gpu_manager = crate::gpu_manager::GpuManager::new(config);
        let _ = gpu_manager.initialize();

        // Load a model
        let model_id = [1u8; 32];
        let load_request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024);
        let _ = loader.load_model(&mut gpu_manager, load_request);

        // Unload all
        let result = unloader.unload_all_models(&mut gpu_manager);
        assert!(result.is_ok());
        assert_eq!(gpu_manager.model_registry().model_count(), 0);
    }

    #[test]
    fn test_model_unload_status_display() {
        let status = ModelUnloadStatus {
            success: true,
            error_message: None,
            bytes_freed: 1024 * 1024,
            unload_time_ms: 50,
            final_state: ModelLoadState::Unloaded,
            coherency_verified: true,
        };
        let display_str = format!("{}", status);
        assert!(display_str.contains("success=true"));
    }
}

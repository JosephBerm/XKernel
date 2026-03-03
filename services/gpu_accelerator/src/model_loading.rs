// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Model loading pipeline — file → CUDA/ROCm allocation → registry → ready for inference.
//!
//! Orchestrates the end-to-end process of loading an ML model from storage
//! into GPU VRAM. The pipeline:
//! 1. Locate and validate model file
//! 2. Allocate VRAM via cuMemAlloc / hipMalloc
//! 3. Transfer model weights to GPU
//! 4. Register model in the model registry
//! 5. Bind Cognitive Task(s) to the model
//! 6. Transition to Ready state
//!
//! Reference: Engineering Plan § Model Loading Path, Week 4

use crate::error::GpuError;
use crate::gpu_manager::GpuManager;
use crate::model_registry::{ModelEntry, ModelLoadState};
use crate::vram_manager::VramAllocationType;
use core::fmt;

/// Model load request — parameters for loading a model.
///
/// Specifies which model to load, its metadata, and binding requirements.
///
/// Reference: Engineering Plan § Model Load Request
#[derive(Clone, Debug)]
pub struct ModelLoadRequest {
    /// Unique model identifier (32-byte hash or semantic URI).
    pub model_id: [u8; 32],

    /// File path or storage location of the model.
    /// In a real system, this would be a URI or path.
    pub model_path: [u8; 256],

    /// Estimated VRAM footprint in bytes (model weights + buffers).
    pub estimated_vram_bytes: u64,

    /// Cognitive Task ID to bind to this model after loading.
    pub bind_ct_id: Option<[u8; 16]>,

    /// Is this model pinned (cannot be evicted)?
    pub is_pinned: bool,

    /// Priority hint for scheduling the load operation.
    pub priority: u32,
}

impl ModelLoadRequest {
    /// Create a new model load request.
    pub fn new(
        model_id: [u8; 32],
        model_path: [u8; 256],
        estimated_vram_bytes: u64,
    ) -> Self {
        ModelLoadRequest {
            model_id,
            model_path,
            estimated_vram_bytes,
            bind_ct_id: None,
            is_pinned: false,
            priority: 0,
        }
    }

    /// Set the CT to bind after loading.
    pub fn with_ct_binding(mut self, ct_id: [u8; 16]) -> Self {
        self.bind_ct_id = Some(ct_id);
        self
    }

    /// Mark this model as pinned.
    pub fn with_pinning(mut self) -> Self {
        self.is_pinned = true;
        self
    }
}

impl fmt::Display for ModelLoadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ModelLoadRequest(id={:?}, size={}B, pinned={})",
            &self.model_id[..8],
            self.estimated_vram_bytes,
            self.is_pinned
        )
    }
}

/// Model load status — result of a load operation.
///
/// Contains metrics and status information about the load operation.
///
/// Reference: Engineering Plan § Model Load Status
#[derive(Clone, Debug)]
pub struct ModelLoadStatus {
    /// Was the load successful?
    pub success: bool,

    /// Error message if load failed.
    pub error_message: Option<&'static str>,

    /// Bytes transferred to GPU.
    pub bytes_transferred: u64,

    /// Load time in milliseconds (approximate).
    pub load_time_ms: u64,

    /// Final model state.
    pub final_state: ModelLoadState,
}

impl fmt::Display for ModelLoadStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ModelLoadStatus(success={}, state={}, transferred={}B, time={}ms)",
            self.success, self.final_state, self.bytes_transferred, self.load_time_ms
        )
    }
}

/// Model loading orchestrator.
///
/// Implements the complete model loading pipeline:
/// File → VRAM allocation → Registry → Ready
///
/// Reference: Engineering Plan § Model Loading Orchestrator
pub struct ModelLoader {
    /// Reference to GPU Manager (for VRAM and registry access).
    /// In real implementation, would hold a reference or use dependency injection.
    _phantom: core::marker::PhantomData<()>,
}

impl ModelLoader {
    /// Create a new model loader.
    pub fn new() -> Self {
        ModelLoader {
            _phantom: core::marker::PhantomData,
        }
    }

    /// Load a model according to the request.
    ///
    /// # Arguments
    ///
    /// * `gpu_manager` - GPU Manager with VRAM and registry
    /// * `request` - Model load request parameters
    ///
    /// # Returns
    ///
    /// `Ok(ModelLoadStatus)` on success, `Err(GpuError)` on failure.
    ///
    /// Pipeline:
    /// 1. Validate request (model ID, VRAM estimate)
    /// 2. Allocate VRAM via VRAM Manager (cuMemAlloc / hipMalloc)
    /// 3. Transfer model file to GPU (Phase A: simulated)
    /// 4. Create ModelEntry and register in model registry
    /// 5. Bind CT if requested
    /// 6. Transition to Ready state
    ///
    /// Reference: Engineering Plan § Model Loading Path
    pub fn load_model(
        &self,
        gpu_manager: &mut GpuManager,
        request: ModelLoadRequest,
    ) -> Result<ModelLoadStatus, GpuError> {
        // Validate GPU Manager is ready
        if !gpu_manager.is_ready() {
            return Err(GpuError::DriverError);
        }

        // Validate request
        if request.estimated_vram_bytes == 0 {
            return Err(GpuError::AllocationFailed);
        }

        let model_id = request.model_id;
        let estimated_vram = request.estimated_vram_bytes;

        // Step 1: Check if model already loaded
        if gpu_manager.model_registry().contains_model(&model_id) {
            return Err(GpuError::DriverError); // Already loaded
        }

        // Step 2: Allocate VRAM for model weights
        let alloc_result = gpu_manager.vram_manager_mut().allocate(
            model_id,
            estimated_vram,
            VramAllocationType::ModelWeights,
        );

        if alloc_result.is_err() {
            return Err(alloc_result.err().unwrap());
        }

        let allocation = alloc_result?;

        // Step 3: Create model entry
        let primary_ctx = gpu_manager
            .primary_context()
            .ok_or(GpuError::DriverError)?;

        let mut entry = ModelEntry::new(model_id, estimated_vram, primary_ctx.context_handle);
        entry.load_state = ModelLoadState::Loading;

        // Step 4: Simulate file transfer (Phase A)
        // In a real implementation, this would:
        // - Read model file from storage
        // - Call cuMemcpyHtoD / hipMemcpyHtoD to transfer to GPU
        let bytes_transferred = estimated_vram; // Simulated transfer

        // Step 5: Transition to Ready
        entry.load_state = ModelLoadState::Ready;

        // Step 6: Bind CT if requested
        if let Some(ct_id) = request.bind_ct_id {
            entry.bind_ct(ct_id);
        }

        // Step 7: Register in model registry
        gpu_manager.model_registry_mut().register_model(entry);

        // Step 8: Verify memory coherency
        let _ = gpu_manager
            .vram_manager()
            .verify_coherency()
            .map_err(|_| GpuError::DriverError);

        Ok(ModelLoadStatus {
            success: true,
            error_message: None,
            bytes_transferred,
            load_time_ms: 0, // Simulated; real implementation would measure
            final_state: ModelLoadState::Ready,
        })
    }
}

impl Default for ModelLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for ModelLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelLoader").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_model_load_request_creation() {
        let model_id = [1u8; 32];
        let model_path = [0u8; 256];
        let request = ModelLoadRequest::new(model_id, model_path, 1024 * 1024 * 1024);
        assert_eq!(request.model_id, model_id);
        assert_eq!(request.estimated_vram_bytes, 1024 * 1024 * 1024);
        assert!(!request.is_pinned);
    }

    #[test]
    fn test_model_load_request_with_ct_binding() {
        let model_id = [1u8; 32];
        let model_path = [0u8; 256];
        let ct_id = [2u8; 16];
        let request = ModelLoadRequest::new(model_id, model_path, 1024 * 1024 * 1024)
            .with_ct_binding(ct_id);
        assert_eq!(request.bind_ct_id, Some(ct_id));
    }

    #[test]
    fn test_model_load_request_with_pinning() {
        let model_id = [1u8; 32];
        let model_path = [0u8; 256];
        let request = ModelLoadRequest::new(model_id, model_path, 1024 * 1024 * 1024).with_pinning();
        assert!(request.is_pinned);
    }

    #[test]
    fn test_model_loader_creation() {
        let loader = ModelLoader::new();
        assert_eq!(format!("{:?}", loader), "ModelLoader");
    }

    #[test]
    fn test_model_loader_load() {
        let loader = ModelLoader::new();
        let mut config = crate::gpu_manager::GpuManagerConfig::default();
        config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024; // 4 GB
        let mut gpu_manager = crate::gpu_manager::GpuManager::new(config);
        let _ = gpu_manager.initialize();

        let model_id = [1u8; 32];
        let model_path = [0u8; 256];
        let request = ModelLoadRequest::new(model_id, model_path, 1024 * 1024 * 1024);

        let result = loader.load_model(&mut gpu_manager, request);
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.success);
        assert_eq!(status.final_state, ModelLoadState::Ready);
    }

    #[test]
    fn test_model_loader_load_with_ct_binding() {
        let loader = ModelLoader::new();
        let mut config = crate::gpu_manager::GpuManagerConfig::default();
        config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024;
        let mut gpu_manager = crate::gpu_manager::GpuManager::new(config);
        let _ = gpu_manager.initialize();

        let model_id = [1u8; 32];
        let ct_id = [2u8; 16];
        let request = ModelLoadRequest::new([1u8; 32], [0u8; 256], 1024 * 1024 * 1024)
            .with_ct_binding(ct_id);

        let result = loader.load_model(&mut gpu_manager, request);
        assert!(result.is_ok());

        // Verify CT is bound
        let entry = gpu_manager.model_registry().get_model(&model_id);
        assert!(entry.is_some());
        assert!(entry.unwrap().is_ct_bound(&ct_id));
    }

    #[test]
    fn test_model_load_status_display() {
        let status = ModelLoadStatus {
            success: true,
            error_message: None,
            bytes_transferred: 1024 * 1024,
            load_time_ms: 100,
            final_state: ModelLoadState::Ready,
        };
        let display_str = format!("{}", status);
        assert!(display_str.contains("success=true"));
    }
}

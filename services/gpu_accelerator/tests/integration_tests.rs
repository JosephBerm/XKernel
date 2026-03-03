// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Integration test suite for GPU Manager Week 04 (Model Management & Registry).
//!
//! Tests the complete workflow:
//! 1. Initialize GPU Manager
//! 2. Load a single model
//! 3. Verify model registry state
//! 4. Verify VRAM state transitions
//! 5. Bind Cognitive Task to model
//! 6. Unbind and unload model
//! 7. Verify cleanup and memory coherency

use cs_gpu_accelerator::{
    GpuManager, GpuManagerConfig, ModelLoadRequest, ModelLoadState, ModelLoader, ModelRegistry,
    ModelUnloadRequest, ModelUnloader, VramAllocationType, VramManager,
};

#[test]
fn test_integration_gpu_manager_initialization() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    assert!(!manager.is_ready());

    let result = manager.initialize();
    assert!(result.is_ok());
    assert!(manager.is_ready());
    assert!(!manager.devices().is_empty());
    assert!(manager.primary_context().is_some());
}

#[test]
fn test_integration_load_single_model() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024);

    let result = loader.load_model(&mut manager, request);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(status.success);
    assert_eq!(status.final_state, ModelLoadState::Ready);
    assert!(status.bytes_transferred > 0);
}

#[test]
fn test_integration_model_registry_state() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let request = ModelLoadRequest::new(model_id, [0u8; 256], 2048 * 1024 * 1024);

    let _ = loader.load_model(&mut manager, request);

    let registry = manager.model_registry();
    assert_eq!(registry.model_count(), 1);
    assert!(registry.contains_model(&model_id));

    let entry = registry.get_model(&model_id);
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.load_state, ModelLoadState::Ready);
    assert_eq!(entry.vram_footprint_bytes, 2048 * 1024 * 1024);
}

#[test]
fn test_integration_vram_state_transitions() {
    let mut config = GpuManagerConfig::default();
    config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024; // 4 GB
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let vram = manager.vram_manager();
    let initial_free = vram.free_vram_bytes();
    assert_eq!(initial_free, 4 * 1024 * 1024 * 1024);

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let alloc_size = 1024 * 1024 * 1024; // 1 GB
    let request = ModelLoadRequest::new(model_id, [0u8; 256], alloc_size);

    let _ = loader.load_model(&mut manager, request);

    let vram = manager.vram_manager();
    let after_load_free = vram.free_vram_bytes();
    assert_eq!(after_load_free, initial_free - alloc_size);

    let used = vram.used_vram_bytes();
    assert_eq!(used, alloc_size);
}

#[test]
fn test_integration_bind_ct_to_model() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let ct_id = [2u8; 16];

    let request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024)
        .with_ct_binding(ct_id);

    let _ = loader.load_model(&mut manager, request);

    let registry = manager.model_registry();
    let entry = registry.get_model(&model_id).unwrap();
    assert!(entry.is_ct_bound(&ct_id));
    assert_eq!(entry.bound_ct_count(), 1);
}

#[test]
fn test_integration_unbind_ct_and_unload() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let unloader = ModelUnloader::new();
    let model_id = [1u8; 32];
    let ct_id = [2u8; 16];

    let load_request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024)
        .with_ct_binding(ct_id);
    let _ = loader.load_model(&mut manager, load_request);

    // Unbind CT
    let registry_mut = manager.model_registry_mut();
    if let Some(entry) = registry_mut.get_model_mut(&model_id) {
        entry.unbind_ct(&ct_id);
    }

    // Now unload should succeed
    let unload_request = ModelUnloadRequest::new(model_id);
    let result = unloader.unload_model(&mut manager, unload_request);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(status.success);
    assert_eq!(status.final_state, ModelLoadState::Unloaded);
}

#[test]
fn test_integration_cannot_unload_with_bound_ct() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let unloader = ModelUnloader::new();
    let model_id = [1u8; 32];
    let ct_id = [2u8; 16];

    let load_request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024)
        .with_ct_binding(ct_id);
    let _ = loader.load_model(&mut manager, load_request);

    // Try to unload with CT still bound
    let unload_request = ModelUnloadRequest::new(model_id);
    let result = unloader.unload_model(&mut manager, unload_request);
    assert!(result.is_err()); // Should fail
}

#[test]
fn test_integration_force_unload() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let unloader = ModelUnloader::new();
    let model_id = [1u8; 32];
    let ct_id = [2u8; 16];

    let load_request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024)
        .with_ct_binding(ct_id);
    let _ = loader.load_model(&mut manager, load_request);

    // Force unload even with CT bound
    let unload_request = ModelUnloadRequest::new(model_id).with_force_unload();
    let result = unloader.unload_model(&mut manager, unload_request);
    assert!(result.is_ok());
}

#[test]
fn test_integration_vram_cleanup_after_unload() {
    let mut config = GpuManagerConfig::default();
    config.single_model_vram_partition_bytes = 4 * 1024 * 1024 * 1024;
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let unloader = ModelUnloader::new();
    let model_id = [1u8; 32];
    let alloc_size = 1024 * 1024 * 1024;

    let vram_before = manager.vram_manager().free_vram_bytes();

    let load_request = ModelLoadRequest::new(model_id, [0u8; 256], alloc_size);
    let _ = loader.load_model(&mut manager, load_request);

    let vram_after_load = manager.vram_manager().free_vram_bytes();
    assert_eq!(vram_after_load, vram_before - alloc_size);

    let unload_request = ModelUnloadRequest::new(model_id);
    let _ = unloader.unload_model(&mut manager, unload_request);

    let vram_after_unload = manager.vram_manager().free_vram_bytes();
    assert_eq!(vram_after_unload, vram_before);
}

#[test]
fn test_integration_memory_coherency_verification() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024);

    let _ = loader.load_model(&mut manager, request);

    let vram = manager.vram_manager();
    let coherency_result = vram.verify_coherency();
    assert!(coherency_result.is_ok());
}

#[test]
fn test_integration_unload_all_models() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let unloader = ModelUnloader::new();

    // Load a model
    let model_id = [1u8; 32];
    let request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024);
    let _ = loader.load_model(&mut manager, request);
    assert_eq!(manager.model_registry().model_count(), 1);

    // Unload all
    let result = unloader.unload_all_models(&mut manager);
    assert!(result.is_ok());
    assert_eq!(manager.model_registry().model_count(), 0);
}

#[test]
fn test_integration_gpu_manager_shutdown() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024);
    let _ = loader.load_model(&mut manager, request);

    let result = manager.shutdown();
    assert!(result.is_ok());
    assert_eq!(manager.model_registry().model_count(), 0);
}

#[test]
fn test_integration_sequential_load_unload() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let unloader = ModelUnloader::new();

    for i in 0..3 {
        let model_id = [i as u8; 32];
        let request = ModelLoadRequest::new(model_id, [0u8; 256], 512 * 1024 * 1024);
        let load_result = loader.load_model(&mut manager, request);
        assert!(load_result.is_ok());
        assert_eq!(manager.model_registry().model_count(), 1);

        let unload_request = ModelUnloadRequest::new(model_id);
        let unload_result = unloader.unload_model(&mut manager, unload_request);
        assert!(unload_result.is_ok());
        assert_eq!(manager.model_registry().model_count(), 0);
    }
}

#[test]
fn test_integration_vram_exhaustion() {
    let mut config = GpuManagerConfig::default();
    config.single_model_vram_partition_bytes = 2 * 1024 * 1024 * 1024; // 2 GB only
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();

    // Try to load a model larger than available VRAM
    let model_id = [1u8; 32];
    let request = ModelLoadRequest::new(model_id, [0u8; 256], 3 * 1024 * 1024 * 1024);
    let result = loader.load_model(&mut manager, request);
    assert!(result.is_err());
}

#[test]
fn test_integration_multiple_ct_bindings() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let ct_id_1 = [2u8; 16];

    let request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024)
        .with_ct_binding(ct_id_1);
    let _ = loader.load_model(&mut manager, request);

    // Bind a second CT
    let ct_id_2 = [3u8; 16];
    let registry_mut = manager.model_registry_mut();
    if let Some(entry) = registry_mut.get_model_mut(&model_id) {
        entry.bind_ct(ct_id_2);
    }

    let registry = manager.model_registry();
    let entry = registry.get_model(&model_id).unwrap();
    assert_eq!(entry.bound_ct_count(), 2);
    assert!(entry.is_ct_bound(&ct_id_1));
    assert!(entry.is_ct_bound(&ct_id_2));
}

#[test]
fn test_integration_model_pinning() {
    let config = GpuManagerConfig::default();
    let mut manager = GpuManager::new(config);
    let _ = manager.initialize();

    let loader = ModelLoader::new();
    let model_id = [1u8; 32];
    let request = ModelLoadRequest::new(model_id, [0u8; 256], 1024 * 1024 * 1024).with_pinning();

    let _ = loader.load_model(&mut manager, request);

    let registry = manager.model_registry();
    let entry = registry.get_model(&model_id).unwrap();
    assert!(entry.is_pinned);
    assert!(!entry.can_unload());
}

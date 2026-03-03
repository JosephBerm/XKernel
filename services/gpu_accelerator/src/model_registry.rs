// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! Model registry — central tracking of loaded ML models.
//!
//! Maintains an inventory of all models currently loaded in VRAM, including
//! model identifiers, VRAM footprints, bound Cognitive Tasks, and load state.
//!
//! For Phase 0 (Week 4), supports single-model scenarios. Each model entry
//! tracks the model_id, vram_footprint_bytes, bound_ct_list, load_state,
//! and cuda_device_handle.
//!
//! Reference: Engineering Plan § Model Registry, Phase 0 Single-Model

use crate::cuda_abstraction::CudaContext;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

/// Model load state — lifecycle of a model in VRAM.
///
/// Tracks the progression of a model from initial load request
/// through resident execution to eventual unload.
///
/// Reference: Engineering Plan § Model Load State Machine
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelLoadState {
    /// Model file located but not yet loaded to VRAM.
    Pending,

    /// Model in the process of being loaded to VRAM.
    Loading,

    /// Model fully resident in VRAM and ready for inference.
    Ready,

    /// Model is currently being evicted from VRAM.
    Unloading,

    /// Model has been unloaded or evicted from VRAM.
    Unloaded,

    /// Model load/unload operation encountered an error.
    Error,
}

impl fmt::Display for ModelLoadState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelLoadState::Pending => write!(f, "Pending"),
            ModelLoadState::Loading => write!(f, "Loading"),
            ModelLoadState::Ready => write!(f, "Ready"),
            ModelLoadState::Unloading => write!(f, "Unloading"),
            ModelLoadState::Unloaded => write!(f, "Unloaded"),
            ModelLoadState::Error => write!(f, "Error"),
        }
    }
}

/// Model registry entry — metadata for a loaded model.
///
/// Represents a single model in VRAM with all associated metadata.
/// Bound CTs (Cognitive Tasks) are those currently using this model.
///
/// Reference: Engineering Plan § Model Registry Entry
#[derive(Clone, Debug)]
pub struct ModelEntry {
    /// Unique model identifier (e.g., SHA256 hash or semantic URI).
    pub model_id: [u8; 32],

    /// VRAM footprint in bytes (model weights + buffers).
    pub vram_footprint_bytes: u64,

    /// List of bound Cognitive Task IDs currently using this model.
    pub bound_ct_list: Vec<[u8; 16]>,

    /// Current load state of the model.
    pub load_state: ModelLoadState,

    /// CUDA device context handle for this model.
    /// Used to reference the GPU and context where the model is allocated.
    pub cuda_device_handle: u64,

    /// Timestamp of when the model was loaded (in arbitrary units).
    pub load_timestamp_ms: u64,

    /// Flag indicating if this model is pinned (cannot be evicted).
    pub is_pinned: bool,
}

impl ModelEntry {
    /// Create a new model entry.
    ///
    /// # Arguments
    ///
    /// * `model_id` - 32-byte model identifier
    /// * `vram_footprint_bytes` - Model size in VRAM
    /// * `cuda_device_handle` - GPU device handle
    pub fn new(
        model_id: [u8; 32],
        vram_footprint_bytes: u64,
        cuda_device_handle: u64,
    ) -> Self {
        ModelEntry {
            model_id,
            vram_footprint_bytes,
            bound_ct_list: Vec::new(),
            load_state: ModelLoadState::Pending,
            cuda_device_handle,
            load_timestamp_ms: 0,
            is_pinned: false,
        }
    }

    /// Bind a Cognitive Task to this model.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - 16-byte Cognitive Task identifier
    pub fn bind_ct(&mut self, ct_id: [u8; 16]) {
        if !self.bound_ct_list.contains(&ct_id) {
            self.bound_ct_list.push(ct_id);
        }
    }

    /// Unbind a Cognitive Task from this model.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - 16-byte Cognitive Task identifier
    pub fn unbind_ct(&mut self, ct_id: &[u8; 16]) {
        self.bound_ct_list.retain(|id| id != ct_id);
    }

    /// Check if a Cognitive Task is bound to this model.
    pub fn is_ct_bound(&self, ct_id: &[u8; 16]) -> bool {
        self.bound_ct_list.contains(ct_id)
    }

    /// Get the number of bound Cognitive Tasks.
    pub fn bound_ct_count(&self) -> usize {
        self.bound_ct_list.len()
    }

    /// Check if this model can be unloaded (no bound CTs).
    pub fn can_unload(&self) -> bool {
        self.bound_ct_list.is_empty() && !self.is_pinned
    }
}

impl fmt::Display for ModelEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ModelEntry(id={:?}, vram={}B, state={}, bound_cts={})",
            &self.model_id[..8],
            self.vram_footprint_bytes,
            self.load_state,
            self.bound_ct_count()
        )
    }
}

/// Model registry — central tracking of all loaded models.
///
/// Maintains a collection of loaded models indexed by model_id.
/// For Phase 0, typically contains 0 or 1 model (single-model scenario).
///
/// Reference: Engineering Plan § Model Registry
#[derive(Clone, Debug)]
pub struct ModelRegistry {
    /// Models indexed by model_id (first 8 bytes as u64 for BTreeMap key).
    /// In a production system, would use full model_id or a proper hash.
    models: BTreeMap<[u8; 8], ModelEntry>,

    /// Total VRAM in use by all models (running sum).
    total_vram_in_use_bytes: u64,
}

impl ModelRegistry {
    /// Create a new empty model registry.
    pub fn new() -> Self {
        ModelRegistry {
            models: BTreeMap::new(),
            total_vram_in_use_bytes: 0,
        }
    }

    /// Register (load) a model into the registry.
    ///
    /// # Arguments
    ///
    /// * `entry` - ModelEntry with model metadata
    ///
    /// Returns the ModelEntry if a model with the same ID already existed.
    pub fn register_model(&mut self, entry: ModelEntry) -> Option<ModelEntry> {
        let key = Self::model_id_to_key(&entry.model_id);
        self.total_vram_in_use_bytes += entry.vram_footprint_bytes;
        self.models.insert(key, entry)
    }

    /// Unregister (unload) a model from the registry.
    ///
    /// # Arguments
    ///
    /// * `model_id` - 32-byte model identifier
    ///
    /// Returns the ModelEntry if it was found, or None if not in registry.
    pub fn unregister_model(&mut self, model_id: &[u8; 32]) -> Option<ModelEntry> {
        let key = Self::model_id_to_key(model_id);
        if let Some(entry) = self.models.remove(&key) {
            self.total_vram_in_use_bytes = self
                .total_vram_in_use_bytes
                .saturating_sub(entry.vram_footprint_bytes);
            Some(entry)
        } else {
            None
        }
    }

    /// Get a reference to a model by ID.
    pub fn get_model(&self, model_id: &[u8; 32]) -> Option<&ModelEntry> {
        let key = Self::model_id_to_key(model_id);
        self.models.get(&key)
    }

    /// Get a mutable reference to a model by ID.
    pub fn get_model_mut(&mut self, model_id: &[u8; 32]) -> Option<&mut ModelEntry> {
        let key = Self::model_id_to_key(model_id);
        self.models.get_mut(&key)
    }

    /// Check if a model is registered.
    pub fn contains_model(&self, model_id: &[u8; 32]) -> bool {
        let key = Self::model_id_to_key(model_id);
        self.models.contains_key(&key)
    }

    /// Get the number of registered models.
    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Get total VRAM in use by all models.
    pub fn total_vram_in_use_bytes(&self) -> u64 {
        self.total_vram_in_use_bytes
    }

    /// Get all models (iteration).
    pub fn iter(&self) -> impl Iterator<Item = &ModelEntry> {
        self.models.values()
    }

    /// Get all models (mutable iteration).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ModelEntry> {
        self.models.values_mut()
    }

    /// Find model by CT binding.
    ///
    /// Returns the model entry if a model with a bound CT is found.
    pub fn find_by_ct(&self, ct_id: &[u8; 16]) -> Option<&ModelEntry> {
        self.models.values().find(|e| e.is_ct_bound(ct_id))
    }

    /// Clear all models from the registry.
    pub fn clear(&mut self) {
        self.models.clear();
        self.total_vram_in_use_bytes = 0;
    }

    /// Helper: Convert full model_id to 8-byte key for BTreeMap.
    fn model_id_to_key(model_id: &[u8; 32]) -> [u8; 8] {
        let mut key = [0u8; 8];
        key.copy_from_slice(&model_id[..8]);
        key
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;

    #[test]
    fn test_model_entry_creation() {
        let model_id = [1u8; 32];
        let entry = ModelEntry::new(model_id, 1024 * 1024, 0x1000);
        assert_eq!(entry.model_id, model_id);
        assert_eq!(entry.vram_footprint_bytes, 1024 * 1024);
        assert_eq!(entry.load_state, ModelLoadState::Pending);
        assert_eq!(entry.bound_ct_count(), 0);
    }

    #[test]
    fn test_bind_ct() {
        let mut entry = ModelEntry::new([1u8; 32], 1024, 0x1000);
        let ct_id = [2u8; 16];
        entry.bind_ct(ct_id);
        assert!(entry.is_ct_bound(&ct_id));
        assert_eq!(entry.bound_ct_count(), 1);
    }

    #[test]
    fn test_bind_ct_idempotent() {
        let mut entry = ModelEntry::new([1u8; 32], 1024, 0x1000);
        let ct_id = [2u8; 16];
        entry.bind_ct(ct_id);
        entry.bind_ct(ct_id);
        assert_eq!(entry.bound_ct_count(), 1);
    }

    #[test]
    fn test_unbind_ct() {
        let mut entry = ModelEntry::new([1u8; 32], 1024, 0x1000);
        let ct_id = [2u8; 16];
        entry.bind_ct(ct_id);
        entry.unbind_ct(&ct_id);
        assert!(!entry.is_ct_bound(&ct_id));
        assert_eq!(entry.bound_ct_count(), 0);
    }

    #[test]
    fn test_can_unload() {
        let mut entry = ModelEntry::new([1u8; 32], 1024, 0x1000);
        assert!(entry.can_unload());

        let ct_id = [2u8; 16];
        entry.bind_ct(ct_id);
        assert!(!entry.can_unload());

        entry.unbind_ct(&ct_id);
        assert!(entry.can_unload());
    }

    #[test]
    fn test_can_unload_pinned() {
        let mut entry = ModelEntry::new([1u8; 32], 1024, 0x1000);
        entry.is_pinned = true;
        assert!(!entry.can_unload());
    }

    #[test]
    fn test_registry_register_model() {
        let mut registry = ModelRegistry::new();
        let entry = ModelEntry::new([1u8; 32], 2048, 0x1000);
        registry.register_model(entry);
        assert_eq!(registry.model_count(), 1);
        assert_eq!(registry.total_vram_in_use_bytes(), 2048);
    }

    #[test]
    fn test_registry_unregister_model() {
        let mut registry = ModelRegistry::new();
        let model_id = [1u8; 32];
        let entry = ModelEntry::new(model_id, 2048, 0x1000);
        registry.register_model(entry);
        let unregistered = registry.unregister_model(&model_id);
        assert!(unregistered.is_some());
        assert_eq!(registry.model_count(), 0);
        assert_eq!(registry.total_vram_in_use_bytes(), 0);
    }

    #[test]
    fn test_registry_contains_model() {
        let mut registry = ModelRegistry::new();
        let model_id = [1u8; 32];
        let entry = ModelEntry::new(model_id, 2048, 0x1000);
        registry.register_model(entry);
        assert!(registry.contains_model(&model_id));
    }

    #[test]
    fn test_registry_get_model() {
        let mut registry = ModelRegistry::new();
        let model_id = [1u8; 32];
        let entry = ModelEntry::new(model_id, 2048, 0x1000);
        registry.register_model(entry);
        let retrieved = registry.get_model(&model_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().vram_footprint_bytes, 2048);
    }

    #[test]
    fn test_registry_get_model_mut() {
        let mut registry = ModelRegistry::new();
        let model_id = [1u8; 32];
        let mut entry = ModelEntry::new(model_id, 2048, 0x1000);
        entry.load_state = ModelLoadState::Pending;
        registry.register_model(entry);
        let retrieved = registry.get_model_mut(&model_id);
        assert!(retrieved.is_some());
        retrieved.unwrap().load_state = ModelLoadState::Ready;
        assert_eq!(
            registry.get_model(&model_id).unwrap().load_state,
            ModelLoadState::Ready
        );
    }

    #[test]
    fn test_registry_find_by_ct() {
        let mut registry = ModelRegistry::new();
        let model_id = [1u8; 32];
        let ct_id = [2u8; 16];
        let mut entry = ModelEntry::new(model_id, 2048, 0x1000);
        entry.bind_ct(ct_id);
        registry.register_model(entry);
        let found = registry.find_by_ct(&ct_id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().model_id, model_id);
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = ModelRegistry::new();
        let entry = ModelEntry::new([1u8; 32], 2048, 0x1000);
        registry.register_model(entry);
        assert_eq!(registry.model_count(), 1);
        registry.clear();
        assert_eq!(registry.model_count(), 0);
        assert_eq!(registry.total_vram_in_use_bytes(), 0);
    }

    #[test]
    fn test_model_load_state_display() {
        assert_eq!(format!("{}", ModelLoadState::Pending), "Pending");
        assert_eq!(format!("{}", ModelLoadState::Ready), "Ready");
        assert_eq!(format!("{}", ModelLoadState::Error), "Error");
    }
}

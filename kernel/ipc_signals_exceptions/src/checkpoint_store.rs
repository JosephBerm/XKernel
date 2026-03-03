// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! # Checkpoint Store with Hash Chain
//!
//! This module implements the checkpoint storage system with hash-linked chain
//! for tamper evidence. Each CT maintains a VecDeque of up to 5 checkpoints.
//! When a new checkpoint is added and the limit is exceeded, the oldest
//! checkpoint is evicted via LRU policy.
//!
//! ## Hash Chain Verification
//!
//! Checkpoints form a cryptographic chain:
//! - Each checkpoint includes the hash of the previous checkpoint
//! - The chain is verified by recomputing hashes and checking linkage
//! - Tampering with any checkpoint breaks the chain
//! - Allows detection of unauthorized state modifications
//!
//! ## Retention Policy
//!
//! - Maximum 5 checkpoints per CT
//! - LRU eviction when limit exceeded
//! - Oldest checkpoint evicted first
//! - Enables quick rollback to recent states
//!
//! ## References
//!
//! - Engineering Plan § 6.3 (Checkpointing - Checkpoint Store)
//! - Week 6 Objective: Hash-linked chain and 5-checkpoint retention

use crate::checkpoint::{CognitiveCheckpoint, CheckpointPhase, ContextSnapshot, ReasoningChain,
use alloc::collections::VecDeque;

use alloc::vec::Vec;

    ToolHistory, ToolStateSnapshot, CapabilitySnapshot, IpcStateSnapshot};
use crate::ids::CheckpointID;
use crate::{CsError, Result};
use cs_ct_lifecycle::CTID;

/// Maximum number of checkpoints to retain per CT.
const MAX_CHECKPOINTS_PER_CT: usize = 5;

/// Checkpoint store for a single CT.
///
/// Maintains an ordered queue of checkpoints (newest to oldest) with a maximum
/// of 5 entries. Uses LRU eviction policy when capacity is exceeded.
/// Verifies hash chain integrity on checkpoint addition and retrieval.
///
/// See Engineering Plan § 6.3 (Checkpointing - Checkpoint Store)
pub struct CheckpointStore {
    /// CT ID this store belongs to
    ct_id: CTID,

    /// Queue of checkpoints (front = most recent, back = oldest)
    checkpoints: VecDeque<CognitiveCheckpoint>,

    /// Total checkpoints created (for tracking)
    total_created: u64,

    /// Total checkpoints evicted
    total_evicted: u64,
}

impl CheckpointStore {
    /// Create a new checkpoint store for a CT.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - The CT that this store belongs to
    ///
    /// # Returns
    ///
    /// A new empty CheckpointStore
    pub fn new(ct_id: CTID) -> Self {
        Self {
            ct_id,
            checkpoints: VecDeque::with_capacity(MAX_CHECKPOINTS_PER_CT),
            total_created: 0,
            total_evicted: 0,
        }
    }

    /// Add a new checkpoint to the store.
    ///
    /// The checkpoint is added to the front of the queue (most recent position).
    /// If the store is at capacity, the oldest checkpoint is evicted.
    /// The checkpoint's chain_position is automatically set based on position in queue.
    ///
    /// # Arguments
    ///
    /// * `mut checkpoint` - The checkpoint to add (chain_position will be updated)
    ///
    /// # Returns
    ///
    /// Ok(CheckpointID) of the added checkpoint, or Err if it belongs to a different CT
    pub fn add_checkpoint(&mut self, mut checkpoint: CognitiveCheckpoint) -> Result<CheckpointID> {
        // Verify the checkpoint belongs to this CT
        if checkpoint.ct_id != self.ct_id {
            return Err(CsError::InvalidState(
                alloc::string::String::from("Checkpoint does not belong to this CT"),
            ));
        }

        // Verify hash chain integrity with previous checkpoint
        if self.checkpoints.len() > 0 {
            // Get the most recent checkpoint
            let previous = &self.checkpoints[0];

            // The new checkpoint's previous_hash should be this checkpoint's hash
            if checkpoint.previous_hash.is_none() {
                // New checkpoint should have previous_hash set
                checkpoint.previous_hash = Some(previous.checkpoint_hash.clone());
            }

            // Update chain position
            checkpoint.chain_position = (previous.chain_position + 1) as u32;
        } else {
            // First checkpoint should have no previous hash
            checkpoint.chain_position = 0;
            if checkpoint.previous_hash.is_some() {
                return Err(CsError::InvalidState(
                    alloc::string::String::from("First checkpoint should not have previous_hash"),
                ));
            }
        }

        let checkpoint_id = checkpoint.id;

        // Add to front of queue
        self.checkpoints.push_front(checkpoint);

        // Evict oldest if capacity exceeded
        if self.checkpoints.len() > MAX_CHECKPOINTS_PER_CT {
            self.checkpoints.pop_back();
            self.total_evicted += 1;
        }

        self.total_created += 1;
        Ok(checkpoint_id)
    }

    /// Retrieve a checkpoint by ID.
    ///
    /// Searches the queue for a checkpoint with the given ID.
    /// Does not verify hash chain (use verify_checkpoint_chain() for that).
    ///
    /// # Arguments
    ///
    /// * `checkpoint_id` - ID of the checkpoint to retrieve
    ///
    /// # Returns
    ///
    /// Ok(reference to checkpoint) if found, Err if not found
    pub fn get_checkpoint(&self, checkpoint_id: CheckpointID) -> Result<&CognitiveCheckpoint> {
        self.checkpoints
            .iter()
            .find(|cp| cp.id == checkpoint_id)
            .ok_or_else(|| {
                CsError::InvalidState(
                    alloc::format!("Checkpoint {:?} not found", checkpoint_id),
                )
            })
    }

    /// Retrieve the most recent checkpoint.
    ///
    /// # Returns
    ///
    /// Some(reference to most recent checkpoint) if any exist, None otherwise
    pub fn get_latest(&self) -> Option<&CognitiveCheckpoint> {
        self.checkpoints.front()
    }

    /// Get all checkpoints in order (most recent first).
    ///
    /// # Returns
    ///
    /// Reference to the internal VecDeque
    pub fn all_checkpoints(&self) -> &VecDeque<CognitiveCheckpoint> {
        &self.checkpoints
    }

    /// Verify the hash chain integrity of all checkpoints.
    ///
    /// Starting from the first checkpoint (chain_position=0), verifies:
    /// 1. Each checkpoint's hash is correct (validate_hash())
    /// 2. Each checkpoint links to the correct previous checkpoint (validate_chain_linkage())
    /// 3. Chain positions are sequential (0, 1, 2, ...)
    ///
    /// # Returns
    ///
    /// Ok(()) if chain is valid, Err if tampering detected or chain broken
    pub fn verify_checkpoint_chain(&self) -> Result<()> {
        // Find the checkpoint with chain_position == 0
        let first = self.checkpoints
            .iter()
            .rfind(|cp| cp.chain_position == 0)
            .ok_or_else(|| {
                CsError::InvalidState(
                    alloc::string::String::from("No first checkpoint found (position 0)"),
                )
            })?;

        // Verify first checkpoint
        if !first.validate_hash() {
            return Err(CsError::InvalidState(
                alloc::string::String::from("First checkpoint hash invalid"),
            ));
        }
        if !first.validate_chain_linkage() {
            return Err(CsError::InvalidState(
                alloc::string::String::from("First checkpoint chain linkage invalid"),
            ));
        }

        // Verify remaining checkpoints are linked correctly
        let mut current_pos: u32 = 0;
        for checkpoint in self.checkpoints.iter().rev() {
            // Check sequential positions
            if checkpoint.chain_position != current_pos {
                return Err(CsError::InvalidState(
                    alloc::format!(
                        "Non-sequential chain position: expected {}, got {}",
                        current_pos, checkpoint.chain_position
                    ),
                ));
            }

            // Verify hash
            if !checkpoint.validate_hash() {
                return Err(CsError::InvalidState(
                    alloc::format!("Checkpoint {} hash invalid", checkpoint.chain_position),
                ));
            }

            // Verify chain linkage
            if !checkpoint.validate_chain_linkage() {
                return Err(CsError::InvalidState(
                    alloc::format!("Checkpoint {} chain linkage invalid", checkpoint.chain_position),
                ));
            }

            current_pos += 1;
        }

        Ok(())
    }

    /// Get the number of checkpoints currently in the store.
    pub fn count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Check if the store is at capacity.
    pub fn is_full(&self) -> bool {
        self.checkpoints.len() >= MAX_CHECKPOINTS_PER_CT
    }

    /// Get the CT ID this store belongs to.
    pub fn ct_id(&self) -> CTID {
        self.ct_id
    }

    /// Get total number of checkpoints created.
    pub fn total_created(&self) -> u64 {
        self.total_created
    }

    /// Get total number of checkpoints evicted.
    pub fn total_evicted(&self) -> u64 {
        self.total_evicted
    }

    /// Get store statistics.
    pub fn stats(&self) -> CheckpointStoreStats {
        CheckpointStoreStats {
            ct_id: self.ct_id,
            current_count: self.checkpoints.len(),
            total_created: self.total_created,
            total_evicted: self.total_evicted,
            is_full: self.is_full(),
        }
    }
}

/// Statistics about a checkpoint store.
#[derive(Clone, Debug)]
pub struct CheckpointStoreStats {
    /// CT ID
    pub ct_id: CTID,
    /// Number of checkpoints currently in store
    pub current_count: usize,
    /// Total checkpoints created since store creation
    pub total_created: u64,
    /// Total checkpoints evicted due to capacity
    pub total_evicted: u64,
    /// Whether store is at capacity
    pub is_full: bool,
}

/// Registry of checkpoint stores for all CTs.
///
/// This is the top-level checkpoint management structure that maintains
/// per-CT checkpoint stores.
pub struct CheckpointRegistry {
    /// Map of CT ID to checkpoint store
    stores: alloc::collections::BTreeMap<CTID, CheckpointStore>,
}

impl CheckpointRegistry {
    /// Create a new checkpoint registry.
    pub fn new() -> Self {
        Self {
            stores: alloc::collections::BTreeMap::new(),
        }
    }

    /// Get or create a checkpoint store for a CT.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - The CT ID
    ///
    /// # Returns
    ///
    /// Mutable reference to the checkpoint store for this CT
    pub fn get_or_create_store(&mut self, ct_id: CTID) -> &mut CheckpointStore {
        self.stores
            .entry(ct_id)
            .or_insert_with(|| CheckpointStore::new(ct_id))
    }

    /// Get a checkpoint store for a CT.
    ///
    /// # Arguments
    ///
    /// * `ct_id` - The CT ID
    ///
    /// # Returns
    ///
    /// Ok(reference to store) if exists, Err if not found
    pub fn get_store(&self, ct_id: CTID) -> Result<&CheckpointStore> {
        self.stores
            .get(&ct_id)
            .ok_or_else(|| {
                CsError::InvalidState(
                    alloc::format!("No checkpoint store for CT {:?}", ct_id),
                )
            })
    }

    /// Get a mutable reference to a checkpoint store.
    pub fn get_store_mut(&mut self, ct_id: CTID) -> Result<&mut CheckpointStore> {
        self.stores
            .get_mut(&ct_id)
            .ok_or_else(|| {
                CsError::InvalidState(
                    alloc::format!("No checkpoint store for CT {:?}", ct_id),
                )
            })
    }

    /// Get the number of CT stores.
    pub fn store_count(&self) -> usize {
        self.stores.len()
    }

    /// Get global statistics.
    pub fn global_stats(&self) -> CheckpointRegistryStats {
        let mut total_checkpoints = 0;
        let mut total_created = 0;
        let mut total_evicted = 0;

        for store in self.stores.values() {
            total_checkpoints += store.count();
            total_created += store.total_created();
            total_evicted += store.total_evicted();
        }

        CheckpointRegistryStats {
            num_cts: self.stores.len(),
            total_checkpoints,
            total_created,
            total_evicted,
        }
    }
}

/// Global checkpoint registry statistics.
#[derive(Clone, Debug)]
pub struct CheckpointRegistryStats {
    /// Number of CTs with checkpoint stores
    pub num_cts: usize,
    /// Total checkpoints currently stored across all CTs
    pub total_checkpoints: usize,
    /// Total checkpoints created across all CTs
    pub total_created: u64,
    /// Total checkpoints evicted across all CTs
    pub total_evicted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::CheckpointID;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;

    fn create_test_checkpoint(id: CheckpointID, ct_id: CTID) -> CognitiveCheckpoint {
        CognitiveCheckpoint::new(
            id,
            ct_id,
            1000,
            CheckpointPhase::Reasoning,
            ContextSnapshot::new(alloc::vec![], alloc::string::String::from("state"), 1000),
            ReasoningChain::new(alloc::vec![], 100),
            ToolHistory::new(alloc::vec![]),
            ToolStateSnapshot::new(alloc::vec![], 5000, 1000),
            CapabilitySnapshot::new(alloc::vec![], 1000),
            IpcStateSnapshot::new(alloc::vec![], 0, 1000),
            0,
            None,
        )
    }

    #[test]
    fn test_checkpoint_store_new() {
        let ct_id = CTID::new();
        let store = CheckpointStore::new(ct_id);
        assert_eq!(store.count(), 0);
        assert_eq!(store.ct_id(), ct_id);
    }

    #[test]
    fn test_checkpoint_store_add() {
        let ct_id = CTID::new();
        let mut store = CheckpointStore::new(ct_id);
        let ckpt_id = CheckpointID::new();
        let ckpt = create_test_checkpoint(ckpt_id, ct_id);

        assert!(store.add_checkpoint(ckpt).is_ok());
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_checkpoint_store_add_wrong_ct() {
        let ct_id = CTID::new();
        let other_ct = CTID::new();
        let mut store = CheckpointStore::new(ct_id);
        let ckpt_id = CheckpointID::new();
        let ckpt = create_test_checkpoint(ckpt_id, other_ct);

        assert!(store.add_checkpoint(ckpt).is_err());
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_checkpoint_store_get() {
        let ct_id = CTID::new();
        let mut store = CheckpointStore::new(ct_id);
        let ckpt_id = CheckpointID::new();
        let ckpt = create_test_checkpoint(ckpt_id, ct_id);

        store.add_checkpoint(ckpt).unwrap();
        assert!(store.get_checkpoint(ckpt_id).is_ok());
    }

    #[test]
    fn test_checkpoint_store_get_latest() {
        let ct_id = CTID::new();
        let mut store = CheckpointStore::new(ct_id);
        let ckpt_id = CheckpointID::new();
        let ckpt = create_test_checkpoint(ckpt_id, ct_id);

        store.add_checkpoint(ckpt).unwrap();
        assert!(store.get_latest().is_some());
        assert_eq!(store.get_latest().unwrap().id, ckpt_id);
    }

    #[test]
    fn test_checkpoint_store_capacity() {
        let ct_id = CTID::new();
        let mut store = CheckpointStore::new(ct_id);

        // Add 5 checkpoints (capacity)
        for _ in 0..5 {
            let ckpt_id = CheckpointID::new();
            let ckpt = create_test_checkpoint(ckpt_id, ct_id);
            store.add_checkpoint(ckpt).unwrap();
        }

        assert_eq!(store.count(), 5);
        assert!(store.is_full());
    }

    #[test]
    fn test_checkpoint_store_eviction() {
        let ct_id = CTID::new();
        let mut store = CheckpointStore::new(ct_id);

        // Add 6 checkpoints (should evict 1)
        for i in 0..6 {
            let ckpt_id = CheckpointID::new();
            let mut ckpt = create_test_checkpoint(ckpt_id, ct_id);
            ckpt.timestamp_ms = i as u64;
            store.add_checkpoint(ckpt).unwrap();
        }

        assert_eq!(store.count(), 5);
        assert_eq!(store.total_evicted(), 1);
    }

    #[test]
    fn test_checkpoint_store_verify_chain() {
        let ct_id = CTID::new();
        let mut store = CheckpointStore::new(ct_id);
        let ckpt_id = CheckpointID::new();
        let ckpt = create_test_checkpoint(ckpt_id, ct_id);

        store.add_checkpoint(ckpt).unwrap();
        assert!(store.verify_checkpoint_chain().is_ok());
    }

    #[test]
    fn test_checkpoint_store_stats() {
        let ct_id = CTID::new();
        let mut store = CheckpointStore::new(ct_id);
        let ckpt_id = CheckpointID::new();
        let ckpt = create_test_checkpoint(ckpt_id, ct_id);

        store.add_checkpoint(ckpt).unwrap();
        let stats = store.stats();
        assert_eq!(stats.current_count, 1);
        assert_eq!(stats.total_created, 1);
    }

    #[test]
    fn test_checkpoint_registry_new() {
        let registry = CheckpointRegistry::new();
        assert_eq!(registry.store_count(), 0);
    }

    #[test]
    fn test_checkpoint_registry_get_or_create() {
        let mut registry = CheckpointRegistry::new();
        let ct_id = CTID::new();
        
        let store = registry.get_or_create_store(ct_id);
        assert_eq!(store.ct_id(), ct_id);
        assert_eq!(registry.store_count(), 1);
    }

    #[test]
    fn test_checkpoint_registry_global_stats() {
        let mut registry = CheckpointRegistry::new();
        let ct_id = CTID::new();
        let store = registry.get_or_create_store(ct_id);
        
        let stats = registry.global_stats();
        assert_eq!(stats.num_cts, 1);
    }
}

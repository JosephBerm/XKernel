// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 XKernal Contributors
//! Checkpoint, snapshot, and CRDT-based state synchronization

use alloc::vec;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Checkpoint errors
#[derive(Debug, Clone, Error)]
pub enum CheckpointError {
    /// Checkpoint not found
    #[error("checkpoint {0} not found")]
    NotFound(u64),
    /// Restore failed
    #[error("restore failed: {0}")]
    RestoreFailed(alloc::string::String),
    /// Invalid checkpoint format
    #[error("invalid format: {0}")]
    InvalidFormat(alloc::string::String),
}

pub type Result<T> = core::result::Result<T, CheckpointError>;

/// Format for checkpoint snapshots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotFormat {
    /// Binary format
    Binary,
    /// JSON format
    Json,
    /// Custom format
    Custom(u32),
}

impl SnapshotFormat {
    /// Get format description
    pub fn description(&self) -> &'static str {
        match self {
            SnapshotFormat::Binary => "Binary",
            SnapshotFormat::Json => "JSON",
            SnapshotFormat::Custom(_) => "Custom",
        }
    }
}

/// Checkpoint containing a task snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint ID
    pub id: u64,
    /// Task that was checkpointed
    pub task_id: u64,
    /// Snapshot format
    pub format: SnapshotFormat,
    /// Snapshot data
    pub data: Vec<u8>,
    /// Version for CRDT
    pub version: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Metadata
    pub metadata: CheckpointMetadata,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(
        id: u64,
        task_id: u64,
        format: SnapshotFormat,
        data: Vec<u8>,
    ) -> Self {
        Self {
            id,
            task_id,
            format,
            data,
            version: 0,
            timestamp: 0,
            metadata: CheckpointMetadata::default(),
        }
    }

    /// Get the checkpoint size
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if checkpoint is valid
    pub fn is_valid(&self) -> bool {
        !self.data.is_empty()
    }
}

/// Checkpoint metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Memory usage at checkpoint time
    pub memory_bytes: usize,
    /// Number of child tasks
    pub child_count: usize,
    /// Capabilities granted
    pub capabilities: u64,
}

/// Checkpoint provider trait
pub trait CheckpointProvider {
    /// Create a checkpoint
    fn checkpoint(&mut self, task_id: u64) -> Result<u64>;

    /// Restore from a checkpoint
    fn restore(&mut self, checkpoint_id: u64) -> Result<()>;

    /// List available checkpoints
    fn list_checkpoints(&self, task_id: u64) -> Result<Vec<u64>>;
}

/// Simple checkpoint manager
#[derive(Debug)]
pub struct CheckpointManager {
    checkpoints: Vec<Checkpoint>,
    next_id: u64,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new() -> Self {
        Self {
            checkpoints: Vec::new(),
            next_id: 1,
        }
    }

    /// Create a checkpoint
    pub fn create_checkpoint(
        &mut self,
        task_id: u64,
        format: SnapshotFormat,
        data: Vec<u8>,
    ) -> Result<u64> {
        let checkpoint = Checkpoint::new(self.next_id, task_id, format, data);
        let id = checkpoint.id;
        self.checkpoints.push(checkpoint);
        self.next_id += 1;
        Ok(id)
    }

    /// Get a checkpoint
    pub fn get_checkpoint(&self, id: u64) -> Result<&Checkpoint> {
        self.checkpoints
            .iter()
            .find(|c| c.id == id)
            .ok_or(CheckpointError::NotFound(id))
    }

    /// List checkpoints for a task
    pub fn list_task_checkpoints(&self, task_id: u64) -> Vec<u64> {
        self.checkpoints
            .iter()
            .filter(|c| c.task_id == task_id)
            .map(|c| c.id)
            .collect()
    }

    /// Delete a checkpoint
    pub fn delete_checkpoint(&mut self, id: u64) -> Result<()> {
        let initial_len = self.checkpoints.len();
        self.checkpoints.retain(|c| c.id != id);

        if self.checkpoints.len() < initial_len {
            Ok(())
        } else {
            Err(CheckpointError::NotFound(id))
        }
    }

    /// Get the number of checkpoints
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /// Get total checkpoint storage used
    pub fn total_size(&self) -> usize {
        self.checkpoints.iter().map(|c| c.size()).sum()
    }
}

impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new()
    }
}

/// CRDT-based state reconciliation
#[derive(Debug, Clone)]
pub struct CrdtState {
    /// Vector clock for causality tracking
    pub clock: Vec<u64>,
    /// Operation log for reconciliation
    pub operations: Vec<alloc::string::String>,
}

impl CrdtState {
    /// Create a new CRDT state
    pub fn new(replicas: usize) -> Self {
        Self {
            clock: vec![0; replicas],
            operations: Vec::new(),
        }
    }

    /// Increment the logical clock for a replica
    pub fn increment(&mut self, replica_id: usize) {
        if replica_id < self.clock.len() {
            self.clock[replica_id] += 1;
        }
    }

    /// Merge with another CRDT state
    pub fn merge(&mut self, other: &CrdtState) {
        for i in 0..core::cmp::min(self.clock.len(), other.clock.len()) {
            self.clock[i] = core::cmp::max(self.clock[i], other.clock[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_creation() {
        let cp = Checkpoint::new(1, 1, SnapshotFormat::Binary, vec![1, 2, 3]);
        assert_eq!(cp.id, 1);
        assert_eq!(cp.size(), 3);
        assert!(cp.is_valid());
    }

    #[test]
    fn test_checkpoint_manager() {
        let mut mgr = CheckpointManager::new();
        let id = mgr
            .create_checkpoint(1, SnapshotFormat::Binary, vec![1, 2, 3])
            .unwrap();

        assert!(mgr.get_checkpoint(id).is_ok());
        assert_eq!(mgr.checkpoint_count(), 1);
    }

    #[test]
    fn test_list_task_checkpoints() {
        let mut mgr = CheckpointManager::new();
        mgr.create_checkpoint(1, SnapshotFormat::Binary, vec![1]).unwrap();
        mgr.create_checkpoint(1, SnapshotFormat::Binary, vec![2]).unwrap();
        mgr.create_checkpoint(2, SnapshotFormat::Binary, vec![3]).unwrap();

        let task1_cps = mgr.list_task_checkpoints(1);
        assert_eq!(task1_cps.len(), 2);

        let task2_cps = mgr.list_task_checkpoints(2);
        assert_eq!(task2_cps.len(), 1);
    }

    #[test]
    fn test_crdt_state() {
        let mut state1 = CrdtState::new(2);
        let mut state2 = CrdtState::new(2);

        state1.increment(0);
        state2.increment(1);

        state1.merge(&state2);

        assert_eq!(state1.clock[0], 1);
        assert_eq!(state1.clock[1], 1);
    }

    #[test]
    fn test_checkpoint_deletion() {
        let mut mgr = CheckpointManager::new();
        let id = mgr
            .create_checkpoint(1, SnapshotFormat::Binary, vec![1])
            .unwrap();

        assert!(mgr.delete_checkpoint(id).is_ok());
        assert!(mgr.get_checkpoint(id).is_err());
    }
}

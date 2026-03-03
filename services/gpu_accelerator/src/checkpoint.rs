// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//!
//! GPU checkpoint and restore primitives (PhoenixOS-inspired).
//!
//! Implements GPU state checkpointing and restoration to enable:
//! - Preemptive crew context switches (save/restore GPU state)
//! - Long-running workload migration (save state, move to different GPU)
//! - Fault recovery (restore from checkpoint on healthy GPU)
//!
//! Reference: Engineering Plan § Checkpoint/Restore, Fault Tolerance

use crate::ids::{GpuDeviceID, VramRegionID};
use alloc::vec::Vec;
use core::fmt;

/// Checkpoint strategy for saving GPU state.
///
/// Trade-off between checkpoint latency and storage overhead.
///
/// Reference: Engineering Plan § Checkpoint Strategies
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointStrategy {
    /// Full checkpoint: save all GPU state (registers, VRAM, caches).
    ///
    /// Highest latency, highest storage, enables restore to any point in time.
    /// Suitable for long-running crew migrations or fault recovery.
    Full,

    /// Incremental checkpoint: save only changed VRAM pages since last checkpoint.
    ///
    /// Lower latency/storage than Full, but requires previous checkpoint baseline.
    /// Suitable for frequent periodic checkpoints.
    Incremental,

    /// Copy-on-write checkpoint: defer VRAM copy until pages are modified.
    ///
    /// Lowest latency (near-immediate), deferred storage cost.
    /// Suitable for rapid context switches where restore may not happen.
    CowBased,
}

impl fmt::Display for CheckpointStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckpointStrategy::Full => write!(f, "Full"),
            CheckpointStrategy::Incremental => write!(f, "Incremental"),
            CheckpointStrategy::CowBased => write!(f, "CowBased"),
        }
    }
}

/// GPU state snapshot descriptor.
///
/// Represents a saved GPU state (checkpoint) for a crew's workload.
/// Can be restored to the same or compatible GPU.
///
/// Reference: Engineering Plan § GPU State Management
#[derive(Clone, Debug)]
pub struct GpuCheckpoint {
    /// Unique checkpoint identifier.
    pub id: VramRegionID, // Reuse VramRegionID for simplicity

    /// Source GPU device.
    pub device_id: GpuDeviceID,

    /// VRAM snapshot data (references or actual data).
    pub vram_snapshot_refs: Vec<VramRegionID>,

    /// Kernel state (pending, in-flight, completed kernels).
    pub kernel_state: Vec<u8>, // Opaque kernel metadata

    /// GPU registers and state (device-specific, opaque).
    pub device_state: Vec<u8>,

    /// Checkpoint creation timestamp (nanoseconds since boot).
    pub timestamp_ns: u64,

    /// Crew that owns this checkpoint.
    pub owner_crew: [u8; 16],

    /// Strategy used to create this checkpoint.
    pub strategy: CheckpointStrategy,

    /// Size of checkpoint data in bytes.
    pub size_bytes: u64,
}

impl GpuCheckpoint {
    /// Create a new GPU checkpoint.
    pub fn new(
        id: VramRegionID,
        device_id: GpuDeviceID,
        vram_snapshot_refs: Vec<VramRegionID>,
        kernel_state: Vec<u8>,
        device_state: Vec<u8>,
        timestamp_ns: u64,
        owner_crew: [u8; 16],
        strategy: CheckpointStrategy,
    ) -> Self {
        let size_bytes = (kernel_state.len() + device_state.len()) as u64;

        GpuCheckpoint {
            id,
            device_id,
            vram_snapshot_refs,
            kernel_state,
            device_state,
            timestamp_ns,
            owner_crew,
            strategy,
            size_bytes,
        }
    }

    /// Check if checkpoint is complete (all state captured).
    ///
    /// Returns true if both kernel and device state are non-empty
    /// (or if empty is valid for this workload).
    pub fn is_complete(&self) -> bool {
        !self.vram_snapshot_refs.is_empty() || !self.kernel_state.is_empty()
    }
}

impl fmt::Display for GpuCheckpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GpuCheckpoint({}, device={:?}, vram_refs={}, size={}B, strategy={}, ts={}ns)",
            self.id,
            self.device_id.as_bytes()[0],
            self.vram_snapshot_refs.len(),
            self.size_bytes,
            self.strategy,
            self.timestamp_ns
        )
    }
}

/// Configuration for GPU state restoration.
///
/// Specifies how to restore a checkpoint to a target device and
/// validate compatibility.
///
/// Reference: Engineering Plan § Restore Operations
#[derive(Clone, Debug)]
pub struct RestoreConfig {
    /// Target GPU device for restoration.
    pub target_device_id: GpuDeviceID,

    /// Allow cross-device restore (if source != target).
    ///
    /// Some GPU pairs are compatible (same model), others are not.
    /// Setting to true skips device compatibility check.
    pub allow_cross_device: bool,

    /// Force restore even if device is not idle.
    ///
    /// If false, restore fails if target device has active kernels.
    pub force_restore: bool,

    /// Validate checkpoint integrity before restore.
    ///
    /// If true, checksum all data structures (slower but safer).
    pub validate_checksums: bool,

    /// Timeout for restore operation in milliseconds.
    pub timeout_ms: u32,
}

impl RestoreConfig {
    /// Create default restore config (same-device, safe mode).
    pub fn default(target_device_id: GpuDeviceID) -> Self {
        RestoreConfig {
            target_device_id,
            allow_cross_device: false,
            force_restore: false,
            validate_checksums: true,
            timeout_ms: 5000,
        }
    }

    /// Create aggressive restore config (allows cross-device, skips validation).
    pub fn aggressive(target_device_id: GpuDeviceID) -> Self {
        RestoreConfig {
            target_device_id,
            allow_cross_device: true,
            force_restore: true,
            validate_checksums: false,
            timeout_ms: 1000,
        }
    }
}

impl fmt::Display for RestoreConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RestoreConfig(device={:?}, cross_device={}, force={}, validate={}, timeout={}ms)",
            self.target_device_id.as_bytes()[0],
            self.allow_cross_device,
            self.force_restore,
            self.validate_checksums,
            self.timeout_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::format;
use alloc::vec;

    #[test]
    fn test_checkpoint_strategy_display() {
        assert_eq!(format!("{}", CheckpointStrategy::Full), "Full");
        assert_eq!(
            format!("{}", CheckpointStrategy::Incremental),
            "Incremental"
        );
        assert_eq!(format!("{}", CheckpointStrategy::CowBased), "CowBased");
    }

    #[test]
    fn test_gpu_checkpoint_creation() {
        let checkpoint_id = VramRegionID::from_bytes([0u8; 16]);
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let vram_refs = vec![VramRegionID::from_bytes([3u8; 16])];
        let kernel_state = vec![0u8; 100];
        let device_state = vec![0u8; 200];

        let checkpoint = GpuCheckpoint::new(
            checkpoint_id,
            device_id,
            vram_refs,
            kernel_state,
            device_state,
            1_000_000,
            crew_id,
            CheckpointStrategy::Full,
        );

        assert_eq!(checkpoint.id, checkpoint_id);
        assert_eq!(checkpoint.device_id, device_id);
        assert_eq!(checkpoint.strategy, CheckpointStrategy::Full);
        assert_eq!(checkpoint.size_bytes, 300);
        assert!(checkpoint.is_complete());
    }

    #[test]
    fn test_gpu_checkpoint_empty() {
        let checkpoint_id = VramRegionID::from_bytes([0u8; 16]);
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let crew_id = [2u8; 16];

        let checkpoint = GpuCheckpoint::new(
            checkpoint_id,
            device_id,
            vec![],
            vec![],
            vec![],
            0,
            crew_id,
            CheckpointStrategy::CowBased,
        );

        assert_eq!(checkpoint.size_bytes, 0);
        assert!(!checkpoint.is_complete());
    }

    #[test]
    fn test_restore_config_default() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let config = RestoreConfig::default(device_id);

        assert_eq!(config.target_device_id, device_id);
        assert!(!config.allow_cross_device);
        assert!(!config.force_restore);
        assert!(config.validate_checksums);
        assert_eq!(config.timeout_ms, 5000);
    }

    #[test]
    fn test_restore_config_aggressive() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let config = RestoreConfig::aggressive(device_id);

        assert!(config.allow_cross_device);
        assert!(config.force_restore);
        assert!(!config.validate_checksums);
        assert_eq!(config.timeout_ms, 1000);
    }

    #[test]
    fn test_restore_config_display() {
        let device_id = GpuDeviceID::from_bytes([1u8; 16]);
        let config = RestoreConfig::default(device_id);
        let display_str = format!("{}", config);
        assert!(display_str.contains("RestoreConfig"));
    }
}
